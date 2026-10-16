//! Cue completion and microphone readiness independently fence delayed mute.
//! Old capture/cue/timer callbacks never acquire a newer recording's output.
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCue(u64),
    StopCue(u64),
    Mute(u64, Duration),
    Restore,
}
struct Capture {
    id: u64,
    ready: bool,
    cue_finished: bool,
    mute: bool,
    scheduled: bool,
    mute_at: Option<Instant>,
}
pub struct State {
    active: Option<Capture>,
    last: u64,
    delay: Duration,
    cue_on_admission: bool,
}
impl State {
    pub fn new(delay: Duration, cue_on_admission: bool) -> Self {
        Self {
            active: None,
            last: 0,
            delay,
            cue_on_admission,
        }
    }
    pub fn start(&mut self, id: u64, play: bool) -> Vec<Effect> {
        if id <= self.last {
            return Vec::new();
        }
        self.last = id;
        let mut effects = Vec::new();
        if self.active.take().is_some() {
            effects.push(Effect::Restore);
        }
        self.active = Some(Capture {
            id,
            ready: false,
            cue_finished: !play && self.cue_on_admission,
            mute: false,
            scheduled: false,
            mute_at: None,
        });
        if play && self.cue_on_admission {
            effects.push(Effect::StartCue(id));
        }
        effects
    }
    pub fn ready(&mut self, id: u64, play: bool, mute: bool, now: Instant) -> Vec<Effect> {
        let Some(capture) = self
            .active
            .as_mut()
            .filter(|capture| capture.id == id && !capture.ready)
        else {
            return Vec::new();
        };
        capture.ready = true;
        capture.mute = mute;
        capture.mute_at = now.checked_add(self.delay);
        if !self.cue_on_admission {
            capture.cue_finished = !play;
            if play {
                return vec![Effect::StartCue(id)];
            }
        }
        self.schedule(now)
    }
    pub fn cue_finished(&mut self, id: u64, now: Instant) -> Vec<Effect> {
        if let Some(capture) = self.active.as_mut().filter(|capture| capture.id == id) {
            capture.cue_finished = true;
        }
        self.schedule(now)
    }
    fn schedule(&mut self, now: Instant) -> Vec<Effect> {
        let Some(capture) = self.active.as_mut().filter(|capture| {
            capture.ready && capture.cue_finished && capture.mute && !capture.scheduled
        }) else {
            return Vec::new();
        };
        capture.scheduled = true;
        vec![Effect::Mute(
            capture.id,
            capture
                .mute_at
                .map_or(Duration::ZERO, |at| at.saturating_duration_since(now)),
        )]
    }
    pub fn cue_allowed(&self, id: u64, starting: bool) -> bool {
        if starting {
            self.active.as_ref().is_some_and(|capture| capture.id == id)
        } else {
            self.last == id && self.active.is_none()
        }
    }
    pub fn owns_mute(&self, id: u64) -> bool {
        self.active.as_ref().is_some_and(|capture| {
            capture.id == id && capture.ready && capture.cue_finished && capture.mute
        })
    }
    pub fn finish(&mut self, id: u64, play: bool) -> Vec<Effect> {
        if self.active.as_ref().is_none_or(|capture| capture.id != id) {
            return Vec::new();
        }
        let capture = self.active.take().unwrap();
        let mut effects = vec![Effect::Restore];
        if capture.ready && play {
            effects.push(Effect::StopCue(id));
        }
        effects
    }
    pub fn cancel(&mut self) -> Vec<Effect> {
        self.active = None;
        vec![Effect::Restore]
    }
}
