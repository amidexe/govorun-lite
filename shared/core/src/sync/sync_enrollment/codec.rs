//! Decode current string Defaults and numeric C# enum JSON without migration.
use super::{DataSyncMode, EnrollmentError, EnrollmentSnapshot};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
enum Source {
    Macos,
    Windows,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Raw {
    source: Source,
    stored: Value,
}
pub(super) struct Physical {
    source: Source,
    original: Value,
    choices: Value,
    requested: Value,
}
/// A desktop base recreated without its copy lost every sync choice: each account on
/// this computer is asked once (desktop/src-tauri/src/persistence/document_db.rs).
const ANY_ACCOUNT: &str = "0000000000000000000000000000000000000000000000000000000000000000";
fn parse(raw: &str) -> Result<Value, EnrollmentError> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|_| EnrollmentError::InvalidStorage)
}
fn slot(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn mode(value: &Value, source: Source) -> Result<DataSyncMode, EnrollmentError> {
    match source {
        Source::Macos => {
            serde_json::from_value(value.clone()).map_err(|_| EnrollmentError::InvalidStorage)
        }
        Source::Windows => match value.as_u64() {
            Some(0) => Ok(DataSyncMode::Evaluating),
            Some(1) => Ok(DataSyncMode::Pending),
            Some(2) => Ok(DataSyncMode::Enabled),
            Some(3) => Ok(DataSyncMode::LocalOnly),
            _ => Err(EnrollmentError::InvalidStorage),
        },
    }
}
impl Physical {
    pub fn read(value: Value) -> Result<Self, EnrollmentError> {
        if value.to_string().len() > 8 * 1_048_576 {
            return Err(EnrollmentError::InvalidStorage);
        }
        let raw: Raw =
            serde_json::from_value(value).map_err(|_| EnrollmentError::InvalidStorage)?;
        let (choices, requested) = match raw.source {
            Source::Macos => {
                let object = raw
                    .stored
                    .as_object()
                    .ok_or(EnrollmentError::InvalidStorage)?;
                if object.len() != 2 {
                    return Err(EnrollmentError::InvalidStorage);
                }
                (
                    raw.stored["choices"].clone(),
                    raw.stored["requested"].clone(),
                )
            }
            Source::Windows => {
                let root = if raw.stored.is_null() {
                    json!({})
                } else {
                    parse(raw.stored.as_str().ok_or(EnrollmentError::InvalidStorage)?)?
                };
                let object = root.as_object().ok_or(EnrollmentError::InvalidStorage)?;
                let mut choices = json!({});
                let mut requested = json!({});
                for (key, value) in object {
                    if let Some(key) = key.strip_prefix("requested:") {
                        requested[key] = value.clone();
                    } else {
                        choices[key] = value.clone();
                    }
                }
                (choices, requested)
            }
        };
        for (key, value) in choices.as_object().ok_or(EnrollmentError::InvalidStorage)? {
            if !slot(key) {
                return Err(EnrollmentError::InvalidStorage);
            }
            mode(value, raw.source)?;
        }
        for (key, value) in requested
            .as_object()
            .ok_or(EnrollmentError::InvalidStorage)?
        {
            if !slot(key) {
                return Err(EnrollmentError::InvalidStorage);
            }
            match raw.source {
                Source::Macos => {
                    value.as_bool().ok_or(EnrollmentError::InvalidStorage)?;
                }
                Source::Windows => {
                    mode(value, Source::Windows)?;
                }
            }
        }
        Ok(Self {
            source: raw.source,
            original: raw.stored,
            choices,
            requested,
        })
    }
    pub fn snapshot(&self, slot: &str) -> Result<EnrollmentSnapshot, EnrollmentError> {
        Ok(EnrollmentSnapshot {
            stored: self
                .choices
                .get(slot)
                .map(|v| mode(v, self.source))
                .transpose()?,
            requested: [slot, ANY_ACCOUNT].iter().any(|slot| match self.source {
                Source::Macos => self.requested[*slot].as_bool().unwrap_or(false),
                Source::Windows => self.requested.get(*slot).is_some(),
            }),
            other_account_history: self
                .choices
                .as_object()
                .ok_or(EnrollmentError::InvalidStorage)?
                .keys()
                .any(|key| key != slot),
        })
    }
    pub fn set(
        &mut self,
        slot: &str,
        mode: DataSyncMode,
        requested: bool,
    ) -> Result<(), EnrollmentError> {
        if mode == DataSyncMode::Evaluating {
            return Err(EnrollmentError::InvalidRequest);
        }
        self.choices[slot] = match self.source {
            Source::Macos => {
                serde_json::to_value(mode).map_err(|_| EnrollmentError::InvalidStorage)?
            }
            Source::Windows => json!(match mode {
                DataSyncMode::Pending => 1,
                DataSyncMode::Enabled => 2,
                DataSyncMode::LocalOnly => 3,
                _ => unreachable!(),
            }),
        };
        if requested {
            self.requested[slot] = match self.source {
                Source::Macos => json!(true),
                Source::Windows => json!(1),
            };
        } else if mode != DataSyncMode::Pending {
            let requested = self.requested.as_object_mut().ok_or(EnrollmentError::InvalidStorage)?;
            requested.remove(slot);
            requested.remove(ANY_ACCOUNT);
        }
        Ok(())
    }
    pub fn value(&self) -> Result<Value, EnrollmentError> {
        let stored = match self.source {
            Source::Macos => json!({"choices":self.choices,"requested":self.requested}),
            Source::Windows => {
                let mut root = self.choices.clone();
                for (key, value) in self
                    .requested
                    .as_object()
                    .ok_or(EnrollmentError::InvalidStorage)?
                {
                    root[format!("requested:{key}")] = value.clone();
                }
                if !self.original.is_null()
                    && parse(
                        self.original
                            .as_str()
                            .ok_or(EnrollmentError::InvalidStorage)?,
                    )? == root
                {
                    self.original.clone()
                } else {
                    json!(root.to_string())
                }
            }
        };
        Ok(
            json!({"source":match self.source {Source::Macos=>"macos",Source::Windows=>"windows"},"stored":stored}),
        )
    }
}
