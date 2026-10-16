use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Server observations and content-free device accounting. This view cannot
/// issue credits or authorize a request; the session and server retain that authority.
#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StyleWindowFacts {
    /// Optional for admission-only callers; native display ports supply the
    /// server access DTO. Session-bound commands always replace it themselves.
    #[serde(default)]
    pub presentation_access: Option<Value>,
    pub signed_in: bool,
    #[serde(default = "verified_profile")]
    pub profile_fresh: bool,
    pub entitlement_active: bool,
    pub processing_ready: bool,
    pub free_access_available: bool,
    pub access_type: String,
    pub access_active: bool,
    pub balance_kind: String,
    pub starter_allocated: bool,
    pub daily_limit: u64,
    pub total_limit: u64,
    pub remaining_today: u64,
    pub remaining_total: u64,
    pub usage_accounting: bool,
    pub usage_fresh: bool,
    pub pending_usage_today: u64,
    pub pending_usage_total: u64,
    pub usage_ledger: Option<crate::usage_ledger::Ledger>,
    pub usage_scope: Option<crate::usage_ledger::Scope>,
    pub local_known: bool,
    pub local_bound: bool,
    pub ready_credits: u64,
    pub pending_credits: u64,
    pub issuance_in_flight: bool,
    pub recovery_in_flight: bool,
    pub daily_reset_passed: bool,
    pub reserve_return: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct StyleWindowPresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<crate::style_quota::QuotaPresentation>,
    pub remaining_today: u64,
    pub remaining_total: u64,
    pub starter_locally_exhausted: bool,
    pub starter_limit_exhausted: bool,
    pub usable_starter: bool,
    pub setup_needs_subscription: bool,
    pub availability: &'static str,
    pub processing_readiness: &'static str,
    pub pause_reason: &'static str,
    pub fresh_usage_available: bool,
    pub prepare_on_demand: bool,
    pub prepare_in_background: bool,
}

/// The bound desktop command obtains prices/allowances and access type only
/// from its current server profile. Native ports contribute local observations.
pub(crate) fn bind_profile(f: &mut StyleWindowFacts, profile: &Value) -> Result<()> {
    let access = &profile["styleAccess"];
    f.presentation_access = Some(access.clone());
    let usage = &access["usageBalance"];
    let count = |value: &Value, name: &str| -> Result<u64> {
        match value.get(name) {
            None | Some(Value::Null) => Ok(0),
            Some(number) => number.as_u64().ok_or(Error::InvalidResponse),
        }
    };
    f.access_type = access["type"].as_str().unwrap_or("none").to_owned();
    f.access_active = access["state"] == "active";
    f.balance_kind = access["balanceKind"].as_str().unwrap_or("").to_owned();
    f.starter_allocated = profile["starterGrant"]["state"] == "allocated";
    f.daily_limit = count(access, "dailyLimit")?;
    f.total_limit = count(access, "totalLimit")?;
    f.usage_accounting = usage.is_object();
    let counters = if f.usage_accounting { usage } else { access };
    f.remaining_today = count(counters, "remainingToday")?;
    f.remaining_total = count(counters, "remainingTotal")?;
    f.reserve_return = access["reserveReturnProtocol"] == "v1_receipt_deposit";
    if let Some(ledger) = f.usage_ledger.take() {
        if f.usage_accounting {
            let scope: crate::usage_ledger::Scope = serde_json::from_value(serde_json::json!({
                "scopeId": usage["scopeId"], "day": usage["day"],
                "settlementScopeIds": usage.get("settlementScopeIds").filter(|v| !v.is_null()).cloned().unwrap_or(serde_json::json!([])),
                "dailySettlementScopeIds": usage.get("dailySettlementScopeIds").filter(|v| !v.is_null()).cloned().unwrap_or(serde_json::json!([])),
            })).map_err(|_| Error::InvalidResponse)?;
            let event = crate::usage_ledger::Event::Observe { scope: scope.clone(),
                device_days: serde_json::from_value(usage.get("deviceDays").filter(|v| !v.is_null()).cloned().unwrap_or(serde_json::json!({})))
                    .map_err(|_| Error::InvalidResponse)?,
                device_requests: serde_json::from_value(usage.get("deviceRequests").filter(|v| !v.is_null()).cloned().unwrap_or(serde_json::json!({})))
                    .map_err(|_| Error::InvalidResponse)?, protected_scopes: vec![] };
            // Display the server-confirmed acknowledgement immediately; the
            // durable maintenance operation commits the same transition later.
            f.usage_ledger = Some(crate::usage_ledger::apply(ledger, event)?.ledger);
            f.usage_scope = Some(scope);
        } else {
            f.usage_ledger = Some(ledger);
            f.usage_scope = None;
        }
    }
    Ok(())
}

