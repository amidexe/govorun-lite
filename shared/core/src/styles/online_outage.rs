use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A server that says when to come back (Retry-After of this much or more) pauses it at once.
const SERVER_PAUSE_MILLIS: i64 = 30_000;

/// When failures pause online improvement and for how long. Built in: two temporary failures in a row within two
/// minutes, one alone never does; pauses of 15, 30, 60, then 120 s. The account card's `stylePolicy` may replace
/// the numbers; each is clamped to what a client accepts, and anything malformed keeps the built-in ones.
#[derive(Clone, Debug, PartialEq)]
pub struct Policy {
    failures_to_open: u8,
    failure_window_millis: i64,
    backoff_millis: Vec<i64>,
}

impl Default for Policy {
    fn default() -> Self {
        Self { failures_to_open: 2, failure_window_millis: 120_000, backoff_millis: vec![15_000, 30_000, 60_000, 120_000] }
    }
}

impl Policy {
    pub fn from_card(card: &Value) -> Self {
        let (policy, pause) = (&card["stylePolicy"], &card["stylePolicy"]["pause"]);
        let (Some(failures), Some(window), Some(steps)) =
            (pause["failuresToOpen"].as_u64(), pause["failureWindowMs"].as_u64(), pause["backoffMs"].as_array())
        else {
            return Self::default();
        };
        let steps: Option<Vec<i64>> = steps.iter().take(8).map(|step| step.as_u64().map(|v| v.clamp(5_000, 600_000) as i64)).collect();
        match steps {
            Some(steps) if policy["version"] == 1 && !steps.is_empty() => Self {
                failures_to_open: failures.clamp(1, 5) as u8,
                failure_window_millis: window.clamp(10_000, 600_000) as i64,
                backoff_millis: steps,
            },
            _ => Self::default(),
        }
    }
}

/// Process-local availability only. No account, credit, text, or wall clock.
/// While paused, dictation is inserted at once; after the pause the next real dictation is the trial: its success
/// closes the pause, its failure pauses again for longer. No availability probes, no shortened deadline.
#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    paused: bool,
    generation: u64,
    trial_at: i64,
    trial_active: bool,
    failures: u8,
    last_failure: Option<i64>,
    backoff: u8,
}

/// Probe events (Demand, Recording, NextProbe, FinishProbe, AbandonProbe) stay accepted from the canary era's
/// callers; this policy has no probes and ignores them.
#[allow(dead_code)]
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
    transition_with(state, event, now, &Policy::default())
}

pub(crate) fn transition_with(state: Option<State>, event: Event, now: i64, policy: &Policy) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    let mut action = "none";
    let mut ticket = None;
    let mut deadline = None;
    match event {
        Event::BeginRequest => {
            if !state.paused {
                action = "request";
                ticket = Some(state.generation);
            } else if !state.trial_active && now >= state.trial_at {
                state.trial_active = true;
                action = "request";
                ticket = Some(state.generation);
            } else {
                action = "paused";
            }
        }
        Event::Deadline { normal } => deadline = Some(normal),
        Event::FinishRequest { ticket, failure, cancelled, retry_after, jitter, retry_hints } => {
            let retry_after = crate::online_protocol::retry_after(retry_after, &retry_hints);
            if ticket == state.generation {
                let transient = !cancelled && failure.transient();
                if state.paused {
                    // The trial dictation after a pause.
                    state.trial_active = false;
                    if transient {
                        state.backoff = state.backoff.saturating_add(1);
                        state.open(policy, now, retry_after, jitter);
                    } else if !cancelled {
                        state.close();
                    }
                } else if transient {
                    let recent = state.last_failure.is_some_and(|last| now.saturating_sub(last) <= policy.failure_window_millis);
                    state.failures = if recent { state.failures.saturating_add(1) } else { 1 };
                    state.last_failure = Some(now);
                    if state.failures >= policy.failures_to_open || retry_after >= SERVER_PAUSE_MILLIS {
                        state.open(policy, now, retry_after, jitter);
                    }
                } else if !cancelled {
                    state.failures = 0;
                    state.last_failure = None;
                }
            }
        }
        // Availability probes belong to builds before 2.0.46; this policy never asks for one.
        Event::Demand | Event::Recording | Event::FinishProbe { .. } | Event::AbandonProbe { .. } => {}
        Event::NextProbe => action = "stop",
    }
    Ok(json!({"paused":state.paused,"probeAt":state.trial_at,"action":action,
        "ticket":ticket,"delayMillis":0,"deadlineMillis":deadline,"state":state}))
}

impl State {
    fn open(&mut self, policy: &Policy, now: i64, retry_after: i64, jitter: f64) {
        let last = policy.backoff_millis.len().saturating_sub(1);
        self.backoff = self.backoff.min(last as u8);
        let base = policy.backoff_millis.get(usize::from(self.backoff)).copied().unwrap_or(120_000);
        let delay = (base as f64 * (0.8 + 0.4 * jitter.clamp(0.0, 1.0))) as i64;
        self.trial_at = now.saturating_add(delay.max(retry_after.max(0)));
        self.paused = true;
        self.trial_active = false;
        self.failures = 0;
        self.generation = self.generation.saturating_add(1);
    }

    fn close(&mut self) {
        self.paused = false;
        self.backoff = 0;
        self.failures = 0;
        self.last_failure = None;
        self.generation = self.generation.saturating_add(1);
    }
}
