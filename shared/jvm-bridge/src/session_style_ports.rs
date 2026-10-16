//! Android executes physical effects; the common owner retains custody, retries
//! and settlement. The anonymous gateway cannot represent account credentials.
use crate::session_host::SessionHost;
use govorun_core::{
    style_pass_vault::{StoredVault, VaultStore},
    Error, Future, StoreError, StoreResult,
};
#[cfg(feature = "private-fixtures")]
use govorun_core::stylization::{Failure, Gateway, Kind, Request, Response};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) struct StylePorts(pub Arc<SessionHost>);
fn storage_error(error: Error) -> StoreError {
    match error {
        Error::StorageInvalid | Error::InvalidResponse => StoreError::Invalid,
        _ => StoreError::Unavailable,
    }
}
fn revision(value: &Value) -> StoreResult<String> {
    value["revision"].as_str().filter(|s| !s.is_empty() && s.len() <= 128)
        .map(str::to_owned).ok_or(StoreError::Invalid)
}
impl VaultStore for StylePorts {
    fn read_revision(&self, allow_interaction: bool) -> Future<'_, StoreResult<govorun_core::style_pass_vault::VaultRevision>> {
        use govorun_core::style_pass_vault::VaultRevision;
        Box::pin(async move {
            let value = self.0.effect(json!({"operation":"styleVault","action":"revision",
                "allowInteraction":allow_interaction}), false).await.map_err(storage_error)?;
            match value["status"].as_str() {
                Some("missing") => Ok(VaultRevision::Missing),
                Some("available") => Ok(VaultRevision::Present(revision(&value)?)),
                Some("invalid") => Err(StoreError::Invalid),
                _ => Err(StoreError::Unavailable),
            }
        })
    }
    fn read(&self, allow_interaction: bool) -> Future<'_, StoreResult<Option<StoredVault>>> {
        Box::pin(async move {
            let value = self.0.effect(json!({"operation":"styleVault","action":"read",
                "allowInteraction":allow_interaction}), false).await.map_err(storage_error)?;
            match value["status"].as_str() {
                Some("missing") => Ok(None),
                Some("available") => Ok(Some(StoredVault {
                    revision:revision(&value)?,
                    raw:value["raw"].as_str().filter(|s| !s.is_empty() && s.len() <= 4*1024*1024)
                        .map(str::to_owned).ok_or(StoreError::Invalid)?,
                })),
                Some("invalid") => Err(StoreError::Invalid),
                _ => Err(StoreError::Unavailable),
            }
        })
    }
    fn compare_write(&self, expected_revision: Option<String>, raw: String,
        allow_interaction: bool) -> Future<'_, StoreResult<String>> {
        Box::pin(async move {
            let value = self.0.effect(json!({"operation":"styleVault","action":"write",
                "expectedRevision":expected_revision,"raw":raw,
                "allowInteraction":allow_interaction}),false).await.map_err(storage_error)?;
            match value["status"].as_str() {
                Some("available") => revision(&value),
                Some("conflict") => Err(StoreError::Conflict),
                Some("invalid") => Err(StoreError::Invalid),
                _ => Err(StoreError::Unavailable),
            }
        })
    }
}
#[cfg(feature = "private-fixtures")]
impl Gateway for StylePorts {
    fn send(&self, request: Request) -> Future<'_, std::result::Result<Response, Failure>> {
        Box::pin(async move {
            let headers = match request.kind() {
                Kind::Start => json!({"X-Govorun-Request-ID":request.request_id(),"Prefer":request.prefer_header()}),
                Kind::Status => json!({"X-Govorun-Job-Token":request.job_token(),"Prefer":request.prefer_header()}),
                Kind::Outcome => json!({"X-Govorun-Style-Pass":request.outcome_pass()}),
                _ => json!({}),
            };
            let method = if matches!(request.kind(),Kind::Status|Kind::Availability) {"GET"} else {"POST"};
            let value = self.0.effect(json!({"operation":"stylesHttp","method":method,
                "path":request.path(),"headers":headers,"body":request.body(),
                "timeoutMillis":request.timeout().as_millis() as u64}),true).await.map_err(|e|match e {
                    Error::StyleTransport(failure)=>failure,
                    Error::SessionChanged=>Failure::Cancelled,
                    Error::ResponseTooLarge=>Failure::TooLarge,
                    Error::InvalidResponse=>Failure::InvalidBody,
                    _=>Failure::Network,
                })?;
            let status = value["status"].as_u64().and_then(|v|u16::try_from(v).ok())
                .filter(|v|(100..=599).contains(v)).ok_or(Failure::InvalidBody)?;
            let body = value["body"].as_str().ok_or(Failure::InvalidBody)?.to_owned();
            let header = |name:&str|->std::result::Result<Option<String>,Failure> {
                match value.get(name).filter(|v|!v.is_null()) {
                    None=>Ok(None),
                    Some(v)=>v.as_str().map(|s|Some(s.to_owned())).ok_or(Failure::InvalidBody),
                }
            };
            Ok(Response {status,body,retry_after:header("retryAfter")?,date:header("date")?,last_modified:header("lastModified")?})
        })
    }
}
