//! Private native callers share the same restored account and protected vault.
//! Renderer commands cannot invoke this path or receive a pass/raw-text reply.
use super::AccountHost;
use crate::{
    agent::{storage::AgentStore, AgentClient},
    features::stylization::Stylization,
};
use govorun_core::{
    style_pass_vault::{format::Format, Repository, VaultStore},
    stylization::Gateway,
    Error,
};
#[cfg(any(test, feature = "private-fixtures"))]
use crate::{features::ports::HostRequest, features::stylization::{commands::Command, ipc::Deliver}};
#[cfg(any(test, feature = "private-fixtures"))]
use serde_json::json;
use std::sync::Arc;
#[cfg(any(test, feature = "private-fixtures"))]
use std::sync::atomic::Ordering;

impl AccountHost {
    pub(super) fn initialize_stylization(&self, agent: AgentClient) -> Result<(), &'static str> {
        let network = self.network.get().ok_or("account_invalid_configuration")?;
        let gateway = (network.gateway)()?;
        self.initialize_stylization_with_ports(
            Arc::new(AgentStore::new(agent)),
            network.format,
            gateway,
            Arc::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            }),
        )
    }
    pub fn initialize_stylization_with_ports(
        &self,
        store: Arc<dyn VaultStore>,
        format: Format,
        gateway: Arc<dyn Gateway>,
        clock: crate::features::stylization::Clock,
    ) -> Result<(), &'static str> {
        let core = self
            .core
            .get()
            .ok_or("account_storage_unavailable")?
            .clone();
        let owner = Stylization::new(
            core.clone(),
            Arc::new(Repository::new(Arc::new(crate::style_pass_vault::SessionStore::new(store, format)), format)),
            gateway,
            clock,
        );
        let documents=Arc::downgrade(&self.documents_owner()?);
        let ports=Arc::downgrade(self.document_ports.get().ok_or("documents_unavailable")?);
        owner.attach_statistics(Arc::new(move |epoch,failed,slow| {
            let core=core.clone(); let documents=documents.clone(); let ports=ports.clone();
            Box::pin(async move {
                let profile=core.profile();
                if core.current(epoch).is_err() {return;}
                let Some(profile)=profile else {return;};
                if let Some(documents)=documents.upgrade() {
                    if let Err(code)=documents.record_style_processing(profile.id().to_owned(),failed,slow).await {
                        if let Some(ports)=ports.upgrade() {(ports.diagnostics())("stats_sync_failed",code.into());}
                    }
                }
            })
        })).map_err(|error| error.code())?;
        self.stylization
            .set(owner.clone())
            .map_err(|_| "stylization_already_initialized")?;
        self.billing
            .get()
            .ok_or("billing_unavailable")?
            .attach_stylization(owner.clone())?;
        self.document_ports
            .get()
            .ok_or("documents_unavailable")?
            .attach_stylization(owner)
            .map_err(|error| error.code())
    }
    pub async fn start_stylization(self: &Arc<Self>) -> Result<(), &'static str> {
        let ports = self.document_ports.get().ok_or("documents_unavailable")?;
        let owner = self.stylization.get().ok_or("styles_unavailable")?;
        // Read the durable selection first; unknown configuration suspends work.
        let _ = ports.stylization_enabled().await;
        let initially_online = ports.network_available().await.unwrap_or(false);
        let weak = Arc::downgrade(ports);
        let enabled = Arc::new(move || {
            let weak = weak.clone();
            Box::pin(async move {
                let ports = weak.upgrade().ok_or(Error::SessionChanged)?;
                if !ports.network_available().await? {
                    return Ok(false);
                }
                ports.stylization_enabled().await
            })
                as govorun_core::Future<'static, govorun_core::Result<bool>>
        });
        let weak = Arc::downgrade(ports);
        let online = Arc::new(move || {
            let weak = weak.clone();
            Box::pin(async move {
                let Some(ports) = weak.upgrade() else {
                    return false;
                };
                let _ = ports.stylization_enabled().await;
                ports.network_available().await.unwrap_or(false)
            }) as govorun_core::Future<'static, bool>
        });
        let weak = Arc::downgrade(self);
        let changed = Arc::new(move |_| {
            let weak = weak.clone();
            tokio::spawn(async move {
                if let Some(host) = weak.upgrade() {
                    host.publish_documents(false).await;
                }
            });
        });
        owner
            .start_background(enabled, online, initially_online, changed)
            .map_err(|error| error.code())
    }
    #[cfg(any(test, feature = "private-fixtures"))]
    pub(super) fn handle_stylization(self: &Arc<Self>, request: HostRequest) {
        let cancelling = request.params["operation"] == "stylizationCancel";
        // Reserve in pipe order, before spawning: an immediately following
        // cancellation cannot miss a request that has not been polled yet.
        let activity = if cancelling {
            None
        } else {
            Some(self.foreground.enter())
        };
        let registration = if cancelling {
            None
        } else {
            Some(
                self.core
                    .get()
                    .ok_or(Error::StorageUnavailable)
                    .and_then(|core| {
                        self.style_requests
                            .register(&request.id, core.snapshot().session_generation)
                    }),
            )
        };
        let host = self.clone();
        tokio::spawn(async move {
            let result = if cancelling {
                host.style_requests
                    .cancel(request.params)
                    .map(|cancelled| json!({"cancelled":cancelled}))
            } else if host.stopping.load(Ordering::SeqCst) {
                Err(Error::SessionChanged)
            } else if let Some(Err(error)) = &activity {
                Err(*error)
            } else {
                async {
                    let guard = registration
                        .as_ref()
                        .ok_or(Error::InvalidRequest)?
                        .as_ref()
                        .map_err(|error| *error)?;
                    let owner = host.stylization.get().ok_or(Error::StorageUnavailable)?;
                    let native = host.native.get().ok_or(Error::StorageUnavailable)?.clone();
                    let deliver: Deliver = Arc::new(move |value| {
                        let native = native.clone();
                        Box::pin(async move {
                            native
                                .invoke("bridge.reply", value)
                                .await
                                .map(|_| ())
                                .map_err(|_| Error::NetworkUnavailable)
                        })
                    });
                    let command = Command::decode(request.params)?;
                    if matches!(command, Command::BeginRecording { .. } | Command::Process { .. } | Command::Prepare { .. } | Command::PreparationPlan { .. } | Command::JoinPreparation {..}|Command::SettingsChanged {..}) {
                        let ports = host.document_ports.get().ok_or(Error::StorageUnavailable)?;
                        let needs_network = if let Command::PreparationPlan { session_generation, .. } | Command::BeginRecording { session_generation } = &command {
                            host.core.get().ok_or(Error::StorageUnavailable)?.current(*session_generation)?;
                            true
                        } else { false };
                        let selected = ports.stylization_enabled().await?;
                        let online = if needs_network { ports.network_available().await? } else { false };
                        guard.run_configured(owner, command, deliver, selected, online).await
                    } else {
                        guard.run(owner, command, deliver).await
                    }
                }
                .await
            };
            if let Some(native) = host.native.get() {
                let mut reply = match result {
                    Ok(result) => json!({"ok":true,"result":result}),
                    Err(error) => {
                        let mut value = error.wire();
                        value["stylizationFailure"] =
                            json!(govorun_core::stylization::failure::decide(error));
                        value["ok"] = json!(false);
                        value
                    }
                };
                reply["id"] = json!(request.id);
                reply["channel"] = json!("account");
                // No account.project callback while the protected vault waits
                // for an ACK, and no renderer/public event carries this result.
                let _ = native.invoke("bridge.reply", reply).await;
            }
            drop(registration);
            drop(activity);
        });
    }
}
