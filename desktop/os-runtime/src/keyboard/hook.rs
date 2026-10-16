//! A synchronous hook owns physical key pairing. Its sink only enqueues closed
//! recording edges; it must never wait for the UI, audio, model or storage.
use super::{Binding, Event, Pairing, Platform, Target};
use handy_keys::KeyEvent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::{
    thread::{self, JoinHandle},
    time::Instant,
};
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[path = "handy_hook.rs"]
mod platform;

pub(super) struct State {
    pub pairing: Pairing,
    pub capture: super::assignment::Capture,
    pub capture_ports: Option<CapturePorts>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub bindings: Vec<(Target, Binding)>,
}
pub(super) struct CapturePorts {
    pub(super) focus: Arc<dyn Fn() -> bool + Send + Sync>,
    sink: Arc<dyn Fn(super::assignment::CaptureEvent) + Send + Sync>,
}
pub(super) struct Context {
    pub state: Mutex<State>,
    pub running: AtomicBool,
    pub cancel_enabled: AtomicBool,
    pub origin: Instant,
    pub sink: Arc<dyn Fn(Event) + Send + Sync>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub blocking: handy_keys::BlockingHotkeys,
}
impl Context {
    pub fn route(&self, code: u16, raw: KeyEvent) -> bool {
        // Stamp on the callback thread before any asynchronous startup.
        let now = self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let (packet, capture, sink) = match self.state.lock() {
            Ok(mut state) => {
                let allowed = state
                    .capture_ports
                    .as_ref()
                    .is_some_and(|ports| (ports.focus)());
                let capture = state.capture.process(code, raw, allowed);
                let sink = state.capture_ports.as_ref().map(|ports| ports.sink.clone());
                let packet = if capture.exclusive {
                    super::Packet {
                        event: None,
                        block: capture.block,
                    }
                } else {
                    state
                        .pairing
                        .process(raw, now, self.cancel_enabled.load(Ordering::Acquire))
                };
                (packet, capture.event, sink)
            }
            Err(_) => return false,
        };
        if let (Some(event), Some(sink)) = (capture, sink) {
            sink(event);
        }
        if let Some(event) = packet.event {
            (self.sink)(event);
        }
        packet.block
    }
    pub fn recover(&self) {
        let now = self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let (event, capture, sink) = if let Ok(mut state) = self.state.lock() {
            let capture = state.capture.recover();
            let sink = state.capture_ports.as_ref().map(|ports| ports.sink.clone());
            (state.pairing.reset(now), capture, sink)
        } else {
            (None, None, None)
        };
        if let Some(event) = event {
            (self.sink)(event);
        }
        if let (Some(event), Some(sink)) = (capture, sink) {
            sink(event);
        }
    }
}
pub struct Hook {
    context: Arc<Context>,
    thread: Option<JoinHandle<()>>,
}
impl Hook {
    pub fn start(
        bindings: Vec<(Target, Binding)>,
        origin: Instant,
        sink: Arc<dyn Fn(Event) + Send + Sync>,
    ) -> Result<Self, &'static str> {
        let platform = if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            Platform::Windows
        };
        let context = Arc::new(Context {
            state: Mutex::new(State {
                pairing: Pairing::new(platform, bindings.clone()),
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                bindings,
                capture: super::assignment::Capture::new(platform),
                capture_ports: None,
            }),
            running: AtomicBool::new(true),
            cancel_enabled: AtomicBool::new(false),
            origin,
            sink,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            blocking: Default::default(),
        });
        let (send, receive) = mpsc::sync_channel(1);
        let run = context.clone();
        let thread = thread::Builder::new()
            .name("Govorun input".into())
            .spawn(move || {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                platform::run(run, send);
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    let _ = run;
                    let _ = send.send(Err("hotkeys_unsupported_platform"));
                }
            })
            .map_err(|_| "hotkeys_unavailable")?;
        match receive.recv().unwrap_or(Err("hotkeys_unavailable")) {
            Ok(()) => Ok(Self {
                context,
                thread: Some(thread),
            }),
            Err(error) => {
                context.running.store(false, Ordering::Release);
                let _ = thread.join();
                Err(error)
            }
        }
    }
    pub fn capture(
        &self,
        id: String,
        focus: Arc<dyn Fn() -> bool + Send + Sync>,
        sink: Arc<dyn Fn(super::assignment::CaptureEvent) + Send + Sync>,
    ) -> Result<(), &'static str> {
        if !focus() {
            return Err("hotkeys_capture_unfocused");
        }
        let event = {
            let mut state = self
                .context
                .state
                .lock()
                .map_err(|_| "hotkeys_unavailable")?;
            state.capture.start(id)?;
            state.capture_ports = Some(CapturePorts { focus, sink });
            let now = self
                .context
                .origin
                .elapsed()
                .as_nanos()
                .min(u64::MAX as u128) as u64;
            state.pairing.reset(now)
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        platform::synchronize(&self.context);
        if let Some(event) = event {
            (self.context.sink)(event);
        }
        Ok(())
    }
    pub fn stop_capture(&self, id: &str) -> Result<(), &'static str> {
        self.context
            .state
            .lock()
            .map_err(|_| "hotkeys_unavailable")?
            .capture
            .stop(id);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        platform::synchronize(&self.context);
        Ok(())
    }
    pub fn set_cancel_enabled(&self, enabled: bool) {
        self.context
            .cancel_enabled
            .store(enabled, Ordering::Release);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        platform::synchronize(&self.context);
    }
    pub fn replace(&self, bindings: Vec<(Target, Binding)>) -> Result<(), &'static str> {
        let now = self
            .context
            .origin
            .elapsed()
            .as_nanos()
            .min(u64::MAX as u128) as u64;
        let event = {
            let mut state = self
                .context
                .state
                .lock()
                .map_err(|_| "hotkeys_unavailable")?;
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                state.bindings = bindings.clone();
            }
            state.pairing.replace(bindings, now)
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        platform::synchronize(&self.context);
        if let Some(event) = event {
            (self.context.sink)(event);
        }
        Ok(())
    }
    pub fn recover(&self) {
        self.context.recover();
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        self.context.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.context.recover();
    }
}
