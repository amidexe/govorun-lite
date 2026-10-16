//! Direct user-selected HTTP. No official session, quota, fallback or timer.
use govorun_core::self_hosted_flow::{configuration::Configuration, endpoint, wire};

pub use govorun_core::features::self_host::network::Failure;

pub struct Client {
    http: reqwest::Client,
}
impl Client {
    pub fn new(configuration: &Configuration) -> Result<Self, Failure> {
        let mut builder = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none());
        if configuration.proxy_enabled {
            let proxy =
                endpoint::proxy(&configuration.proxy_url).map_err(|_| Failure::Configuration)?;
            builder = builder
                .proxy(reqwest::Proxy::all(proxy.as_str()).map_err(|_| Failure::Configuration)?);
        }
        // Neither an official request budget nor a product timeout applies here.
        let http = builder.build().map_err(|_| Failure::Network)?;
        Ok(Self { http })
    }
    pub async fn complete(
        &self,
        configuration: &Configuration,
        key: Option<&str>,
        text: &str,
    ) -> Result<String, Failure> {
        let request =
            wire::request(configuration, key, text).map_err(|_| Failure::Configuration)?;
        let mut outgoing = self
            .http
            .post(request.endpoint)
            .header("Accept", "application/json")
            .header("Cache-Control", "no-store")
            .header("Content-Type", "application/json");
        if let Some((name, value)) = request.key_header {
            outgoing = outgoing.header(name, value);
        }
        let body = serde_json::to_vec(&request.body).map_err(|_| Failure::Configuration)?;
        let incoming = outgoing
            .body(body)
            .send()
            .await
            .map_err(|_| Failure::Network)?;
        let status = incoming.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(Failure::Http(status));
        }
        // User-owned processing has no official output-size limit. Keep the
        // request cancellable by dropping this future; never log body/key/URL.
        let bytes = incoming.bytes().await.map_err(|_| Failure::Network)?;
        let body = serde_json::from_slice(&bytes).map_err(|_| Failure::InvalidResponse)?;
        wire::response(request.protocol, &body).map_err(|_| Failure::InvalidResponse)
    }
}
