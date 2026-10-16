//! Typed execution budget returned by Core. The host schedules these deadlines
//! on its monotonic clock and binds every callback to its capture lease.
use super::recording::Lease;
use super::{core, Failure};
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub stop_after_millis: u64,
    pub warn_after_millis: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Warn(Lease),
    Stop(Lease),
}
struct Pending {
    lease: Lease,
    stop: Instant,
    warning: Option<Instant>,
}
#[derive(Default)]
pub struct Schedule {
    pending: Option<Pending>,
}
impl Schedule {
    pub fn install(&mut self, lease: Lease, ready_at: Instant, plan: Plan, current: Option<Lease>) {
        if current != Some(lease) {
            return;
        }
        self.pending = plan.stop_at(ready_at).map(|stop| Pending {
            lease,
            stop,
            warning: plan.warn_at(ready_at),
        });
    }
    pub fn next(&mut self, current: Option<Lease>) -> Option<Instant> {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| Some(p.lease) != current)
        {
            self.pending = None;
        }
        let p = self.pending.as_ref()?;
        Some(p.warning.map_or(p.stop, |warn| warn.min(p.stop)))
    }
    pub fn tick(&mut self, now: Instant, current: Option<Lease>) -> Option<Action> {
        if self.next(current).is_none_or(|next| next > now) {
            return None;
        }
        let pending = self.pending.as_mut()?;
        // Delayed delivery past stop must not flash an expired warning.
        if now >= pending.stop {
            let lease = pending.lease;
            self.pending = None;
            Some(Action::Stop(lease))
        } else {
            pending.warning = None;
            Some(Action::Warn(pending.lease))
        }
    }
}
pub fn plan(
    self_hosted: bool,
    selected: bool,
    ready: bool,
    streaming: bool,
    local_timer_enabled: bool,
    configured_minutes: u32,
) -> Result<Plan, Failure> {
    serde_json::from_value(core::evaluate(json!({"kind":"recording","facts":{
        "selfHosted":self_hosted,"officialSelected":selected,"officialReady":ready,
        "streaming":streaming,"localTimerEnabled":local_timer_enabled,
        "configuredMinutes":configured_minutes}}))?)
    .map_err(|_| Failure::Effect("recording_timer_invalid"))
}
impl Plan {
    pub fn stop_at(&self, ready_at: Instant) -> Option<Instant> {
        if self.stop_after_millis == 0 {
            None
        } else {
            ready_at.checked_add(Duration::from_millis(self.stop_after_millis))
        }
    }
    pub fn warn_at(&self, ready_at: Instant) -> Option<Instant> {
        ready_at.checked_add(Duration::from_millis(self.warn_after_millis?))
    }
}
