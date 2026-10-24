//! One persisted choice owner; the existing enrollment rule decides admission.
#[path="sync_enrollment/codec.rs"]
mod codec;
pub use crate::synchronization::Mode as DataSyncMode;
use crate::{DocumentStore, StoreError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnrollmentError {
    InvalidRequest,
    InvalidStorage,
    Unavailable,
    Conflict,
}
impl From<StoreError> for EnrollmentError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Invalid => Self::InvalidStorage,
            StoreError::Unavailable => Self::Unavailable,
            StoreError::Conflict => Self::Conflict,
        }
    }
}
pub struct LocalFacts {
    pub content: Value,
    pub statistics: Value,
    pub history: bool,
    pub remote_data: Option<bool>,
    /// Some local data could not be read: the person decides, as if they had asked.
    pub unread: bool,
}
pub struct EnrollmentSnapshot {
    pub stored: Option<DataSyncMode>,
    pub requested: bool,
    pub other_account_history: bool,
}
#[derive(Clone, Copy)]
pub enum Choice {
    Merge,
    KeepLocal,
    RequestDecision,
}
pub struct SyncEnrollmentRepository {
    store: Arc<dyn DocumentStore>,
    serial: Mutex<()>,
}
impl SyncEnrollmentRepository {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self {
        Self {
            store,
            serial: Mutex::new(()),
        }
    }
    pub async fn snapshot(&self, account: &str) -> Result<EnrollmentSnapshot, EnrollmentError> {
        let slot = account_slot(account)?;
        let _serial = self.serial.lock().await;
        let (_, physical) = self.read().await?;
        physical.snapshot(&slot)
    }
    /// The host holds its mutation gate so local facts and this decision have
    /// the same scope. This method never fetches or uploads any user data.
    pub async fn resolve(
        &self,
        account: &str,
        facts: LocalFacts,
    ) -> Result<DataSyncMode, EnrollmentError> {
        let slot = account_slot(account)?;
        let _serial = self.serial.lock().await;
        let (revision, mut physical) = self.read().await?;
        let current = physical.snapshot(&slot)?;
        let command = serde_json::from_value(json!({"kind":"enrollment","stored":current.stored,
            "content":facts.content,"statistics":facts.statistics,"history":facts.history,
            "otherAccountHistory":current.other_account_history,"requested":current.requested || facts.unread,
            "remoteData":facts.remote_data}))
        .map_err(|_| EnrollmentError::InvalidRequest)?;
        let result = crate::synchronization::evaluate(command)
            .map_err(|_| EnrollmentError::InvalidRequest)?;
        let mode: DataSyncMode = serde_json::from_value(result["mode"].clone())
            .map_err(|_| EnrollmentError::InvalidStorage)?;
        physical.set(&slot, mode, false)?;
        self.write(revision, physical.value()?).await?;
        Ok(mode)
    }
    pub async fn choose(
        &self,
        account: &str,
        choice: Choice,
    ) -> Result<DataSyncMode, EnrollmentError> {
        let slot = account_slot(account)?;
        let _serial = self.serial.lock().await;
        let (revision, mut physical) = self.read().await?;
        let mode = match choice {
            Choice::Merge => DataSyncMode::Enabled,
            Choice::KeepLocal => DataSyncMode::LocalOnly,
            Choice::RequestDecision => DataSyncMode::Pending,
        };
        physical.set(&slot, mode, matches!(choice, Choice::RequestDecision))?;
        self.write(revision, physical.value()?).await?;
        Ok(mode)
    }
    async fn read(&self) -> Result<(String, codec::Physical), EnrollmentError> {
        let document = self
            .store
            .read_document("sync.enrollment".into())
            .await?
            .ok_or(EnrollmentError::InvalidStorage)?;
        valid_revision(&document.revision)?;
        Ok((document.revision, codec::Physical::read(document.value)?))
    }
    async fn write(&self, expected: String, value: Value) -> Result<(), EnrollmentError> {
        let current = self
            .store
            .read_document("sync.enrollment".into())
            .await?
            .ok_or(EnrollmentError::InvalidStorage)?;
        if current.revision != expected {
            return Err(EnrollmentError::Conflict);
        }
        if current.value == value {
            return Ok(());
        }
        let ack = self
            .store
            .write_document("sync.enrollment".into(), Some(expected), value.clone())
            .await?;
        valid_revision(&ack.revision)?;
        if ack.value != value {
            return Err(EnrollmentError::InvalidStorage);
        }
        Ok(())
    }
}
fn account_slot(account: &str) -> Result<String, EnrollmentError> {
    crate::statistics_ledger_model::account_key(account)
        .map_err(|_| EnrollmentError::InvalidRequest)?;
    Ok(format!("{:x}", Sha256::digest(account.as_bytes())))
}
fn valid_revision(revision: &str) -> Result<(), EnrollmentError> {
    if revision.is_empty() || revision.len() > 128 {
        Err(EnrollmentError::InvalidStorage)
    } else {
        Ok(())
    }
}
