use crate::{style_window::{project, StyleWindowFacts}, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BATCH: u64 = crate::style_inventory::BATCH as u64;
pub(crate) const TARGET: u64 = crate::credit_limits::RESERVE_REQUESTS as u64 * BATCH;
// Fit the full 20-minute jitter and bounded background observation before
// the one-hour lead to withdrawal.
pub(crate) const PREFILL_LEAD_MILLIS:u64 = crate::style_inventory::RETIREMENT_WINDOW + 82*60*1_000;
fn default_target() -> u64 { TARGET }
fn supplied_counters() -> bool { true }

/// Issuance allowance is distinct from the user's visible usage balance.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Context {
    #[serde(default="default_target")]
    pub reserve_credits: u64,
    pub issuance_remaining_today: u64,
    pub issuance_remaining_total: u64,
    pub replacement_receipts: u64,
    pub pending_issuance_credits: u64,
    pub retiring_credits: u64,
    pub explicit_demand: bool,
    pub background_allowed: bool,
    // Pure commands require both counters. A bound server profile may omit
    // them; treating omission conservatively as zero must not label it a
    // confirmed capacity denial.
    #[serde(default = "supplied_counters")]
    pub issuance_counters_observed: bool,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Plan {
    pub batch_credits: u64,
    pub target_ready_credits: u64,
    pub maintenance_required: bool,
    pub refresh_profile: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<&'static str>,
}

pub(crate) fn bind_profile(context: &mut Context, profile: &Value) -> Result<()> {
    context.issuance_counters_observed = ["remainingToday", "remainingTotal"].iter()
        .all(|name| profile["styleAccess"][name].as_u64().is_some());
    let count = |name| match profile["styleAccess"].get(name) {
        None | Some(Value::Null) => Ok(0),
        Some(number) => number.as_u64().ok_or(Error::InvalidResponse),
    };
    context.reserve_credits = match profile["styleAccess"].get("reserveCredits") {
        None | Some(Value::Null) => TARGET,
        Some(number) => number.as_u64().filter(|v| *v>=BATCH && *v<=TARGET && v.is_multiple_of(BATCH))
            .ok_or(Error::InvalidResponse)?,
    };
    context.issuance_remaining_today = count("remainingToday")?;
    context.issuance_remaining_total = count("remainingTotal")?;
    Ok(())
}

/// Content-independent preparation. A durable journal always replays its exact
/// batch; replacement receipts do not create new consumption allowance.
pub(crate) fn plan(facts: StyleWindowFacts, context: Context) -> Result<Plan> {
    let protected_ready=facts.ready_credits;
    plan_for_reserve(facts,context,protected_ready)
}

/// Live stored pass state and admission keep every usable pass. Only the reserve target
/// discounts the cohort approaching withdrawal, before any token is removed.
pub(crate) fn plan_for_reserve(facts: StyleWindowFacts, context: Context, protected_ready:u64) -> Result<Plan> {
    if protected_ready>facts.ready_credits {return Err(Error::InvalidRequest);}
    if [context.issuance_remaining_today, context.issuance_remaining_total,
        context.replacement_receipts, context.retiring_credits].into_iter().any(|v| v > i64::MAX as u64)
        || context.pending_issuance_credits > BATCH { return Err(Error::InvalidRequest); }
    let target=context.reserve_credits;
    if target<BATCH || target>TARGET || !target.is_multiple_of(BATCH) {return Err(Error::InvalidRequest);}
    let ready = facts.ready_credits;
    let eligible = facts.signed_in && facts.entitlement_active && facts.processing_ready;
    let empty = facts.local_known && facts.local_bound && ready == 0 && facts.pending_credits == 0
        && !facts.issuance_in_flight && !facts.recovery_in_flight;
    let can_deposit=facts.reserve_return;
    let window = project(facts)?;
    let fresh = if context.explicit_demand { window.prepare_on_demand }
        else { context.background_allowed && window.prepare_in_background };
    let replacements = context.replacement_receipts.min(BATCH);
    let remaining = context.issuance_remaining_today.min(context.issuance_remaining_total);
    let batch = if !eligible { 0 } else if context.pending_issuance_credits > 0 {
        context.pending_issuance_credits
    } else {
        // New batches have a fixed size, including near the reserve target.
        // A short refill must not disclose the previous text's credit cost.
        // Keep insufficient receipts/capacity for later; never enlarge a
        // durable pending journal or bypass the server's issuance allowance.
        let capacity = replacements.saturating_add(if fresh { remaining } else { 0 });
        if (replacements > 0 || fresh && protected_ready < target) && capacity >= BATCH { BATCH } else { 0 }
    };
    // Partial receipts without a deposit endpoint cannot form a fixed batch.
    // Keep them durably for a later allowance instead of cycling without I/O.
    // A fresh observation with no issuable batch is a completed decision, not
    // work to refresh again. Native profile publication may enqueue maintenance;
    // requiring another refresh here would feed the same observation back into
    // itself indefinitely. Durable replay, retirement and replacements still win.
    let maintenance = eligible && (context.pending_issuance_credits > 0 || context.retiring_credits > 0
        || can_deposit && replacements > 0 || batch > 0);
    // A positive successful-use balance is not proof that the anonymous issuer
    // can mint another pass. Preserve that distinction without bypassing the
    // server ceiling or blocking durable replay/replacement/recovery work.
    let blocked_reason = (eligible && context.explicit_demand && context.issuance_counters_observed
        && window.fresh_usage_available
        && empty && remaining == 0 && batch == 0 && context.retiring_credits == 0
        && context.replacement_receipts == 0)
        .then_some("issuanceCapacityUnavailable");
    Ok(Plan {
        batch_credits: batch,
        target_ready_credits: target.min(ready.saturating_add(batch)),
        maintenance_required: maintenance,
        refresh_profile: maintenance && context.pending_issuance_credits == 0 && replacements == 0 && remaining == 0,
        blocked_reason,
    })
}
