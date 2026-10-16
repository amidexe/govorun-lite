use crate::{session_host::SessionHost, session_ports::SessionPorts, session_styles::StylesBridge};
use govorun_core::features::stylization::{commands::Command as StyleCommand, ipc::Requests as StyleRequests};
use govorun_core::{
    account_protocol::auth, AccountSession, AuthConfig, AuthFlow, Error, Result,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};
use tokio::{
    runtime::Runtime,
    sync::{Mutex as AsyncMutex, Notify, Semaphore, OnceCell},
    task::JoinHandle,
};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HANDLES: OnceLock<Mutex<HashMap<u64, Arc<SessionBridge>>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

fn runtime() -> Result<&'static Runtime> {
    if RUNTIME.get().is_none() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("account-jni")
            .enable_all()
            .build()
            .map_err(|_| Error::StorageUnavailable)?;
        let _ = RUNTIME.set(runtime);
    }
    RUNTIME.get().ok_or(Error::StorageUnavailable)
}
fn handles() -> &'static Mutex<HashMap<u64, Arc<SessionBridge>>> {
    HANDLES.get_or_init(Mutex::default)
}
pub(crate) fn lookup(id: u64) -> Result<Arc<SessionBridge>> {
    handles()
        .lock()
        .map_err(|_| Error::StorageUnavailable)?
        .get(&id)
        .cloned()
        .ok_or(Error::SessionChanged)
}

/// Registry entries are resource handles, not alternate account state. Each
/// entry owns the same core session/flow as desktop for its application lifetime.
pub(crate) struct SessionBridge {
    pub host: Arc<SessionHost>,
    core: AccountSession,
    flow: AuthFlow,
    latest_intent: Mutex<u64>,
    closing: Mutex<bool>,
    requests: Arc<Semaphore>,
    active: AtomicUsize,
    finished: Notify,
    projection: AsyncMutex<()>,
    observer: Mutex<Option<JoinHandle<()>>>,
    styles: OnceCell<Arc<StylesBridge>>,
    style_requests: Arc<StyleRequests>,
}

impl SessionBridge {
    pub fn create(host: Arc<SessionHost>, terms: u32, origin: &str) -> Result<u64> {
        let runtime = runtime()?;
        let ports = Arc::new(SessionPorts(host.clone()));
        let core = AccountSession::new(ports.clone(), ports.clone(), terms)?;
        let config = AuthConfig::new(origin, "android")?;
        // Trusted build composition selects import capability in the common owner.
        // A native command cannot expand it in an ordinary build.
        let config = if govorun_core::configured_refresh_import_allowed(origin) {
            config.with_refresh_import()
        } else { config };
        let flow = AuthFlow::new(core.clone(), config, ports);
        let bridge = Arc::new(Self {
            host,
            core,
            flow,
            latest_intent: Mutex::new(0),
            closing: Mutex::new(false),
            requests: Arc::new(Semaphore::new(32)),
            active: AtomicUsize::new(0),
            finished: Notify::new(),
            projection: AsyncMutex::new(()),
            observer: Mutex::new(None),
            styles: OnceCell::new(),
            style_requests: Arc::new(StyleRequests::default()),
        });
        let mut all = handles().lock().map_err(|_| Error::StorageUnavailable)?;
        if all.len() >= 16 {
            return Err(Error::StorageUnavailable);
        }
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        all.insert(id, bridge.clone());
        let mut changes = bridge.core.subscribe();
        let weak = Arc::downgrade(&bridge);
        *bridge.observer.lock().expect("session observer") = Some(runtime.spawn(async move {
            while changes.changed().await.is_ok() {
                let Some(bridge) = weak.upgrade() else {
                    break;
                };
                if *bridge.closing.lock().expect("session closing") {
                    break;
                }
                let _ = bridge.publish().await;
            }
        }));
        Ok(id)
    }

