//! The key receiver can release/cancel an attempt while a driver is opening.
//! An old signal never belongs to the next attempt, even on a reused stream.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
/// Physical timestamps only; no user data or device identity.
#[derive(Clone, Copy)]
pub struct StartupObservation {
    pub started: std::time::Instant,
    pub opening: std::time::Instant,
    pub opened: std::time::Instant,
    pub first_packet: std::time::Instant,
    pub reported: std::time::Instant,
}
pub struct Admission {
    start: Option<u64>,
    end: OnceLock<u64>,
    cancelled: AtomicBool,
    started: std::time::Instant,
    opening: OnceLock<std::time::Instant>,
    opened: OnceLock<std::time::Instant>,
    first_packet: OnceLock<std::time::Instant>,
    native_failure: Mutex<Option<govorun_core::journal_report::capture_failure::Native>>,
}
impl Admission {
    pub fn new(start: Option<u64>) -> Arc<Self> {
        Arc::new(Self {
            start,
            end: OnceLock::new(),
            cancelled: AtomicBool::new(false),
            started: std::time::Instant::now(),
            opening: OnceLock::new(),
            opened: OnceLock::new(),
            first_packet: OnceLock::new(),
            native_failure: Mutex::new(None),
        })
    }
    pub(super) fn input_opening(&self) {
        let _ = self.opening.set(std::time::Instant::now());
    }
    pub(super) fn input_opened(&self) {
        let _ = self.opened.set(std::time::Instant::now());
    }
    pub fn set_native_failure(&self, failure: Option<govorun_core::journal_report::capture_failure::Native>) {
        if let Ok(mut current) = self.native_failure.lock() { *current = failure; }
    }
    pub fn native_failure(&self) -> Option<govorun_core::journal_report::capture_failure::Native> {
        self.native_failure.lock().ok().and_then(|value| *value)
    }
    pub fn start_nanos(&self) -> Option<u64> { self.start }
    pub fn mark_first_packet(&self) {
        if self.start.is_some() && !self.is_cancelled() {
            let _ = self.first_packet.set(std::time::Instant::now());
        }
    }
    pub fn first_packet_at(&self) -> Option<std::time::Instant> {
        self.first_packet.get().copied()
    }
    pub fn startup_observation(&self) -> Option<StartupObservation> {
        Some(StartupObservation {
            started: self.started,
            opening: *self.opening.get()?,
            opened: *self.opened.get()?,
            first_packet: self.first_packet_at()?,
            reported: std::time::Instant::now(),
        })
    }
    pub fn release(&self, deadline: u64) -> Result<(), &'static str> {
        if self.start.is_none_or(|start| deadline < start) {
            return Err("recording_boundary_invalid");
        }
        let _ = self.end.set(deadline);
        Ok(())
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub fn deadline(&self) -> Option<u64> {
        self.end.get().copied()
    }
}
