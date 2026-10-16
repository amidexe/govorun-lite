use crate::{Credentials, Future, Profile, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StoredSession {
    pub version: u32,
    pub tokens: Credentials,
    pub last_confirmed_profile: Option<Profile>,
}

pub enum StorageRead { Missing, Session(StoredSession) }

/// Implementations adapt existing Keychain/DPAPI identities and envelopes.
/// Reads must distinguish missing from inaccessible/corrupt data. `end` writes
/// the durable logout fence before best-effort credential deletion, so denied
/// deletion cannot resurrect the session. The core serializes writes and ends.
/// No storage method may change consent or the user's enabled-style preference.
pub trait Storage: Send + Sync {
    fn load(&self, allow_interaction: bool) -> Future<'_, Result<StorageRead>>;
    fn save(&self, session: StoredSession, allow_interaction: bool) -> Future<'_, Result<()>>;
    fn end(&self) -> Future<'_, Result<()>>;
    /// Opaque protected checkout journal; unavailable must never mean missing.
    fn load_checkout(&self, _slot: String) -> Future<'_, Result<Option<serde_json::Value>>> {
        Box::pin(async { Err(crate::Error::StorageUnavailable) })
    }
    fn save_checkout(&self, _slot: String, _record: Option<serde_json::Value>) -> Future<'_, Result<()>> {
        Box::pin(async { Err(crate::Error::StorageUnavailable) })
    }
    /// Select a fresh protected namespace without reading or deleting the old
    /// one. Both session and OAuth state must be absent after success.
    fn restart(&self) -> Future<'_, Result<()>> {
        Box::pin(async { Err(crate::Error::StorageUnavailable) })
    }
}
