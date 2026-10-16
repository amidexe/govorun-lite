#[path="codec/macos.rs"]
mod macos;
#[path="codec/metadata.rs"]
mod metadata;
#[path="codec/windows.rs"]
mod windows;
use super::{call, StyleDocumentError as Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Source {
    Macos,
    Windows,
}
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(super) struct Physical {
    source: Source,
    values: Value,
}
pub(super) struct Logical {
    pub context: Value,
    pub rules: Value,
    pub options_present: bool,
}
impl Physical {
    pub fn metadata(&self) -> Result<Value, Error> {
        metadata::read(&self.source, &self.values)
    }
    pub fn metadata_action(&mut self, action: Value) -> Result<bool, Error> {
        metadata::apply(&self.source, &mut self.values, action)
    }
    pub fn read(value: Value) -> Result<Self, Error> {
        if value.to_string().len() > 8 * 1_048_576 {
            return Err(Error::InvalidStorage);
        }
        serde_json::from_value(value).map_err(|_| Error::InvalidStorage)
    }
    pub fn logical(&self) -> Result<Logical, Error> {
        let logical = match self.source {
            Source::Macos => macos::read(&self.values),
            Source::Windows => windows::read(&self.values),
        }?;
        crate::style_library::validate_stored_application_rules(&logical.rules)
            .map_err(|_| Error::InvalidStorage)?;
        Ok(logical)
    }
    pub fn set(&mut self, original: &Logical, context: &Value, rules: &Value) -> Result<(), Error> {
        match self.source {
            Source::Macos => macos::write(&mut self.values, original, context, rules),
            Source::Windows => windows::write(&mut self.values, original, context, rules),
        }
    }
    pub fn value(&self) -> Result<Value, Error> {
        serde_json::to_value(self).map_err(|_| Error::InvalidStorage)
    }
}
fn parse(raw: &Value) -> Result<Option<Value>, Error> {
    if raw.is_null() {
        return Ok(None);
    }
    let raw = raw.as_str().ok_or(Error::InvalidStorage)?;
    serde_json::from_str(raw.strip_prefix('\u{feff}').unwrap_or(raw))
        .map(Some)
        .map_err(|_| Error::InvalidStorage)
}
