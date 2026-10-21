#[cfg(feature = "private-fixtures")]
#[path = "service_fixture.rs"]
mod private_fixture;
#[path = "service_lifecycle.rs"]
mod lifecycle;
pub(crate) use lifecycle::deliver;
use govorun_core::features::ports::{NativeEvent, Reply};
use crate::features::runtime::session::Session as HostSession;
#[cfg(feature="private-fixtures")]
use private_fixture::{NativeService, Options};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
#[cfg(feature="private-fixtures")]
use std::sync::atomic::AtomicU64;

#[derive(Clone)]
struct Resources { #[cfg(feature="private-fixtures")] inspection: Option<(PathBuf, Vec<String>)>, identity: Option<String>, models: Option<(PathBuf, PathBuf)> }

impl Resources {
    fn locate(app: &AppHandle) -> Result<Self, &'static str> {
        if cfg!(debug_assertions) && !cfg!(feature = "packaged-native") {
            #[cfg(feature="private-fixtures")]
            {
            if let Some(executable) = std::env::var_os("GOVORUN_NATIVE_WORKER") {
                let mut args: Vec<String> = std::env::var("GOVORUN_NATIVE_WORKER_ARGS")
                    .ok().map(|text| serde_json::from_str(&text)).transpose().map_err(|_| "invalid_worker_arguments")?
                    .unwrap_or_default();
                if args.len() > 8 || args.iter().any(|arg| arg.len() > 1024) { return Err("invalid_worker_arguments"); }
                // Development never opens the owner's account, models, microphone or data stores.
                args.push("--bridge-inspect".into());
                return Ok(Self { inspection: Some((executable.into(), args)), identity: None, models: None });
            }
            // Real-host acceptance uses the isolated compiled profile and the
            // packaged resource path. A normal unbundled debug app stays closed.
            if app.config().identifier != "local.govorun.tauri-acceptance" {
                return Err("native_not_configured");
            }
            }
            #[cfg(not(feature="private-fixtures"))]
            { return Err("native_not_configured"); }
        }
        let resources = app.path().resource_dir().map_err(|_| "native_not_configured")?;
        let identity = Some(app.config().identifier.clone());
        let local = app.path().local_data_dir().map_err(|_| "native_not_configured")?;
        let (bundled, root) = crate::model_store::application_paths(&resources, &local, &app.config().identifier)?;
        Ok(Self { #[cfg(feature="private-fixtures")] inspection: None, identity, models: Some((bundled, root)) })
    }
}

#[cfg(feature = "private-fixtures")]
pub(crate) struct PrivateRuntimeAdapter(pub fn(Value) -> Value);

#[derive(Clone)]
pub struct Services {
    #[cfg(feature = "private-fixtures")]
    private_callbacks: Arc<std::sync::Mutex<Option<fn(&str, Value) -> Option<Value>>>>,
    #[cfg(feature = "private-fixtures")]
    private_runtime: Option<fn(Value) -> Value>,
    worker: Result<Resources, &'static str>,
    current: Arc<Mutex<Option<HostSession>>>,
    connecting: Arc<Mutex<()>>,
    #[cfg(feature="private-fixtures")]
    generation: Arc<AtomicU64>,
    account: Arc<Mutex<Option<Arc<crate::account_host::AccountHost>>>>,
    agent: Arc<Mutex<Option<crate::agent::AgentClient>>>,
    clipboard: Arc<Mutex<Option<Arc<crate::features::clipboard::Clipboard>>>>,
    hotkeys: Arc<Mutex<Option<Arc<crate::features::hotkeys::Hotkeys>>>>,
    recording: Arc<Mutex<Option<Arc<crate::recording_host::Host>>>>,
    benchmark: Arc<Mutex<Option<Arc<crate::features::benchmark::runner::Runner>>>>,
    stopping: Arc<AtomicBool>,
}

impl Services {
    pub async fn account_session(&self) -> Option<govorun_core::AccountSession> {
        self.account.lock().await.as_ref().and_then(|owner| crate::account_host::AccountHost::account_session(owner))
    }
    pub fn new(app: &AppHandle) -> Self {
        Self {
            #[cfg(feature = "private-fixtures")]
            private_callbacks: Arc::new(std::sync::Mutex::new(None)),
            #[cfg(feature = "private-fixtures")]
            private_runtime: app.try_state::<PrivateRuntimeAdapter>().map(|adapter| adapter.0),
            worker: Resources::locate(app), current: Arc::new(Mutex::new(None)), connecting: Arc::new(Mutex::new(())), #[cfg(feature="private-fixtures")] generation: Arc::new(AtomicU64::new(0)), account: Arc::new(Mutex::new(None)),
            stopping: Arc::new(AtomicBool::new(false)), agent: Arc::new(Mutex::new(None)),
            clipboard: Arc::new(Mutex::new(None)), hotkeys: Arc::new(Mutex::new(None)), recording: Arc::new(Mutex::new(None)), benchmark: Arc::new(Mutex::new(None)) }
    }

