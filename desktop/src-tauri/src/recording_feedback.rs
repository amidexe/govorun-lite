//! Recording cues/mute on one OS worker, composed with the existing indicator.
use crate::{
    features::dictation::recording::Lease,
    local_dictation::Presentation,
    recording_runtime::{Runtime, View},
};
use govorun_core::features::dictation::feedback_worker as worker;
use govorun_core::journal_report::capture_failure::Native;
use govorun_os_runtime::media::{Cue, Mute, Output};
use std::{
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};
struct Media {
    cue: Cue,
    output: Option<Mute<Output>>,
}
impl worker::Output for Media {
    fn cue(&mut self, starting: bool) -> Result<(), &'static str> {
        self.cue.play(starting)
    }
    fn mute(&mut self) -> Result<(), &'static str> {
        if self.output.is_none() {
            self.output = Some(Mute::new(Output::new()?));
        }
        self.output.as_mut().unwrap().mute()
    }
    fn restore(&mut self) -> Result<(), &'static str> {
        self.output.as_mut().map_or(Ok(()), Mute::restore)
    }
}
pub struct Adapter {
    worker: worker::Worker,
    indicator: Arc<dyn View>,
    runtime: OnceLock<Weak<Runtime>>,
    capture_failure: OnceLock<Arc<dyn Fn(&'static str, Option<Native>) + Send + Sync>>,
    capture_diagnostics: OnceLock<Arc<dyn Fn(String) + Send + Sync>>,
}
impl Adapter {
    pub fn new(
        indicator: Arc<dyn View>,
        error: Arc<dyn Fn(&'static str) + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            worker: worker::Worker::new(
                if cfg!(target_os = "macos") {
                    Duration::from_millis(250)
                } else {
                    Duration::ZERO
                },
                cfg!(target_os = "macos"),
                || Media {
                    cue: Cue::new(),
                    output: None,
                },
                error,
            ),
            indicator,
            runtime: OnceLock::new(),
            capture_failure: OnceLock::new(),
            capture_diagnostics: OnceLock::new(),
        })
    }
    pub fn attach_capture_failure(&self, observer: Arc<dyn Fn(&'static str, Option<Native>) + Send + Sync>) {
        let _ = self.capture_failure.set(observer);
    }
    pub fn attach_capture_diagnostics(&self, observer: Arc<dyn Fn(String) + Send + Sync>) {
        let _ = self.capture_diagnostics.set(observer);
    }
    pub fn attach(&self, runtime: &Arc<Runtime>) {
        let _ = self.runtime.set(Arc::downgrade(runtime));
    }
    fn options(&self) -> (bool, bool) {
        self.runtime
            .get()
            .and_then(Weak::upgrade)
            .and_then(|runtime| runtime.feedback_options().ok())
            .unwrap_or((false, false))
    }
    pub fn shutdown(&self) {
        self.worker.shutdown();
    }
}
impl View for Adapter {
    fn presentation(&self, lease: Lease) -> Arc<dyn Presentation> {
        self.indicator.presentation(lease)
    }
    fn admitted(&self, lease: Lease) {
        self.worker.start(lease.generation, self.options().0);
        self.indicator.admitted(lease);
    }
    fn capture_timing(&self, detail: String) {
        if let Some(observer) = self.capture_diagnostics.get() {
            observer(detail);
        }
    }
    fn ready(&self, lease: Lease) {
        let (play, mute) = self.options();
        self.worker.ready(lease.generation, play, mute);
        self.indicator.ready(lease);
    }
    fn stopped(&self, lease: Lease) {
        self.worker.finish(lease.generation, self.options().0);
        self.indicator.stopped(lease);
    }
    fn failed(&self, lease: Lease, code: &'static str) {
        self.failed_native(lease, code, None);
    }
    fn failed_native(&self, lease: Lease, code: &'static str, native: Option<Native>) {
        if let Some(observer) = self.capture_failure.get() {
            observer(code, native);
        }
        self.worker.finish(lease.generation, self.options().0);
        self.indicator.failed(lease, code);
    }
    fn cancelled(&self) {
        self.worker.cancel();
        self.indicator.cancelled();
    }
    fn warning(&self, lease: Lease) {
        self.indicator.warning(lease);
    }
    fn notice(&self, lease: Lease, code: &'static str) {
        self.indicator.notice(lease, code);
    }
}
