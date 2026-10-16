//! Rust-only physical composition. The hook and capture callback enqueue typed
//! edges; one actor connects them to Core and the foreground account workflow.
use crate::features::dictation::timer;
use crate::local_dictation::{Destination, Presentation, Processor};
use crate::{
    account_host::OsContext,
    features::dictation::{capture, recording::Lease},
};
use govorun_os_runtime::{
    capture::{admission::Admission, cpal_owner::RecorderDriver, executor::Executor},
    inference::Job,
    keyboard::{Binding, Hook, Target},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};
use tokio::sync::{mpsc, oneshot};

#[path = "settings.rs"]
mod settings;
pub use govorun_os_runtime::inference_model::Model;
pub use settings::Settings;
#[path = "configuration.rs"]
mod configuration;
use govorun_core::features::dictation::hook_registration;
pub trait View: Send + Sync {
    fn presentation(&self, lease: Lease) -> Arc<dyn Presentation>;
    fn admitted(&self, lease: Lease);
    fn ready(&self, lease: Lease);
    fn stopped(&self, lease: Lease);
    fn failed(&self, lease: Lease, code: &'static str);
    fn failed_native(&self, lease: Lease, code: &'static str, _native: Option<govorun_core::journal_report::capture_failure::Native>) {
        self.failed(lease, code);
    }
    fn cancelled(&self);
    fn warning(&self, lease: Lease);
    fn capture_timing(&self, _detail: String) {}
    fn notice(&self, lease: Lease, code: &'static str);
}
struct Context {
    connection: Option<crate::prepared_connection::Prepared>,
    destination: Destination,
    os: OsContext,
    keep: bool,
    presentation: Arc<dyn Presentation>,
}
struct Physical {
    clock: Instant,
    executor: Arc<Executor<()>>,
    processor: Arc<Processor>,
    settings: Arc<RwLock<Settings>>,
    view: Arc<dyn View>,
    send: mpsc::UnboundedSender<Command>,
}
impl capture::Ports for Physical {
    type Context = Context;
    type Job = Job;
    fn permission(&mut self, lease: Lease) -> Result<(), &'static str> {
        use govorun_core::permission_setup::Microphone as Fact;
        use govorun_os_runtime::setup::access::Microphone as Os;
        let fact = match govorun_os_runtime::setup::microphone() {
            Os::Authorized => Fact::Authorized,
            Os::NotDetermined => Fact::NotDetermined,
            Os::Denied => Fact::Denied,
            Os::Restricted => Fact::Restricted,
            Os::Unknown => Fact::Unknown,
        };
        if fact.can_record(std::env::consts::OS) {
            return Ok(());
        }
        self.view.notice(lease, "microphone_permission_required");
        Err("microphone_permission_required")
    }
    fn begin(&mut self, lease: Lease) -> Result<Context, &'static str> {
        let admission = Admission::new(Some(lease.started_at));
        let settings = self
            .settings
            .read()
            .map_err(|_| "dictation_settings_unavailable")?
            .clone();
        let destination = Destination::capture();
        let application = destination.application();
        let mut context = Context {
            connection: None,
            destination,
            os: OsContext {
                application,
                application_identity: if cfg!(windows) { govorun_core::style_repository::ApplicationIdentity::Windows } else { govorun_core::style_repository::ApplicationIdentity::Exact },
                input_mode: "hold",
                marker: settings.marker,
                streaming: false,
                bypass: lease.without_styling,
                draft: false,
            },
            keep: settings.keep_clipboard,
            presentation: self.view.presentation(lease),
        };
        self.executor
            .begin(lease.generation, admission, settings.input, ())?;
        self.view.admitted(lease);
        if !lease.without_styling {
            context.connection = Some(self.processor.prepare_capture());
        }
        Ok(context)
    }
    fn release(
        &mut self,
        finish: crate::features::dictation::recording::Finish,
    ) -> Result<(), &'static str> {
        if self
            .executor
            .release(finish.capture.generation, finish.tail.deadline_nanos())?
        {
            self.view.stopped(finish.capture);
            Ok(())
        } else {
            Err("recording_not_active")
        }
    }
    fn cancel_capture(&mut self) {
        self.executor.cancel();
    }
    fn cancel_processing(&mut self) {
        self.processor.cancel();
    }
    fn discard(&mut self, job: Job) {
        job.cancel();
    }
    fn ready(&mut self, lease: Lease) {
        let ready_at = Instant::now();
        self.view.ready(lease);
        let settings = self.settings.read().ok().map(|v| v.clone());
        if let Some(settings) = settings {
            let processor = self.processor.clone();
            let view = self.view.clone();
            let send = self.send.clone();
            tokio::spawn(async move {
                let plan = match processor
                    .recording_timer(settings.local_timer_enabled, settings.configured_minutes)
                    .await
                {
                    Ok(plan) => Ok(plan),
                    Err(_) => {
                        view.notice(lease, "recording_timer_unavailable");
                        timer::plan(
                            false,
                            false,
                            false,
                            false,
                            settings.local_timer_enabled,
                            settings.configured_minutes,
                        )
                    }
                };
                if let Ok(plan) = plan {
                    let _ = send.send(Command::Timer(lease, ready_at, plan));
                }
            });
        }
    }
    fn warning(&mut self, lease: Lease) {
        self.view.warning(lease);
    }
    fn process(
        &mut self,
        finish: crate::features::dictation::recording::Finish,
        mut context: Context,
        job: Job,
    ) {
        context.os.input_mode = finish.input_mode;
        if let Err(code) = self.processor.submit(
            job,
            context.os,
            context.destination,
            context.keep,
            context.presentation,
            context.connection,
            Some(crate::features::dictation::measurement::Origin {
                released: self.clock + std::time::Duration::from_nanos(finish.tail.deadline_nanos().saturating_sub(govorun_core::recording_controls::STOP_TAIL_NANOS)),
                audio_millis: finish.tail.deadline_nanos().saturating_sub(govorun_core::recording_controls::STOP_TAIL_NANOS).saturating_sub(finish.capture.started_at)/1_000_000,
            }),
        ) {
            self.view.failed(finish.capture, code);
        }
    }
    fn failed(&mut self, lease: Lease, code: &'static str) {
        self.view.failed(lease, code);
    }
    fn failed_native(&mut self, lease: Lease, code: &'static str, native: Option<govorun_core::journal_report::capture_failure::Native>) {
        self.view.failed_native(lease, code, native);
    }
    fn cancelled(&mut self) {
        self.view.cancelled();
    }
}
#[path = "actor.rs"]
mod actor;
type Command = actor::Command<Job>;

