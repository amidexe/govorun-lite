//! Clipboard preference policy. Physical paste and revision ownership stay in OS ports.
use crate::{DocumentStore, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
#[serde(deny_unknown_fields)]
struct Change { clipboard: Preference }

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClipboardSnapshot { pub clipboard: Preference }

pub struct ClipboardSettings(Arc<dyn DocumentStore>);
impl ClipboardSettings {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self { Self(store) }

    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let change = match method {
            "clipboard.snapshot" if params.as_object().is_some_and(|p| p.is_empty()) => None,
            "clipboard.action" => Some(serde_json::from_value::<Change>(params)
                .map_err(|_| "clipboard_invalid_request")?),
            _ => return Err("clipboard_invalid_request"),
        };
        let mut document = self.0.read_document(KEY.into()).await.map_err(code)?
            .ok_or("clipboard_storage_unavailable")?;
        if document.value["source"] != "macos" || document.value.as_object().is_none_or(|v| v.len() != 2) {
            return Err("clipboard_storage_invalid");
        }
        let values = document.value["values"].as_object_mut().ok_or("clipboard_storage_invalid")?;
        if values.keys().any(|key| !matches!(key.as_str(), "paste.clipboard" | "experimentalWindowsAppPaste")) {
            return Err("clipboard_storage_invalid");
        }
        if let Some(change) = change {
            values.insert("paste.clipboard".into(), json!(change.clipboard.name()));
            // Existing explicit preference changes retire this obsolete experiment only.
            values.remove("experimentalWindowsAppPaste");
            let next = document.value.clone();
            document = self.0.write_document(KEY.into(), Some(document.revision), next.clone()).await.map_err(code)?;
            if document.value != next { return Err("clipboard_storage_invalid"); }
        }
        let preference = match document.value["values"]["paste.clipboard"].as_str() {
            Some("copy") => Preference::Copy,
            _ => Preference::Preserve,
        };
        serde_json::to_value(ClipboardSnapshot { clipboard: preference }).map_err(|_| "clipboard_storage_invalid")
    }
}
fn code(error: StoreError) -> &'static str {
    match error {
        StoreError::Conflict => "clipboard_storage_conflict",
        StoreError::Invalid => "clipboard_storage_invalid",
        StoreError::Unavailable => "clipboard_storage_unavailable",
    }
}
