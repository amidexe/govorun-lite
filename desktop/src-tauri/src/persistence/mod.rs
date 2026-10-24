//! OS storage adapters at the existing account and settings locations.
pub mod account_ports;
#[cfg(target_os = "macos")]
pub mod database_defaults;
#[cfg(target_os = "windows")]
pub mod database_files;
pub mod defaults_documents;
pub mod document_db;
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

/// The runtime snapshot names a base that cannot be opened; dictation never waits on it.
pub fn mark_storage(snapshot: &mut Value) {
    if document_db::base_unavailable() {
        snapshot["storageUnavailable"] = serde_json::json!(true);
    }
}
#[cfg(target_os = "macos")]
type Base = database_defaults::Documents<Arc<macos_defaults::Defaults>>;
#[cfg(target_os = "windows")]
type Base = database_files::Documents;
/// The journal that hears about moves: the one of the newest transport over the base.
type Listener = Arc<std::sync::Mutex<std::sync::Weak<journal::Journal>>>;
/// One base per file and process. A worker restart builds a new transport while the old one
/// still lives, so the new one takes the open base instead of opening and checking it again
/// (since the journal moved into the base, a restart waited on that). Held weakly: the base
/// closes with the last transport.
fn shared_base(path: std::path::PathBuf, open: impl FnOnce(document_db::Notice) -> Base) -> (Arc<Base>, Listener) {
    static BASES: std::sync::Mutex<Vec<(std::path::PathBuf, std::sync::Weak<Base>, Listener)>> =
        std::sync::Mutex::new(Vec::new());
    let mut bases = BASES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    bases.retain(|(_, base, _)| base.strong_count() > 0);
    if let Some((_, base, listener)) = bases.iter().find(|(known, _, _)| *known == path) {
        if let Some(base) = base.upgrade() {
            return (base, listener.clone());
        }
    }
    let listener: Listener = Arc::default();
    let base = Arc::new(open(notice(listener.clone())));
    bases.push((path, Arc::downgrade(&base), listener.clone()));
    (base, listener)
}
/// One journal row per move code and launch. The journal lives in the same base, so it is
/// bound once the base exists and held weakly: a strong link would keep the base and its
/// connection alive after the transport ends. The journal's own call may be the one that
/// opened the base, so the row is written on a thread of its own, never inside that call.
fn notice(journal: Listener) -> document_db::Notice {
    Box::new(move |code| {
        let current = journal.lock().map(|journal| journal.upgrade());
        let Ok(Some(journal)) = current else { return };
        let write = move || {
            let _ = journal.invoke(
                "data.diagnostic",
                &serde_json::json!({"event":"local_data_moved","status":null,"reason":code}),
            );
        };
        #[cfg(target_os = "macos")]
        std::thread::spawn(move || objc2::rc::autoreleasepool(|_| write()));
        #[cfg(target_os = "windows")]
        std::thread::spawn(write);
    })
}

pub struct Transport {
    native: HostSession,
    account: account_ports::AccountPorts,
    journal: Arc<journal::Journal>,
    benchmark: std::sync::Mutex<Option<Arc<crate::features::benchmark::runner::Runner>>>,
    observations: Arc<os_facts::Observations>,
    #[cfg(target_os = "windows")]
    documents: Arc<database_files::Documents>,
    #[cfg(target_os = "windows")]
    protected: Arc<secure_files::Store<windows_dpapi::Dpapi>>,
    #[cfg(target_os = "macos")]
    defaults: Arc<macos_defaults::Defaults>,
    #[cfg(target_os = "macos")]
    documents: Arc<database_defaults::Documents<Arc<macos_defaults::Defaults>>>,
    #[cfg(target_os = "macos")]
    identity: String,
    #[cfg(target_os = "macos")]
    protected: Arc<
        macos_secrets::Owner<database_defaults::Account<Arc<macos_defaults::Defaults>>, macos_secrets::Files>,
    >,
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
        let root = {
            let root = namespace.root;
            let _guard = file_documents::WRITER
                .lock()
                .map_err(|_| "storage_unavailable")?;
            stable_transaction::restore_before_open(&stable_transaction::Files(root.clone()))
                .map_err(|_| "storage_unavailable")?;
            root
        };
        #[cfg(target_os = "windows")]
        let protected = Arc::new(secure_files::Store::new(
            root.join("secure"),
            windows_dpapi::Dpapi::new(namespace.entropy),
        ));
        #[cfg(target_os = "macos")]
        let folder = dirs::data_dir().ok_or("storage_unavailable")?.join(identity);
        #[cfg(target_os = "macos")]
        let (documents, listener) = shared_base(folder.join(document_db::FILE), |notice| {
            database_defaults::Documents::new(folder.clone(), defaults.clone(), Some(notice))
        });
        #[cfg(target_os = "macos")]
        let protected = Arc::new(macos_secrets::Owner::new(
            database_defaults::Account(documents.clone()),
            macos_secrets::Files::with_user_keys(folder.join("secrets"), macos_secrets::SystemUserKeys),
            identity.into(),
        ));
        #[cfg(target_os = "windows")]
        let (documents, listener) = shared_base(root.join(document_db::FILE), |notice| {
            database_files::Documents::new(root, Some(notice))
        });
        let journal = Arc::new(journal::Journal::new(documents.clone()));
        if let Ok(mut current) = listener.lock() {
            *current = Arc::downgrade(&journal);
        }
        Ok(Self {
            benchmark: std::sync::Mutex::new(None),
            native: native.into(),
            journal,
            account: account_ports::AccountPorts::new(),
            observations: Arc::new(os_facts::Observations::new()),
            documents,
            protected,
            #[cfg(target_os = "macos")]
            defaults,
            #[cfg(target_os = "macos")]
            identity: identity.into(),
        })
    }
    fn handles(key: &str) -> bool {
        document_db::DOCUMENTS.contains(&key)
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
                let documents = self.documents.clone();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "macos")]
                    {
                        objc2::rc::autoreleasepool(|_| {
                            observations.load(&os_facts::saved(documents.as_ref()))
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
                let documents = self.documents.clone();
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
                        let vault = windows_secrets::invoke(store.as_ref(),&documents.scope("account"),"secret.read",
                            &serde_json::json!({"key":"stylePass.vault","allowInteraction":recovery}));
                        if recovery {
                            return match vault["status"].as_str() {
                                Some("available"|"missing") => Ok(Value::Null),
                                _ => Err("storage_unavailable"),
                            };
                        }
                        let session = windows_secrets::invoke(store.as_ref(),&documents.scope("account"),"secret.read",
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
                let documents = self.documents.clone();
                let method = method.to_owned();
                return tokio::task::spawn_blocking(move || {
                    windows_secrets::invoke(store.as_ref(), &documents.scope("account"), &method, &params)
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
                let store = self.documents.clone();
                return tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "windows")]
                    {
                        let owner = params["key"].as_str().unwrap_or_default().to_owned();
                        file_documents::invoke(&store.scope(&owner), &method, &params)
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
