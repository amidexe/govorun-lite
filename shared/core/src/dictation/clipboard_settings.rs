//! Clipboard preference policy. Physical paste and revision ownership stay in OS ports.
use crate::{DocumentStore, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::Arc;

pub const KEY: &str = "clipboard.document";

#[derive(Clone, Copy, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Preference { Preserve, Copy }
impl Preference {
    fn name(self) -> &'static str {
        match self { Self::Preserve => "preserve", Self::Copy => "copy" }
    }
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Change { clipboard: Option<Preference>, remember_last: Option<bool> }

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSnapshot {
    pub clipboard: Preference,
    /// Off unless the person turns it on: the last dictation stays in memory for the tray copy.
    pub remember_last: bool,
}

pub struct ClipboardSettings(Arc<dyn DocumentStore>);
impl ClipboardSettings {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self { Self(store) }

    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let change = match method {
            "clipboard.snapshot" if params.as_object().is_some_and(|p| p.is_empty()) => None,
            "clipboard.action" => Some(serde_json::from_value::<Change>(params).ok()
                .filter(|change| change.clipboard.is_some() || change.remember_last.is_some())
                .ok_or("clipboard_invalid_request")?),
            _ => return Err("clipboard_invalid_request"),
        };
        let mut document = self.0.read_document(KEY.into()).await.map_err(code)?
            .ok_or("clipboard_storage_unavailable")?;
        if let Some(change) = change {
            let next = changed(&document.value, &change)?;
            document = self.0.write_document(KEY.into(), Some(document.revision), next.clone()).await.map_err(code)?;
            if document.value != next { return Err("clipboard_storage_invalid"); }
        }
        serde_json::to_value(snapshot(&document.value)?).map_err(|_| "clipboard_storage_invalid")
    }
}
// (macOS UserDefaults key, Windows settings.json field). The paste choice keeps the macOS values.
const PASTE: (&str, &str) = ("paste.clipboard", "pasteClipboard");
const REMEMBER: (&str, &str) = ("lastDictation.remember", "rememberLastDictation");

fn windows_values(document: &Value) -> Result<Map<String, Value>, &'static str> {
    match &document["values"] {
        Value::Null => Ok(Map::new()),
        Value::String(raw) => serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}')).ok()
            .and_then(|value| value.as_object().cloned()).ok_or("clipboard_storage_invalid"),
        _ => Err("clipboard_storage_invalid"),
    }
}
fn mac_values(document: &Value) -> Result<&Map<String, Value>, &'static str> {
    let values = document["values"].as_object().ok_or("clipboard_storage_invalid")?;
    if values.keys().any(|key| !matches!(key.as_str(), "paste.clipboard" | "lastDictation.remember" | "experimentalWindowsAppPaste")) {
        return Err("clipboard_storage_invalid");
    }
    Ok(values)
}
fn checked(document: &Value) -> Result<&str, &'static str> {
    if document.as_object().is_none_or(|v| v.len() != 2) { return Err("clipboard_storage_invalid"); }
    document["source"].as_str().filter(|source| matches!(*source, "macos" | "windows")).ok_or("clipboard_storage_invalid")
}
fn stored(document: &Value, (mac, windows): (&str, &str)) -> Result<Option<Value>, &'static str> {
    Ok(match checked(document)? {
        "macos" => mac_values(document)?.get(mac).cloned(),
        _ => windows_values(document)?.into_iter().find(|(key, _)| key.eq_ignore_ascii_case(windows)).map(|(_, value)| value),
    })
}
fn snapshot(document: &Value) -> Result<ClipboardSnapshot, &'static str> {
    let clipboard = if stored(document, PASTE)?.as_ref().and_then(Value::as_str) == Some("copy") {
        Preference::Copy
    } else {
        Preference::Preserve
    };
    Ok(ClipboardSnapshot { clipboard, remember_last: stored(document, REMEMBER)? == Some(json!(true)) })
}
fn changed(document: &Value, change: &Change) -> Result<Value, &'static str> {
    let mut updates = Vec::new();
    if let Some(preference) = change.clipboard { updates.push((PASTE, json!(preference.name()))); }
    if let Some(remember) = change.remember_last { updates.push((REMEMBER, json!(remember))); }
    match checked(document)? {
        "macos" => {
            let mut values = mac_values(document)?.clone();
            for ((mac, _), value) in updates { values.insert(mac.into(), value); }
            // Existing explicit preference changes retire this obsolete experiment only.
            values.remove("experimentalWindowsAppPaste");
            Ok(json!({"source":"macos","values":values}))
        }
        _ => {
            let mut values = windows_values(document)?;
            for ((_, windows), value) in updates {
                values.retain(|key, _| !key.eq_ignore_ascii_case(windows));
                values.insert(windows.into(), value);
            }
            Ok(json!({"source":"windows","values":serde_json::to_string(&values).map_err(|_| "clipboard_storage_invalid")?}))
        }
    }
}
fn code(error: StoreError) -> &'static str {
    match error {
        StoreError::Conflict => "clipboard_storage_conflict",
        StoreError::Invalid => "clipboard_storage_invalid",
        StoreError::Unavailable => "clipboard_storage_unavailable",
    }
}