    pub async fn connect(&self, app: &AppHandle) -> Reply {
        let _connecting = self.connecting.lock().await;
        let mut current = self.current.lock().await;
        if self.stopping.load(Ordering::SeqCst) { return Err("native_closing"); }
        if let Some(service) = current.as_ref() {
            if !service.is_closed() {
                let service = service.clone();
                drop(current);
                let started = std::time::Instant::now();
                let result = service.snapshot().await;
                crate::extended_diagnostics::finished(app, "runtime.snapshot", started, result.as_ref().err().copied());
                return self.public_runtime(result).await;
            }
            self.stop_recording().await;
            if let Some(owner) = self.hotkeys.lock().await.take() { owner.stop().await; }
            if let Some(account) = self.account.lock().await.take() { account.stop().await; }
            service.stop().await;
        }
        if self.stopping.load(Ordering::SeqCst) { return Err("native_closing"); }
        let started = std::time::Instant::now();
        let result = async {
            let worker = self.worker.as_ref().map_err(|code| *code)?;
            let models = if let Some((bundled, root)) = worker.models.clone() {
                Some(tauri::async_runtime::spawn_blocking(move || crate::model_store::PRODUCTION.prepare(&bundled, &root))
                    .await.map_err(|_| "model_storage_unavailable")??)
            } else { None };
            if self.stopping.load(Ordering::SeqCst) { return Err("native_closing"); }
            let rust_recording = models.is_some();
            #[cfg(feature="private-fixtures")]
            let event_app = app.clone();
            let billing_app = app.clone();
            let account = crate::account_host::with_effects(Arc::new(move |event,data| {
                deliver(&billing_app,NativeEvent {event:event.into(),data});
            }), Arc::new(crate::features::about::open::open));
            *self.account.lock().await = Some(account.clone());
            #[cfg(feature="private-fixtures")]
            let account_requests = account.clone();
            #[cfg(feature="private-fixtures")]
            let account_events = account.clone();
            let capture_owner = Arc::new(Mutex::new(None::<std::sync::Weak<crate::features::hotkeys::Hotkeys>>));
            #[cfg(feature="private-fixtures")]
            let capture_events = capture_owner.clone();
            #[cfg(feature="private-fixtures")]
            let recording_events = self.recording.clone();
            #[cfg(feature="private-fixtures")]
            let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
            #[cfg(feature="private-fixtures")]
            let generations = self.generation.clone();
            #[cfg(feature = "private-fixtures")]
            let private_runtime = self.private_runtime;
            let service = if rust_recording {
                let facts=crate::features::runtime::session::facts(
                    if cfg!(target_os="macos"){ "macos" }else{ "windows" },
                    worker.identity.as_deref().ok_or("native_identity_mismatch")?,
                    app.config().product_name.as_deref().unwrap_or("Говорун"),
                    env!("CARGO_PKG_VERSION"), option_env!("GOVORUN_BUILD_ID").unwrap_or(""));
                let lifetime_app=app.clone();
                HostSession::local_events(facts,Arc::new(move|event|deliver(&lifetime_app,event)))?
            } else {
                #[cfg(feature="private-fixtures")]
                {
                let (executable, args)=worker.inspection.as_ref().ok_or("native_not_configured")?;
                NativeService::start_with_host(executable, args, Options::default(), models.as_deref(), move |mut event| {
                if generations.load(Ordering::Acquire) != generation { return; }
                if rust_recording && matches!(event.event.as_str(), "hotkeys.capture" | "dictation.changed" | "runtime.snapshot") { return; }
                if event.event == "hotkeys.capture" {
                    let owner = capture_events.clone();
                    tokio::spawn(async move {
                        if let Some(owner) = owner.lock().await.as_ref().and_then(std::sync::Weak::upgrade) { let _ = owner.captured(event.data).await; }
                    });
                    return;
                }
                if matches!(event.event.as_str(), "runtime.exited" | "runtime.startFailed") {
                    let recording = recording_events.clone();
                    let generations = generations.clone();
                    tokio::spawn(async move {
                        let mut current = recording.lock().await;
                        if generations.load(Ordering::Acquire) == generation {
                            if let Some(owner) = current.take() { owner.shutdown().await; }
                        }
                    });
                    let owner = capture_events.clone();
                    tokio::spawn(async move {
                        if let Some(owner) = owner.lock().await.as_ref().and_then(std::sync::Weak::upgrade) { owner.stop().await; }
                    });
                }
                if event.event == "runtime.snapshot" {
                    let Ok(snapshot) = crate::features::runtime::project(event.data, account_events.documents_available()) else { return; };
                    event.data = snapshot;
                    #[cfg(feature = "private-fixtures")]
                    if let Some(adapter) = private_runtime { event.data = adapter(event.data); }
                }
                if matches!(event.event.as_str(), "billing.changed" | "account.changed") {
                    let host=account_events.clone();
                    tokio::spawn(async move {host.publish_billing().await;host.publish_account().await;});
                    return;
                }
                if matches!(event.event.as_str(), "dictionary.changed" | "statistics.changed" | "selfhost.changed") {
                    // The common owners already emitted complete public snapshots.
                    return;
                }
                if matches!(event.event.as_str(),"styles.changed"|"dictation.changed") {
                    let host = account_events.clone();
                    tokio::spawn(async move {
                        host.publish_documents(false).await;
                        host.publish_billing().await;
                        host.publish_account().await;
                    });
                    if event.event == "styles.changed" { return; }
                }
                deliver(&event_app, event);
            }, move |request| account_requests.handle(request)).map(HostSession::from)?
                }
                #[cfg(not(feature="private-fixtures"))]
                { return Err("native_not_configured"); }
            };
            *current = Some(service.clone());
            // Shutdown can interrupt a slow first snapshot instead of waiting for its timeout.
            drop(current);
            let mut result = service.snapshot().await;
            if let (Some(expected), Ok(snapshot)) = (&worker.identity, &result) {
                if snapshot.get("bundleIdentifier").and_then(Value::as_str) != Some(expected.as_str()) {
                    service.stop().await;
                    result = Err("native_identity_mismatch");
                } else if snapshot.get("maintenanceProtocol").and_then(Value::as_u64) != Some(1)
                    || rust_recording && snapshot.get("recordingOwner").and_then(Value::as_str) != Some("rust") {
                    service.stop().await;
                    result = Err("native_update_required");
                }
            }
            if let Ok(snapshot) = &result {
                let mut physical_snapshot = snapshot.clone();
                let transport: Arc<dyn crate::agent::AgentTransport> = match worker.identity.as_deref() {
                    Some(identity) => {
                        let direct = crate::persistence::Transport::new(service.clone(), identity)?;
                        direct.complete_snapshot(&mut physical_snapshot)?;
                        Arc::new(direct)
                    },
                    None => Arc::new(service.clone()), // inspect-only/synthetic development, never owner stores
                };
                match crate::agent::AgentClient::with_transport(transport, &physical_snapshot) {
                    Ok(client) => {
                        account.attach_physical_agent(client.clone())?;
                        let hotkeys_app = app.clone();
                        let hotkeys = crate::features::hotkeys::Hotkeys::new(client.clone(),
                            Arc::new(move |event, data| deliver(&hotkeys_app, NativeEvent {event:event.into(),data})));
                        *self.hotkeys.lock().await = Some(hotkeys.clone());
                        *capture_owner.lock().await = Some(Arc::downgrade(&hotkeys));
                        if !rust_recording && client.supports(&crate::agent::Capability::named("hotkeys.localFacts")?) {
                            if let Err(error) = hotkeys.invoke("hotkeys.snapshot", json!({})).await {
                                service.stop().await; return Err(error);
                            }
                        }
                        let store=Arc::new(crate::agent::storage::AgentStore::new(client.clone()));
                        let self_host_app=app.clone();
                        let self_host_account=Arc::downgrade(&account);
                        let clipboard_app=app.clone();
                        let clipboard_recording=self.recording.clone();
                        *self.clipboard.lock().await=Some(crate::features::clipboard::Clipboard::new(store.clone(),
                            Arc::new(move|event,data| {
                                // Apply «После вставки текста» and «Помнить последнюю диктовку» without a restart.
                                let recording=clipboard_recording.clone();
                                tokio::spawn(async move {
                                    let host=recording.lock().await.clone();
                                    if let Some(host)=host { let _=host.settings.refresh_execution().await; }
                                });
                                deliver(&clipboard_app,NativeEvent {event:event.into(),data})
                            })));
                        let owner=crate::features::self_host::SelfHost::new(store.clone(),store,
                            Arc::new(crate::features::self_host::network::Http),Arc::new(move|event,data| {
                                if event == "styles.changed" {
                                    if let Some(owner)=self_host_account.upgrade() {
                                        tokio::spawn(async move {owner.publish_documents(false).await;});
                                    }
                                } else {deliver(&self_host_app,NativeEvent {event:event.into(),data});}
                            }));
                        account.attach_self_host(owner.clone());
                        owner.publish().await;
                        *self.agent.lock().await = Some(client);
                    },
                    Err(error) => { service.stop().await; result = Err(error); }
                }
            }
            if let Ok(snapshot) = &result {
                if snapshot.get("accountProtocol").and_then(Value::as_u64) == Some(1) {
                    let origin = snapshot.get("accountOrigin").and_then(Value::as_str).ok_or("account_invalid_configuration")?;
                    let terms = snapshot.get("onlineTermsVersion").and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok()).ok_or("account_invalid_configuration")?;
                    let expected_origin = govorun_core::configured_account_origin();
                    let initialization = if origin != expected_origin { Err("account_origin_mismatch") }
                        else { account.initialize(service.clone(), origin, terms).await };
                    if let Err(error) = initialization {
                        service.stop().await;
                        result = Err(error);
                    }
                }
            }
            if result.is_ok() {
                if let Some(models) = models.as_deref() {
                    let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                    match self.start_recording(app, account.clone(), agent, models).await {
                        Ok(()) => {},
                        Err(error) => { self.stop_recording().await; service.stop().await; result = Err(error); }
                    }
                }
            }
            result = self.public_runtime(result).await;
            if let Ok(snapshot) = &result {
                deliver(app, NativeEvent { event: "runtime.snapshot".into(), data: snapshot.clone() });
                deliver(app, NativeEvent { event: "runtime.connected".into(), data: Value::Null });
            }
            result
        }.await;
        let result = self.finish_startup(result).await;
        crate::extended_diagnostics::finished(app, "runtime.connect", started, result.as_ref().err().copied());
        result
    }