pub struct Runtime {
    send: mpsc::UnboundedSender<Command>,
    executor: Arc<Executor<()>>,
    hook: Mutex<hook_registration::Registration<Vec<(Target, Binding)>, Hook>>,
    busy: AtomicBool,
    settings: Arc<RwLock<Settings>>,
    processor: Arc<Processor>,
    clock: Instant,
    model: Arc<Model>,
}
fn now(clock: Instant) -> u64 {
    clock.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
impl Runtime {
    pub fn feedback_options(&self) -> Result<(bool, bool), &'static str> {
        let settings = self
            .settings
            .read()
            .map_err(|_| "dictation_settings_unavailable")?;
        Ok((settings.play_sounds, settings.mute_audio))
    }
    /// Compose on a blocking executor after native input ownership was stopped.
    /// Debug inspection never calls this constructor or opens an input device.
    pub fn start(
        model: Arc<Model>,
        settings: Settings,
        bindings: Vec<(Target, Binding)>,
        processor: Arc<Processor>,
        view: Arc<dyn View>,
    ) -> Result<Arc<Self>, &'static str> {
        let clock = Instant::now();
        let settings = Arc::new(RwLock::new(settings));
        let (send, receive) = mpsc::unbounded_channel();
        let audio = send.clone();
        let capture_model = model.clone();
        let executor = Arc::new(Executor::spawn(
            clock,
            move || RecorderDriver,
            move |_: &()| capture_model.begin(),
            Arc::new(move |event| {
                let _ = audio.send(Command::Audio(event));
            }),
        )?);
        #[cfg(target_os = "windows")]
        {
            let diagnostics = view.clone();
            executor.observe_startup(Arc::new(move |facts| {
                use govorun_core::journal_report::capture_startup::Startup;
                let milliseconds = |at: Instant| {
                    at.saturating_duration_since(facts.started)
                        .as_millis()
                        .min(u64::MAX as u128) as u64
                };
                let mut timing = Startup::new(0);
                timing.input_opening(milliseconds(facts.opening));
                timing.input_opened(milliseconds(facts.opened));
                timing.first_packet(milliseconds(facts.first_packet));
                diagnostics.capture_timing(timing.detail(milliseconds(facts.reported)));
            }))?;
        }
        let input = settings
            .read()
            .map_err(|_| "dictation_settings_unavailable")?
            .input
            .clone();
        let configured = executor
            .configure(input, false)
            .and_then(|done| done.recv().map_err(|_| "microphone_executor_unavailable")?);
        if let Err(code) = configured {
            executor.shutdown();
            return Err(code);
        }
        let mut hook = hook_registration::Registration::new(bindings.clone());
        let keys = send.clone();
        if let Err(code) = hook.configure(
            bindings,
            govorun_os_runtime::keyboard::permission_ready(),
            move |bindings| {
                Hook::start(
                    bindings.clone(),
                    clock,
                    Arc::new(move |event| {
                        let _ = keys.send(Command::Key(event));
                    }),
                )
            },
            |hook, bindings| hook.replace(bindings.clone()),
        ) {
            executor.shutdown();
            return Err(code);
        }
        let runtime = Arc::new(Self {
            send,
            executor: executor.clone(),
            hook: Mutex::new(hook),
            busy: AtomicBool::new(false),
            settings: settings.clone(),
            processor: processor.clone(),
            clock,
            model,
        });
        let actor = capture::Owner::new(Physical {
            clock,
            executor,
            processor,
            settings,
            view,
            send: runtime.send.clone(),
        });
        let weak = Arc::downgrade(&runtime);
        tauri::async_runtime::spawn(async move {
            actor::run(actor, receive, Arc::new(RuntimeObserver(weak)), clock).await;
        });
        Ok(runtime)
    }
    pub fn stop_recording(&self) -> Result<(), &'static str> {
        self.send
            .send(Command::Stop(None, now(self.clock)))
            .map_err(|_| "dictation_stopped")
    }
    pub fn timer_stop(&self, lease: Lease) {
        let _ = self.send.send(Command::Stop(Some(lease), now(self.clock)));
    }
    pub fn cancel(&self) {
        let _ = self.send.send(Command::Cancel);
    }
    pub fn processing_changed(&self) {
        let _ = self.send.send(Command::ProcessingChanged);
    }
    pub fn model_readiness(&self) -> govorun_os_runtime::model_cache::Readiness {
        self.model.readiness()
    }
    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire) || self.processor.count() > 0
    }
    pub fn capture_keys(
        &self,
        id: String,
        focus: Arc<dyn Fn() -> bool + Send + Sync>,
        sink: Arc<dyn Fn(govorun_os_runtime::keyboard::assignment::CaptureEvent) + Send + Sync>,
    ) -> Result<(), &'static str> {
        if !govorun_os_runtime::keyboard::permission_ready() {
            return Err("hotkeys_permission_required");
        }
        if self.busy() {
            return Err("hotkeys_capture_unavailable");
        }
        self.hook
            .lock()
            .map_err(|_| "hotkeys_unavailable")?
            .hook
            .as_ref()
            .ok_or("hotkeys_unavailable")?
            .capture(
                id,
                focus,
                sink,
            )
    }
    pub fn stop_key_capture(&self, id: &str) -> Result<(), &'static str> {
        self.hook
            .lock()
            .map_err(|_| "hotkeys_unavailable")?
            .hook
            .as_ref()
            .ok_or("hotkeys_unavailable")?
            .stop_capture(id)
    }
    pub fn hook_ready(&self) -> bool {
        self.hook
            .lock()
            .is_ok_and(|registration| registration.hook.is_some())
            && govorun_os_runtime::keyboard::permission_ready()
    }
    fn register_bindings(
        &self,
        registration: &mut hook_registration::Registration<Vec<(Target, Binding)>, Hook>,
        bindings: Vec<(Target, Binding)>,
    ) -> Result<(), &'static str> {
        let keys = self.send.clone();
        let clock = self.clock;
        registration.configure(
            bindings,
            govorun_os_runtime::keyboard::permission_ready(),
            move |bindings| {
                Hook::start(
                    bindings.clone(),
                    clock,
                    Arc::new(move |event| {
                        let _ = keys.send(Command::Key(event));
                    }),
                )
            },
            |hook, bindings| hook.replace(bindings.clone()),
        )
    }
    pub fn refresh_hook(&self) -> Result<(), &'static str> {
        let mut registration = self.hook.lock().map_err(|_| "hotkeys_unavailable")?;
        let bindings = registration.bindings.clone();
        self.register_bindings(&mut registration, bindings)
    }
    pub fn configure_bindings(&self, bindings: Vec<(Target, Binding)>) -> Result<(), &'static str> {
        let mut registration = self.hook.lock().map_err(|_| "hotkeys_unavailable")?;
        self.register_bindings(&mut registration, bindings)
    }
    pub async fn quiesce(&self) -> Result<(), &'static str> {
        let (send, done) = oneshot::channel();
        self.send
            .send(Command::Maintenance(send))
            .map_err(|_| "dictation_stopped")?;
        if done.await.map_err(|_| "dictation_stopped")? {
            Ok(())
        } else {
            Err("maintenance_busy")
        }
    }
    pub fn resume(&self) {
        let _ = self.send.send(Command::Resume);
    }
    pub async fn shutdown(self: &Arc<Self>) {
        let (send, done) = oneshot::channel();
        if self.send.send(Command::Shutdown(send)).is_ok() {
            let _ = done.await;
        }
        let runtime = self.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut hook) = runtime.hook.lock() {
                hook.shutdown();
            }
            runtime.executor.shutdown();
        })
        .await;
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        let (send, _) = oneshot::channel();
        let _ = self.send.send(Command::Shutdown(send));
        self.executor.cancel();
        // Move potentially blocking hook shutdown away from the renderer thread.
        if let Ok(mut hook) = self.hook.lock() {
            if let Some(hook) = hook.hook.take() {
                let _ = std::thread::spawn(move || drop(hook));
            }
        }
    }
}
struct RuntimeObserver(std::sync::Weak<Runtime>);
impl actor::State for RuntimeObserver {
    fn mode(&self) -> Option<govorun_core::recording_controls::Mode> {
        self.0
            .upgrade()
            .and_then(|r| r.settings.read().ok().map(|s| s.mode))
    }
    fn processing_count(&self) -> Option<usize> {
        self.0.upgrade().map(|r| r.processor.count())
    }
    fn model_deadline(&self, active: bool) -> Option<Instant> {
        self.0.upgrade().and_then(|r| {
            r.model
                .idle(Instant::now(), active || r.processor.count() > 0)
        })
    }
    fn input_changed(&self, active: bool) {
        if let Some(runtime) = self.0.upgrade() {
            let busy = active || runtime.processor.count() > 0;
            runtime.busy.store(busy, Ordering::Release);
            if let Ok(hook) = runtime.hook.lock() {
                if let Some(hook) = hook.hook.as_ref() {
                    hook.set_cancel_enabled(busy);
                }
            }
        }
    }
}
