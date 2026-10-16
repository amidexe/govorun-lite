//! Retain the current native envelopes and every other account's raw entry.
use super::{JournalError, JournalKind};
use serde::Deserialize;
use serde_json::{json, Value};

pub(super) struct Entry {
    pub revision: u64,
    pub state: Value,
    pub pending: bool,
    pub intents: Vec<Value>,
}
impl Default for Entry {
    fn default() -> Self {
        Self {
            revision: 0,
            state: Value::Null,
            pending: false,
            intents: vec![],
        }
    }
}
pub(super) struct Physical {
    source: Source,
    kind: JournalKind,
    original: Value,
    entries: Value,
    mutation_revision: Option<u64>,
}
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
    #[serde(rename = "mutationRevision")]
    mutation_revision: Option<u64>,
}
fn invalid<T>() -> Result<T, JournalError> {
    Err(JournalError::InvalidStorage)
}
fn number(value: &Value) -> Result<u64, JournalError> {
    value
        .as_u64()
        .filter(|n| *n <= i64::MAX as u64)
        .ok_or(JournalError::InvalidStorage)
}
fn parse(raw: &str) -> Result<Value, JournalError> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|_| JournalError::InvalidStorage)
}
impl Physical {
    pub fn read(value: Value, kind: JournalKind) -> Result<Self, JournalError> {
        if value.to_string().len() > 8 * 1024 * 1024 {
            return invalid();
        }
        let has_clock = value.get("mutationRevision").is_some();
        let raw: Raw = serde_json::from_value(value).map_err(|_| JournalError::InvalidStorage)?;
        if has_clock != (matches!(raw.source, Source::Macos) && kind == JournalKind::Content)
            || raw.mutation_revision.is_some_and(|n| n > i64::MAX as u64)
        {
            return invalid();
        }
        let entries = match raw.source {
            Source::Macos => {
                if !raw.stored.is_object()
                    || raw
                        .stored
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|v| !v.is_string())
                {
                    return invalid();
                }
                raw.stored.clone()
            }
            Source::Windows => {
                let decoded = if raw.stored.is_null() {
                    if kind == JournalKind::Content {
                        json!({"schemaVersion":1,"revision":0,"accounts":{}})
                    } else {
                        json!({})
                    }
                } else {
                    parse(raw.stored.as_str().ok_or(JournalError::InvalidStorage)?)?
                };
                if !decoded.is_object() {
                    return invalid();
                }
                if kind == JournalKind::Content {
                    if decoded["schemaVersion"] != 1 || !decoded["accounts"].is_object() {
                        return invalid();
                    }
                    number(&decoded["revision"])?;
                }
                decoded
            }
        };
        let physical = Self {
            source: raw.source,
            kind,
            original: raw.stored,
            entries,
            mutation_revision: raw.mutation_revision,
        };
        if matches!(physical.source, Source::Windows) {
            let entries = if kind == JournalKind::Content {
                &physical.entries["accounts"]
            } else {
                &physical.entries
            };
            for key in entries
                .as_object()
                .ok_or(JournalError::InvalidStorage)?
                .keys()
            {
                physical.entry(key)?;
            }
        }
        Ok(physical)
    }
    pub fn entry(&self, account: &str) -> Result<(Entry, u64, bool), JournalError> {
        let raw = match (self.source, self.kind) {
            (Source::Macos, _) => self
                .entries
                .get(account)
                .map(|raw| parse(raw.as_str().ok_or(JournalError::InvalidStorage)?))
                .transpose()?,
            (Source::Windows, JournalKind::Content) => {
                self.entries["accounts"].get(account).cloned()
            }
            (Source::Windows, JournalKind::Preferences) => self.entries.get(account).cloned(),
        };
        let history = raw.is_some();
        let entry = if let Some(raw) = raw {
            if !raw.is_object()
                || (matches!(self.source, Source::Macos) || self.kind == JournalKind::Preferences)
                    && raw["schemaVersion"] != 1
            {
                return invalid();
            }
            let revision = number(&raw["revision"])?;
            if matches!(self.source, Source::Windows)
                && self.kind == JournalKind::Preferences
                && revision > i32::MAX as u64
            {
                return invalid();
            }
            let state = raw
                .get("state")
                .ok_or(JournalError::InvalidStorage)?
                .clone();
            if !state.is_null() && !state.is_object() {
                return invalid();
            }
            let pending = raw["hasPending"]
                .as_bool()
                .ok_or(JournalError::InvalidStorage)?;
            let intents = if self.kind == JournalKind::Preferences {
                raw["intents"]
                    .as_array()
                    .filter(|v| v.iter().all(Value::is_object))
                    .ok_or(JournalError::InvalidStorage)?
                    .clone()
            } else {
                vec![]
            };
            Entry {
                revision,
                state,
                pending,
                intents,
            }
        } else {
            Entry::default()
        };
        let revision =
            if matches!(self.source, Source::Windows) && self.kind == JournalKind::Content {
                let revision = number(&self.entries["revision"])?;
                if entry.revision > revision {
                    return invalid();
                }
                revision
            } else if matches!(self.source, Source::Macos) && self.kind == JournalKind::Content {
                self.mutation_revision.unwrap_or(0)
            } else {
                entry.revision
            };
        Ok((entry, revision, history))
    }
    pub fn increment(&self, value: u64) -> Result<u64, JournalError> {
        let maximum =
            if matches!(self.source, Source::Windows) && self.kind == JournalKind::Preferences {
                i32::MAX as u64
            } else {
                i64::MAX as u64
            };
        value
            .checked_add(1)
            .filter(|v| *v <= maximum)
            .ok_or(JournalError::InvalidStorage)
    }
    pub fn set_entry(&mut self, account: &str, entry: &Entry) -> Result<(), JournalError> {
        // Preserve unknown fields in a current envelope; never rewrite neighbours.
        let mut raw = match (self.source, self.kind) {
            (Source::Macos, _) => self
                .entries
                .get(account)
                .map(|v| parse(v.as_str().ok_or(JournalError::InvalidStorage)?))
                .transpose()?
                .unwrap_or(json!({})),
            (Source::Windows, JournalKind::Content) => self.entries["accounts"]
                .get(account)
                .cloned()
                .unwrap_or(json!({})),
            (Source::Windows, JournalKind::Preferences) => {
                self.entries.get(account).cloned().unwrap_or(json!({}))
            }
        };
        let before = raw.clone();
        if matches!(self.source, Source::Macos) || self.kind == JournalKind::Preferences {
            raw["schemaVersion"] = json!(1);
        }
        raw["revision"] = json!(entry.revision);
        raw["state"] = entry.state.clone();
        raw["hasPending"] = json!(entry.pending);
        if self.kind == JournalKind::Preferences {
            raw["intents"] = json!(entry.intents);
        }
        if raw == before {
            return Ok(());
        }
        match (self.source, self.kind) {
            (Source::Macos, _) => self.entries[account] = json!(raw.to_string()),
            (Source::Windows, JournalKind::Content) => self.entries["accounts"][account] = raw,
            (Source::Windows, JournalKind::Preferences) => self.entries[account] = raw,
        }
        Ok(())
    }
    pub fn mark_content_changed(&mut self) -> Result<bool, JournalError> {
        match self.source {
            Source::Windows => {
                self.entries["revision"] =
                    json!(self.increment(number(&self.entries["revision"])?)?)
            }
            Source::Macos => {
                self.mutation_revision = Some(self.increment(self.mutation_revision.unwrap_or(0))?);
            }
        }
        Ok(true)
    }
    pub fn value(&self) -> Result<Value, JournalError> {
        let stored = match self.source {
            Source::Macos => self.entries.clone(),
            Source::Windows => {
                // A read-only plan does not turn a missing/unchanged file into a new file.
                if !self.original.is_null()
                    && parse(self.original.as_str().ok_or(JournalError::InvalidStorage)?)?
                        == self.entries
                {
                    self.original.clone()
                } else {
                    json!(self.entries.to_string())
                }
            }
        };
        let mut value = json!({"source":match self.source {Source::Macos=>"macos",Source::Windows=>"windows"},"stored":stored});
        if matches!(self.source, Source::Macos) && self.kind == JournalKind::Content {
            value["mutationRevision"] = json!(self.mutation_revision);
        }
        Ok(value)
    }
}
