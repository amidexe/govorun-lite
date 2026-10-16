use crate::account_storage::NativeStorage;
use crate::features::billing::host::{BillingHost, Events, Opener};
#[cfg(any(test,feature="private-fixtures"))]
use govorun_core::account_protocol::auth;
use govorun_core::{AccountSession, AuthConfig, AuthFlow, Error};
#[cfg(any(test,feature="private-fixtures"))]
use crate::features::ports::HostRequest;
use crate::features::runtime::session::Session as HostSession;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use tokio::sync::Semaphore;
#[path = "account_dictation.rs"]
mod dictation;
pub use dictation::{OsContext, OsEffects};
#[path = "account_connectivity.rs"]
mod connectivity;
#[path = "account_documents.rs"]
mod documents;
#[path = "account_preparation.rs"]
mod preparation;
#[path = "account_recording_connection.rs"]
mod recording_connection;
#[path = "account_promotions.rs"]
mod promotions;
#[path = "account_stylization.rs"]
mod stylization;
#[path="account_initialization.rs"]
mod initialization;
#[path = "account_foreground.rs"]
mod foreground;
pub struct NetworkPorts {
    pub platform: &'static str,
    pub format: crate::style_pass_vault::format::Format,
    pub account: Arc<dyn Fn(&str) -> Result<Arc<dyn crate::Transport>, &'static str> + Send + Sync>,
    pub gateway: Arc<dyn Fn() -> Result<Arc<dyn crate::stylization::Gateway>, &'static str> + Send + Sync>,
}


/// One account lifetime composed with the Rust physical stores and workers.
/// Frozen private fixtures can substitute the former pipe without granting it authority.
pub struct AccountHost {
    network: OnceLock<NetworkPorts>,
    self_host: Mutex<Option<Arc<crate::features::self_host::SelfHost>>>,
    documents: tokio::sync::watch::Sender<Option<Arc<crate::features::documents::DocumentsHost>>>,
    document_ports: OnceLock<Arc<crate::features::documents::native::NativeDocuments>>,
    core: OnceLock<AccountSession>,
    physical_agent: OnceLock<crate::agent::AgentClient>,
    auth: OnceLock<Arc<AuthFlow>>,
    #[cfg(any(test,feature="private-fixtures"))]
    native_auth_intent: Mutex<u64>,
    native: OnceLock<HostSession>,
    billing: OnceLock<Arc<BillingHost>>,
    account_feature: OnceLock<Arc<crate::features::account::host::AccountFeature>>,
    stylization: OnceLock<Arc<crate::features::stylization::Stylization>>,
    connectivity_task: OnceLock<tokio::task::AbortHandle>,
    style_requests: Arc<crate::features::stylization::ipc::Requests>,
    dictation_preparation: OnceLock<
        Arc<crate::features::dictation::preparation::Preparation<preparation::AccountPreparation>>,
    >,
    events: Events,
    open: Opener,
    requests: Arc<Semaphore>,
    foreground: Arc<foreground::Foreground>,
    stopping: AtomicBool,
}

