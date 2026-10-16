use crate::{dictation_plan::MINIMUM, style_text, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const SYMBOL: &str = "🔊";
const SIGNATURE: &str = "🔊 Набрано голосом в „Говоруне“";

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Marker { Off, Microphone, Text, #[serde(rename = "legacy_text")] LegacyText }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    Constants,
    Prepare {
        text: Vec<u16>,
        original: Vec<u16>,
        marker: Marker,
        fallback: bool,
        has_outcome_pass: bool,
        latency_known: bool,
    },
    Stream {
        state: Option<StreamState>,
        marker: Marker,
        event: StreamEvent,
    },
}

/// Counts and flags only. The OS adapter owns any buffered transcript.
#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StreamState {
    characters: usize,
    released: bool,
    flushed: bool,
    suffixed: bool,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum StreamEvent {
    Fragment { units: Vec<u16> },
    Flush { had_text: bool },
    Suffix { had_text: bool },
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::Constants => Ok(json!({"signature": SIGNATURE, "minimumCharacters": MINIMUM})),
        Command::Prepare { text, original, marker, fallback, has_outcome_pass, latency_known } => {
            let text = String::from_utf16(&text).map_err(|_| Error::InvalidRequest)?;
            let characters = style_text::count(&original)?;
            let insert = !text.trim().is_empty();
            let rendered = if insert && (characters >= MINIMUM || marker == Marker::LegacyText) {
                match marker {
                    Marker::Off => text,
                    Marker::Microphone => format!("{SYMBOL} {}", text.trim()),
                    Marker::Text => format!("{}\n{SIGNATURE}", text.trim()),
                    Marker::LegacyText => format!("{}\n[текст распознан с голоса]", text.trim()),
                }
            } else { text };
            let report = has_outcome_pass && latency_known;
            Ok(json!({"text": rendered, "insert": insert,
                "success": {"recordFailure": false, "showFallback": fallback,
                    "outcome": if report { Some("delivered") } else { None }},
                "failure": {"recordFailure": true, "showFallback": true,
                    "outcome": if report { Some("insertion_failed") } else { None }}}))
        }
        Command::Stream { state, marker, event } => stream(state.unwrap_or_default(), marker, event),
    }
}

fn stream(mut state: StreamState, marker: Marker, event: StreamEvent) -> Result<Value> {
    let mut fragment = String::new();
    let mut prefix = "";
    let action = match event {
        StreamEvent::Fragment { units } => {
            if state.flushed { return Err(Error::InvalidRequest); }
            fragment = String::from_utf16(&units).map_err(|_| Error::InvalidRequest)?.trim().to_owned();
            let count = fragment.chars().count();
            if count == 0 { "skip" }
            else {
                state.characters = state.characters.checked_add(count + usize::from(state.characters > 0))
                    .ok_or(Error::InvalidRequest)?;
                if marker != Marker::Microphone || state.released { "insert" }
                else if state.characters < MINIMUM { "buffer" }
                else {
                    state.released = true;
                    prefix = "🔊 ";
                    "release"
                }
            }
        }
        StreamEvent::Flush { had_text } => {
            let release = !state.flushed && had_text && marker == Marker::Microphone
                && !state.released && state.characters > 0;
            state.flushed = true;
            if release { "release" } else { "skip" }
        }
        StreamEvent::Suffix { had_text } => {
            if !state.suffixed && had_text && (marker == Marker::LegacyText || marker == Marker::Text && state.characters >= MINIMUM) {
                state.suffixed = true;
                fragment = if marker == Marker::LegacyText { "\n[текст распознан с голоса]".into() } else { format!("\n{SIGNATURE}") };
                "insert"
            } else { "skip" }
        }
    };
    Ok(json!({"state": state, "action": action, "fragment": fragment, "prefix": prefix}))
}
