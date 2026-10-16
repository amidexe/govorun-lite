//! General persistence over the agent. No paths or protected identities move.
use super::{AgentClient, Cancellation, Capability, Request};
use govorun_core::{Document, DocumentStore, Future, SecretStore, StoreError, StoreResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{sync::OnceLock, time::Duration};

pub const SESSION: &str = "account.session";
pub const AUTH_ATTEMPT: &str = "account.authAttempt";

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Reply {
    status: String,
    value: Option<Value>,
    revision: Option<String>,
}
impl Reply {
    fn checked(self) -> StoreResult<Self> {
        match self.status.as_str() {
            "available" | "missing" => Ok(self),
            "conflict" => Err(StoreError::Conflict),
            "invalid" => Err(StoreError::Invalid),
            _ => Err(StoreError::Unavailable),
        }
    }
    fn available(self, missing: StoreError) -> StoreResult<()> {
        if self.status == "available" { Ok(()) } else { Err(missing) }
    }
    fn document(self) -> StoreResult<Document> {
        let revision = self
            .revision
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .ok_or(StoreError::Invalid)?;
        let value = self
            .value
            .filter(Value::is_object)
            .ok_or(StoreError::Invalid)?;
        Ok(Document { revision, value })
    }
}

fn keys() -> &'static Value {
    static KEYS: OnceLock<Value> = OnceLock::new();
    KEYS.get_or_init(|| {
        serde_json::from_str(include_str!("../../../contracts/storage-keys.json"))
            .expect("bundled storage key/location table")
    })
}
pub fn secret_key(key: &str) -> bool {
    keys()["secrets"].get(key).is_some()
}
pub fn document_key(key: &str) -> bool {
    if keys()["documents"][key]["fixedKey"].as_str() == Some(key) {
        return true;
    }
    let spec = &keys()["documents"]["account.checkout"];
    let Some(prefix) = spec["prefix"].as_str() else {
        return false;
    };
    let Some(suffix) = key.strip_prefix(prefix) else {
        return false;
    };
    Some(suffix.len() as u64) == spec["suffixLowerHexBytes"].as_u64()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub struct AgentStore {
    client: AgentClient,
}
impl AgentStore {
    pub fn new(client: AgentClient) -> Self {
        Self { client }
    }
    async fn invoke(&self, method: &str, params: Value) -> StoreResult<Reply> {
        if matches!(method, "secret.read" | "secret.write" | "secret.delete")
            && matches!(
                params["key"].as_str(),
                Some("selfHost.ollama" | "selfHost.openai" | "selfHost.gemini")
            )
        {
            let reply: Reply = self
                .client
                .self_host_secret(method, params)
                .await
                .map_err(|_| StoreError::Unavailable)?;
            return reply.checked();
        }
        let reply: Reply = self
            .client
            .call(
                Request {
                    capability: Capability::named(method).map_err(|_| StoreError::Invalid)?,
                    params,
                },
                Duration::from_secs(20),
                &Cancellation::default(),
            )
            .await
            .map_err(|_| StoreError::Unavailable)?;
        reply.checked()
    }
}

/// Separate trait so generic secret writes cannot accidentally bypass vault CAS.
impl govorun_core::style_pass_vault::VaultStore for AgentStore {
    fn read(
        &self,
        allow_interaction: bool,
    ) -> Future<'_, StoreResult<Option<govorun_core::style_pass_vault::StoredVault>>> {
        Box::pin(async move {
            let reply = self
                .invoke(
                    "secret.read",
                    json!({"key":"stylePass.vault", "allowInteraction":allow_interaction}),
                )
                .await?;
            if reply.status == "missing" {
                return Ok(None);
            }
            let revision = reply
                .revision
                .filter(|s| !s.is_empty() && s.len() <= 128)
                .ok_or(StoreError::Invalid)?;
            let raw = reply
                .value
                .and_then(|v| v.as_str().map(str::to_owned))
                .filter(|s| !s.is_empty() && s.len() <= 4 * 1024 * 1024)
                .ok_or(StoreError::Invalid)?;
            Ok(Some(govorun_core::style_pass_vault::StoredVault {
                revision,
                raw,
            }))
        })
    }
    fn compare_write(
        &self,
        expected_revision: Option<String>,
        raw: String,
        allow_interaction: bool,
    ) -> Future<'_, StoreResult<String>> {
        Box::pin(async move {
            if raw.is_empty()
                || raw.len() > 4 * 1024 * 1024
                || expected_revision
                    .as_ref()
                    .is_some_and(|s| s.is_empty() || s.len() > 128)
            {
                return Err(StoreError::Invalid);
            }
            let reply = self.invoke("secret.write", json!({"key":"stylePass.vault", "expectedRevision":expected_revision, "value":raw, "allowInteraction":allow_interaction})).await?;
            if reply.status != "available" {
                return Err(StoreError::Invalid);
            }
            reply
                .revision
                .filter(|s| !s.is_empty() && s.len() <= 128)
                .ok_or(StoreError::Invalid)
        })
    }
}

