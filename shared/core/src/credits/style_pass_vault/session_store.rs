//! Disk holds ready passes and unreported numeric usage. Requests stay in memory.
use super::{format::Format, repository::empty, Envelope, StoredVault, VaultRevision, VaultStore};
use crate::{Future, StoreError, StoreResult};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;
const TAG: &str = "govorun-pass-pool-1";
const MAXIMUM: usize = 256 * 1024;
struct State { raw: String, revision: String, pool: String, usage: Value, disk_revision: Option<String> }
pub struct SessionStore { disk: Arc<dyn VaultStore>, format: Format, state: Mutex<Option<State>> }
fn revision(raw: &str) -> String { format!("{:x}", Sha256::digest(raw.as_bytes())) }
fn invalid(_: crate::Error) -> StoreError { StoreError::Invalid }
impl SessionStore {
    pub fn new(disk: Arc<dyn VaultStore>, format: Format) -> Self {
        Self { disk, format, state: Mutex::new(None) }
    }
    async fn write(&self, expected: Option<String>, raw: &str, allow: bool) -> StoreResult<String> {
        match self.disk.compare_write(expected, raw.to_owned(), allow).await {
            Ok(ack) if !ack.is_empty() => Ok(ack),
            result => match self.disk.read(allow).await? {
                Some(stored) if stored.raw == raw && !stored.revision.is_empty() => Ok(stored.revision),
                _ => Err(result.err().unwrap_or(StoreError::Unavailable)),
            },
        }
    }
    async fn load(&self, state: &mut Option<State>, allow: bool) -> StoreResult<()> {
        if state.is_some() { return Ok(()); }
        let stored = self.disk.read(allow).await?;
        let mut value = empty(self.format).map_err(invalid)?;
        let mut disk_revision = None;
        let mut usage = json!([]);
        if let Some(stored) = stored {
            disk_revision = Some(stored.revision);
            // Unsupported formats start empty; they are never decrypted or imported here.
            if stored.raw.len() > MAXIMUM { return Err(StoreError::Invalid); }
            let old: Value = serde_json::from_str(&stored.raw).unwrap_or(Value::Null);
            if old["format"] == TAG {
                let fields = old.as_object().ok_or(StoreError::Invalid)?;
                if !(4..=5).contains(&fields.len()) || fields.keys().any(|key| !["format", "passes", "identityKey", "scopeKey", "usageCounters"].contains(&key.as_str())) || !["format", "passes", "identityKey", "scopeKey"].iter().all(|key| fields.contains_key(*key)) {
                    return Err(StoreError::Invalid);
                }
                usage = old.get("usageCounters").cloned().unwrap_or_else(||json!([]));
                let mut source = value.stored_value().clone();
                source["usage"] = super::usage_counters::restore(&usage)?;
                // Binding anchors are persisted; no requests survive a process restart.
                source["scopeEpoch"] = json!(0);
                let passes = old["passes"].as_array().ok_or(StoreError::Invalid)?;
                let mut available = Vec::new();
                for pass in passes {
                    let mut decoded = pass.clone();
                    decoded[self.format.key_id_field()] = pass["keyId"].clone();
                    if self.format.key_id_field() != "keyId" { decoded.as_object_mut().ok_or(StoreError::Invalid)?.remove("keyId"); }
                    decoded["expiresAt"] = self.format.date_value(pass["expiresAt"].as_u64().ok_or(StoreError::Invalid)?).map_err(invalid)?;
                    available.push(decoded);
                }
                let ledger = if self.format == Format::LedgerV5 { &mut source["ledger"] } else { &mut source };
                ledger["available"] = json!(available);
                for key in ["identityKey", "scopeKey"] { source[key] = old[key].clone(); }
                value = Envelope::decode(&source.to_string(), self.format).map_err(invalid)?;
            }
        }
        let raw = value.encoded().map_err(invalid)?;
        *state = Some(State { revision: revision(&raw), pool: pool(&value, &usage)?, usage, raw, disk_revision });
        Ok(())
    }
}
fn pool(value: &Envelope, usage: &Value) -> StoreResult<String> {
    let source = value.stored_value();
    let ledger = if value.format() == Format::LedgerV5 { &source["ledger"] } else { source };
    let mut passes = Vec::new();
    for pass in ledger["available"].as_array().ok_or(StoreError::Invalid)? {
        let mut encoded = pass.clone();
        encoded["keyId"] = pass[value.format().key_id_field()].clone();
        if value.format().key_id_field() != "keyId" { encoded.as_object_mut().ok_or(StoreError::Invalid)?.remove(value.format().key_id_field()); }
        encoded["expiresAt"] = json!(value.format().date_millis(&pass["expiresAt"]).map_err(invalid)?);
        passes.push(encoded);
    }
    let raw = json!({"format":TAG,"passes":passes,"identityKey":source["identityKey"].as_str().unwrap_or(""),"scopeKey":source["scopeKey"].as_str().unwrap_or(""),"usageCounters":usage}).to_string();
    if raw.len() > MAXIMUM { return Err(StoreError::Invalid); }
    Ok(raw)
}
impl VaultStore for SessionStore {
    fn read(&self, allow: bool) -> Future<'_, StoreResult<Option<StoredVault>>> {
        Box::pin(async move {
            let mut locked = self.state.lock().await;
            self.load(&mut locked, allow).await?;
            let state = locked.as_ref().ok_or(StoreError::Unavailable)?;
            Ok(Some(StoredVault { revision: state.revision.clone(), raw: state.raw.clone() }))
        })
    }
    fn read_revision(&self, allow: bool) -> Future<'_, StoreResult<VaultRevision>> {
        Box::pin(async move {
            let mut locked = self.state.lock().await;
            self.load(&mut locked, allow).await?;
            Ok(VaultRevision::Present(locked.as_ref().ok_or(StoreError::Unavailable)?.revision.clone()))
        })
    }
    fn checkpoint_usage(&self) -> Future<'_, StoreResult<()>> {
        Box::pin(async move {
            let mut locked = self.state.lock().await;
            self.load(&mut locked, false).await?;
            let state = locked.as_mut().ok_or(StoreError::Unavailable)?;
            let value = Envelope::decode(&state.raw, self.format).map_err(invalid)?;
            let usage = super::usage_counters::pending(&value)?;
            let next = pool(&value, &usage)?;
            if next != state.pool {
                state.disk_revision = Some(self.write(state.disk_revision.clone(), &next, false).await?);
                state.pool = next;
                state.usage = usage;
            }
            Ok(())
        })
    }
    fn compare_write(&self, expected: Option<String>, raw: String, allow: bool) -> Future<'_, StoreResult<String>> {
        Box::pin(async move {
            let mut locked = self.state.lock().await;
            self.load(&mut locked, allow).await?;
            let state = locked.as_mut().ok_or(StoreError::Unavailable)?;
            if expected.as_ref() != Some(&state.revision) { return Err(StoreError::Conflict); }
            let value = Envelope::decode(&raw, self.format).map_err(invalid)?;
            let prior = Envelope::decode(&state.raw, self.format).map_err(invalid)?;
            let usage = if prior.stored_value()["identityKey"] != value.stored_value()["identityKey"] {json!([])} else {state.usage.clone()};
            let next = pool(&value, &usage)?;
            if next != state.pool {
                let ack = self.write(state.disk_revision.clone(), &next, allow).await?;
                state.disk_revision = Some(ack);
                state.pool = next;
                state.usage = usage;
            }
            state.raw = raw;
            state.revision = revision(&state.raw);
            Ok(state.revision.clone())
        })
    }
}
