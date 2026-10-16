use crate::{AccountRequest, Error, Future, Result};
use serde_json::Value;

#[derive(Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
    pub etag: Option<String>,
    pub retry_after_seconds: Option<u64>,
}

impl HttpResponse {
    pub fn success(self) -> Result<Self> {
        if (200..300).contains(&self.status) { Ok(self) } else { Err(Error::Http(self.status)) }
    }
}

pub trait Transport: Send + Sync {
    fn send(&self, request: AccountRequest, bearer: Option<String>) -> Future<'_, Result<HttpResponse>>;
}
