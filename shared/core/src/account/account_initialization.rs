//! Compose one account lifetime from caller-provided network and storage ports.
use super::*;
impl AccountHost {
    pub async fn initialize(
        self: &Arc<Self>,
        native: impl Into<HostSession>,
        origin: &str,
        terms: u32,
    ) -> Result<(), &'static str> {
        let transport = (self.network.get().ok_or("account_invalid_configuration")?.account)(origin)?;
        self.initialize_configured(native.into(), transport, terms, origin, true)
            .await
    }

    pub async fn initialize_with_transport(
        self: &Arc<Self>,
        native: impl Into<HostSession>,
        transport: Arc<dyn govorun_core::Transport>,
        terms: u32,
    ) -> Result<(), &'static str> {
        self.initialize_configured(
            native.into(),
            transport,
            terms,
            "https://accounts.govorun.club",
            false,
        )
        .await
    }

    #[cfg(any(test,feature="private-fixtures"))]
    pub async fn initialize_with_runtime_transport(
        self: &Arc<Self>,
        native: impl Into<HostSession>,
        transport: Arc<dyn govorun_core::Transport>,
        terms: u32,
    ) -> Result<(), &'static str> {
        self.initialize_configured(
            native.into(),
            transport,
            terms,
            "https://accounts.govorun.club",
            true,
        )
        .await
    }

    async fn initialize_configured(
        self: &Arc<Self>,
        native: HostSession,
        transport: Arc<dyn govorun_core::Transport>,
        terms: u32,
        origin: &str,
        compose_stylization: bool,
    ) -> Result<(), &'static str> {
        let runtime = native.invoke("runtime.snapshot", json!({})).await?;
        let agent = match self.physical_agent.get() {
            Some(agent) => agent.clone(),
            None => crate::agent::AgentClient::with_transport(Arc::new(native.clone()), &runtime)?,
        };
        // Credentials, OAuth attempts and checkout journals must use the same
        // composed physical client as documents and the pass vault.
        let storage = Arc::new(NativeStorage::from_client(agent.clone()));
        let core =
            AccountSession::new(storage.clone(), transport, terms).map_err(|error| error.code())?;
        let config = configured_auth(origin, self.network.get().ok_or("account_invalid_configuration")?.platform)?;
        self.auth
            .set(Arc::new(AuthFlow::new(core.clone(), config, storage)))
            .map_err(|_| "account_already_initialized")?;
        self.core
            .set(core.clone())
            .map_err(|_| "account_already_initialized")?;
        let weak = Arc::downgrade(self);
        agent.bind_generation(Arc::new(move || {
            weak.upgrade().and_then(|host| {
                (!host.stopping.load(Ordering::SeqCst))
                    .then(|| {
                        host.core
                            .get()
                            .map(|core| core.snapshot().session_generation)
                    })
                    .flatten()
            })
        }))?;
        self.native
            .set(native.clone())
            .map_err(|_| "account_already_initialized")?;
        let scheme = runtime["callbackScheme"]
            .as_str()
            .ok_or("account_invalid_configuration")?
            .to_owned();
        self.billing
            .set(BillingHost::new(
                core.clone(),
                native.clone(),
                terms,
                scheme,
                self.events.clone(),
                self.open.clone(),
                self.requests.clone(),
            ))
            .map_err(|_| "account_already_initialized")?;
        self.account_feature
            .set(crate::features::account::host::AccountFeature::new(
                core.clone(),
                self.auth.get().expect("auth configured").clone(),
                agent.clone(),
                native.clone(),
                self.events.clone(),
                self.open.clone(),
                self.requests.clone(),
            ))
            .map_err(|_| "account_already_initialized")?;
        self.initialize_documents(core.clone(), agent.clone())?;
        let diagnostics_owner = Arc::downgrade(self);
        agent.bind_diagnostics(Arc::new(move || {
            if let Some(owner) = diagnostics_owner.upgrade() {
                if !owner.stopping.load(Ordering::SeqCst) {
                    (owner.events)("diagnostics.changed", serde_json::Value::Null);
                }
            }
        }))?;
        let mut changes = core.subscribe();
        // Startup never requests OS authorization. If the existing item is
        // inaccessible, publish its recovery state without erasing it; only
        // the user's recoverStorage action may open the Keychain dialog.
        let _ = core.restore(false).await;
        if compose_stylization {
            self.initialize_stylization(agent.clone())?;
            // A denied/corrupt vault cannot prevent offline recognition or
            // account recovery. Processing still requires the protected ACK.
            let _ = self
                .stylization
                .get()
                .expect("composed stylization")
                .bind_profile()
                .await;
        }
        // The initial view below already publishes restoration. Consume
        // that notification before reading it; later changes remain pending.
        changes.borrow_and_update();
        native
            .publish_authority(core.native_projection())
            .await?;
        self.publish_documents(true).await;
        if compose_stylization {
            self.start_stylization().await?;
            self.start_connectivity(&agent)?;
        }
        let host = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = native.wait_closed() => { core.disconnect(); break; },
                    changed = changes.changed() => {
                        if changed.is_err() || host.stopping.load(Ordering::SeqCst) { break; }
                        if let Some(owner) = host.stylization.get() { let _ = owner.bind_profile().await; }
                        if native.publish_authority(core.native_projection()).await.is_err() { break; }
                        host.publish_documents(true).await;
                        host.publish_billing().await;
                        host.publish_account().await;
                    }
                }
            }
        });
        let host = self.clone();
        let mut changes = self.core.get().expect("initialized account").subscribe();
        tokio::spawn(async move {
            loop {
                let (Some(native), Some(core)) = (host.native.get(), host.core.get()) else {
                    break;
                };
                tokio::select! {
                    _ = native.wait_closed() => break,
                    changed = changes.changed() => { if changed.is_err() { break; } },
                    _ = tokio::time::sleep(core.activity_refresh_delay()) => {
                        if host.stopping.load(Ordering::SeqCst) { break; }
                        host.refresh_for_activity(std::time::Duration::ZERO).await;
                    }
                }
            }
        });
        Ok(())
    }

}
