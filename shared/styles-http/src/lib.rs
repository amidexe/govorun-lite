//! Anonymous Styles transport shared by desktop and Android. No account state,
//! cookies, redirects, ambient authentication, proxy headers or cross-dictation pool.
mod connection;
use connection::Connection;
use govorun_core::{
    stylization::{timing::ExchangeEvent, Failure, Gateway, Kind, Request, Response},
    Error, Future, Result,
};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Request as HttpRequest};
use rustls::{client::Resumption, ClientConfig, RootCertStore};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;

pub struct Client {
    origin: url::Url,
    user_agent: String,
    version: Option<String>,
    tls: Arc<ClientConfig>,
    prepared: Mutex<Option<(String, JoinHandle<std::result::Result<Connection, Failure>>)>>,
}
fn tls(roots: RootCertStore) -> Arc<ClientConfig> {
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("standard TLS versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
    config.resumption = Resumption::disabled();
    Arc::new(config)
}
impl Client {
    pub fn new(origin: &str, user_agent: &str) -> Result<Self> {
        let origin = govorun_core::stylization::origin(origin)?;
        if !["Govorun-macOS/2", "Govorun-Windows/2", "Govorun-Android/2"].contains(&user_agent) {
            return Err(Error::InvalidRequest);
        }
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Ok(Self {
            origin,
            user_agent: user_agent.into(),
            version: None,
            tls: tls(roots),
            prepared: Mutex::new(None),
        })
    }
    pub fn with_version(mut self, version: &str) -> Result<Self> {
        if version.is_empty()
            || version.len() > 32
            || !version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
        {
            return Err(Error::InvalidRequest);
        }
        self.version = Some(version.into());
        Ok(self)
    }
    /// Only one upcoming dictation is prepared. Requests with another random ID
    /// always open their own connection, including background and status requests.
    pub fn warm(&self, request_id: String) {
        let mut pending = self.prepared.lock().expect("connection state");
        if let Some((_, task)) = pending.take() {
            task.abort();
        }
        let origin = self.origin.clone();
        let tls = self.tls.clone();
        let task = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(5), connection::open(&origin, tls))
                .await
                .unwrap_or(Err(Failure::Network))
        });
        *pending = Some((request_id, task));
    }
    pub fn close_prepared(&self, request_id: Option<&str>) {
        let mut pending = self.prepared.lock().expect("connection state");
        if pending
            .as_ref()
            .is_some_and(|(id, _)| request_id.is_none_or(|expected| expected == id))
        {
            if let Some((_, task)) = pending.take() {
                task.abort();
            }
        }
    }
    async fn take_prepared(&self, request: &Request) -> Option<Connection> {
        let pending = {
            let mut pending = self.prepared.lock().expect("connection state");
            if request.kind() != Kind::Start
                || pending
                    .as_ref()
                    .is_none_or(|(id, _)| Some(id.as_str()) != request.request_id())
            {
                return None;
            }
            pending.take()
        }?;
        if !pending.1.is_finished() {
            pending.1.abort();
            return None;
        }
        match pending.1.await {
            Ok(Ok(connection)) if connection.usable() => Some(connection),
            _ => None,
        }
    }
    pub(crate) fn prepare(&self, request: &Request) -> Result<HttpRequest<Full<Bytes>>> {
        let method = if matches!(request.kind(), Kind::Status | Kind::Availability) {
            "GET"
        } else {
            "POST"
        };
        let host = self.origin[url::Position::BeforeHost..url::Position::AfterPort].to_owned();
        let mut outgoing = HttpRequest::builder()
            .method(method)
            .uri(request.path())
            .header("Host", host)
            .header(
                "User-Agent",
                if request.kind() == Kind::Reliability {
                    "Govorun"
                } else {
                    &self.user_agent
                },
            );
        if request.kind() != Kind::Reliability {
            outgoing = outgoing
                .header("Accept", "application/json")
                .header("Cache-Control", "no-store");
            if let Some(version) = &self.version {
                outgoing = outgoing.header("X-Govorun-App-Version", version);
            }
        }
        if let Some(id) = request.request_id() {
            outgoing = outgoing.header("X-Govorun-Request-ID", id);
        }
        if let Some(token) = request.job_token() {
            outgoing = outgoing.header("X-Govorun-Job-Token", token);
        }
        if let Some(prefer) = request.prefer_header() {
            outgoing = outgoing.header("Prefer", prefer);
        }
        if let Some(pass) = request.outcome_pass() {
            outgoing = outgoing.header("X-Govorun-Style-Pass", pass);
        }
        let body = if let Some(body) = request.body() {
            outgoing = outgoing.header(
                "Content-Type",
                if request.kind() == Kind::Reliability {
                    "application/json"
                } else {
                    "application/json; charset=utf-8"
                },
            );
            serde_json::to_vec(body).map_err(|_| Error::InvalidRequest)?
        } else {
            vec![]
        };
        outgoing
            .body(Full::new(Bytes::from(body)))
            .map_err(|_| Error::InvalidRequest)
    }
    async fn perform(
        &self,
        request: Request,
        observe: &(dyn Fn(ExchangeEvent) + Send + Sync),
    ) -> std::result::Result<Response, Failure> {
        let timeout = request.timeout();
        let operation = async {
            let prepared = self.prepare(&request).map_err(|_| Failure::Other)?;
            let mut connection = match self.take_prepared(&request).await {
                Some(connection) => connection,
                None => tokio::time::timeout(
                    Duration::from_secs(5),
                    connection::open(&self.origin, self.tls.clone()),
                )
                .await
                .unwrap_or(Err(Failure::Network))?,
            };
            // Never replay a transmitted body here; the common owner controls one exact retry.
            let response = connection
                .sender
                .send_request(prepared)
                .await
                .map_err(|_| Failure::Network)?;
            observe(ExchangeEvent::Headers(response.status().as_u16()));
            const MAX: usize = 196_608;
            if response
                .headers()
                .get("Content-Length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|v| v > MAX as u64)
            {
                return Err(Failure::TooLarge);
            }
            let header = |name: &str| {
                response
                    .headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .filter(|v| v.len() <= 128)
                    .map(str::to_owned)
            };
            let status = response.status().as_u16();
            let retry_after = header("Retry-After");
            let date = header("Date");
            let last_modified = header("Last-Modified");
            let mut body = response.into_body();
            let mut raw = Vec::new();
            while let Some(frame) = body.frame().await {
                if let Ok(bytes) = frame.map_err(|_| Failure::Network)?.into_data() {
                    if raw.len() + bytes.len() > MAX {
                        return Err(Failure::TooLarge);
                    }
                    raw.extend_from_slice(&bytes);
                }
            }
            let body = String::from_utf8(raw).map_err(|_| Failure::InvalidBody)?;
            observe(ExchangeEvent::Complete);
            Ok(Response {
                status,
                body,
                retry_after,
                date,
                last_modified,
            })
        };
        tokio::time::timeout(timeout, operation)
            .await
            .unwrap_or_else(|_| {
                observe(ExchangeEvent::Deadline(timeout.as_millis() as u64));
                Err(Failure::Network)
            })
    }
    #[cfg(test)]
    pub(crate) fn synthetic_peer(origin: &str, root: Option<&[u8]>) -> Result<Self> {
        let parsed = url::Url::parse(origin).map_err(|_| Error::InvalidRequest)?;
        if parsed.scheme() != "https"
            || parsed.host_str() != Some("127.0.0.1")
            || parsed.port().is_none()
            || parsed.path() != "/"
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::InvalidRequest);
        }
        let mut roots = RootCertStore::empty();
        if let Some(root) = root {
            roots
                .add(rustls::pki_types::CertificateDer::from(root.to_vec()))
                .map_err(|_| Error::InvalidRequest)?;
        }
        Ok(Self {
            origin: parsed,
            user_agent: "Govorun-Windows/2".into(),
            version: None,
            tls: tls(roots),
            prepared: Mutex::new(None),
        })
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.close_prepared(None);
    }
}
impl Gateway for Client {
    fn prepare_connection(&self, request_id: String) {
        self.warm(request_id);
    }
    fn close_connection(&self, request_id: Option<&str>) {
        self.close_prepared(request_id);
    }
    fn send(&self, request: Request) -> Future<'_, std::result::Result<Response, Failure>> {
        Box::pin(self.perform(request, &|_| {}))
    }
    fn send_observed<'a>(
        &'a self,
        request: Request,
        observe: &'a (dyn Fn(ExchangeEvent) + Send + Sync),
    ) -> Future<'a, std::result::Result<Response, Failure>> {
        Box::pin(self.perform(request, observe))
    }
}