impl AccountHost {
    pub fn attach_network(&self, ports: NetworkPorts) -> Result<(), &'static str> {
        self.network.set(ports).map_err(|_| "network_already_composed")
    }
    pub async fn recording_capture_ready(&self, detail: String) {
        if let Some(ports) = self.document_ports.get() { ports.recording_capture_ready(detail).await; }
    }
    pub async fn recording_feedback_error(&self, code: &'static str) {
        if let Some(ports) = self.document_ports.get() {
            (ports.diagnostics())("recording_feedback_failed", code.into());
        }
    }
    pub fn with_effects(events: Events, open: Opener) -> Arc<Self> {
        Arc::new(Self {
            network: OnceLock::new(),
            self_host: Mutex::new(None),
            documents: tokio::sync::watch::channel(None).0,
            document_ports: OnceLock::new(),
            core: OnceLock::new(),
            physical_agent: OnceLock::new(),
            auth: OnceLock::new(),
            #[cfg(any(test,feature="private-fixtures"))]
            native_auth_intent: Mutex::new(0),
            native: OnceLock::new(),
            requests: Arc::new(Semaphore::new(32)),
            foreground: Arc::new(Default::default()),
            billing: OnceLock::new(),
            account_feature: OnceLock::new(),
            events,
            open,
            stylization: OnceLock::new(),
            connectivity_task: OnceLock::new(),
            style_requests: Arc::new(Default::default()),
            dictation_preparation: OnceLock::new(),
            stopping: AtomicBool::new(false),
        })
    }

    pub fn attach_self_host(&self, owner: Arc<crate::features::self_host::SelfHost>) {
        *self.self_host.lock().unwrap() = Some(owner);
    }
    pub fn self_host(&self) -> Option<Arc<crate::features::self_host::SelfHost>> {
        self.self_host.lock().unwrap().clone()
    }
    /// Native effect lifetimes use the existing account boundary, never a second session.
    pub fn account_session(&self) -> Option<AccountSession> { self.core.get().cloned() }
    #[cfg(any(test,feature="private-fixtures"))]
    pub fn fixture_core(&self) -> AccountSession {
        self.core.get().expect("private composed account fixture").clone()
    }

    pub fn documents_available(&self) -> bool {
        self.documents.borrow().is_some()
    }

    pub async fn stop(&self) {
        if let Some(task) = self.connectivity_task.get() {
            task.abort();
        }
        if let Some(preparation) = self.dictation_preparation.get() {
            preparation.cancel();
        }
        self.style_requests.close();
        self.foreground.close();
        if let Some(owner) = self.stylization.get() {
            owner.shutdown().await;
        }
        if let Some(owner) = self.self_host() {
            owner.stop();
        }
        if let Some(billing) = self.billing.get() {
            billing.stop();
        }
        if let Some(account) = self.account_feature.get() {
            account.stop();
        }
        if let Ok(documents) = self.documents_owner() {
            documents.stop().await;
        }
        if let Some(ports) = self.document_ports.get() {
            ports.drain_diagnostics().await;
        }
        if let Some(native) = self.native.get() {
            // The acknowledgement follows all earlier worker RPC frames. It
            // also closes the worker's admission of any new account operation.
            native.flush().await;
        }
        self.stopping.store(true, Ordering::SeqCst);
        // Wake composition waiters so they observe shutdown without clearing
        // the document owner or interrupting its durable writes.
        self.documents.send_modify(|_| {});
        // Complete credential rotation / durable logout before closing the
        // worker's storage pipe. Dropping an in-flight write is not a rollback.
        let _requests = self.requests.acquire_many(32).await;
        self.foreground.drain().await;
        if let Some(core) = self.core.get() {
            core.disconnect();
        }
    }

    pub fn attach_physical_agent(
        &self,
        agent: crate::agent::AgentClient,
    ) -> Result<(), &'static str> {
        self.physical_agent
            .set(agent)
            .map_err(|_| "storage_already_composed")
    }

    pub async fn account(
        self: &Arc<Self>,
        method: &str,
        params: serde_json::Value,
    ) -> crate::features::ports::Reply {
        self.account_feature
            .get()
            .ok_or("account_storage_unavailable")?
            .invoke(method, params)
            .await
    }
    pub async fn publish_account(&self) {
        if let Some(account) = self.account_feature.get() {
            account.publish().await;
        }
    }

    pub async fn refresh_for_activity(&self, max_age: std::time::Duration) -> bool {
        if self.stopping.load(Ordering::SeqCst) {
            return false;
        }
        let Ok(_permit) = self.requests.clone().acquire_owned().await else {
            return false;
        };
        if self.stopping.load(Ordering::SeqCst) {
            return false;
        }
        if let Some(core) = self.core.get() {
            return matches!(
                core.refresh_for_activity(max_age).await,
                Ok(_) | Err(Error::SignedOut | Error::SessionChanged)
            );
        }
        false
    }

    #[cfg(any(test,feature="private-fixtures"))]
    pub fn handle(self: &Arc<Self>, request: HostRequest) {
        if request.method == "account"
            && request.params["operation"]
                .as_str()
                .is_some_and(|operation| {
                    operation == "stylizationCancel"
                        || crate::features::stylization::commands::Command::recognizes(operation)
                })
        {
            self.handle_stylization(request);
            return;
        }
        let host = self.clone();
        // Ledger decisions run while the native vault is locked. They have no
        // authority effects and must not re-enter native account publication.
        let publish_authority = !matches!(
            request.params["operation"].as_str(),
            Some(
                "stylePassCrypto"
                    | "reconnectRecovery"
                    | "styleUsageLedger"
                    | "prepareStyleIssuance"
                    | "styleIssuanceJournal"
                    | "styleInventory"
                    | "styleJobPolling"
                    | "styleJobResponse"
                    | "styleStartAdmission"
                    | "styleTextAdmission"
                    | "creditRetirementWire"
                    | "styleReserveBinding"
                    | "styleReserveTransition"
                    | "sameStyleAccessScope"
                    | "reserveMaintenance"
                    | "creditRecovery"
                    | "creditLease"
                    | "creditRecoveryWire"
                    | "creditRecoveryRequest"
            )
        );
        let permit = self.requests.clone().try_acquire_owned();
        // Reserve user intent in pipe order. Spawned tasks may start in a
        // different order; an old Begin must not run after a later Cancel.
        let intent = if permit.is_ok() && !self.stopping.load(Ordering::SeqCst) {
            self.auth
                .get()
                .ok_or(Error::StorageUnavailable)
                .and_then(|flow| {
                    auth::reserve_native(flow, &request.params, &self.native_auth_intent)
                })
        } else {
            Err(Error::NetworkUnavailable)
        };
        tokio::spawn(async move {
            let result = match intent {
                Ok(intent) if !host.stopping.load(Ordering::SeqCst) => {
                    match (host.core.get(), host.auth.get()) {
                        (Some(core), Some(flow)) => {
                            auth::dispatch(core, flow, intent, request.params).await
                        }
                        _ => Err(Error::StorageUnavailable),
                    }
                }
                Err(error) => Err(error),
                _ => Err(Error::NetworkUnavailable),
            };
            if let Some(native) = host.native.get() {
                // Publish authority before completing a native consumer's HTTP
                // operation (including expiry/failure). Platform DTO consumers
                // cannot briefly re-use their previous authorization afterward.
                if publish_authority {
                    if let Some(core) = host.core.get() {
                        let _ = native
                            .publish_authority(core.native_projection())
                            .await;
                    }
                }
                let mut reply = match result {
                    Ok(result) => json!({"id": request.id, "ok": true, "result": result}),
                    Err(error) => {
                        let mut reply = error.wire();
                        reply["id"] = json!(request.id);
                        reply["ok"] = json!(false);
                        reply
                    }
                };
                reply["channel"] = json!("account");
                let _ = native.invoke("bridge.reply", reply).await;
            }
            drop(permit);
        });
    }
}

/// Trusted composition chooses capabilities; native/UI commands cannot enable import.
pub fn configured_auth(origin: &str, platform: &str) -> Result<AuthConfig, &'static str> {
    let config = AuthConfig::new(origin, platform)
        .map_err(|error| error.code())?;
    Ok(
        if govorun_core::configured_refresh_import_allowed(origin) {
            config.with_refresh_import()
        } else {
            config
        },
    )
}
