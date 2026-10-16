//! Connect Core's capture leases to the blocking audio executor. All input
//! timestamps use the hook's clock; OS completion cannot shorten Core's tail.
use super::recording::{Controller, Effect, Finish, Lease};
use govorun_core::recording_controls::Mode;

pub trait Ports {
    type Context;
    type Job;
    fn permission(&mut self, _: Lease) -> Result<(), &'static str> {
        Ok(())
    }
    fn begin(&mut self, lease: Lease) -> Result<Self::Context, &'static str>;
    fn release(&mut self, finish: Finish) -> Result<(), &'static str>;
    fn cancel_capture(&mut self);
    fn cancel_processing(&mut self);
    fn discard(&mut self, job: Self::Job);
    fn ready(&mut self, lease: Lease);
    fn warning(&mut self, lease: Lease);
    fn process(&mut self, finish: Finish, context: Self::Context, job: Self::Job);
    fn failed(&mut self, lease: Lease, code: &'static str);
    fn failed_native(&mut self, lease: Lease, code: &'static str, _native: Option<crate::journal_report::capture_failure::Native>) {
        self.failed(lease, code);
    }
    fn cancelled(&mut self);
}
struct Attempt<C, J> {
    lease: Lease,
    context: C,
    finish: Option<Finish>,
    job: Option<J>,
    ready: bool,
}
pub struct Owner<P: Ports> {
    controller: Controller,
    attempt: Option<Attempt<P::Context, P::Job>>,
    ports: P,
}
impl<P: Ports> Owner<P> {
    pub fn new(ports: P) -> Self {
        Self {
            controller: Controller::default(),
            attempt: None,
            ports,
        }
    }
    pub fn active(&self) -> Option<Lease> {
        self.controller.active()
    }
    pub fn recording(&self) -> Option<Lease> {
        self.attempt
            .as_ref()
            .filter(|v| v.finish.is_none())
            .map(|v| v.lease)
    }
    pub fn deadline(&self) -> Option<u64> {
        self.attempt
            .as_ref()?
            .finish
            .map(|finish| finish.tail.deadline_nanos())
    }
    pub fn press(&mut self, at: u64, mode: Mode, plain: bool) -> Result<(), &'static str> {
        let effect = self.controller.press(at, mode, plain)?;
        self.effect(effect)
    }
    pub fn release(&mut self, at: u64) -> Result<(), &'static str> {
        let effect = self.controller.release(at)?;
        self.effect(effect)
    }
    pub fn stop(&mut self, lease: Lease, at: u64) -> Result<(), &'static str> {
        let effect = self.controller.stop(lease, at)?.map(Effect::Stop);
        self.effect(effect)
    }
    fn effect(&mut self, effect: Option<Effect>) -> Result<(), &'static str> {
        match effect {
            None => Ok(()),
            Some(Effect::Start(lease)) => match self
                .ports
                .permission(lease)
                .and_then(|_| self.ports.begin(lease))
            {
                Ok(context) => {
                    self.attempt = Some(Attempt {
                        lease,
                        context,
                        finish: None,
                        job: None,
                        ready: false,
                    });
                    Ok(())
                }
                Err(code) => {
                    self.controller.failed(lease);
                    self.ports.cancel_capture();
                    self.ports.failed(lease, code);
                    Err(code)
                }
            },
            Some(Effect::Stop(finish)) => {
                self.attempt.as_mut().ok_or("recording_not_active")?.finish = Some(finish);
                if let Err(code) = self.ports.release(finish) {
                    self.failed(finish.capture.generation, code);
                    return Err(code);
                }
                Ok(())
            }
        }
    }
    pub fn ready(&mut self, id: u64) {
        if let Some(attempt) = self.attempt.as_mut().filter(|v| v.lease.generation == id) {
            if !attempt.ready && attempt.finish.is_none() {
                attempt.ready = true;
                self.ports.ready(attempt.lease);
            }
        }
    }
    pub fn warning(&mut self, lease: Lease) {
        if self.recording() == Some(lease) {
            self.ports.warning(lease);
        }
    }
    pub fn captured(&mut self, id: u64, job: P::Job, now: u64) {
        let Some(attempt) = self.attempt.as_mut().filter(|v| v.lease.generation == id) else {
            self.ports.discard(job);
            return;
        };
        if attempt.finish.is_none() || attempt.job.is_some() {
            self.ports.discard(job);
            self.failed(id, "recording_completion_invalid");
            return;
        }
        attempt.job = Some(job);
        self.tick(now);
    }
    pub fn tick(&mut self, now: u64) {
        let Some(attempt) = self.attempt.as_ref() else {
            return;
        };
        let Some(finish) = attempt.finish.filter(|_| attempt.job.is_some()) else {
            return;
        };
        if !self.controller.drained(finish, now) {
            return;
        }
        let mut attempt = self.attempt.take().expect("current drained attempt");
        self.ports.process(
            finish,
            attempt.context,
            attempt.job.take().expect("drained job"),
        );
    }
    pub fn failed(&mut self, id: u64, code: &'static str) {
        self.failed_native(id, code, None);
    }
    pub(super) fn failed_native(&mut self, id: u64, code: &'static str, native: Option<crate::journal_report::capture_failure::Native>) {
        let Some(lease) = self.active().filter(|v| v.generation == id) else {
            return;
        };
        if self.controller.failed(lease) {
            if let Some(job) = self.attempt.take().and_then(|v| v.job) {
                self.ports.discard(job);
            }
            self.ports.cancel_capture();
            self.ports.failed_native(lease, code, native);
        }
    }
    /// Escape cancels queued recognition as well as the current microphone.
    pub fn cancel(&mut self) {
        self.controller.cancel();
        if let Some(job) = self.attempt.take().and_then(|v| v.job) {
            self.ports.discard(job);
        }
        self.ports.cancel_capture();
        self.ports.cancel_processing();
        self.ports.cancelled();
    }
}
