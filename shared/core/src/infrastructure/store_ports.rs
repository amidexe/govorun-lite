//! Persistence ports. Adapters keep platform identities and envelopes unchanged.
use crate::Future;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    Unavailable,
    Invalid,
    Conflict,
}
pub type StoreResult<T> = std::result::Result<T, StoreError>;

/// A revision is an opaque token supplied by the adapter, never a UI counter.
/// Equal content may share a revision. Missing is distinct from unreadable.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub revision: String,
    pub value: Value,
}

pub trait DocumentStore: Send + Sync {
    fn read_document(&self, key: String) -> Future<'_, StoreResult<Option<Document>>>;
    /// None means create only while the document is missing. Compare and write
    /// must be one serialized adapter operation; conflicts never overwrite.
    fn write_document(&self, key: String, expected_revision: Option<String>, value: Value)
        -> Future<'_, StoreResult<Document>>;
    /// Deletion is always explicit and also compares the observed revision.
    /// Adapters may retain a tombstone required to prevent old data returning.
    fn delete_document(&self, key: String, expected_revision: Option<String>)
        -> Future<'_, StoreResult<()>>;
}

/// Protected values stay inside the core/host/OS exchange and are never emitted
/// to renderer snapshots, diagnostics or support reports.
pub trait SecretStore: Send + Sync {
    fn read_secret(&self, key: String, allow_interaction: bool)
        -> Future<'_, StoreResult<Option<Value>>>;
    fn write_secret(&self, key: String, value: Value, allow_interaction: bool)
        -> Future<'_, StoreResult<()>>;
    /// Session adapters publish their durable logout fence before credential
    /// deletion. An inaccessible store must not be reported as missing.
    fn delete_secret(&self, key: String) -> Future<'_, StoreResult<()>>;
    /// Select a fresh protected namespace without reading/deleting the old one.
    fn restart_secret_namespace(&self, _key: String) -> Future<'_, StoreResult<()>> {
        Box::pin(async { Err(StoreError::Unavailable) })
    }
}