impl DocumentStore for AgentStore {
    fn read_document(&self, key: String) -> Future<'_, StoreResult<Option<Document>>> {
        Box::pin(async move {
            if !document_key(&key) {
                return Err(StoreError::Invalid);
            }
            let reply = self.invoke("storage.read", json!({"key":key})).await?;
            if reply.status == "missing" {
                Ok(None)
            } else {
                reply.document().map(Some)
            }
        })
    }
    fn write_document(
        &self,
        key: String,
        expected_revision: Option<String>,
        value: Value,
    ) -> Future<'_, StoreResult<Document>> {
        Box::pin(async move {
            if !document_key(&key) || !value.is_object() {
                return Err(StoreError::Invalid);
            }
            let reply = self
                .invoke(
                    "storage.write",
                    json!({"key":key,"expectedRevision":expected_revision,"value":value}),
                )
                .await?;
            if reply.status != "available" {
                return Err(StoreError::Invalid);
            }
            let document = reply.document()?;
            if document.value != value {
                return Err(StoreError::Invalid);
            }
            Ok(document)
        })
    }
    fn delete_document(
        &self,
        key: String,
        expected_revision: Option<String>,
    ) -> Future<'_, StoreResult<()>> {
        Box::pin(async move {
            if !document_key(&key) {
                return Err(StoreError::Invalid);
            }
            let reply = self
                .invoke(
                    "storage.delete",
                    json!({"key":key,"expectedRevision":expected_revision}),
                )
                .await?;
            reply.available(StoreError::Invalid)
        })
    }
}

impl SecretStore for AgentStore {
    fn read_secret(
        &self,
        key: String,
        allow_interaction: bool,
    ) -> Future<'_, StoreResult<Option<Value>>> {
        Box::pin(async move {
            if !secret_key(&key) || key == "stylePass.vault" {
                return Err(StoreError::Invalid);
            }
            let reply = self
                .invoke(
                    "secret.read",
                    json!({"key":key,"allowInteraction":allow_interaction}),
                )
                .await?;
            if reply.status == "missing" {
                Ok(None)
            } else {
                reply
                    .value
                    .filter(Value::is_object)
                    .map(Some)
                    .ok_or(StoreError::Invalid)
            }
        })
    }
    fn write_secret(
        &self,
        key: String,
        value: Value,
        allow_interaction: bool,
    ) -> Future<'_, StoreResult<()>> {
        Box::pin(async move {
            if !secret_key(&key) || key == "stylePass.vault" || !value.is_object() {
                return Err(StoreError::Invalid);
            }
            let reply = self
                .invoke(
                    "secret.write",
                    json!({"key":key,"value":value,"allowInteraction":allow_interaction}),
                )
                .await?;
            reply.available(StoreError::Invalid)
        })
    }
    fn delete_secret(&self, key: String) -> Future<'_, StoreResult<()>> {
        Box::pin(async move {
            if !secret_key(&key) || key == "stylePass.vault" {
                return Err(StoreError::Invalid);
            }
            let reply = self.invoke("secret.delete", json!({"key":key})).await?;
            reply.available(StoreError::Unavailable)
        })
    }
    fn restart_secret_namespace(&self, key: String) -> Future<'_, StoreResult<()>> {
        Box::pin(async move {
            if key != SESSION {
                return Err(StoreError::Invalid);
            }
            let reply = self.invoke("secret.restart", json!({"key":key})).await?;
            reply.available(StoreError::Unavailable)
        })
    }
}