    pub async fn invoke(&self, method: &str, params: Value) -> Reply {
        if self.is_stopping() { return Err("native_closing"); }
        let service = self.current.lock().await.clone().ok_or("native_unavailable")?;
        // Private synthetic UI controls use the isolated inspection pipe; the
        // product's cancellation route accepts only its real runtime command.
        #[cfg(feature = "private-fixtures")]
        if method == "recording.cancel" && self.worker.as_ref().is_ok_and(|worker| worker.inspection.is_some()) {
            return service.invoke(method, params).await;
        }
        // A private acceptance binary can isolate OS callback/window routing
        // from authentication. This injection is absent from product builds.
        #[cfg(feature = "private-fixtures")]
        if matches!(method, "account.callback" | "billing.callback") {
            let adapter = *self.private_callbacks.lock().expect("private callback adapter");
            if let Some(params) = adapter.and_then(|adapter| adapter(method, params.clone())) {
                return service.invoke("recording.cancel", params).await;
            }
        }
        match crate::features::route(method)? {
            crate::features::Route::PermissionSetup => {
                if let Some(owner) = self.recording.lock().await.clone() {
                    let result = if method == "dictation.snapshot" {
                        if params.as_object().is_none_or(|value| !value.is_empty()) { return Err("invalid_command"); }
                        owner.refresh().await
                    } else { owner.settings.invoke(method, params).await };
                    owner.publish();
                    result
                } else {
                    let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                    crate::features::permission_setup::invoke(&agent, method, params).await
                }
            }
            crate::features::Route::RecordingSupport => {
                let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                crate::features::recording_support::invoke(&agent, method, params).await
            }
            crate::features::Route::Benchmark => {
                let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                crate::features::benchmark::invoke(&agent, method, params).await
            }
            crate::features::Route::Diagnostics => {
                let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                crate::features::diagnostics::invoke(&agent, method, params).await
            }
            crate::features::Route::Hotkeys => {
                let owner=self.hotkeys.lock().await.clone().ok_or("hotkeys_unavailable")?;
                owner.invoke(method,params).await
            }
            crate::features::Route::Clipboard => {
                if let Some(reply)=crate::last_dictation::invoke(method,&params) { return reply; }
                let owner=self.clipboard.lock().await.clone().ok_or("clipboard_unavailable")?;
                owner.invoke(method,params).await
            }
            crate::features::Route::SelfHost => {
                let account=self.account.lock().await.clone().ok_or("self_host_unavailable")?;
                let owner=account.self_host().ok_or("self_host_unavailable")?;
                match method {
                    "selfHost.snapshot" if params.as_object().is_some_and(|value|value.is_empty()) => owner.snapshot().await,
                    "selfHost.action" => owner.action(params).await,
                    _ => Err("self_host_invalid_request"),
                }
            }
            crate::features::Route::Billing => {
                let account = self.account.lock().await.clone().ok_or("account_storage_unavailable")?;
                account.billing(method, params).await
            }
            crate::features::Route::Account => {
                let account=self.account.lock().await.clone().ok_or("account_storage_unavailable")?;
                account.account(method,params).await
            }
            crate::features::Route::Documents => {
                let account = self.account.lock().await.clone().ok_or("account_storage_unavailable")?;
                account.document(method, params).await
            }
            crate::features::Route::About => {
                let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
                crate::features::about::invoke(&agent, env!("CARGO_PKG_VERSION"),
                    option_env!("GOVORUN_BUILD_ID").unwrap_or(""), method, params,
                    crate::features::about::open::open).await
            }
            crate::features::Route::Runtime => { self.invoke_runtime(method,params).await }
            crate::features::Route::Agent => Err("unsupported_command"),
        }
    }

