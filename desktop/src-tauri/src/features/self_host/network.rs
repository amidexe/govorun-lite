//! Explicit self-host HTTP adapter, with no fallback to official access.
pub use govorun_core::features::self_host::network::*;
use govorun_core_http::self_hosted::Client;
use govorun_core::{self_hosted_flow::configuration::Configuration, Future};
pub struct Http;
impl Network for Http {
    fn complete<'a>(
        &'a self,
        configuration: &'a Configuration,
        key: Option<&'a str>,
        text: &'a str,
    ) -> Future<'a, Result<String, Failure>> {
        Box::pin(async move {
            Client::new(configuration)?
                .complete(configuration, key, text)
                .await
        })
    }
}
