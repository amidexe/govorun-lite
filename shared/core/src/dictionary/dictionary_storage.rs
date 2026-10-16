use crate::{dictionary_document as document, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    #[serde(default)]
    schema_version: Option<u32>,
    enabled: bool,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    entries: Option<Vec<document::Entry>>,
}

pub(crate) fn read(value: Value) -> Result<Value> {
    let stored: Stored = serde_json::from_value(value).map_err(|_| Error::InvalidRequest)?;
    let raw = match (stored.schema_version, stored.text, stored.entries) {
        (Some(1), Some(raw), _) => raw,
        (None, None, Some(entries)) => document::edit("", entries)?,
        _ => return Err(Error::InvalidRequest),
    };
    write(raw, stored.enabled)
}

/// Preserve portable text exactly, with a validated legacy view for a stable
/// rollback. The older Windows reader requires a non-null `entries` array.
pub(crate) fn write(raw: String, enabled: bool) -> Result<Value> {
    let entries = document::parse(&raw)?;
    Ok(json!({"storage": {"schemaVersion": 1, "enabled": enabled, "text": raw,
        "entries": entries}}))
}

/// One-time import after explicit stable return. The saved portable document
/// preserves comments and whitespace; edits in stable take precedence.
pub(crate) fn read_split(entries: Value, portable: Option<Value>, enabled: bool) -> Result<Value> {
    // The first shared client briefly stored this envelope at the array key.
    // Read it without discarding text; the next write restores the stable array.
    if entries.is_object() {
        let mut envelope = entries;
        envelope["enabled"] = json!(enabled);
        return read(envelope);
    }
    let actual: Vec<document::Entry> = serde_json::from_value(entries)
        .map_err(|_| Error::InvalidRequest)?;
    let raw = if let Some(portable) = portable {
        let saved: Stored = serde_json::from_value(portable).map_err(|_| Error::InvalidRequest)?;
        let (Some(1), Some(raw), Some(expected)) =
            (saved.schema_version, saved.text, saved.entries) else { return Err(Error::InvalidRequest) };
        if document::parse(&raw)? != expected { return Err(Error::InvalidRequest); }
        if actual == expected { raw } else { document::edit(&raw, actual)? }
    } else {
        document::edit("", actual)?
    };
    write(raw, enabled)
}
