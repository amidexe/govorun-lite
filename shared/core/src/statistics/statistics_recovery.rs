use crate::{statistics_continuity::Previous, statistics_ledger_model::State, Error, Result};
use serde_json::Value;

/// Only persisted input is recoverable. Invalid events and server replies still
/// fail validation; a local read failure never authorizes deleting cloud data.
pub(crate) fn load(raw: Option<Value>, previous: Option<Value>, recover: bool) -> Result<State> {
    let decoded = match raw.as_ref() {
        Some(value) => serde_json::from_value::<State>(value.clone()).map_err(|_| Error::InvalidRequest),
        None => Ok(State::default()),
    };
    let mut state = match decoded.and_then(|state| { state.validate()?; Ok(state) }) {
        Ok(state) => state,
        Err(_) if recover => return Ok(recover_local(raw.as_ref())),
        Err(error) => return Err(error),
    };
    if state.epoch > 0 { return Ok(state); }
    if let Some(value) = previous {
        let mut restored = state.clone();
        let imported = serde_json::from_value::<Previous>(value.clone()).map_err(|_| Error::InvalidRequest)
            .and_then(|previous| crate::statistics_continuity::restore(&mut restored, previous));
        if let Err(error) = imported {
            if recover {
                return Ok(recover_previous(&state, value).unwrap_or_else(|| recover_local(raw.as_ref())));
            }
            return Err(error);
        }
        state = restored;
    }
    Ok(state)
}

fn recover_previous(state: &State, mut value: Value) -> Option<State> {
    value.as_object_mut()?.insert("account".into(), Value::Null);
    for discard_days in [false, true] {
        if discard_days {
            // A readable lifetime total does not depend on a readable daily cache.
            if value.get("total").map_or(true, Value::is_null) { break; }
            value["days"] = serde_json::json!([]);
        }
        let mut restored = state.clone();
        if serde_json::from_value::<Previous>(value.clone()).ok()
            .and_then(|previous| crate::statistics_continuity::restore(&mut restored, previous).ok()).is_some() {
            restored.epoch = 1;
            return Some(restored);
        }
    }
    None
}

fn recover_local(raw: Option<&Value>) -> State {
    let total = raw.and_then(|value| value.get("total"))
        .and_then(|value| serde_json::from_value::<crate::statistics_sync::Total>(value.clone()).ok())
        .filter(|total| total.validate().is_ok());
    let mut state = State { epoch: 1, continuity_imported: true,
        storage_recovered: total.is_none(), total: total.unwrap_or_default(), ..State::default() };
    if let Some(days) = raw.and_then(|value| value.get("days")) {
        state.days = serde_json::from_value(days.clone()).unwrap_or_default();
        if state.validate().is_err() { state.days.clear(); }
    }
    state
}