    async fn publish(&self) -> Result<()> {
        let _publication = self.projection.lock().await;
        if *self.closing.lock().expect("session closing") {
            return Ok(());
        }
        if let Some(styles) = self.styles.get() { let _ = styles.publish().await; }
        self.host
            .effect(
                json!({"operation":"project","value":self.core.native_projection()}),
                false,
            )
            .await?;
        Ok(())
    }

    pub fn start(self: &Arc<Self>, id: u64, command: Value) -> Result<()> {
        let closing = self.closing.lock().map_err(|_| Error::StorageUnavailable)?;
        if *closing {
            return Err(Error::SessionChanged);
        }
        if command["operation"] == "stylizationCancel" {
            let result = self.style_requests.cancel(command).map(|cancelled|json!({"cancelled":cancelled}));
            self.host.complete(id,result);
            return Ok(());
        }
        let deliver_text = command["operation"] == "stylizationProcess";
        let style_call = command["operation"].as_str().is_some_and(|op|op == "stylizationStart" || StyleCommand::recognizes(op));
        let style_guard = if style_call {
            Some(self.style_requests.register(&id.to_string(),self.core.snapshot().session_generation)?)
        } else {None};
        let permit = self
            .requests
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::NetworkUnavailable)?;
        // Reserve in JNI arrival order, before scheduling on the runtime.
        let intent = auth::reserve_native(&self.flow, &command, &self.latest_intent)?;
        self.active.fetch_add(1, Ordering::SeqCst);
        let guard = CallGuard {
            bridge: self.clone(),
            id,
            replied: false,
        };
        runtime()?.spawn(async move {
            let mut guard = guard;
            let bridge = &guard.bridge;
            let (result, timing) = crate::session_timing::capture(async { if let Some(style_guard) = &style_guard {
                style_guard.run_value(async { match bridge.styles.get_or_try_init(||StylesBridge::create(bridge.core.clone(),bridge.host.clone())).await {
                    Ok(styles)=>styles.dispatch(command,style_guard,id).await,
                    Err(error)=>Err(error),
                }}).await
            } else {auth::dispatch(&bridge.core, &bridge.flow, intent, command).await} }).await;
            let published = if deliver_text { Ok(()) } else { bridge.publish().await };
            let result = if style_call {result} else {result.and_then(|value|published.map(|_|value))};
            if style_call {bridge.host.complete_style(id,result,timing);} else {bridge.host.complete(id,result);}
            guard.replied = true;
            drop(permit);
        });
        Ok(())
    }

    pub fn close(self: &Arc<Self>, handle: u64, id: u64) -> Result<()> {
        let mut closing = self.closing.lock().map_err(|_| Error::StorageUnavailable)?;
        if *closing {
            return Err(Error::SessionChanged);
        }
        *closing = true;
        if let Some(observer) = self.observer.lock().expect("session observer").take() {
            observer.abort();
        }
        self.style_requests.close();
        if let Some(styles) = self.styles.get() {styles.owner.stop();}
        self.core.disconnect();
        self.host.close_network();
        let bridge = self.clone();
        runtime()?.spawn(async move {
            // Commands retain writes until core finalization. Closing rejects
            // new work and cancels network, but never abandons protected storage.
            loop {
                let finished = bridge.finished.notified();
                if bridge.active.load(Ordering::SeqCst) == 0 {
                    break;
                }
                finished.await;
            }
            if let Some(styles) = bridge.styles.get() {styles.shutdown().await;}
            handles().lock().expect("session handles").remove(&handle);
            bridge.host.complete(id, Ok(json!({})));
        });
        Ok(())
    }
}

struct CallGuard {
    bridge: Arc<SessionBridge>,
    id: u64,
    replied: bool,
}
impl Drop for CallGuard {
    fn drop(&mut self) {
        if !self.replied {
            self.bridge
                .host
                .complete(self.id, Err(Error::NetworkUnavailable));
        }
        self.bridge.active.fetch_sub(1, Ordering::SeqCst);
        self.bridge.finished.notify_one();
    }
}
