//! Current dictionary representations; no migration or new storage identities.
use super::DictionaryDocumentError as Error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Layout { Split, Combined }

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(super) struct Physical {
    layout: Layout,
    stored: Option<String>,
    portable: Option<String>,
    enabled: Option<bool>,
}
impl Physical {
    pub(super) fn read(value: Value) -> Result<Self, Error> {
        let value: Self = serde_json::from_value(value).map_err(|_| Error::InvalidStorage)?;
        if value.stored.as_ref().is_some_and(|raw| raw.len() > 2 * 1_048_576)
            || value.portable.as_ref().is_some_and(|raw| raw.len() > 2 * 1_048_576)
            || matches!(value.layout, Layout::Split) && value.enabled.is_none()
            || matches!(value.layout, Layout::Combined) && value.enabled.is_some() {
            return Err(Error::InvalidStorage);
        }
        Ok(value)
    }

    pub(super) fn logical(&self) -> Result<Value, Error> {
        let stored = self.stored.as_deref().map(parse).transpose()?;
        let result = match self.layout {
            Layout::Split => {
                // The split reader validates its return value on every read,
                // matching the existing Defaults adapter's corruption behavior.
                let portable = self.portable.as_deref().map(parse).transpose()?;
                let enabled = self.enabled.ok_or(Error::InvalidStorage)?;
                match stored {
                    Some(mut object) if object.is_object() => {
                        object["enabled"] = json!(enabled);
                        crate::dictionary_storage::read(object)
                    }
                    entries => crate::dictionary_storage::read_split(
                        entries.unwrap_or_else(|| json!([])), portable, enabled),
                }
            }
            Layout::Combined => match stored {
                None => crate::dictionary_storage::write(String::new(), false),
                Some(object) if object.get("schemaVersion").is_none() && self.portable.is_some() => {
                    let enabled = object["enabled"].as_bool().ok_or(Error::InvalidStorage)?;
                    crate::dictionary_storage::read_split(object["entries"].clone(),
                        Some(parse(self.portable.as_deref().ok_or(Error::InvalidStorage)?)?), enabled)
                }
                Some(object) => crate::dictionary_storage::read(object),
            },
        }.map_err(|_| Error::InvalidStorage)?;
        Ok(result["storage"].clone())
    }

    pub(super) fn set(&mut self, logical: &Value) -> Result<(), Error> {
        self.stored = Some(serde_json::to_string(logical).map_err(|_| Error::InvalidStorage)?);
        if matches!(self.layout, Layout::Split) {
            self.enabled = Some(logical["enabled"].as_bool().ok_or(Error::InvalidStorage)?);
        }
        // retain the existing return copy. It is not an additional writer
        // and cannot supersede a current schemaVersion=1 document.
        Ok(())
    }
}

fn parse(raw: &str) -> Result<Value, Error> {
    serde_json::from_str(raw.strip_prefix('\u{feff}').unwrap_or(raw))
        .map_err(|_| Error::InvalidStorage)
}
