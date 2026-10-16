use serde::Deserialize;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Phase { Begin, Reconcile, Refresh, Warm }

/// Native facts and the last instructed I/O step. The core owns sequencing,
/// coalescing and the single follow-up budget; adapters only execute actions.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Facts {
    phase: Phase,
    completed: bool,
    fresh_demand_pending: bool,
    observation_changed: bool,
    followup_used: bool,
    can_warm: bool,
    allowed: bool,
}

pub(crate) fn action(facts: Facts) -> &'static str {
    if !facts.allowed { return "retainForLaterTrigger"; }
    match facts.phase {
        Phase::Begin => return "reconcile",
        Phase::Reconcile if facts.completed => return "refresh",
        Phase::Refresh if facts.completed && facts.can_warm => return "warm",
        _ => {}
    }
    if facts.completed && !facts.observation_changed {
        "finishSatisfied"
    } else if !facts.followup_used && (facts.fresh_demand_pending || facts.observation_changed) {
        "runSequentialFollowUp"
    } else {
        "retainForLaterTrigger"
    }
}
