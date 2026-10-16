use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    closed: bool,
    fragment_count: usize,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Fragment { units: Vec<u16> },
    Complete,
    Discard,
}

/// The continuation contains no text; native adapters own ephemeral buffers.
pub(crate) fn transition(state: Option<State>, deferred: bool, event: Event) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    let mut fragment = String::new();
    let action = match event {
        Event::Discard => { state.closed = true; state.fragment_count = 0; "discard" }
        _ if state.closed => "ignore",
        Event::Fragment { units } => {
            fragment = String::from_utf16(&units).map_err(|_| Error::InvalidRequest)?.trim().to_owned();
            if fragment.is_empty() { "ignore" }
            else {
                state.fragment_count = state.fragment_count.checked_add(1).ok_or(Error::InvalidRequest)?;
                if deferred { "buffer" } else { "insert" }
            }
        }
        Event::Complete => {
            state.closed = true;
            if deferred && state.fragment_count > 0 { "release" } else { "ignore" }
        }
    };
    Ok(json!({"state": state, "action": action, "fragment": fragment, "separator": " "}))
}
