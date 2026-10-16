//! The desktop account workflow. The agent supplies OS facts and storage effects.
#[path = "actions.rs"]
mod actions;
use super::snapshot;
use crate::{
    agent::{AgentClient, Cancellation, Capability, Request},
    features::{
        billing::host::{Events, Opener},
        snapshot_gate::SnapshotGate,
    },
};
use govorun_core::{AccountSession, AuthDevice, AuthFlow, AuthIntent, AuthProvider, Error};
use crate::features::ports::Reply;
use crate::features::runtime::session::Session as HostSession;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::Semaphore;

#[derive(Default)]
struct State {
    serial: u64,
    operation: Option<u64>,
    waiting: bool,
    message: Option<String>,
}
pub struct AccountFeature {
    core: AccountSession,
    auth: Arc<AuthFlow>,
    agent: AgentClient,
    native: HostSession,
    documents: std::sync::OnceLock<Arc<crate::features::documents::DocumentsHost>>,
    state: Mutex<State>,
    snapshots: tokio::sync::Mutex<()>,
    gate: SnapshotGate,
    instance: String,
    events: Events,
    open: Opener,
    requests: Arc<Semaphore>,
    stopped: AtomicBool,
}
impl AccountFeature {
    pub fn new(
        core: AccountSession,
        auth: Arc<AuthFlow>,
        agent: AgentClient,
        native: HostSession,
        events: Events,
        open: Opener,
        requests: Arc<Semaphore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            core,
            auth,
            agent,
            native,
            documents: std::sync::OnceLock::new(),
            state: Mutex::new(State::default()),
            snapshots: tokio::sync::Mutex::new(()),
            gate: SnapshotGate::default(),
            instance: crate::instance::next(),
            events,
            open,
            requests,
            stopped: AtomicBool::new(false),
        })
    }
    pub fn attach_documents(
        &self,
        documents: Arc<crate::features::documents::DocumentsHost>,
    ) -> Result<(), &'static str> {
        self.documents
            .set(documents)
            .map_err(|_| "documents_already_initialized")
    }
    fn documents(&self) -> Result<&Arc<crate::features::documents::DocumentsHost>, Error> {
        self.documents.get().ok_or(Error::StorageUnavailable)
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("account feature state")
    }
    fn scope(&self) -> String {
        format!(
            "{}:{}",
            self.instance,
            self.core.snapshot().session_generation
        )
    }
    fn current(&self, epoch: u64) -> bool {
        !self.stopped.load(Ordering::SeqCst) && self.core.snapshot().session_generation == epoch
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
    async fn agent(&self, method: &str, params: Value) -> Reply {
        self.agent
            .call(
                Request {
                    capability: Capability::named(method)?,
                    params,
                },
                Duration::from_secs(20),
                &Cancellation::default(),
            )
            .await
    }
    async fn view(&self) -> Reply {
        let mut facts = self.agent("account.localFacts", json!({})).await?;
        facts["syncMode"] = serde_json::to_value(self.documents().map_err(|v| v.code())?.mode())
            .map_err(|_| "account_invalid_response")?;
        let projection = self.core.native_projection();
        let state = self.state();
        let view = snapshot::snapshot(
            &projection,
            &facts,
            self.scope(),
            state.operation.is_some(),
            state.waiting,
            state.message.clone(),
        )?;
        serde_json::to_value(view).map_err(|_| "account_invalid_response")
    }
    pub async fn snapshot(&self) -> Reply {
        let _read = self.snapshots.lock().await;
        self.view().await
    }
    pub async fn publish(&self) {
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }
        let _read = self.snapshots.lock().await;
        if let Ok(view) = self.view().await {
            if self.gate.changed(&view) {
                (self.events)("account.changed", view);
            }
        }
    }
    pub async fn invoke(self: &Arc<Self>, method: &str, params: Value) -> Reply {
        if self.stopped.load(Ordering::SeqCst) {
            return Err("native_closing");
        }
        if method == "account.snapshot" {
            return self.snapshot().await;
        }
        let action = if method == "account.callback" {
            "callback"
        } else if method == "account.action" {
            params["action"].as_str().ok_or("invalid_parameters")?
        } else {
            return Err("unsupported_command");
        };
        if action != "callback" && params["scope"] != self.scope() {
            return Err("account_session_changed");
        }
        if action == "openLink" {
            let url = match params["link"].as_str() {
                Some("agreement" | "privacy") => "https://govorun.club/legal",
                _ => return Err("invalid_parameters"),
            };
            self.open(url.to_owned()).await?;
            return self.snapshot().await;
        }
        if !matches!(
            action,
            "signIn"
                | "callback"
                | "cancelSignIn"
                | "signOut"
                | "refresh"
                | "merge"
                | "keepLocal"
                | "requestSync"
                | "recoverStorage"
                | "restartStorage"
                | "revokeDevice"
                | "replaceDevice"
                | "deleteAccount"
        ) {
            return Err("invalid_parameters");
        }
        let snapshot = self.core.snapshot();
        if matches!(
            action,
            "refresh" | "merge" | "keepLocal" | "requestSync" | "revokeDevice" | "deleteAccount"
        ) && !snapshot.signed_in
        {
            return Err("account_signed_out");
        }
        if matches!(
            action,
            "restartStorage" | "revokeDevice" | "replaceDevice" | "deleteAccount"
        ) && params["confirmed"] != true
        {
            return Err("invalid_parameters");
        }
        if action == "callback" && snapshot.signed_in {
            return self.snapshot().await;
        }
        let cancel = matches!(action, "cancelSignIn" | "signOut");
        if self.state().operation.is_some() && !cancel {
            return Err("account_busy");
        }
        let provider: Option<AuthProvider> = if action == "signIn" {
            let provider: AuthProvider = serde_json::from_value(params["provider"].clone())
                .map_err(|_| "invalid_parameters")?;
            let view = self.view().await?;
            if !view["providers"].as_array().is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value["id"] == params["provider"] && value["enabled"] == true)
            }) {
                return Err("invalid_parameters");
            }
            Some(provider)
        } else {
            None
        };
        if action == "restartStorage" && self.view().await?["storage"]["available"] == true {
            return Err("invalid_parameters");
        }
        if matches!(action, "revokeDevice" | "replaceDevice") {
            let view = self.view().await?;
            let id = params["deviceId"].as_str().ok_or("invalid_parameters")?;
            if !view["devices"].as_array().is_some_and(|values| {
                values.iter().any(|device| {
                    device["id"] == id && (action == "replaceDevice" || device["current"] != true)
                })
            }) {
                return Err("invalid_parameters");
            }
            if action == "replaceDevice" && view["replacingDevice"] != true {
                return Err("invalid_parameters");
            }
        }
        if action != "callback" && params["scope"] != self.scope() {
            return Err("account_session_changed");
        }
        let permit = self
            .requests
            .clone()
            .try_acquire_owned()
            .map_err(|_| "account_busy")?;
        let intent = match action {
            "signIn" => self.auth.reserve_begin().map_err(|error| error.code())?,
            "cancelSignIn" | "signOut" | "restartStorage" => self.auth.reserve_cancel(),
            _ => self.auth.pending_intent(),
        };
        let epoch = self.core.snapshot().session_generation;
        let serial = {
            let mut state = self.state();
            state.serial += 1;
            state.operation = Some(state.serial);
            state.message = None;
            if action == "signIn" || action == "callback" {
                state.waiting = true;
            } else if cancel {
                state.waiting = false;
            }
            state.serial
        };
        let host = self.clone();
        let action = action.to_owned();
        tokio::spawn(async move {
            let result = host.execute(&action, params, provider, intent, epoch).await;
            {
                let mut state = host.state();
                if state.operation == Some(serial) {
                    state.operation = None;
                    if let Err(error) = result {
                        if error != Error::SessionChanged && !host.stopped.load(Ordering::SeqCst) {
                            state.message = Some(
                                if !host.core.snapshot().signed_in
                                    && matches!(
                                        action.as_str(),
                                        "signIn" | "callback" | "replaceDevice"
                                    )
                                {
                                    "account_sign_in_failed"
                                } else {
                                    error.code()
                                }
                                .into(),
                            );
                        }
                        state.waiting = false;
                    } else if action != "signIn" {
                        state.waiting = false;
                    }
                }
            }
            let _ = host
                .native
                .publish_authority(host.core.native_projection())
                .await;
            host.publish().await;
            drop(permit);
        });
        self.publish().await;
        self.snapshot().await
    }
    async fn device(&self) -> Result<AuthDevice, Error> {
        let value = self
            .agent("account.deviceIdentity", json!({}))
            .await
            .map_err(|_| Error::StorageUnavailable)?;
        Ok(AuthDevice {
            installation_id: value["installationId"]
                .as_str()
                .ok_or(Error::InvalidResponse)?
                .into(),
            name: value["deviceName"]
                .as_str()
                .ok_or(Error::InvalidResponse)?
                .into(),
        })
    }
    async fn open(&self, url: String) -> Result<(), &'static str> {
        let opener = self.open.clone();
        tokio::task::spawn_blocking(move || opener(&url))
            .await
            .map_err(|_| "account_browser_unavailable")?
    }
}
