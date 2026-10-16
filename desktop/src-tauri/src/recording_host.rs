//! Production composition for direct Rust recording; the native adapter has
//! already relinquished physical input before this owner is constructed.
use crate::{
    account_host::AccountHost,
    agent::{storage::AgentStore, AgentClient},
    features::{permission_setup::settings::Owner, snapshot_gate::SnapshotGate},
    local_dictation::Processor,
    recording_indicator::Adapter,
    recording_runtime::{Runtime, Settings},
    recording_setup::Physical,
};
use govorun_core::features::ports::NativeEvent;
use govorun_os_runtime::{
    inference_model::{Files, Model},
    insertion::Inserter,
    model_cache::Readiness,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};
use tauri::{AppHandle, Manager};
pub struct Host {
    pub runtime: Arc<Runtime>,
    pub settings: Arc<Owner>,
    pub ports: Arc<Physical>,
    indicator: Arc<Adapter>,
    feedback: Arc<crate::recording_feedback::Adapter>,
    app: AppHandle,
    base: Mutex<Option<Value>>,
    gate: SnapshotGate,
    closed: AtomicBool,
}
impl Host {
    pub async fn start(
        app: &AppHandle,
        account: Arc<AccountHost>,
        agent: AgentClient,
        models: &Path,
    ) -> Result<Arc<Self>, &'static str> {
        let store = Arc::new(AgentStore::new(agent.clone()));
        let ports = Physical::new(app, account.clone(), store.clone())?;
        let snapshot = ports.observed().await?;
        let settings = Settings::from_snapshot(&snapshot, ports.keep_clipboard().await?)?;
        let resources = app
            .path()
            .resource_dir()
            .map_err(|_| "recognition_model_unavailable")?;
        #[cfg(target_os = "macos")]
        let files = {
            let frameworks = resources
                .parent()
                .ok_or("recognition_model_unavailable")?
                .join("Frameworks");
            Files {
                model: models.join("gigaam-v3-e2e-rnnt-Q8_0.gguf"),
                vad: models.join("silero_vad.onnx"),
                runtime: frameworks.join("CTranscribe.framework/CTranscribe"),
                vad_runtime: frameworks,
            }
        };
        #[cfg(target_os = "windows")]
        let files = {
            let native = resources.join("native");
            let vad = native.join("runtimes/win-x64/native");
            Files {
                model: models.join("gigaam-v3-e2e-rnnt-Q8_0.gguf"),
                vad: models.join("silero_vad.onnx"),
                runtime: native.join("transcribe.dll"),
                vad_runtime: if vad.join("sherpa-onnx-c-api.dll").is_file() {
                    vad
                } else {
                    native
                },
            }
        };
        let model = Arc::new(Model::new(files, settings.compute, settings.keep_model));
        // The indicator exists in background launches too. Clipboard ownership
        // must not depend on a settings/home window having been opened.
        crate::recording_indicator::ensure(app).await?;
        let app_for_insert = app.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            #[cfg(target_os = "macos")]
            let port = govorun_os_runtime::insertion::Mac::new();
            #[cfg(target_os = "windows")]
            let port = app_for_insert
                .get_webview_window("recording")
                .ok_or("clipboard_owner_invalid")
                .and_then(|window| window.hwnd().map_err(|_| "clipboard_owner_invalid"))
                .and_then(|window| govorun_os_runtime::insertion::Windows::new(window.0 as usize));
            #[cfg(target_os = "macos")]
            let _ = app_for_insert;
            let _ = send.send(port.map(|port| Arc::new(Inserter::new(port))));
        })
        .map_err(|_| "clipboard_owner_invalid")?;
        let insertion = receive.await.map_err(|_| "clipboard_owner_invalid")??;
        let slot = Arc::new(OnceLock::<Weak<Runtime>>::new());
        let changes = slot.clone();
        let processor = Processor::new(
            account.clone(),
            insertion,
            Arc::new(move |_| {
                if let Some(runtime) = changes.get().and_then(Weak::upgrade) {
                    runtime.processing_changed();
                }
            }),
        );
        let capture_agent = agent.clone();
        let indicator = Adapter::new(app, Arc::new(move |code| {
            let code = govorun_core::journal_report::insertion::code(code).unwrap_or("insertion_failed");
            let diagnostics = agent.clone();
            tauri::async_runtime::spawn(async move {
                let _: Result<Value, _> = diagnostics.call_named("data.diagnostic",
                    json!({"event":"insertion_failed","code":code}), std::time::Duration::from_secs(2)).await;
            });
        }))?;
        let diagnostics = account.clone();
        let feedback = crate::recording_feedback::Adapter::new(
            indicator.clone(),
            Arc::new(move |code| {
                let diagnostics = diagnostics.clone();
                tauri::async_runtime::spawn(async move {
                    diagnostics.recording_feedback_error(code).await;
                });
            }),
        );
        feedback.attach_capture_failure(Arc::new(move |code, native| {
            let code = govorun_core::journal_report::capture_failure::code(code).unwrap_or("recording_start_failed");
            let diagnostics = capture_agent.clone();
            tauri::async_runtime::spawn(async move {
                let mut event = json!({"event":"audio_capture_failed","code":code});
                if let Some(native) = native { event["native"] = json!(native); }
                let _: Result<Value, _> = diagnostics.call_named("data.diagnostic",
                    event, std::time::Duration::from_secs(2)).await;
            });
        }));
        let diagnostics = account.clone();
        feedback.attach_capture_diagnostics(Arc::new(move |detail| {
            let diagnostics = diagnostics.clone();
            tauri::async_runtime::spawn(async move { diagnostics.recording_capture_ready(detail).await; });
        }));
        let view = feedback.clone();
        let runtime = tokio::task::spawn_blocking(move || {
            Runtime::start(model, settings, Vec::new(), processor, view)
        })
        .await
        .map_err(|_| "dictation_setup_unavailable")??;
        let _ = slot.set(Arc::downgrade(&runtime));
        indicator.attach(&runtime);
        feedback.attach(&runtime);
        ports.attach_runtime(&runtime);
        let events_app = app.clone();
        let settings = Owner::new(
            store,
            ports.clone(),
            Arc::new(move |event, data| {
                crate::service::deliver(
                    &events_app,
                    NativeEvent {
                        event: event.into(),
                        data,
                    },
                )
            }),
        );
        ports.attach(&settings);
        let host = Arc::new(Self {
            runtime,
            settings,
            ports,
            indicator: indicator.clone(),
            feedback,
            app: app.clone(),
            base: Mutex::new(None),
            gate: Default::default(),
            closed: AtomicBool::new(false),
        });
        let readiness = Arc::downgrade(&host);
        account.attach_recording_readiness(Arc::new(move || {
            readiness
                .upgrade()
                .is_some_and(|host| host.runtime.hook_ready() && host.ports.microphone_ready())
        }))?;
        let owner = Arc::downgrade(&host);
        indicator.on_change(Arc::new(move || {
            if let Some(owner) = owner.upgrade() {
                owner.publish();
            }
        }));
        Ok(host)
    }
    fn projected(&self, mut value: Value) -> Result<Value, &'static str> {
        let setup = self.ports.cached()?;
        let (projection, style_status) = self.indicator.projection()?;
        value["styleStatus"] = json!(style_status);
        value["issue"] = serde_json::to_value(projection.issue)
            .map_err(|_| "dictation_invalid_facts")?;
        value["phase"] = serde_json::to_value(projection.phase)
            .map_err(|_| "dictation_invalid_facts")?;
        value["readiness"] = json!({"model":match self.runtime.model_readiness(){Readiness::Standby=>"standby",Readiness::Loading=>"loading",Readiness::Ready=>"ready",Readiness::Failed=>"failed"},
            "microphone":self.ports.microphone_ready(),"microphoneAvailable":!setup.microphones.is_empty(),
            "accessibility":self.runtime.hook_ready(),"hotkey":self.runtime.hook_ready()});
        Ok(value)
    }
    pub fn project(&self, value: Value) -> Result<Value, &'static str> {
        *self
            .base
            .lock()
            .map_err(|_| "dictation_setup_unavailable")? = Some(value.clone());
        let value = self.projected(value)?;
        self.gate.changed(&value);
        Ok(value)
    }
    pub fn publish(&self) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let base = self.base.lock().ok().and_then(|value| value.clone());
        if let Some(base) = base {
            if let Ok(value) = self.projected(base) {
                if self.gate.changed(&value) {
                    crate::service::deliver(
                        &self.app,
                        NativeEvent {
                            event: "runtime.snapshot".into(),
                            data: value,
                        },
                    );
                }
            }
        }
    }
    pub async fn refresh(&self) -> Result<Value, &'static str> {
        let result = self.settings.refresh_execution().await;
        self.publish();
        result
    }
    pub fn prepare(self: &Arc<Self>) {
        let owner = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            if let Some(owner) = owner.upgrade() {
                let _ = owner.runtime.prepare().await;
                owner.publish();
            }
        });
        self.publish();
    }
    pub async fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.runtime.shutdown().await;
        let feedback = self.feedback.clone();
        let _ = tokio::task::spawn_blocking(move || feedback.shutdown()).await;
    }
}