pub(crate) fn project(mut f: StyleWindowFacts) -> Result<StyleWindowPresentation> {
    if let Some(ledger) = f.usage_ledger.take() {
        let event = if f.usage_accounting {
            crate::usage_ledger::Event::Inspect { scope: f.usage_scope.ok_or(Error::InvalidRequest)? }
        } else { crate::usage_ledger::Event::Validate };
        let value = crate::usage_ledger::apply(ledger, event)?;
        f.pending_usage_today = u64::from(value.outstanding_today.unwrap_or(0));
        f.pending_usage_total = u64::from(value.outstanding_total.unwrap_or(0));
    }
    // Native ports use signed 32/64-bit counters. Reject overflowing wire values.
    if [f.daily_limit, f.total_limit, f.remaining_today, f.remaining_total,
        f.pending_usage_today, f.pending_usage_total, f.ready_credits, f.pending_credits]
        .into_iter().any(|v| v > i64::MAX as u64) { return Err(Error::InvalidRequest); }
    let single = f.balance_kind == "single_grant";
    let bound = f.local_known && f.local_bound;
    let held = if bound { f.ready_credits } else { 0 };
    let empty = bound && held == 0 && f.pending_credits == 0 &&
        !f.issuance_in_flight && !f.recovery_in_flight;
    let remaining_total = if f.usage_accounting {
        f.remaining_total.saturating_sub(f.pending_usage_total)
    } else { f.total_limit.min(f.remaining_total.saturating_add(held)) };
    let remaining_today = if single { remaining_total } else if f.usage_accounting {
        f.remaining_today.saturating_sub(f.pending_usage_today)
    } else { f.daily_limit.min(f.remaining_today.saturating_add(held)) };
    let starter = f.signed_in && f.entitlement_active && f.access_active && f.access_type == "starter";
    let starter_locally_exhausted = starter && single && f.starter_allocated &&
        !f.usage_accounting && empty;
    // Last confirmed usage remains displayable while refreshing or offline.
    // It never grants admission: preparation below still needs fresh authority.
    let observed_usage = f.signed_in && f.access_active && f.presentation_access.as_ref()
        .is_some_and(|access| access["usageBalance"].is_object());
    let observed_zero = observed_usage && (remaining_total == 0 || remaining_today == 0);
    let known = if f.usage_accounting { f.usage_fresh || observed_zero } else { bound };
    let period_empty = known && f.total_limit > 0 && remaining_total == 0;
    let starter_limit_exhausted = starter && (starter_locally_exhausted ||
        f.usage_accounting && period_empty);
    let setup_needs_subscription = f.signed_in && f.profile_fresh && !f.free_access_available &&
        (!f.entitlement_active || period_empty);
    let ready = f.signed_in && f.entitlement_active && f.processing_ready;
    let availability = if observed_zero {
            if remaining_total == 0 { "periodLimit" } else { "dailyLimit" }
        } else if !ready { "unavailable" }
        else if f.usage_accounting {
            if !f.usage_fresh && !observed_usage { "checking" }
            else if remaining_total == 0 { "periodLimit" }
            else if remaining_today == 0 { "dailyLimit" }
            else { "ready" }
        } else if held > 0 || f.remaining_total > 0 && (single || f.remaining_today > 0) {
            "ready"
        } else if !empty { "checking" }
        else if f.remaining_total == 0 { "periodLimit" }
        else if f.daily_reset_passed { "checking" }
        else { "dailyLimit" };
    let fresh_usage_available = f.usage_accounting && f.usage_fresh &&
        remaining_total > 0 && remaining_today > 0;
    let usage_available = f.usage_accounting && (f.usage_fresh || observed_usage) &&
        remaining_total > 0 && remaining_today > 0;
    // an idle/empty reserve can be prepared immediately. Background work
    // and stale observations cannot interrupt previously confirmed access.
    let processing_readiness = if availability == "ready" && bound && held > 0 { "ready" }
        else if availability == "ready" && f.reserve_return && empty && fresh_usage_available { "onDemand" }
        else if matches!(availability, "unavailable" | "dailyLimit" | "periodLimit") { "unavailable" }
        else if f.issuance_in_flight || f.recovery_in_flight { "preparing" }
        else { "needsPreparation" };
    let prepare_on_demand = ready && (!f.usage_accounting || usage_available);
    let prepare_in_background = ready && usage_available;
    let pause_reason = if observed_zero { availability }
        else if !f.signed_in || !f.entitlement_active { "none" }
        else { match availability {
            "ready" => "none", "checking" if !f.usage_accounting => "none",
            "checking" => "refreshing", other => other,
        }};
    let quota = f.presentation_access.as_ref().map(|access| crate::style_quota::project(
        access, f.signed_in, f.entitlement_active || observed_usage,
        f.usage_accounting && !f.usage_fresh && !observed_usage || !f.usage_accounting && !bound && f.remaining_total == 0,
        remaining_today, remaining_total, f.daily_limit, f.total_limit));
    Ok(StyleWindowPresentation { quota, remaining_today, remaining_total, starter_locally_exhausted,
        starter_limit_exhausted, usable_starter: starter && single && !starter_locally_exhausted,
        setup_needs_subscription, availability, processing_readiness, pause_reason, fresh_usage_available,
        prepare_on_demand, prepare_in_background })
}

fn verified_profile() -> bool { true }
