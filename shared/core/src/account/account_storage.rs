//! Account persistence adapts the general agent ports; the existing stores stay put.
use crate::agent::{
    storage::{AgentStore, AUTH_ATTEMPT, SESSION},
    AgentClient,
};
use govorun_core::{
    AuthAttempt, AuthStorage, DocumentStore, Error, Future, Result, SecretStore, Storage,
    StorageRead, StoreError, StoredSession,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::Mutex;

pub struct NativeStorage {
    store: AgentStore,
    checkout_revisions: Mutex<BTreeMap<String, Option<String>>>,
}
fn account_error(error: StoreError) -> Error {
    match error {
        StoreError::Invalid => Error::StorageInvalid,
        StoreError::Unavailable | StoreError::Conflict => Error::StorageUnavailable,
    }
}
impl NativeStorage {
    pub async fn connect<T: crate::agent::AgentTransport + 'static>(native: T) -> Result<Self> {
        let snapshot = native
            .invoke("runtime.snapshot", json!({}))
            .await
            .map_err(|_| Error::StorageUnavailable)?;
        let client =
            AgentClient::from_snapshot(native, &snapshot).map_err(|_| Error::StorageInvalid)?;
        Ok(Self::from_client(client))
    }
    pub fn from_client(client: AgentClient) -> Self {
        Self {
            store: AgentStore::new(client),
            checkout_revisions: Mutex::new(BTreeMap::new()),
        }
    }
}
impl AuthStorage for NativeStorage {
    fn load_attempt(&self) -> Future<'_, Result<Option<AuthAttempt>>> {
        Box::pin(async move {
            let Some(value) = self
                .store
                .read_secret(AUTH_ATTEMPT.into(), false)
                .await
                .map_err(account_error)?
            else {
                return Ok(None);
            };
            let attempt: AuthAttempt =
                serde_json::from_value(value).map_err(|_| Error::StorageInvalid)?;
            attempt.validate()?;
            Ok(Some(attempt))
        })
    }
    fn save_attempt(
        &self,
        attempt: Option<AuthAttempt>,
        allow_interaction: bool,
    ) -> Future<'_, Result<()>> {
        Box::pin(async move {
            match attempt {
                Some(attempt) => self
                    .store
                    .write_secret(
                        AUTH_ATTEMPT.into(),
                        serde_json::to_value(attempt).map_err(|_| Error::StorageInvalid)?,
                        allow_interaction,
                    )
                    .await
                    .map_err(account_error),
                None => self
                    .store
                    .delete_secret(AUTH_ATTEMPT.into())
                    .await
                    .map_err(account_error),
            }
        })
    }
}
impl Storage for NativeStorage {
    fn load_checkout(&self, slot: String) -> Future<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let mut revisions = self.checkout_revisions.lock().await;
            let document = self
                .store
                .read_document(slot.clone())
                .await
                .map_err(account_error)?;
            revisions.insert(slot, document.as_ref().map(|value| value.revision.clone()));
            Ok(document.map(|value| value.value))
        })
    }
    fn save_checkout(&self, slot: String, record: Option<Value>) -> Future<'_, Result<()>> {
        Box::pin(async move {
            let mut revisions = self.checkout_revisions.lock().await;
            let revision = if let Some(revision) = revisions.get(&slot) {
                revision.clone()
            } else {
                self.store
                    .read_document(slot.clone())
                    .await
                    .map_err(account_error)?
                    .map(|document| document.revision)
            };
            match record {
                Some(value) => {
                    let document = self
                        .store
                        .write_document(slot.clone(), revision, value)
                        .await
                        .map_err(account_error)?;
                    revisions.insert(slot, Some(document.revision));
                }
                None => {
                    self.store
                        .delete_document(slot.clone(), revision)
                        .await
                        .map_err(account_error)?;
                    revisions.insert(slot, None);
                }
            }
            Ok(())
        })
    }
    fn restart(&self) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.store
                .restart_secret_namespace(SESSION.into())
                .await
                .map_err(account_error)?;
            self.checkout_revisions.lock().await.clear();
            Ok(())
        })
    }
    fn load(&self, allow_interaction: bool) -> Future<'_, Result<StorageRead>> {
        Box::pin(async move {
            let Some(value) = self
                .store
                .read_secret(SESSION.into(), allow_interaction)
                .await
                .map_err(account_error)?
            else {
                return Ok(StorageRead::Missing);
            };
            let session = serde_json::from_value(value).map_err(|_| Error::StorageInvalid)?;
            Ok(StorageRead::Session(session))
        })
    }
    fn save(&self, session: StoredSession, allow_interaction: bool) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.store
                .write_secret(
                    SESSION.into(),
                    serde_json::to_value(session).map_err(|_| Error::StorageInvalid)?,
                    allow_interaction,
                )
                .await
                .map_err(account_error)
        })
    }
    fn end(&self) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.store
                .delete_secret(SESSION.into())
                .await
                .map_err(account_error)
        })
    }
}
