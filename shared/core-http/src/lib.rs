pub mod self_hosted;
pub mod stylization;
use govorun_core::{validate_account_origin, AccountRequest, Error, Future, HttpResponse, Method, Result, Transport};
use serde_json::Value;
use std::time::Duration;

pub struct HttpTransport { client: reqwest::Client, origin: url::Url, app_version: String }

impl HttpTransport {
    /// The host supplies the build's trusted origin, never a renderer preference.
    pub fn new(origin: &str, app_version: &str) -> Result<Self> {
        if app_version.is_empty() || app_version.len() > 64 ||
            !app_version.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'+')) {
            return Err(Error::InvalidRequest);
        }
        let origin = validate_account_origin(origin)?;
        let client = Self::builder().build().map_err(|_| Error::NetworkUnavailable)?;
        Ok(Self { client, origin, app_version: app_version.into() })
    }

    fn builder() -> reqwest::ClientBuilder {
        reqwest::Client::builder().https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(10))
    }

    /// Private source-inclusion tests share the production builder. No alternate
    /// endpoint or trust root is compiled into the shipped account transport.
    #[cfg(test)]
    pub(crate) fn synthetic_peer(origin: &str, root_der: &[u8]) -> Result<Self> {
        let parsed = url::Url::parse(origin).map_err(|_| Error::InvalidRequest)?;
        let port = parsed.port().filter(|port| *port > 0).ok_or(Error::InvalidRequest)?;
        if origin != format!("https://127.0.0.1:{port}/") { return Err(Error::InvalidRequest); }
        let root = reqwest::Certificate::from_der(root_der).map_err(|_| Error::InvalidRequest)?;
        let client = Self::builder().no_proxy().add_root_certificate(root)
            .build().map_err(|_| Error::NetworkUnavailable)?;
        Ok(Self { client, origin: parsed, app_version: "2.1.62".into() })
    }

    async fn perform(&self, request: AccountRequest, bearer: Option<String>) -> Result<HttpResponse> {
        let method = match request.method() {
            Method::Get => reqwest::Method::GET, Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT, Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        };
        let url = self.origin.join(request.path()).map_err(|_| Error::InvalidRequest)?;
        let mut outgoing = self.client.request(method, url).header("Accept", "application/json")
            .header("Cache-Control", "no-store").header("X-Govorun-App-Version", &self.app_version);
        if let Some(token) = bearer { outgoing = outgoing.bearer_auth(token); }
        if let Some(key) = request.idempotency_key() { outgoing = outgoing.header("Idempotency-Key", key); }
        if let Some(etag) = request.etag() {
            outgoing = outgoing.header(if request.method() == Method::Get { "If-None-Match" } else { "If-Match" }, etag);
        }
        if let Some(body) = request.body() {
            outgoing = outgoing.header("Content-Type", "application/json")
                .body(serde_json::to_vec(body).map_err(|_| Error::InvalidRequest)?);
        }
        let mut incoming = outgoing.send().await.map_err(|_| Error::NetworkUnavailable)?;
        const MAX_BYTES: usize = 1_048_576;
        if incoming.content_length().is_some_and(|length| length > MAX_BYTES as u64) {
            return Err(Error::ResponseTooLarge);
        }
        let status = incoming.status().as_u16();
        let etag = incoming.headers().get("ETag").and_then(|v| v.to_str().ok()).map(str::to_owned);
        let retry_after_seconds = incoming.headers().get("Retry-After")
            .and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
        let mut bytes = Vec::new();
        while let Some(chunk) = incoming.chunk().await.map_err(|_| Error::NetworkUnavailable)? {
            if bytes.len() + chunk.len() > MAX_BYTES { return Err(Error::ResponseTooLarge); }
            bytes.extend_from_slice(&chunk);
        }
        let body = if bytes.is_empty() { Value::Null } else {
            match serde_json::from_slice(&bytes) {
                Ok(body) => body,
                Err(_) if !(200..300).contains(&status) => Value::Null,
                Err(_) => return Err(Error::InvalidResponse),
            }
        };
        Ok(HttpResponse { status, body, etag, retry_after_seconds })
    }
}

impl Transport for HttpTransport {
    fn send(&self, request: AccountRequest, bearer: Option<String>) -> Future<'_, Result<HttpResponse>> {
        Box::pin(self.perform(request, bearer))
    }
}
