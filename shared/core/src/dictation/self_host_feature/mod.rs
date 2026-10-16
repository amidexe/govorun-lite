//! Own the UI workflow and direct HTTP without an official account barrier.
pub mod network;
mod operation;
use super::snapshot_gate::SnapshotGate;
use crate::features::cancellation::Cancellation;
use govorun_core::{
    self_hosted_flow::{
        configuration::Provider,
        controller::{Action, Controller, Outcome, Work},
        settings::KeyState,
        stored::Stored,
    },
    Document, DocumentStore, SecretStore,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::Mutex as AsyncMutex;

pub type Events = Arc<dyn Fn(&str, Value) + Send + Sync>;
pub struct SelfHost {
    store: Arc<dyn DocumentStore>,
    secrets: Arc<dyn SecretStore>,
    network: Arc<dyn network::Network>,
    events: Events,
    controller: Mutex<Controller>,
    gate: AsyncMutex<()>,
    cancellation: Mutex<Option<Cancellation>>,
    snapshots: SnapshotGate,
    stopped: AtomicBool,
    route_changed: std::sync::OnceLock<Arc<dyn Fn(Option<bool>) + Send + Sync>>,
}
impl SelfHost {
    pub fn new(
        store: Arc<dyn DocumentStore>,
        secrets: Arc<dyn SecretStore>,
        network: Arc<dyn network::Network>,
        events: Events,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            secrets,
            network,
            events,
            controller: Mutex::new(Controller::new(crate::instance::next())),
            gate: AsyncMutex::new(()),
            cancellation: Mutex::new(None),
            snapshots: SnapshotGate::default(),
            stopped: AtomicBool::new(false),
            route_changed: std::sync::OnceLock::new(),
        })
    }
    pub fn attach_route_changed(&self, port: Arc<dyn Fn(Option<bool>) + Send + Sync>) {
        let _ = self.route_changed.set(port);
    }
    fn notify_route(&self, route: Option<bool>) {
        if let Some(port) = self.route_changed.get() {
            port(route);
        }
    }
    pub fn busy(&self) -> bool {
        self.controller.lock().unwrap().busy()
    }
    pub fn operation_id(&self) -> Option<String> {
        self.controller
            .lock()
            .unwrap()
            .operation_id()
            .map(str::to_owned)
    }
    async fn load(&self) -> Result<(Document, Stored), &'static str> {
        let document = self
            .store
            .read_document("selfHost.document".into())
            .await
            .map_err(|_| "self_host_storage_unavailable")?
            .ok_or("self_host_storage_unavailable")?;
        let stored = Stored::decode(document.value.clone())
            .map_err(|_| "self_host_invalid_configuration")?;
        Ok((document, stored))
    }
    async fn save(&self, document: &Document, stored: Stored) -> Result<(), &'static str> {
        let route_enabled = stored.settings.route_enabled;
        let value = stored
            .encode()
            .map_err(|_| "self_host_invalid_configuration")?;
        if value == document.value {
            self.notify_route(Some(route_enabled));
            return Ok(());
        }
        let result = self
            .store
            .write_document(
                "selfHost.document".into(),
                Some(document.revision.clone()),
                value,
            )
            .await;
        // Revoke reporting immediately after the durable decision, before
        // protected-key reads or the native cache acknowledgement can suspend.
        self.notify_route(result.as_ref().ok().map(|_| route_enabled));
        result.map_err(|_| "self_host_storage_unavailable")?;
        Ok(())
    }
    /// Route admission reads no protected key or native readiness mirror.
    pub async fn route_enabled(&self) -> Result<bool, &'static str> {
        self.route_flags().await.map(|(route, _)| route)
    }
    /// Foreground routing needs only durable settings, never an API-key read.
    pub async fn route_flags(&self) -> Result<(bool, bool), &'static str> {
        let _gate = self.gate.lock().await;
        let (_, stored) = self.load().await?;
        Ok((stored.settings.route_enabled, stored.settings.styling_enabled))
    }
    pub async fn snapshot(&self) -> Result<Value, &'static str> {
        let _gate = self.gate.lock().await;
        let (_, stored) = self.load().await?;
        let (key, _) = self
            .key(stored.settings.configuration.provider, false)
            .await;
        Ok(self.controller.lock().unwrap().snapshot(&stored, key))
    }
    pub async fn publish(&self) {
        if self.stopped.load(Ordering::Acquire) {
            return;
        }
        if let Ok(snapshot) = self.snapshot().await {
            if self.snapshots.changed(&snapshot) {
                (self.events)("selfhost.changed", snapshot);
            }
        }
    }
    pub async fn action(self: &Arc<Self>, params: Value) -> Result<Value, &'static str> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("self_host_stopped");
        }
        let action: Action =
            serde_json::from_value(params).map_err(|_| "self_host_invalid_request")?;
        let gate = self.gate.lock().await;
        let (document, mut stored) = self.load().await?;
        let (key, _) = self
            .key(stored.settings.configuration.provider, false)
            .await;
        let mut next = self.controller.lock().unwrap().clone();
        let work = next
            .begin(action, &mut stored, key)
            .map_err(|_| "self_host_invalid_request")?;
        let cancelled = next.cancelled();
        if cancelled {
            if let Some(cancel) = self.cancellation.lock().unwrap().as_ref() {
                cancel.cancel();
            }
        }
        let provider = stored.settings.configuration.provider;
        if let Err(error) = self.save(&document, stored).await {
            // Never start effects after a failed durable disable/consent write.
            self.controller.lock().unwrap().storage_action_failed();
            if let Some(cancel) = self.cancellation.lock().unwrap().as_ref() {
                cancel.cancel();
            }
            return Err(error);
        }
        *self.controller.lock().unwrap() = next;
        if !matches!(work, Work::None) {
            let cancellation = Cancellation::default();
            *self.cancellation.lock().unwrap() = Some(cancellation.clone());
            let owner = self.clone();
            tokio::spawn(async move {
                owner.run(work, provider, cancellation).await;
            });
        }
        drop(gate);
        self.publish().await;
        if !self.stopped.load(Ordering::Acquire) {
            (self.events)("styles.changed", Value::Null);
        }
        self.snapshot().await
    }
    async fn key(&self, provider: Provider, interaction: bool) -> (KeyState, Option<String>) {
        let result = self
            .secrets
            .read_secret(secret(provider), interaction)
            .await;
        match result {
            Ok(None) => (KeyState::Missing, None),
            Ok(Some(value)) => {
                let state = serde_json::from_value::<KeyState>(value["state"].clone())
                    .unwrap_or(KeyState::Inaccessible);
                let key = value["key"]
                    .as_str()
                    .filter(|text| !text.trim().is_empty())
                    .map(str::to_owned);
                if state == KeyState::Saved && key.is_none() {
                    (KeyState::Inaccessible, None)
                } else {
                    (state, key)
                }
            }
            Err(_) => (KeyState::Inaccessible, None),
        }
    }
    async fn write_key(&self, provider: Provider, key: &str) -> bool {
        let result = if key.trim().is_empty() {
            self.secrets.delete_secret(secret(provider)).await
        } else {
            self.secrets
                .write_secret(secret(provider), json!({"key":key.trim()}), true)
                .await
        };
        if result.is_err() {
            return false;
        }
        let (state, _) = self.key(provider, false).await;
        state
            == if key.trim().is_empty() {
                KeyState::Missing
            } else {
                KeyState::Saved
            }
    }
    pub async fn process(
        &self,
        text: &str,
    ) -> Result<String, network::Failure> {
        use network::Failure;
        let (configuration, key) = {
            let _gate = self.gate.lock().await;
            if self.stopped.load(Ordering::Acquire) || self.controller.lock().unwrap().busy() {
                return Err(Failure::Configuration);
            }
            let (_, stored) = self.load().await.map_err(|_| Failure::Configuration)?;
            let (state, key) = self.key(stored.settings.configuration.provider, true).await;
            stored
                .settings
                .admit_request(stored.format, state)
                .map_err(|_| Failure::Configuration)?;
            if !stored.settings.styling_enabled {
                return Err(Failure::Configuration);
            }
            (stored.settings.configuration, key)
        };
        let response = self
            .network
            .complete(&configuration, key.as_deref(), text)
            .await?;
        let (_, stored) = self.load().await.map_err(|_| Failure::Configuration)?;
        if self.stopped.load(Ordering::Acquire)
            || stored.settings.configuration != configuration
            || !stored.settings.consent(stored.format)
            || !stored.settings.styling_enabled
        {
            return Err(Failure::Configuration);
        }
        Ok(response)
    }
    pub async fn set_styling(&self, enabled: bool) -> Result<(), &'static str> {
        let gate = self.gate.lock().await;
        let (document, mut stored) = self.load().await?;
        let (key, _) = self
            .key(stored.settings.configuration.provider, false)
            .await;
        if enabled {
            stored
                .settings
                .admit_request(stored.format, key)
                .map_err(|_| "self_host_invalid_configuration")?;
            if self.controller.lock().unwrap().busy() || !stored.settings.verified() {
                return Err("self_host_invalid_configuration");
            }
        } else if let Some(cancel) = self.cancellation.lock().unwrap().as_ref() {
            cancel.cancel();
        }
        stored.settings.styling_enabled = enabled;
        self.save(&document, stored).await?;
        drop(gate);
        self.publish().await;
        Ok(())
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(cancel) = self.cancellation.lock().unwrap().as_ref() {
            cancel.cancel();
        }
    }
}
fn secret(provider: Provider) -> String {
    format!("selfHost.{}", provider.name())
}
