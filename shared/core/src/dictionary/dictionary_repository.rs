//! One dictionary writer over the existing storage port. No platform paths or I/O.
#[path="dictionary_repository/codec.rs"]
mod codec;
/// In-process dictation uses the same replacement owner as the wire contract.
/// Neither text nor private dictionary comments need to cross an agent pipe.
pub fn apply(raw: &str, text: &str, enabled: bool) -> crate::Result<String> {
    crate::dictionary_replacement::apply(raw, text, enabled)
}
use crate::{DocumentStore, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictionaryDocumentError {
    InvalidAction,
    InvalidStorage,
    StorageUnavailable,
    Conflict,
}
impl From<StoreError> for DictionaryDocumentError {
    fn from(value: StoreError) -> Self {
        match value {
            StoreError::Unavailable => Self::StorageUnavailable,
            StoreError::Invalid => Self::InvalidStorage,
            StoreError::Conflict => Self::Conflict,
        }
    }
}

/// Private configuration, including portable text. Never emit this as a UI event.
pub struct DictionaryDocumentResult {
    pub storage: Value,
    pub revision: String,
    pub changed: bool,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Action {
    Snapshot {},
    SetEnabled { enabled: bool },
    Replace { entries: Vec<crate::dictionary_document::Entry>, enabled: bool },
    ReplaceText { raw: String, enabled: bool },
}

pub struct DictionaryRepository {
    store: Arc<dyn DocumentStore>,
    key: String,
    serial: Mutex<()>,
}
impl DictionaryRepository {
    pub fn new(store: Arc<dyn DocumentStore>, key: String) -> Self {
        Self { store, key, serial: Mutex::new(()) }
    }

    /// A caller editing an observed document supplies its revision. A conflict
    /// is never retried against newer data, including a concurrent sync result.
    pub async fn execute(&self, command: Value, expected_revision: Option<String>)
        -> Result<DictionaryDocumentResult, DictionaryDocumentError> {
        let action: Action = serde_json::from_value(command)
            .map_err(|_| DictionaryDocumentError::InvalidAction)?;
        let _serial = self.serial.lock().await;
        let document = self.store.read_document(self.key.clone()).await?
            .ok_or(DictionaryDocumentError::InvalidStorage)?;
        if document.revision.is_empty() || document.revision.len() > 128 {
            return Err(DictionaryDocumentError::InvalidStorage);
        }
        if expected_revision.as_ref().is_some_and(|expected| expected != &document.revision) {
            return Err(DictionaryDocumentError::Conflict);
        }
        let mut physical = codec::Physical::read(document.value)?;
        let previous = physical.logical()?;
        let raw = previous["text"].as_str().ok_or(DictionaryDocumentError::InvalidStorage)?;
        let next = match action {
            Action::Snapshot {} => previous.clone(),
            Action::SetEnabled { enabled } => prepare(raw.to_owned(), enabled)?,
            Action::Replace { entries, enabled } => {
                let edited = crate::dictionary_document::edit(raw, entries)
                    .map_err(|_| DictionaryDocumentError::InvalidAction)?;
                prepare(edited, enabled)?
            }
            Action::ReplaceText { raw, enabled } => prepare(raw, enabled)?,
        };
        // Reading or repeating an unchanged preference does not import, rewrite,
        // clear a return document, or publish another mutation.
        if next == previous {
            return Ok(DictionaryDocumentResult { storage: next,
                revision: document.revision, changed: false });
        }
        physical.set(&next)?;
        let value = serde_json::to_value(physical)
            .map_err(|_| DictionaryDocumentError::InvalidStorage)?;
        let written = self.store.write_document(self.key.clone(), Some(document.revision), value.clone()).await?;
        if written.value != value || written.revision.is_empty() || written.revision.len() > 128 {
            return Err(DictionaryDocumentError::InvalidStorage);
        }
        Ok(DictionaryDocumentResult { storage: next, revision: written.revision, changed: true })
    }
}

fn prepare(raw: String, enabled: bool) -> Result<Value, DictionaryDocumentError> {
    crate::dictionary_storage::write(raw, enabled)
        .map(|value| value["storage"].clone())
        .map_err(|_| DictionaryDocumentError::InvalidAction)
}

/// A virtual document includes the storage layout even when its physical file
/// or Defaults value is absent. Adapters report unreadable storage separately.
pub fn empty_dictionary_document(split: bool) -> Value {
    json!({"layout":if split {"split"} else {"combined"},
        "stored":null,"portable":null,"enabled":if split {json!(false)} else {Value::Null}})
}
