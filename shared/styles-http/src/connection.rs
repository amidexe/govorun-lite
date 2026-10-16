//! Public library APIs only: TCP/TLS before HTTP, with ownership of one connection.
use govorun_core::stylization::Failure;
use http_body_util::Full;
use hyper::{body::Bytes, client::conn::http1};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioIo};
use rustls::{pki_types::ServerName, ClientConfig};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio_rustls::TlsConnector;
use tower_service::Service;

pub(super) struct Connection {
    pub sender: http1::SendRequest<Full<Bytes>>,
    driver: JoinHandle<()>,
}
impl Connection {
    pub fn usable(&self) -> bool {
        !self.driver.is_finished() && !self.sender.is_closed()
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.driver.abort();
    }
}
pub(super) async fn open(origin: &url::Url, tls: Arc<ClientConfig>) -> Result<Connection, Failure> {
    let host = origin.host_str().ok_or(Failure::Other)?;
    let name = ServerName::try_from(host.to_owned()).map_err(|_| Failure::Security)?;
    // Use Hyper's standard DNS/IPv4/IPv6 connector (Happy Eyeballs), not a
    // custom sequential address loop. No keepalive probes or connection pool.
    let mut connector = HttpConnector::new();
    connector.enforce_http(false);
    connector.set_nodelay(true);
    connector.set_keepalive(None);
    connector.set_connect_timeout(Some(Duration::from_secs(5)));
    let uri = origin.as_str().parse().map_err(|_| Failure::Other)?;
    let tcp = connector
        .call(uri)
        .await
        .map_err(|_| Failure::Network)?
        .into_inner();
    let stream = TlsConnector::from(tls)
        .connect(name, tcp)
        .await
        .map_err(|error| {
            if error
                .get_ref()
                .is_some_and(|cause| cause.is::<rustls::Error>())
            {
                Failure::Security
            } else {
                Failure::Network
            }
        })?;
    let (sender, connection) = http1::handshake(TokioIo::new(stream))
        .await
        .map_err(|_| Failure::Network)?;
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(Connection { sender, driver })
}
