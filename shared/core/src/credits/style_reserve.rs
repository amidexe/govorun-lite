use crate::{Error, Result};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub(crate) struct Projection {
    snapshot: Snapshot,
    profile: Option<Value>,
}

impl Projection {
    pub(crate) fn decode(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(|_| Error::InvalidRequest)
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    session_generation: u64,
    signed_in: bool,
    profile_fresh: bool,
    storage_available: bool,
    logout_pending: bool,
    authority: Authority,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct Authority { authorized: bool }

/// A decision about the last successfully applied native view. Callers
/// must retain that view until its storage effect has committed. Retrying
/// the same pair after a failed write therefore yields the same decision.
/// Unavailable storage and stale profiles preserve recoverable accounting.
pub(crate) fn transition(previous: Option<Projection>, current: Projection) -> Result<Value> {
    let state = &current.snapshot;
    let identity_changed = previous.as_ref().is_some_and(|old|
        old.profile.as_ref().zip(current.profile.as_ref()).is_some_and(|(a, b)|
            a["id"].as_str().zip(b["id"].as_str()).is_some_and(|(a, b)| a != b)));
    let action = if !state.storage_available || state.logout_pending {
        "preserve"
    } else if !state.signed_in {
        if previous.as_ref().is_some_and(|old| old.snapshot.signed_in || old.snapshot.session_generation != state.session_generation) { "retire" } else { "preserve" }
    } else if identity_changed {
        "clear"
    } else if !state.profile_fresh {
        "preserve"
    } else if !state.authority.authorized || previous.as_ref().is_some_and(|old| {
        old.snapshot.signed_in && old.profile.as_ref().zip(current.profile.as_ref())
            .is_some_and(|(before, after)| scope(before) != scope(after))
    }) {
        "retire"
    } else {
        "preserve"
    };
    let profile = current.profile.as_ref().filter(|_| state.signed_in);
    Ok(json!({"action":action,
        "scopeKey":profile.map(|p| format!("reserve-v2:{:x}",
            Sha256::digest(serde_json::to_vec(&scope(p)).expect("scope JSON")))),
        "identityKey":profile.and_then(|p| p["id"].as_str())}))
}

/// Ordinary profile/usage publications do not invalidate a local transaction.
/// Identity, storage, consent, entitlement and confirmed denial still fence it.
pub(crate) fn same_binding(first: &Value, second: &Value) -> bool {
    ["sessionGeneration", "signedIn", "storageAvailable", "logoutPending"].iter()
        .all(|key| first["snapshot"][key] == second["snapshot"][key])
        && first["snapshot"]["access"]["processingReady"] == second["snapshot"]["access"]["processingReady"]
        && first["profile"]["id"] == second["profile"]["id"]
        && scope(&first["profile"]) == scope(&second["profile"])
}

/// Only entitlement, consent and protocol facts identify the reserve scope.
/// Server usage counters, reset clocks and service availability do not.
pub(crate) fn same_access(first: &Value, second: &Value) -> bool {
    access_scope(first) == access_scope(second)
}

fn scope(profile: &Value) -> Value {
    json!([access_scope(&profile["styleAccess"]), profile["onlineTermsVersion"],
        profile["onlineConsent"]["acceptedVersion"], profile["onlineConsent"]["active"]])
}

fn access_scope(access: &Value) -> Value {
    let fields: Vec<Value> = ["type", "state", "planCode", "balanceKind", "dailyLimit", "totalLimit",
        "quotaUnit", "charactersPerCredit", "maximumCreditsPerRequest", "stylizationProtocol"]
        .iter().map(|key| access[key].clone()).collect();
    json!([fields, instant(&access["startsAt"]), instant(&access["endsAt"])])
}

fn instant(value: &Value) -> Value {
    value.as_str().and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|time| json!([time.timestamp(), time.timestamp_subsec_nanos()]))
        .unwrap_or_else(|| value.clone())
}
