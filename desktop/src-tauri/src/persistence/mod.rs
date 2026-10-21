//! OS storage adapters at the existing account and settings locations.
pub mod account_ports;
pub mod defaults_documents;
pub mod device_name;
pub mod file_documents;
pub mod installation_identity;
pub mod journal;
#[cfg(target_os = "macos")]
pub mod macos_defaults;
pub mod macos_envelope;
#[cfg(target_os = "macos")]
pub mod macos_user_keychain;
pub mod macos_profile;
pub mod macos_secrets;
pub mod os_facts;
pub mod secure_files;
pub mod private_files;
pub mod stable_transaction;
#[cfg(target_os = "windows")]
pub mod windows_dpapi;
#[cfg(target_os = "windows")]
pub mod windows_protocol;
#[cfg(target_os = "windows")]
pub mod windows_profile;
pub mod windows_secrets;
use crate::agent::{AgentFuture, AgentTransport};
use crate::features::runtime::session::Session as HostSession;
use serde_json::Value;
use std::sync::Arc;

pub struct Transport {
    native: HostSession,
    account: account_ports::AccountPorts,
    journal: Arc<journal::Journal>,
    benchmark: std::sync::Mutex<Option<Arc<crate::features::benchmark::runner::Runner>>>,
    observations: Arc<os_facts::Observations>,
    #[cfg(target_os = "windows")]
    files: Arc<file_documents::Files>,
    #[cfg(target_os = "windows")]
    protected: Arc<secure_files::Store<windows_dpapi::Dpapi>>,
    #[cfg(target_os = "macos")]
    defaults: Arc<macos_defaults::Defaults>,
    #[cfg(target_os = "macos")]
    identity: String,
    #[cfg(target_os = "macos")]
    protected:
        Arc<macos_secrets::Owner<Arc<macos_defaults::Defaults>, macos_secrets::Files>>,
}
impl Transport {
    /// Complete direct OS capabilities used by the shared feature owners.
    pub fn complete_snapshot(&self, snapshot: &mut Value) -> Result<(), &'static str> {
        self.account.configure(snapshot)?;
        let capabilities = snapshot["capabilities"]
            .as_array_mut()
            .ok_or("native_invalid_capabilities")?;
        // Durable repositories and Rust feature owners consume these values.
        // Production no longer needs a second native cache acknowledgement.
        capabilities.retain(|value| value != "data.configure");
        for capability in [
            "account.localFacts",
            "account.deviceIdentity",
            "account.localEffect",
            "data.localFacts",
            "data.styleEffect",
            "data.diagnostic",
            "diagnostics.localFacts",
            "diagnostics.localEffect",
            "benchmark.localFacts",
            "benchmark.localEffect",
        ] {
            if !capabilities.iter().any(|value| value == capability) {
                capabilities.push(serde_json::json!(capability));
            }
        }
        Ok(())
    }
    pub fn new(native: impl Into<HostSession>, identity: &str) -> Result<Self, &'static str> {
        #[cfg(target_os = "macos")]
        let defaults = Arc::new(
            macos_defaults::Defaults::new(identity.into()).map_err(|_| "storage_unavailable")?,
        );
        #[cfg(target_os = "windows")]
        let namespace = {
            let configured = std::env::var_os("GOVORUN_DATA_DIRECTORY");
            let local = dirs::data_local_dir();
            windows_profile::select(identity, configured.as_deref(), local.as_deref(),
                cfg!(feature = "private-fixtures"))?
        };
        #[cfg(target_os = "windows")]
        let files = {
            let root = namespace.root;
            let _guard = file_documents::WRITER
                .lock()
                .map_err(|_| "storage_unavailable")?;
            stable_transaction::restore_before_open(&stable_transaction::Files(root.clone()))
                .map_err(|_| "storage_unavailable")?;
            Arc::new(file_documents::Files::new(root))
        };
        #[cfg(target_os = "windows")]
        let protected = {
            let root = files.root().join("secure");
            Arc::new(secure_files::Store::new(
                root,
                windows_dpapi::Dpapi::new(namespace.entropy),
            ))
        };
        #[cfg(target_os = "macos")]
        let protected = Arc::new(macos_secrets::Owner::new(
            defaults.clone(),
            macos_secrets::Files::with_user_keys(
                dirs::data_dir().ok_or("storage_unavailable")?.join(identity).join("secrets"),
                macos_secrets::SystemUserKeys,
            ),
            identity.into(),
        ));
        Ok(Self {
            benchmark: std::sync::Mutex::new(None),
            native: native.into(),
            #[cfg(target_os = "macos")]
            journal: Arc::new(journal::Journal::new(defaults.clone())),
            #[cfg(target_os = "windows")]
            journal: Arc::new(journal::Journal::new(files.clone())),
            account: account_ports::AccountPorts::new(),
            observations: Arc::new(os_facts::Observations::new()),
            #[cfg(target_os = "windows")]
            files,
            #[cfg(target_os = "windows")]
            protected,
            #[cfg(target_os = "macos")]
            defaults,
            #[cfg(target_os = "macos")]
            identity: identity.into(),
            #[cfg(target_os = "macos")]
            protected,
        })
    }
    fn handles(key: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            matches!(
                key,
                "dictionary.document"
                    | "statistics.document"
                    | "sync.content"
                    | "sync.preferences"
                    | "sync.enrollment"
                    | "styles.document"
                    | "selfHost.document"
                    | "clipboard.document"
                    | "hotkeys.document"
                    | "recording.document"
            )
        }
        #[cfg(target_os = "macos")]
        {
            matches!(
                key,
                "dictionary.document"
                    | "statistics.document"
                    | "sync.content"
                    | "sync.preferences"
                    | "sync.enrollment"
                    | "styles.document"
                    | "selfHost.document"
                    | "clipboard.document"
                    | "hotkeys.document"
                    | "recording.document"
            )
        }
    }
}
impl AgentTransport for Transport {
    fn bind_benchmark(
        &self,
        owner: Arc<crate::features::benchmark::runner::Runner>,
    ) -> Result<(), &'static str> {
        let mut current = self.benchmark.lock().map_err(|_| "benchmark_unavailable")?;
        if current.is_some() {
            return Err("benchmark_already_composed");
        }
        *current = Some(owner);
        Ok(())
    }

    fn bind_diagnostics(&self, changed: crate::agent::Diagnostics) -> Result<(), &'static str> {
        self.journal.bind(changed)
    }
    fn bind_connectivity(&self, handler: crate::agent::Connectivity) -> Result<(), &'static str> {
        self.observations.bind(handler)
    }
    fn bind_generation(&self, generation: crate::agent::Generation) -> Result<(), &'static str> {
        self.account.bind(generation)
    }
    fn invoke<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        Box::pin(async move {
            if matches!(method, "benchmark.localFacts" | "benchmark.localEffect") {
                let owner = self
                    .benchmark
                    .lock()
                    .map_err(|_| "benchmark_unavailable")?
                    .clone()
                    .ok_or("benchmark_unavailable")?;
                return owner.invoke(method, &params).await;
            }
            if matches!(
                method,
                "data.diagnostic" | "diagnostics.localFacts" | "diagnostics.localEffect"
            ) {
                let journal = self.journal.clone();
                let method = method.to_owned();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "macos")]
                    {
                        objc2::rc::autoreleasepool(|_| journal.invoke(&method, &params))
                    }
                    #[cfg(target_os = "windows")]
                    {
                        journal.invoke(&method, &params)
                    }
                })
                .await
                .map_err(|_| "storage_unavailable")?;
            }

            if method == "data.localFacts" {
                return self.observations.facts(&params);
            }
            if method == "data.styleEffect" {
                if params["effect"] != "loadApplications"
                    || !params.as_object().is_some_and(|fields| fields.len() == 1)
                {
                    return Err("invalid_parameters");
                }
                let observations = self.observations.clone();
                #[cfg(target_os = "macos")]
                let defaults = self.defaults.clone();
                #[cfg(target_os = "macos")]
                let identity = self.identity.clone();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "macos")]
                    {
                        objc2::rc::autoreleasepool(|_| {
                            observations.load(&os_facts::saved(defaults.as_ref()))
                        })
                    }
                    #[cfg(target_os = "windows")]
                    {
                        observations.load(&[])
                    }
                })
                .await
                .map_err(|_| "styles_unavailable")?;
            }
            if method == "account.localFacts" || method == "account.localEffect" {
                if method == "account.localFacts"
                    && !params.as_object().is_some_and(|v| v.is_empty())
                {
                    return Err("invalid_parameters");
                }
                let effect = if method == "account.localEffect" {
                    Some(self.account.effect(&params)?)
                } else {
                    None
                };
                if effect == Some("prepareSignIn") {
                    self.account.prepare_sign_in()?;
                    return Ok(Value::Null);
                }
                #[cfg(target_os = "windows")]
                let files = self.files.clone();
                let store = self.protected.clone();
                let recovery = effect == Some("recoverVault");
                #[cfg(target_os = "macos")]
                let isolated = macos_profile::isolated(&self.identity);
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "macos")]
                    { objc2::rc::autoreleasepool(|_| {
                        if recovery { return store.recover_vault().map(|_| Value::Null); }
                        Ok(account_ports::facts(store.account_access(), device_name::current(),isolated))
                    }) }
                    #[cfg(target_os = "windows")]
                    {
                        let vault = windows_secrets::invoke(store.as_ref(),files.as_ref(),"secret.read",
                            &serde_json::json!({"key":"stylePass.vault","allowInteraction":recovery}));
                        if recovery {
                            return match vault["status"].as_str() {
                                Some("available"|"missing") => Ok(Value::Null),
                                _ => Err("storage_unavailable"),
                            };
                        }
                        let session = windows_secrets::invoke(store.as_ref(),files.as_ref(),"secret.read",
                            &serde_json::json!({"key":"account.session","allowInteraction":false}));
                        Ok(account_ports::facts(account_ports::windows_access(&session,&vault),device_name::current(),false))
                    }
                }).await.map_err(|_| "storage_unavailable")?;
            }
            if method == "account.deviceIdentity" {
                if !params.as_object().is_some_and(|fields| fields.is_empty()) {
                    return Err("invalid_parameters");
                }
                #[cfg(target_os = "windows")]
                let protected = self.protected.clone();
                #[cfg(target_os = "macos")]
                let defaults = self.defaults.clone();
                #[cfg(target_os = "macos")]
                let identity = self.identity.clone();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "windows")]
                    let id = installation_identity::windows(protected.as_ref());
                    #[cfg(target_os = "macos")]
                    let id = objc2::rc::autoreleasepool(|_| installation_identity::macos(defaults.as_ref(), &installation_identity::SystemPreviousIdentity(identity)));
                    id.map(|id|serde_json::json!({"installationId":id,"deviceName":device_name::current()}))
                        .map_err(|_| "storage_unavailable")
                }).await.map_err(|_| "storage_unavailable")?;
            }
            #[cfg(target_os = "windows")]
            if params["key"].as_str().is_some_and(windows_secrets::handles) {
                let store = self.protected.clone();
                let files = self.files.clone();
                let method = method.to_owned();
                return tokio::task::spawn_blocking(move || {
                    windows_secrets::invoke(store.as_ref(), files.as_ref(), &method, &params)
                })
                .await
                .map_err(|_| "storage_unavailable");
            }
            #[cfg(target_os = "macos")]
            if params["key"].as_str().is_some_and(windows_secrets::handles) {
                let store = self.protected.clone();
                let method = method.to_owned();
                return tokio::task::spawn_blocking(move || {
                    objc2::rc::autoreleasepool(|_| store.invoke(&method, &params))
                })
                .await
                .map_err(|_| "storage_unavailable");
            }
            if method.starts_with("storage.") && params["key"].as_str().is_some_and(Self::handles) {
                let method = method.to_string();
                #[cfg(target_os = "windows")]
                let store = self.files.clone();
                #[cfg(target_os = "macos")]
                let store = self.defaults.clone();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "windows")]
                    {
                        file_documents::invoke(store.as_ref(), &method, &params)
                    }
                    #[cfg(target_os = "macos")]
                    {
                        objc2::rc::autoreleasepool(|_| {
                            defaults_documents::invoke(store.as_ref(), &method, &params)
                        })
                    }
                })
                .await
                .map_err(|_| "storage_unavailable");
            }
            self.native.invoke(method, params).await
        })
    }
    fn self_host_secret<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        self.invoke(method, params)
    }
}