    async fn invoke_runtime(&self, method: &str, params: Value) -> Reply {
        if !params.as_object().is_some_and(|p|p.is_empty()){return Err("invalid_command");}
        let service=self.current.lock().await.clone().ok_or("native_unavailable")?;
        match method {
            "runtime.snapshot"=>self.public_runtime(service.snapshot().await).await,
            "runtime.shutdown"=>{
                let _connecting=self.connecting.lock().await;
                self.stop().await;
                self.stopping.store(false,Ordering::SeqCst);
                Ok(Value::Null)
            },
            "runtime.shutdownWhenIdle"=>{self.stop_for_maintenance().await?;Ok(json!({"accepted":true}))},
            "recording.cancel"=>{
                if let Some(owner)=self.recording.lock().await.clone(){owner.runtime.cancel();Ok(Value::Null)}
                else{service.cancel_fixture(params).await}
            },
            _=>Err("unsupported_command"),
        }
    }

    async fn start_recording(&self, app: &AppHandle, account: Arc<crate::account_host::AccountHost>, agent: crate::agent::AgentClient, models: &std::path::Path) -> Result<(), &'static str> {
        let owner = crate::recording_host::Host::start(app, account, agent.clone(), models).await?;
        *self.recording.lock().await = Some(owner.clone());
        let hotkeys = self.hotkeys.lock().await.clone().ok_or("hotkeys_unavailable")?;
        hotkeys.attach_physical(crate::physical_hotkeys::Physical::new(&owner.runtime, &hotkeys, crate::focus::assignment_guard(app)));
        hotkeys.invoke("hotkeys.snapshot", json!({})).await?;
        owner.refresh().await?;
        let benchmark=crate::benchmark_host::compose(app,&owner.runtime,models)?;
        agent.bind_benchmark(benchmark.clone())?;
        *self.benchmark.lock().await=Some(benchmark);
        owner.prepare();
        Ok(())
    }
    async fn stop_recording(&self) {
        if let Some(owner) = self.benchmark.lock().await.take() {owner.stop().await;}
        if let Some(owner) = self.recording.lock().await.take() { owner.shutdown().await; }
    }

