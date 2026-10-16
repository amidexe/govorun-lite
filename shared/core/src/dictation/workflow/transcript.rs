//! Host-owned ephemeral buffer; Core chooses fragment/complete transitions.
//! Current desktop recording defers fragments until stop on both platforms.
use super::{core, Failure};
use serde_json::{json, Value};
pub fn assemble(fragments: Vec<String>, cancelled: impl Fn() -> bool) -> Result<String, Failure> {
    let mut state = Value::Null;
    let mut text = String::new();
    for fragment in fragments {
        if cancelled() {
            return Err(Failure::Cancelled);
        }
        let step = core::evaluate(json!({"kind":"transcript","state":state,"deferred":true,
            "event":{"kind":"fragment","units":fragment.encode_utf16().collect::<Vec<_>>()}}))?;
        state = step["state"].clone();
        match step["action"].as_str() {
            Some("buffer") => {
                if !text.is_empty() {
                    text.push_str(
                        step["separator"]
                            .as_str()
                            .ok_or(Failure::Effect("dictation_invalid_core_result"))?,
                    );
                }
                text.push_str(
                    step["fragment"]
                        .as_str()
                        .ok_or(Failure::Effect("dictation_invalid_core_result"))?,
                );
            }
            Some("ignore") => {}
            _ => return Err(Failure::Effect("dictation_invalid_core_result")),
        }
    }
    if cancelled() {
        return Err(Failure::Cancelled);
    }
    let complete = core::evaluate(
        json!({"kind":"transcript","state":state,"deferred":true,"event":{"kind":"complete"}}),
    )?;
    match complete["action"].as_str() {
        Some("release") => Ok(text),
        Some("ignore") => Ok(String::new()),
        _ => Err(Failure::Effect("dictation_invalid_core_result")),
    }
}
