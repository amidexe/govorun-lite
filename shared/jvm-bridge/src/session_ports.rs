use crate::session_host::SessionHost;
use govorun_core::{
    AccountRequest, AuthAttempt, AuthStorage, Error, Future, HttpResponse, Result, Storage,
    StorageRead, StoredSession, Transport,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) struct SessionPorts(pub Arc<SessionHost>);

impl SessionPorts {
    async fn storage(&self, action: &str, fields: Value) -> Result<Value> {
        let mut command = fields;
        command["operation"] = json!("storage");
        command["action"] = json!(action);
        let result = self.0.effect(command, false).await?;
        match result["status"].as_str() {
            Some("available") => Ok(result),
            Some("missing") if matches!(action, "load" | "authLoad" | "checkoutLoad") => Ok(result),
            Some("invalid") => Err(Error::StorageInvalid),
            _ => Err(Error::StorageUnavailable),
        }
    }
}

impl Transport for SessionPorts {
    fn send(
        &self,
        request: AccountRequest,
        bearer: Option<String>,
    ) -> Future<'_, Result<HttpResponse>> {
        Box::pin(async move {
            let value = self
                .0
                .effect(
                    json!({"operation":"http", "method":request.method(),
                "path":request.path(), "body":request.body(), "etag":request.etag(),
                "idempotencyKey":request.idempotency_key(), "accessToken":bearer}),
                    true,
                )
                .await?;
            let status = value["status"]
                .as_u64()
                .and_then(|v| u16::try_from(v).ok())
                .filter(|v| (100..=599).contains(v))
                .ok_or(Error::InvalidResponse)?;
            let body = value.get("body").cloned().ok_or(Error::InvalidResponse)?;
            Ok(HttpResponse {
                status,
                body,
                etag: value["etag"].as_str().map(str::to_owned),
                retry_after_seconds: value["retryAfterSeconds"].as_u64(),
            })
        })
    }
}

impl Storage for SessionPorts {
    fn load(&self, allow_interaction: bool) -> Future<'_, Result<StorageRead>> {
        Box::pin(async move {
            let value = self
                .storage("load", json!({"allowInteraction":allow_interaction}))
                .await?;
            if value["status"] == "missing" {
                return Ok(StorageRead::Missing);
            }
            let stored = serde_json::from_value(value["session"].clone())
                .map_err(|_| Error::StorageInvalid)?;
            Ok(StorageRead::Session(stored))
        })
    }
    fn save(&self, session: StoredSession, allow_interaction: bool) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.storage(
                "save",
                json!({"session":session, "allowInteraction":allow_interaction}),
            )
            .await?;
            Ok(())
        })
    }
    fn end(&self) -> Future<'_, Result<()>> {
        Box::pin(async { self.storage("end", json!({})).await.map(|_| ()) })
    }
    fn restart(&self) -> Future<'_, Result<()>> {
        Box::pin(async { self.storage("restart", json!({})).await.map(|_| ()) })
    }
    fn load_checkout(&self, slot: String) -> Future<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let value = self.storage("checkoutLoad", json!({"slot":slot})).await?;
            if value["status"] == "missing" {
                return Ok(None);
            }
            value
                .get("record")
                .filter(|v| v.is_object())
                .cloned()
                .map(Some)
                .ok_or(Error::StorageInvalid)
        })
    }
    fn save_checkout(&self, slot: String, record: Option<Value>) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.storage("checkoutSave", json!({"slot":slot,"record":record}))
                .await
                .map(|_| ())
        })
    }
}

impl AuthStorage for SessionPorts {
    fn load_attempt(&self) -> Future<'_, Result<Option<AuthAttempt>>> {
        Box::pin(async {
            let value = self.storage("authLoad", json!({})).await?;
            if value["status"] == "missing" {
                return Ok(None);
            }
            let attempt: AuthAttempt = serde_json::from_value(value["attempt"].clone())
                .map_err(|_| Error::StorageInvalid)?;
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
            self.storage(
                "authSave",
                json!({"attempt":attempt,"allowInteraction":allow_interaction}),
            )
            .await
            .map(|_| ())
        })
    }
}
