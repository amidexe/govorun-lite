//! The existing recording capsule, owned by Tauri without activating the app.
//! Only closed frames cross this webview boundary; account/text/PCM never do.
use crate::{
    features::dictation::{
        presentation::{Frame, Issue, Phase, Stage, State as Projection},
        recording::Lease,
    },
    local_dictation::Presentation,
    recording_runtime::{Runtime, View},
};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow};

#[cfg(target_os = "macos")]
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel, StyleMask};
#[cfg(target_os = "macos")]
tauri_panel! { panel!(GovorunRecordingPanel { config: { can_become_key_window: false, is_floating_panel: true } }) }

#[derive(Default)]
struct Display {
    session: u64,
    projection: Projection,
    runtime: Weak<Runtime>,
}
#[derive(Default)]
pub struct State(Mutex<Display>);
#[derive(Clone)]
pub struct Adapter {
    app: AppHandle,
    session: u64,
    changed: Arc<OnceLock<Arc<dyn Fn() + Send + Sync>>>,
    failure: Arc<dyn Fn(&'static str) + Send + Sync>,
}
struct Attempt {
    adapter: Adapter,
    id: u64,
}
impl Adapter {
    pub fn new(app: &AppHandle, failure: Arc<dyn Fn(&'static str) + Send + Sync>) -> Result<Arc<Self>, &'static str> {
        let state = app.state::<State>();
        let mut display = state
            .0
            .lock()
            .map_err(|_| "recording_indicator_unavailable")?;
        display.session = display
            .session
            .checked_add(1)
            .ok_or("recording_indicator_unavailable")?;
        display.projection.restart();
        display.runtime = Weak::new();
        Ok(Arc::new(Self {
            app: app.clone(),
            session: display.session,
            changed: Arc::new(OnceLock::new()),
            failure,
        }))
    }
    pub fn on_change(&self, changed: Arc<dyn Fn() + Send + Sync>) {
        let _ = self.changed.set(changed);
    }
    pub fn projection(
        &self,
    ) -> Result<(crate::features::dictation::presentation::Snapshot, &'static str), &'static str> {
        let state = self.app.state::<State>();
        let display = state
            .0
            .lock()
            .map_err(|_| "recording_indicator_unavailable")?;
        if display.session != self.session {
            return Err("dictation_stopped");
        }
        Ok((display.projection.snapshot(), display.projection.style_status()))
    }
    pub fn attach(&self, runtime: &Arc<Runtime>) {
        if let Ok(mut display) = self.app.state::<State>().0.lock() {
            if display.session == self.session {
                display.runtime = Arc::downgrade(runtime);
            }
        }
    }
    fn update(&self, change: impl FnOnce(&mut Projection)) {
        let frame = {
            let state = self.app.state::<State>();
            let Ok(mut display) = state.0.lock() else {
                return;
            };
            if display.session != self.session {
                return;
            }
            let previous = display.projection.snapshot().phase;
            change(&mut display.projection);
            display.projection.changed().map(|frame| {
                let haptic = previous != Phase::Fallback && frame.snapshot.phase == Phase::Fallback;
                (frame, haptic)
            })
        };
        if let Some((frame, haptic)) = frame {
            #[cfg(not(target_os = "macos"))]
            let _ = haptic;
            if let Some(changed) = self.changed.get() {
                changed();
            }
            let app = self.app.clone();
            let session = self.session;
            let _ = self.app.run_on_main_thread(move || {
                let state = app.state::<State>();
                let Ok(display) = state.0.lock() else {
                    return;
                };
                if display.session != session
                    || display.projection.current_frame().revision != frame.revision
                {
                    return;
                }
                drop(display);
                let Some(window) = app.get_webview_window("recording") else {
                    return;
                };
                let _ = window.emit("recording:state", &frame);
                #[cfg(target_os = "macos")]
                {
                    use tauri_nspanel::ManagerExt;
                    if haptic {
                        use objc2_app_kit::{
                            NSHapticFeedbackManager, NSHapticFeedbackPattern,
                            NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer,
                        };
                        NSHapticFeedbackManager::defaultPerformer()
                            .performFeedbackPattern_performanceTime(
                                NSHapticFeedbackPattern::Alignment,
                                NSHapticFeedbackPerformanceTime::Now,
                            );
                    }
                    if let Ok(panel) = app.get_webview_panel("recording") {
                        if frame.snapshot.phase == Phase::Idle {
                            panel.hide();
                        } else {
                            panel.show();
                        }
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    if frame.snapshot.phase == Phase::Idle {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                    }
                }
            });
        }
    }
    pub fn processing_changed(&self) {
        if let Ok(display) = self.app.state::<State>().0.lock() {
            if display.session == self.session {
                if let Some(runtime) = display.runtime.upgrade() {
                    runtime.processing_changed();
                }
            }
        }
    }
    fn fallback(&self, id: u64) {
        self.update(|state| state.fallback(id));
        let adapter = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            adapter.update(|state| state.clear_fallback(id));
        });
    }
}
impl View for Adapter {
    fn presentation(&self, lease: Lease) -> Arc<dyn Presentation> {
        Arc::new(Attempt {
            adapter: self.clone(),
            id: lease.generation,
        })
    }
    fn admitted(&self, lease: Lease) {
        self.update(|state| state.admit(lease.generation));
    }
    fn ready(&self, lease: Lease) {
        self.update(|state| state.ready(lease.generation));
    }
    fn stopped(&self, lease: Lease) {
        self.update(|state| state.stop(lease.generation));
    }
    fn failed(&self, lease: Lease, _: &'static str) {
        self.update(|state| {
            state.failed_capture(lease.generation);
            state.issue(lease.generation, Issue::CaptureUnavailable);
        });
        self.fallback(lease.generation);
    }
    fn cancelled(&self) {
        self.update(Projection::cancel);
    }
    fn warning(&self, lease: Lease) {
        self.update(|state| state.warning(lease.generation));
    }
    fn notice(&self, lease: Lease, _: &'static str) {
        self.update(|state| state.issue(lease.generation, Issue::TimerUnavailable));
    }
}

/// Private visual acceptance uses the real view edges without opening audio.
#[cfg(feature = "private-fixtures")]
pub async fn visual_fixture(app: &AppHandle, phase: &str) -> Result<(), &'static str> {
    if !matches!(phase, "recording" | "processing") {
        return Err("invalid_visual_phase");
    }
    ensure(app).await?;
    let adapter = Adapter::new(app, Arc::new(|_| {}))?;
    let lease = Lease { generation: 1, started_at: 0, without_styling: true };
    adapter.admitted(lease);
    adapter.ready(lease);
    if phase == "processing" {
        adapter.stopped(lease);
        adapter.presentation(lease).progress(crate::features::dictation::Stage::Recognizing);
    }
    Ok(())
}
impl Presentation for Attempt {
    fn progress(&self, stage: crate::features::dictation::Stage) {
        self.adapter.update(|state| {
            state.progress(
                self.id,
                match stage {
                    crate::features::dictation::Stage::Recognizing => Stage::Recognizing,
                    crate::features::dictation::Stage::Styling => Stage::Processing,
                },
            )
        });
    }
    fn style_progress(&self, stage: govorun_core::stylization::foreground::Stage) {
        use govorun_core::stylization::foreground::Stage as Core;
        self.adapter.update(|state| {
            state.progress(
                self.id,
                match stage {
                    Core::Checking => Stage::CheckingConnection,
                    Core::Accepted => Stage::Accepted,
                    Core::Processing => Stage::Processing,
                },
            )
        });
    }
    fn hide(&self) {
        self.adapter.update(|state| state.complete(self.id));
    }
    fn fallback(&self) {
        self.adapter.fallback(self.id);
    }
    fn insertion_failure(&self, code: &'static str) {
        (self.adapter.failure)(code);
        self.failure(code);
    }
    fn failure(&self, _: &'static str) {
        self.adapter
            .update(|state| state.issue(self.id, Issue::ProcessingFailed));
        self.adapter.fallback(self.id);
    }
}
fn local(url: &tauri::Url) -> bool {
    crate::recording_origin::allows("recording", url, cfg!(debug_assertions))
}
#[tauri::command]
pub async fn recording_indicator(
    app: AppHandle,
    window: WebviewWindow,
    action: String,
) -> Result<Frame, &'static str> {
    if !window.url().is_ok_and(|url| {
        crate::recording_origin::allows(window.label(), &url, cfg!(debug_assertions))
    }) {
        return Err("invalid_origin");
    }
    let state = app.state::<State>();
    let display = state
        .0
        .lock()
        .map_err(|_| "recording_indicator_unavailable")?;
    match action.as_str() {
        "snapshot" => {}
        "cancel" => {
            if let Some(runtime) = display.runtime.upgrade() {
                runtime.cancel();
            }
        }
        _ => return Err("invalid_command"),
    }
    Ok(display.projection.current_frame())
}
pub async fn ensure(app: &AppHandle) -> Result<(), &'static str> {
    let app_on_main = app.clone();
    let (send, done) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = send.send(create(&app_on_main));
    })
    .map_err(|_| "recording_indicator_unavailable")?;
    done.await.map_err(|_| "recording_indicator_unavailable")?
}
fn create(app: &AppHandle) -> Result<(), &'static str> {
    if app.get_webview_window("recording").is_some() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        // MIT panel configuration follows the verified prototype; notices
        // retain Handy's attribution and the pinned tauri-nspanel licence.
        let panel = PanelBuilder::<_, GovorunRecordingPanel>::new(app, "recording")
            .url(WebviewUrl::App("recording.html".into()))
            .title("Говорун")
            .size(tauri::Size::Logical(tauri::LogicalSize::new(112., 44.)))
            .level(PanelLevel::Status)
            .has_shadow(false)
            .transparent(true)
            .no_activate(true)
            .style_mask(StyleMask::empty().borderless().nonactivating_panel())
            .with_window(|window| {
                window
                    .decorations(false)
                    .transparent(true)
                    .focusable(false)
                    .visible(false)
                    .on_navigation(local)
                    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            })
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
            .map_err(|_| "recording_indicator_unavailable")?;
        panel.hide();
    }
    #[cfg(target_os = "windows")]
    tauri::WebviewWindowBuilder::new(app, "recording", WebviewUrl::App("recording.html".into()))
        .title("Говорун")
        .inner_size(112., 44.)
        .decorations(false)
        .shadow(false)
        .transparent(true)
        .focused(false)
        .focusable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .on_navigation(local)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()
        .map_err(|_| "recording_indicator_unavailable")?;
    if let (Some(window), Ok(Some(monitor))) =
        (app.get_webview_window("recording"), app.primary_monitor())
    {
        let area = monitor.work_area();
        let scale = monitor.scale_factor();
        let x = area.position.x as f64 + (area.size.width as f64 - 112. * scale) / 2.;
        let usable_bottom = (area.position.y as f64 + area.size.height as f64) / scale;
        #[cfg(target_os = "macos")]
        let dock = crate::recording_position::dock_reserve(usable_bottom,
            (monitor.position().y as f64 + monitor.size().height as f64) / scale);
        #[cfg(not(target_os = "macos"))]
        let dock = 0.;
        let y = crate::recording_position::bottom(usable_bottom, area.position.y as f64 / scale, dock) * scale;
        window
            .set_position(tauri::PhysicalPosition::new(
                x.round() as i32,
                y.round() as i32,
            ))
            .map_err(|_| "recording_indicator_unavailable")?;
    }
    Ok(())
}
