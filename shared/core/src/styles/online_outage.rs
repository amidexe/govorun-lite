use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PROBE_BUDGET: u8 = 5;
const PROTECTION_MILLIS: i64 = 60_000;

/// Process-local availability only. No account, credit, text, or wall clock.
#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    paused: bool,
    generation: u64,
    probe_at: i64,
    probe_active: bool,
    last_failure: Option<i64>,
    backoff: u8,
    has_opened: bool,
    protected_until: i64,
    remaining: u8,
    fresh_demand: bool,
    last_recording_probe: Option<i64>,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    BeginRequest,
    Deadline { normal: i64 },
    FinishRequest {
        ticket: u64, failure: crate::online_protocol::Failure, cancelled: bool,
        retry_after: i64, jitter: f64,
        #[serde(default)] retry_hints: Vec<crate::online_protocol::RetryHint>,
    },
    Demand,
    Recording,
    NextProbe,
    FinishProbe {
        ticket: u64, healthy: bool, retry_after: i64, jitter: f64,
        sample_age: i64, elapsed: i64,
        #[serde(default)] retry_hints: Vec<crate::online_protocol::RetryHint>,
    },
    AbandonProbe { ticket: u64 },
}

pub(crate) fn transition(state: Option<State>, event: Event, now: i64) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    let mut action = "none";
    let mut ticket = None;
    let mut delay = 0;
    let mut deadline = None;
    match event {
        Event::BeginRequest => {
            action = if state.paused { "paused" } else { "request" };
            if !state.paused { ticket = Some(state.generation); }
        }
        Event::Deadline { normal } => {
            deadline = Some(if now < state.protected_until { normal.min(4_000) } else { normal });
        }
        Event::FinishRequest { ticket, failure, cancelled, retry_after, jitter, retry_hints } => {
            let retry_after = crate::online_protocol::retry_after(retry_after, &retry_hints);
            if ticket == state.generation && !state.paused {
                if state.last_failure.is_some_and(|last| now.saturating_sub(last) > PROTECTION_MILLIS)
                    && now >= state.protected_until {
                    state.last_failure = None;
                    state.backoff = 0;
                    state.has_opened = false;
                }
                // A successful concurrent request cannot erase another request's failure.
                if !cancelled && failure.transient() {
                    state.last_failure = Some(now);
                    state.open(now, retry_after, jitter, state.has_opened);
                }
            }
        }
        Event::Demand => {
            if state.paused {
                if state.remaining == 0 { state.remaining = PROBE_BUDGET; }
                else { state.fresh_demand = true; }
                action = "recover";
            }
        }
        Event::Recording => {
            if state.paused && !state.probe_active
                && state.last_recording_probe.is_none_or(|last| now.saturating_sub(last) >= 5_000) {
                state.last_recording_probe = Some(now);
                state.probe_at = now;
                state.remaining = state.remaining.max(1);
                action = "recover";
            }
        }
        Event::NextProbe => {
            action = if !state.paused || state.remaining == 0 || state.probe_active { "stop" }
                else if now < state.probe_at {
                    delay = state.probe_at.saturating_sub(now);
                    "wait"
                } else {
                    state.probe_active = true;
                    ticket = Some(state.generation);
                    "probe"
                };
        }
        Event::FinishProbe { ticket, healthy, retry_after, jitter, sample_age, elapsed, retry_hints } => {
            let retry_after = crate::online_protocol::retry_after(retry_after, &retry_hints);
            if ticket == state.generation && state.probe_active && state.paused {
                state.probe_active = false;
                state.remaining = state.remaining.saturating_sub(1);
                let age = sample_age.saturating_add(elapsed.max(0)).saturating_add(1_000);
                if healthy && sample_age >= 0 && age <= now.saturating_sub(state.last_failure.unwrap_or(now)) {
                    state.paused = false;
                    state.generation = state.generation.saturating_add(1);
                    state.probe_at = 0;
                    state.last_failure = Some(now);
                    state.protected_until = now.saturating_add(PROTECTION_MILLIS);
                    state.remaining = 0;
                    state.fresh_demand = false;
                } else {
                    state.open(now, retry_after, jitter, !healthy || sample_age < 0);
                    if state.remaining == 0 && state.fresh_demand {
                        state.remaining = PROBE_BUDGET;
                        state.fresh_demand = false;
                    }
                }
            }
        }
        Event::AbandonProbe { ticket } => {
            if ticket == state.generation { state.probe_active = false; }
        }
    }
    Ok(json!({"paused":state.paused,"probeAt":state.probe_at,"action":action,
        "ticket":ticket,"delayMillis":delay,"deadlineMillis":deadline,"state":state}))
}

impl State {
    fn open(&mut self, now: i64, retry_after: i64, jitter: f64, advance: bool) {
        self.backoff = self.backoff.saturating_add(u8::from(advance)).min(3);
        let base = [15_000, 30_000, 60_000, 120_000][usize::from(self.backoff)];
        let delay = (f64::from(base) * (0.8 + 0.4 * jitter.clamp(0.0, 1.0))) as i64;
        self.probe_at = now.saturating_add(delay.max(retry_after.max(0)));
        self.paused = true;
        self.generation = self.generation.saturating_add(1);
        self.probe_active = false;
        self.has_opened = true;
    }
}
