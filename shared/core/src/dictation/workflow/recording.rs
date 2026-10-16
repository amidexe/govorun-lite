//! Capture lifetime in the host. Gesture and tail decisions come from Core;
//! callbacks carry leases so a late driver/timer cannot finish a new capture.
use govorun_core::recording_controls::{
    Action, Gesture, GestureState, Mode, StopTail, TailLease,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    pub generation: u64,
    pub started_at: u64,
    pub without_styling: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finish {
    pub capture: Lease,
    pub tail: TailLease,
    pub input_mode: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Start(Lease),
    Stop(Finish),
}

#[derive(Default)]
pub struct Controller {
    gesture: Gesture,
    tail: StopTail,
    generation: u64,
    active: Option<Lease>,
    finishing: Option<Finish>,
}

impl Controller {
    pub fn active(&self) -> Option<Lease> {
        self.active
    }

    pub fn is_current(&self, lease: Lease) -> bool {
        self.active == Some(lease)
    }

    pub fn press(
        &mut self,
        at: u64,
        mode: Mode,
        without_styling: bool,
    ) -> Result<Option<Effect>, &'static str> {
        if self.finishing.is_some() {
            return Ok(None);
        }
        // Never let a stale queued event advance the gesture of a new capture.
        if self.active.is_some_and(|lease| at < lease.started_at) {
            return Ok(None);
        }
        if self.active.is_none() && self.generation == u64::MAX {
            return Err("recording_generation_exhausted");
        }
        match self.gesture.down(at, mode, false) {
            Action::None => Ok(None),
            Action::Start => {
                self.generation += 1;
                let lease = Lease {
                    generation: self.generation,
                    started_at: at,
                    without_styling,
                };
                self.active = Some(lease);
                Ok(Some(Effect::Start(lease)))
            }
            Action::Finish => self.finish(at).map(|finish| finish.map(Effect::Stop)),
        }
    }

    pub fn release(&mut self, at: u64) -> Result<Option<Effect>, &'static str> {
        if self.finishing.is_some() {
            return Ok(None);
        }
        if self.gesture.up(at) == Action::Finish {
            self.finish(at).map(|finish| finish.map(Effect::Stop))
        } else {
            Ok(None)
        }
    }

    /// Automatic duration timers and UI finish commands use the same tail.
    /// A timer is always bound to the capture that scheduled it.
    pub fn stop(&mut self, lease: Lease, at: u64) -> Result<Option<Finish>, &'static str> {
        if !self.is_current(lease) {
            return Ok(None);
        }
        self.finish(at)
    }

    fn finish(&mut self, at: u64) -> Result<Option<Finish>, &'static str> {
        let Some(capture) = self.active else {
            return Ok(None);
        };
        if self.finishing.is_some() || at < capture.started_at {
            return Ok(None);
        }
        let tail = self
            .tail
            .begin(at)
            .map_err(|_| "recording_boundary_invalid")?;
        let finish = Finish {
            capture,
            tail,
            input_mode: if self.gesture.state() == GestureState::Latched {
                "confirm"
            } else {
                "hold"
            },
        };
        self.finishing = Some(finish);
        Ok(Some(finish))
    }

    /// Invoke only after the driver drained through the Core tail deadline.
    /// Resetting now permits another recording while ASR remains in its queue.
    pub fn drained(&mut self, finish: Finish, now: u64) -> bool {
        if self.finishing != Some(finish) || !self.tail.complete(finish.tail, now) {
            return false;
        }
        self.active = None;
        self.finishing = None;
        self.gesture.reset();
        true
    }

    /// A failed old driver must not hide or cancel the replacement capture.
    pub fn failed(&mut self, lease: Lease) -> bool {
        if !self.is_current(lease) {
            return false;
        }
        self.cancel();
        true
    }

    /// The caller also cancels its processing queue, including when no capture
    /// is active. No detached recognition may deliver after cancellation.
    pub fn cancel(&mut self) -> Option<Lease> {
        self.gesture.reset();
        self.tail.cancel();
        self.finishing = None;
        self.active.take()
    }
}
