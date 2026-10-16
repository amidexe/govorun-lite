//! One writer and an atomic physical-store compare/write. A failed ACK never
//! authorizes handing out a paid token; committed PENDING remains recoverable.
use super::{format::Format, Action, Additions, Envelope, InventoryChange};
use crate::{style_inventory::Event, Error, Future, Result, StoreError, StoreResult};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct StoredVault {
    pub revision: String,
    pub raw: String,
}
pub enum VaultRevision { Unsupported, Missing, Present(String) }
pub trait VaultStore: Send + Sync {
    fn read(&self, allow_interaction: bool) -> Future<'_, StoreResult<Option<StoredVault>>>;
    /// A small physical-slot check lets the owner reuse a validated snapshot.
    /// Adapters without this operation retain the existing full-read behavior.
    fn read_revision(&self, _: bool) -> Future<'_, StoreResult<VaultRevision>> {
        Box::pin(async { Ok(VaultRevision::Unsupported) })
    }
    /// Persist cumulative usage after text delivery, without a request journal.
    fn checkpoint_usage(&self) -> Future<'_, StoreResult<()>> { Box::pin(async { Ok(()) }) }
    /// The SDK adapter compares and writes in one serialized operation. None
    /// means create-only-if-missing, never permission to replace unknown data.
    fn compare_write(
        &self,
        expected_revision: Option<String>,
        raw: String,
        allow_interaction: bool,
    ) -> Future<'_, StoreResult<String>>;
}
pub struct Repository {
    store: Arc<dyn VaultStore>,
    format: Format,
    gate: Mutex<()>,
    cached: std::sync::Mutex<Option<(Envelope, Option<String>)>>,
    changes: tokio::sync::watch::Sender<u64>,
}
fn store_error(error: StoreError) -> Error {
    match error {
        StoreError::Invalid => Error::StorageInvalid,
        StoreError::Unavailable | StoreError::Conflict => Error::StorageUnavailable,
    }
}
impl Repository {
    pub fn new(store: Arc<dyn VaultStore>, format: Format) -> Self {
        Self {
            store,
            format,
            gate: Mutex::new(()),
            cached: std::sync::Mutex::new(None),
            changes: tokio::sync::watch::channel(0).0,
        }
    }
    /// Content-free invalidation after a physical write attempt, including an
    /// ambiguous ACK. Observers re-read confirmed pass state under the gate.
    pub fn subscribe_custody(&self) -> tokio::sync::watch::Receiver<u64> { self.changes.subscribe() }
    pub(super) fn storage_format(&self) -> Format { self.format }
    pub(crate) async fn checkpoint_usage(&self) -> Result<()> {
        self.store.checkpoint_usage().await.map_err(store_error)
    }
    pub async fn inspect(&self, allow_interaction: bool) -> Result<Envelope> {
        let _guard = self.gate.lock().await;
        self.load(allow_interaction).await.map(|(e, _)| e)
    }
    async fn load(&self, allow_interaction: bool) -> Result<(Envelope, Option<String>)> {
        let cached = self.cached.lock().map_err(|_| Error::StorageUnavailable)?.clone();
        if let Some((snapshot, previous)) = cached {
            let revision = self.store.read_revision(allow_interaction).await.map_err(store_error)?;
            let unchanged = match revision {
                VaultRevision::Unsupported => false,
                VaultRevision::Missing => previous.is_none(),
                VaultRevision::Present(value) => previous.as_ref() == Some(&value),
            };
            if unchanged { return Ok((snapshot, previous)); }
        }
        let stored = self
            .store
            .read(allow_interaction)
            .await
            .map_err(store_error)?;
        let decoded = match stored {
            None => Ok((empty(self.format)?, None)),
            Some(stored) => {
                if stored.revision.is_empty() || stored.revision.len() > 128 {
                    return Err(Error::StorageInvalid);
                }
                Ok((
                    Envelope::decode(&stored.raw, self.format)?,
                    Some(stored.revision),
                ))
            }
        }?;
        *self.cached.lock().map_err(|_| Error::StorageUnavailable)? = Some(decoded.clone());
        Ok(decoded)
    }
    pub async fn inventory(
        &self,
        additions: Additions,
        event: Event,
        now: u64,
        retirement_available: bool,
        allow_interaction: bool,
        current: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<InventoryChange> {
        self.commit(allow_interaction, current, |old| {
            old.inventory(additions, event, now, retirement_available)
        })
        .await
    }
    pub async fn change(
        &self,
        action: Action,
        now: u64,
        retirement_available: bool,
        allow_interaction: bool,
        current: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<InventoryChange> {
        self.commit(allow_interaction, current, |old| {
            old.change(action, now, retirement_available)
        })
        .await
    }
    pub(super) async fn commit(
        &self,
        allow_interaction: bool,
        current: &(dyn Fn() -> bool + Send + Sync),
        change: impl FnOnce(&Envelope) -> Result<InventoryChange>,
    ) -> Result<InventoryChange> {
        let _guard = self.gate.lock().await;
        if !current() {
            return Err(Error::SessionChanged);
        }
        let (old, revision) = self.load(allow_interaction).await?;
        if !current() {
            return Err(Error::SessionChanged);
        }
        let decision = change(&old)?;
        if !current() {
            return Err(Error::SessionChanged);
        }
        if decision.candidate.stored_value() != old.stored_value() {
            let written = self
                .store
                .compare_write(revision, decision.candidate.encoded()?, allow_interaction)
                .await;
            *self.cached.lock().map_err(|_| Error::StorageUnavailable)? = None;
            self.changes.send_modify(|revision| *revision = revision.wrapping_add(1));
            let ack = written.map_err(store_error)?;
            if ack.is_empty() || ack.len() > 128 {
                return Err(Error::StorageInvalid);
            }
            *self.cached.lock().map_err(|_| Error::StorageUnavailable)? =
                Some((decision.candidate.clone(), Some(ack)));
        }
        // Cancellation/profile change after a committed write preserves the
        // PENDING ledger. It cannot return credits before server recovery.
        if !current() {
            return Err(Error::SessionChanged);
        }
        Ok(decision)
    }
}
pub(super) fn empty(format: Format) -> Result<Envelope> {
    let usage = json!({"totals":{},"acknowledged":{},"requests":{}});
    let raw = match format {
        Format::LedgerV5 => {
            json!({"version":5,"ledger":{"available":[],"pending":[]},"issuance":null,"retiring":[],"replacementReceipts":[],"usage":usage,"scopeEpoch":null,"quarantinedPendingLeaseIDs":[],"issuanceScopeEpoch":null})
        }
        Format::AndroidV8 => {
            json!({"version":8,"available":[],"pending":[],"retiring":[],"replacementReceipts":[],"usage":usage,"scopeEpoch":0})
        }
        Format::FlatV4 => {
            json!({"version":4,"available":[],"pending":[],"retiring":[],"replacementReceipts":[],"usage":usage,"scopeEpoch":0,"scopeKey":"","identityKey":""})
        }
    };
    Envelope::decode(&raw.to_string(), format)
}