    pub async fn copy_diagnostic_report(&self, text: String) -> Reply {
        if self.is_stopping() { return Err("native_closing"); }
        let agent = self.agent.lock().await.clone().ok_or("native_unavailable")?;
        crate::features::diagnostics::copy(&agent, text).await
    }

    async fn public_runtime(&self, result: Reply) -> Reply {
        let available = self.account.lock().await.as_ref().is_some_and(|owner| owner.documents_available());
        let mut snapshot = crate::features::runtime::project(result?, available)?;
        if let Some(owner) = self.recording.lock().await.clone() { snapshot = owner.project(snapshot)?; }
        #[cfg(feature = "private-fixtures")]
        let snapshot = match self.private_runtime { Some(adapter) => adapter(snapshot), None => snapshot };
        Ok(snapshot)
    }

    pub async fn refresh_account_on_focus(&self) {
        if self.is_stopping() { return; }
        if let Some(owner) = self.recording.lock().await.clone() { let _ = owner.refresh().await; }
        let account = self.account.lock().await.clone();
        if let Some(account) = account {
            account.refresh_for_activity(govorun_core::PROFILE_FOCUS_MAX_AGE).await;
        }
    }

    async fn prepare_maintenance(&self, service: &HostSession) -> Result<(), &'static str> {
        let recording = self.recording.lock().await.clone();
        if let Some(owner) = &recording { owner.runtime.quiesce().await?; }
        let result = service.invoke("runtime.prepareMaintenance", json!({})).await
            .and_then(|value| if value.get("accepted").and_then(Value::as_bool) == Some(true) { Ok(()) } else { Err("maintenance_busy") });
        if result.is_err() { if let Some(owner) = recording { owner.runtime.resume(); } }
        result
    }

    pub async fn stop_for_maintenance(&self) -> Result<(), &'static str> {
        if self.recording.lock().await.as_ref().is_some_and(|owner| owner.runtime.busy()) { return Err("maintenance_busy"); }
        if self.account.lock().await.as_ref().and_then(|owner|owner.self_host()).is_some_and(|owner|owner.busy()) {return Err("maintenance_busy");}
        let current = self.current.lock().await.clone();
        let Some(service) = current else {
            self.stopping.store(true, Ordering::SeqCst);
            self.clear_session().await;
            return Ok(());
        };
        self.prepare_maintenance(&service).await?;
        if let Some(owner) = self.hotkeys.lock().await.take() { owner.stop().await; }
        self.stopping.store(true, Ordering::SeqCst);
        self.stop_recording().await;
        if let Some(account) = self.account.lock().await.take() { account.stop().await; }
        let result = service.shutdown_when_idle().await;
        if result.is_err() { service.stop().await; }
        self.current.lock().await.take();
        result
    }

}
