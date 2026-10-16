//! Gesture and stop-tail rules, driven by monotonic OS event timestamps.
//! Capture and timers execute in the host; this module never opens a device.
use crate::{Error, Result};

pub const HOLD_THRESHOLD_NANOS: u64 = 350_000_000;
pub const STOP_TAIL_NANOS: u64 = 0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// A short tap latches recording; a long hold stops on release.
    #[default]
    Adaptive,
    Hold,
    Confirm,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GestureState {
    #[default]
    Idle,
    Pressed {
        at_nanos: u64,
    },
    Latched,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Start,
    Finish,
}

#[derive(Clone, Debug, Default)]
pub struct Gesture {
    state: GestureState,
    mode: Mode,
}
impl Gesture {
    pub fn state(&self) -> GestureState {
        self.state
    }

    /// The selected mode is captured at the first press, not read again while
    /// audio startup is pending. Auto-repeat cannot start or finish a capture.
    pub fn down(&mut self, at_nanos: u64, mode: Mode, repeated: bool) -> Action {
        if repeated {
            return Action::None;
        }
        match self.state {
            GestureState::Idle => {
                self.mode = mode;
                self.state = GestureState::Pressed { at_nanos };
                Action::Start
            }
            _ if self.mode == Mode::Hold => Action::None,
            _ => Action::Finish,
        }
    }

    pub fn up(&mut self, at_nanos: u64) -> Action {
        let GestureState::Pressed { at_nanos: pressed } = self.state else {
            return Action::None;
        };
        if at_nanos < pressed {
            return Action::None; // A queued release cannot finish a newer capture.
        }
        match self.mode {
            Mode::Hold => Action::Finish,
            Mode::Adaptive if at_nanos - pressed >= HOLD_THRESHOLD_NANOS => Action::Finish,
            _ => {
                self.state = GestureState::Latched;
                Action::None
            }
        }
    }

    /// Completion/cancellation of capture owns this reset, not the OS hook.
    pub fn reset(&mut self) {
        self.state = GestureState::Idle;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailLease {
    generation: u64,
    deadline_nanos: u64,
}
impl TailLease {
    pub fn deadline_nanos(self) -> u64 {
        self.deadline_nanos
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopTail {
    generation: u64,
    pending: Option<TailLease>,
}
impl StopTail {
    /// The host keeps capture active until this exact lease completes. Starting
    /// another wait replaces the old one, even if its timer finishes later.
    pub fn begin(&mut self, now_nanos: u64) -> Result<TailLease> {
        if now_nanos == u64::MAX {
            return Err(Error::InvalidRequest);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(Error::InvalidRequest)?;
        let deadline_nanos = now_nanos
            .checked_add(STOP_TAIL_NANOS)
            .ok_or(Error::InvalidRequest)?;
        let lease = TailLease {
            generation,
            deadline_nanos,
        };
        self.generation = generation;
        self.pending = Some(lease);
        Ok(lease)
    }

    pub fn complete(&mut self, lease: TailLease, now_nanos: u64) -> bool {
        if self.pending != Some(lease) || now_nanos < lease.deadline_nanos {
            return false;
        }
        self.pending = None;
        true
    }

    pub fn fail(&mut self, lease: TailLease) {
        if self.pending == Some(lease) {
            self.pending = None;
        }
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
