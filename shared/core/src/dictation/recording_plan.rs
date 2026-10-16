use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

const ONLINE_MILLIS: i64 = 300_000;

/// Device facts distinguish an OS-managed screen-off stop from an explicit timer.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Facts {
    self_hosted: bool,
    official_selected: bool,
    official_ready: bool,
    streaming: bool,
    local_timer_enabled: bool,
    configured_minutes: i64,
}

pub(crate) fn evaluate(facts: Facts) -> Result<Value> {
    let online = !facts.self_hosted && facts.official_selected && facts.official_ready && !facts.streaming;
    let stop = if facts.self_hosted { 0 }
        else if online { ONLINE_MILLIS }
        else if facts.local_timer_enabled {
            facts.configured_minutes.max(0).checked_mul(60_000).ok_or(Error::InvalidRequest)?
        } else { 0 };
    Ok(json!({"stopAfterMillis": stop,
        "warnAfterMillis": if online { Some(ONLINE_MILLIS - 60_000) } else { None }}))
}
