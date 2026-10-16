//! Lifetime and facts of the in-process host. Private frozen pipe fixtures may
//! substitute a worker; product owners use no second account cache.
use crate::agent::{AgentFuture, AgentTransport};
use crate::features::ports::{NativeEvent, Reply};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::watch;
#[derive(Clone)]
pub struct Session(Arc<Backend>);
enum Backend {
    Local {
        facts: Value,
        closed: watch::Sender<bool>,
        exited: Arc<dyn Fn(NativeEvent) + Send + Sync>,
    },
    #[cfg(feature = "private-fixtures")]
    Pipe(Arc<dyn Lifecycle>),
}
/// Physical lifecycle effects supplied by the host or a private fixture.
#[cfg(feature = "private-fixtures")]
pub trait Lifecycle: AgentTransport {
    fn is_closed(&self) -> bool;
    fn wait_closed(&self) -> crate::Future<'_, ()>;
    fn stop(&self) -> crate::Future<'_, ()>;
    fn shutdown_when_idle(&self) -> crate::Future<'_, Result<(), &'static str>>;
}
#[cfg(feature = "private-fixtures")]
impl From<Arc<dyn Lifecycle>> for Session {
    fn from(port: Arc<dyn Lifecycle>) -> Self { Self(Arc::new(Backend::Pipe(port))) }
}
impl Session {
    pub fn local(facts: Value) -> Result<Self, &'static str> {
        Self::local_events(facts, Arc::new(|_| {}))
    }
    pub fn local_events(
        facts: Value,
        exited: Arc<dyn Fn(NativeEvent) + Send + Sync>,
    ) -> Result<Self, &'static str> {
        if !facts.is_object()
            || facts["recordingOwner"] != "rust"
            || facts["protocolVersion"] != 1
            || facts["maintenanceProtocol"] != 1
            || !matches!(facts["platform"].as_str(), Some("macos" | "windows"))
            || !facts["capabilities"].is_array()
        {
            return Err("native_invalid_runtime");
        }
        Ok(Self(Arc::new(Backend::Local {
            facts,
            closed: watch::channel(false).0,
            exited,
        })))
    }
    pub fn is_closed(&self) -> bool {
        match self.0.as_ref() {
            Backend::Local { closed, .. } => *closed.borrow(),
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.is_closed(),
        }
    }
    pub async fn wait_closed(&self) {
        match self.0.as_ref() {
            Backend::Local { closed, .. } => {
                let mut receiver = closed.subscribe();
                while !*receiver.borrow_and_update() {
                    if receiver.changed().await.is_err() {
                        break;
                    }
                }
            }
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.wait_closed().await,
        }
    }
    pub async fn stop(&self) {
        match self.0.as_ref() {
            Backend::Local { closed, exited, .. } => {
                if !closed.send_replace(true) {
                    exited(NativeEvent {
                        event: "runtime.exited".into(),
                        data: json!({"unexpected":false}),
                    });
                }
            }
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.stop().await,
        }
    }
    /// Call only after recording admission, account writes and OS leases drain.
    pub async fn shutdown_when_idle(&self) -> Result<(), &'static str> {
        match self.0.as_ref() {
            Backend::Local { .. } => {
                self.stop().await;
                Ok(())
            }
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.shutdown_when_idle().await,
        }
    }
    /// Only the private frozen pipe consumes the compatibility view.
    /// Local OS ports read Core's generation directly, without an ACK or cache.
    pub async fn publish_authority(&self, _value: Value) -> Result<(), &'static str> {
        #[cfg(feature = "private-fixtures")]
        if let Backend::Pipe(service) = self.0.as_ref() {
            service
                .invoke("account.project", json!({"schemaVersion":1,"value":_value}))
                .await?;
        }
        Ok(())
    }
    pub async fn flush(&self) {
        #[cfg(feature = "private-fixtures")]
        if let Backend::Pipe(service) = self.0.as_ref() {
            let _ = service.invoke("account.flush", json!({})).await;
        }
    }
    pub async fn snapshot(&self) -> Reply {
        self.invoke("runtime.snapshot", json!({})).await
    }
    pub async fn cancel_fixture(&self, params: Value) -> Reply {
        #[cfg(feature = "private-fixtures")]
        if let Backend::Pipe(service) = self.0.as_ref() {
            return service.invoke("recording.cancel", params).await;
        }
        let _ = params;
        Err("native_unavailable")
    }
    pub async fn invoke(&self, method: &str, params: Value) -> Reply {
        match self.0.as_ref() {
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.invoke(method, params).await,
            Backend::Local { facts, .. } => {
                if self.is_closed() {
                    return Err("native_closing");
                }
                if !params.as_object().is_some_and(|p| p.is_empty()) {
                    return Err("invalid_parameters");
                }
                match method {
                    "runtime.snapshot" => Ok(facts.clone()),
                    "runtime.prepareMaintenance" => Ok(json!({"accepted":true})),
                    _ => Err("unsupported_command"),
                }
            }
        }
    }
    pub async fn invoke_self_host_secret(&self, _method: &str, _params: Value) -> Reply {
        match self.0.as_ref() {
            #[cfg(feature = "private-fixtures")]
            Backend::Pipe(service) => service.self_host_secret(_method, _params).await,
            Backend::Local { .. } => Err("unsupported_command"),
        }
    }
}
impl AgentTransport for Session {
    fn invoke<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        Box::pin(Session::invoke(self, method, params))
    }
    fn self_host_secret<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        Box::pin(self.invoke_self_host_secret(method, params))
    }
}

/// Build-selected identity and trust boundary, identical for both OS hosts.
/// The consent version stays at the existing approved value, 12.
pub fn facts(platform: &str, identity: &str, name: &str, version: &str, build: &str) -> Value {
    json!({"platform":platform,"bundleIdentifier":identity,"displayName":name,
      "protocolVersion":1,"maintenanceProtocol":1,"recordingOwner":"rust","phase":"idle",
      "accountProtocol":1,"accountOrigin":govorun_core::configured_account_origin(),
      "onlineTermsVersion":12,"callbackScheme":option_env!("GOVORUN_AUTH_SCHEME").unwrap_or("govorun"),
      "buildFacts":{"version":version,"platformBuild":version,"buildID":build},
      "readiness":null,"capabilities":["runtime.snapshot","runtime.prepareMaintenance","runtime.shutdown","runtime.shutdownWhenIdle","recording.cancel",
        "storage.read","storage.write","storage.delete","secret.read","secret.write","secret.delete","secret.restart",
        "hotkeys.localFacts","hotkeys.localEffect","dictation.snapshot","dictation.action"]})
}
