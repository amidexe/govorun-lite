use crate::{style_window::{project, StyleWindowFacts}, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Phase { Start, Refreshed, Join, Maintenance, Automatic }

/// Device observations only. A bound session supplies profile freshness itself.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Context {
    pub phase: Phase,
    pub enabled: bool,
    pub custom_server: bool,
    pub network_available: bool,
    pub profile_fresh: bool,
    pub refresh_requested: bool,
    pub required_credits: u64,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Instruction {
    pub action: &'static str,
    pub maximum_join_milliseconds: u64,
}

/// Recording may start maintenance; processing never joins it. No text or pass
/// material enters this decision, and a late maintenance result cannot replay text.
pub(crate) fn plan(facts: StyleWindowFacts, context: Context) -> Result<Instruction> {
    if !(1..=crate::style_inventory::BATCH as u64).contains(&context.required_credits) { return Err(Error::InvalidRequest); }
    let eligible = facts.signed_in && context.enabled && !context.custom_server && context.network_available;
    let processing_ready = facts.processing_ready;
    let fresh = context.profile_fresh && (!facts.usage_accounting || facts.usage_fresh);
    let local_ready = facts.local_known && facts.local_bound && facts.ready_credits >= context.required_credits;
    let window = project(facts)?;
    let refresh_required = !processing_ready || !fresh || window.remaining_today == 0 || window.remaining_total == 0;
    let action = if !eligible { "none" } else { match context.phase {
        // A published, current profile can prepare the user's saved selection
        // without a click. It cannot start a profile polling loop or refill an
        // already usable reserve merely because the window was opened.
        Phase::Automatic if processing_ready && fresh && !local_ready
            && window.remaining_today > 0 && window.remaining_total > 0 => "prepareReserve",
        Phase::Automatic => "none",
        Phase::Start if context.refresh_requested || refresh_required => "refreshProfile",
        Phase::Start => "prepareReserve",
        Phase::Refreshed if processing_ready => "prepareReserve",
        Phase::Refreshed => "none",
        Phase::Join => "none",
        // A restored profile or expired usage observation cannot establish
        // exhaustion. Refresh it before refill; a fresh, confirmed zero still
        // must not create a profile polling loop.
        Phase::Maintenance if context.refresh_requested || !fresh => "refreshProfile",
        Phase::Maintenance if processing_ready => "prepareReserve",
        Phase::Maintenance => "none",
    }};
    Ok(Instruction {
        action,
        maximum_join_milliseconds: 0,
    })
}
