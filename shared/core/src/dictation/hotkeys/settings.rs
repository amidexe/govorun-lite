//! Preserve existing platform documents and unrelated fields; one CAS writer is the host.
use super::{Gesture, Target};
use serde_json::{json, Map, Value};

pub const KEY: &str = "hotkeys.document";
pub const MAC_KEYS: [&str; 9] = ["hotkeyKeyCode", "hotkeyModifiers", "hotkeyKind",
    "plainHotkeyKeyCode", "plainHotkeyModifiers", "plainHotkeyKind",
    "cancelHotkeyKeyCode", "cancelHotkeyModifiers", "cancelHotkeyKind"];

pub fn values(document: &Value) -> Result<Map<String, Value>, &'static str> {
    if document.as_object().is_none_or(|v| v.len() != 2) { return Err("hotkeys_storage_invalid"); }
    match document["source"].as_str() {
        Some("macos") => {
            let values = document["values"].as_object().ok_or("hotkeys_storage_invalid")?;
            if values.keys().any(|key| !MAC_KEYS.contains(&key.as_str())) { return Err("hotkeys_storage_invalid"); }
            Ok(values.clone())
        }
        Some("windows") if document["values"].is_null() => Ok(Map::new()),
        Some("windows") => serde_json::from_str::<Value>(document["values"].as_str().ok_or("hotkeys_storage_invalid")?
            .trim_start_matches('\u{feff}')).ok().and_then(|v| v.as_object().cloned()).ok_or("hotkeys_storage_invalid"),
        _ => Err("hotkeys_storage_invalid"),
    }
}
pub fn assign(document: &Value, target: Target, gesture: Option<&Gesture>) -> Result<Value, &'static str> {
    let mut values = values(document)?;
    match document["source"].as_str() {
        Some("macos") => {
            let prefix = match target { Target::Dictation => "hotkey", Target::Plain => "plainHotkey", Target::Cancel => "cancelHotkey" };
            if let Some(gesture) = gesture {
                let raw = gesture.raw.as_object().ok_or("hotkeys_invalid_gesture")?;
                if raw.len() != 3 || gesture.raw["keyCode"].as_u64().is_none_or(|key| key > 65535)
                    || gesture.raw["modifiers"].as_u64().is_none()
                    || !matches!(gesture.raw["kind"].as_str(), Some("key" | "modifierOnly")) {
                    return Err("hotkeys_invalid_gesture");
                }
                for (field, source) in [("KeyCode", "keyCode"), ("Modifiers", "modifiers"), ("Kind", "kind")] {
                    values.insert(format!("{prefix}{field}"), raw[source].clone());
                }
            } else {
                for field in ["KeyCode", "Modifiers", "Kind"] { values.remove(&format!("{prefix}{field}")); }
            }
            Ok(json!({"source":"macos","values":values}))
        }
        Some("windows") => {
            let field = match target { Target::Dictation => "hotkey", Target::Plain => "plainHotkey", Target::Cancel => "cancelHotkey" };
            let matching: Vec<_> = values.keys().filter(|key| key.eq_ignore_ascii_case(field)).cloned().collect();
            for key in matching { values.remove(&key); }
            let raw = if let Some(gesture) = gesture {
                let text = gesture.raw.as_str().filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
                    .ok_or("hotkeys_invalid_gesture")?;
                json!(text)
            } else { Value::Null };
            values.insert(field.into(), raw);
            if target == Target::Dictation {
                let matching: Vec<_> = values.keys().filter(|key| key.eq_ignore_ascii_case("hotkeySetupCompleted")).cloned().collect();
                for key in matching { values.remove(&key); }
                values.insert("hotkeySetupCompleted".into(), json!(true));
            }
            Ok(json!({"source":"windows","values":serde_json::to_string(&values).map_err(|_|"hotkeys_storage_invalid")?}))
        }
        _ => Err("hotkeys_storage_invalid"),
    }
}
pub fn reset(document: &Value, target: Target, gesture: Option<&Gesture>) -> Result<Value, &'static str> {
    assign(document, target, if target == Target::Cancel && document["source"] == "macos" { None } else { gesture })
}
