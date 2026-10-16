use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Facts {
    previous_identity_key: String,
    previous_scope_key: String,
    incoming_identity_key: Option<String>,
    incoming_scope_key: Option<String>,
    replace_identity: bool,
    invalidate_scope: bool,
}

/// Reconcile the persisted binding with the already-authoritative account
/// view. An unknown old identity is never imported into a new account.
pub(crate) fn decide(facts: Facts) -> Value {
    let identity_changed = facts.incoming_identity_key.as_ref()
        .is_some_and(|key| key != &facts.previous_identity_key);
    let scope_changed = facts.incoming_scope_key.as_ref()
        .is_some_and(|key| key != &facts.previous_scope_key);
    let action = if facts.replace_identity || identity_changed { "clear" }
        else if facts.invalidate_scope || scope_changed { "retire" }
        else { "preserve" };
    json!({"action":action,
        "identityKey":facts.incoming_identity_key.unwrap_or(facts.previous_identity_key),
        "scopeKey":facts.incoming_scope_key.unwrap_or(facts.previous_scope_key)})
}
