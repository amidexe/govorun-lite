//! Current content/preference journals; decisions and receipts belong to Rust.
//! The host serializes local application and this commit under its mutation gate.
#[path="sync_journal/codec.rs"]
mod codec;
use crate::{DocumentStore, StoreError};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalKind {
    Content,
    Preferences,
}
impl JournalKind {
    fn key(self) -> &'static str {
        match self {
            Self::Content => "sync.content",
            Self::Preferences => "sync.preferences",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalError {
    InvalidRequest,
    InvalidStorage,
    Unavailable,
    Conflict,
}
impl From<StoreError> for JournalError {
    fn from(value: StoreError) -> Self {
        match value {
            StoreError::Invalid => Self::InvalidStorage,
            StoreError::Unavailable => Self::Unavailable,
            StoreError::Conflict => Self::Conflict,
        }
    }
}
pub struct Capture {
    kind: JournalKind,
    account: String,
    storage_revision: String,
    physical: codec::Physical,
    entry: codec::Entry,
    revision: u64,
    history: bool,
}
impl Capture {
    pub fn has_history(&self) -> bool {
        self.history
    }
    pub fn has_pending(&self) -> bool {
        self.entry.pending
            || !self.entry.intents.is_empty()
            || (self.kind == JournalKind::Content && self.revision != self.entry.revision)
    }
}
/// A sealed plan: callers can inspect effects but cannot manufacture journal ACKs.
pub struct Prepared {
    capture: Capture,
    decision: Value,
}
impl Prepared {
    pub fn decision(&self) -> &Value {
        &self.decision
    }
}

pub struct SyncJournalRepository {
    store: Arc<dyn DocumentStore>,
    serial: Mutex<()>,
}
impl SyncJournalRepository {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self {
        Self {
            store,
            serial: Mutex::new(()),
        }
    }
    pub async fn capture(&self, kind: JournalKind, account: &str) -> Result<Capture, JournalError> {
        account_key(account)?;
        let _serial = self.serial.lock().await;
        self.read(kind, account).await
    }
    async fn read(&self, kind: JournalKind, account: &str) -> Result<Capture, JournalError> {
        let document = self
            .store
            .read_document(kind.key().into())
            .await?
            .ok_or(JournalError::InvalidStorage)?;
        revision(&document.revision)?;
        let physical = codec::Physical::read(document.value, kind)?;
        let (entry, current_revision, history) = physical.entry(account)?;
        Ok(Capture {
            kind,
            account: account.into(),
            storage_revision: document.revision,
            physical,
            entry,
            revision: current_revision,
            history,
        })
    }
    pub fn prepare_content(
        &self,
        capture: Capture,
        local: Value,
        flow: Value,
        event: Value,
        now_millis: u64,
    ) -> Result<Prepared, JournalError> {
        if capture.kind != JournalKind::Content {
            return Err(JournalError::InvalidRequest);
        }
        let command = serde_json::from_value(json!({"state":capture.entry.state,
            "flow":flow,"local":local,"event":event,"nowMillis":now_millis}))
        .map_err(|_| JournalError::InvalidRequest)?;
        let decision =
            crate::content_workflow::evaluate(command).map_err(|_| JournalError::InvalidRequest)?;
        Ok(Prepared { capture, decision })
    }
    pub fn prepare_preferences(
        &self,
        capture: Capture,
        local: Value,
        flow: Value,
        event: Value,
    ) -> Result<Prepared, JournalError> {
        if capture.kind != JournalKind::Preferences {
            return Err(JournalError::InvalidRequest);
        }
        let command = serde_json::from_value(json!({"state":capture.entry.state,
            "flow":flow,"local":local,"intents":capture.entry.intents,"event":event}))
        .map_err(|_| JournalError::InvalidRequest)?;
        let decision =
            crate::preference_sync::evaluate(command).map_err(|_| JournalError::InvalidRequest)?;
        Ok(Prepared { capture, decision })
    }
    /// Call after applying a decision under the host's shared mutation gate.
    /// Stale plans fail; only an explicit new evaluation can admit newer data.
    pub async fn commit(&self, mut prepared: Prepared) -> Result<(), JournalError> {
        let _serial = self.serial.lock().await;
        let capture = &mut prepared.capture;
        let current = self.read(capture.kind, &capture.account).await?;
        if current.storage_revision != capture.storage_revision {
            return Err(JournalError::Conflict);
        }
        let decision = &prepared.decision;
        capture.entry.state = decision["state"].clone();
        capture.entry.pending = decision["hasPending"]
            .as_bool()
            .ok_or(JournalError::InvalidStorage)?;
        capture.entry.revision = capture.revision;
        if capture.kind == JournalKind::Preferences {
            let consumed = decision["consumed"]
                .as_u64()
                .ok_or(JournalError::InvalidStorage)? as usize;
            if consumed > capture.entry.intents.len() {
                return Err(JournalError::InvalidStorage);
            }
            capture.entry.intents.drain(..consumed);
        }
        capture
            .physical
            .set_entry(&capture.account, &capture.entry)?;
        self.write(
            capture.kind,
            capture.storage_revision.clone(),
            capture.physical.value()?,
        )
        .await
    }
    pub async fn mark_content_changed(&self) -> Result<(), JournalError> {
        let _serial = self.serial.lock().await;
        let mut capture = self.read(JournalKind::Content, "").await?;
        if !capture.physical.mark_content_changed()? {
            return Ok(());
        }
        self.write(
            capture.kind,
            capture.storage_revision,
            capture.physical.value()?,
        )
        .await
    }
    pub async fn append_preference(
        &self,
        account: &str,
        intent: Value,
    ) -> Result<(), JournalError> {
        account_key(account)?;
        crate::preference_sync::validate_intent(intent.clone())
            .map_err(|_| JournalError::InvalidRequest)?;
        let _serial = self.serial.lock().await;
        let mut capture = self.read(JournalKind::Preferences, account).await?;
        if capture.entry.intents.len() >= 4096 {
            return Err(JournalError::InvalidRequest);
        }
        capture.entry.revision = capture.physical.increment(capture.entry.revision)?;
        capture.entry.intents.push(intent);
        capture.physical.set_entry(account, &capture.entry)?;
        self.write(
            capture.kind,
            capture.storage_revision,
            capture.physical.value()?,
        )
        .await
    }
    async fn write(
        &self,
        kind: JournalKind,
        expected: String,
        value: Value,
    ) -> Result<(), JournalError> {
        let current = self
            .store
            .read_document(kind.key().into())
            .await?
            .ok_or(JournalError::InvalidStorage)?;
        if current.revision != expected {
            return Err(JournalError::Conflict);
        }
        if current.value == value {
            return Ok(());
        }
        let written = self
            .store
            .write_document(kind.key().into(), Some(expected), value.clone())
            .await?;
        revision(&written.revision)?;
        if written.value != value {
            return Err(JournalError::InvalidStorage);
        }
        Ok(())
    }
}
fn revision(value: &str) -> Result<(), JournalError> {
    if value.is_empty() || value.len() > 128 {
        Err(JournalError::InvalidStorage)
    } else {
        Ok(())
    }
}
fn account_key(value: &str) -> Result<(), JournalError> {
    if value.is_empty() || value.len() > 256 || value.bytes().any(|b| b.is_ascii_control()) {
        Err(JournalError::InvalidRequest)
    } else {
        Ok(())
    }
}
