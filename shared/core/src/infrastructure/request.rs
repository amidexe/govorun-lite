use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
pub enum Method { Get, Post, Put, Patch, Delete }

/// Account-only request. Construction cannot select another origin, attach
/// arbitrary headers, or expose a caller-supplied bearer to the network.
#[derive(Clone)]
pub struct AccountRequest {
    pub(crate) method: Method,
    pub(crate) path: String,
    pub(crate) body: Option<Value>,
    pub(crate) etag: Option<String>,
    pub(crate) idempotency_key: Option<String>,
}

impl AccountRequest {
    pub fn new(method: Method, path: &str, body: Option<Value>) -> Result<Self> {
        let allowed = match (method, path) {
            (Method::Get, "/v1/account" | "/v1/devices" | "/v1/billing/catalog" |
                "/v1/billing/status" | "/v1/style-passes/key" | "/v1/sync-document" |
                "/v1/preferences" | "/v1/stats") => true,
            (Method::Post, "/v1/consents/online-stylization" | "/v1/style-usage/activate" |
                "/v1/style-passes/issue" | "/v1/style-passes/deposit" | "/v1/billing/starter-grant" | "/v1/billing/promo-program" |
                "/v1/billing/cancel-renewal" | "/v1/billing/payment-method/remove" |
                "/v1/billing/change" | "/v1/billing/refund-requests" | "/v1/billing/tbank/preview" |
                "/v1/billing/tbank/checkout") => true,
            (Method::Patch, "/v1/preferences") => true,
            (Method::Put, "/v1/devices/current" | "/v1/sync-document" |
                "/v1/stats" | "/v3/stats/days" | "/v1/style-usage") => true,
            (Method::Delete, "/v1/account" | "/v1/stats") => true,
            (Method::Delete, value) => value.strip_prefix("/v1/devices/").is_some_and(uuid),
            (Method::Get, value) => value.strip_prefix("/v1/billing/tbank/checkout/")
                .is_some_and(|id| id == "latest" || uuid(id)),
            (Method::Post, value) => value.strip_prefix("/v1/billing/tbank/checkout/")
                .and_then(|rest| rest.strip_suffix("/cancel-binding")).is_some_and(uuid),
            _ => false,
        };
        if !allowed || body.as_ref().is_some_and(|value| !value.is_object()) {
            return Err(Error::InvalidRequest);
        }
        if body.as_ref().is_some_and(|value| value.to_string().len() > 524_288) {
            return Err(Error::InvalidRequest);
        }
        Ok(Self::internal(method, path, body))
    }

    pub fn with_etag(mut self, value: Option<String>) -> Result<Self> {
        if value.as_ref().is_some_and(|v| v.len() > 256 || v.bytes().any(|c| c.is_ascii_control())) {
            return Err(Error::InvalidRequest);
        }
        if self.path != "/v1/sync-document" { return Err(Error::InvalidRequest); }
        self.etag = value;
        Ok(self)
    }

    pub fn with_idempotency_key(mut self, value: String) -> Result<Self> {
        if !matches!(self.path.as_str(), "/v1/style-passes/issue" | "/v1/style-passes/deposit") ||
            !uuid(&value) { return Err(Error::InvalidRequest); }
        self.idempotency_key = Some(value);
        Ok(self)
    }

    pub fn method(&self) -> Method { self.method }
    pub fn path(&self) -> &str { &self.path }
    pub fn body(&self) -> Option<&Value> { self.body.as_ref() }
    pub fn etag(&self) -> Option<&str> { self.etag.as_deref() }
    pub fn idempotency_key(&self) -> Option<&str> { self.idempotency_key.as_deref() }

    pub(crate) fn internal(method: Method, path: &str, body: Option<Value>) -> Self {
        Self { method, path: path.into(), body, etag: None, idempotency_key: None }
    }

    pub(crate) fn may_retry_unauthorized(&self) -> bool { self.path != "/v1/style-passes/issue" }
}

pub(crate) fn uuid(value: &str) -> bool {
    value.len() == 36 && value.bytes().enumerate().all(|(index, c)|
        if [8, 13, 18, 23].contains(&index) { c == b'-' } else { c.is_ascii_hexdigit() })
}
