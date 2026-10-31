use crate::{style_start::{retry_after_delay, Failure}, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    retries: u8,
    phase: Phase,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Phase { #[default] Ready, AwaitingResponse, Received, Done }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Begin { remaining_millis: u64 },
    Response { status: u16, retry_after: Option<String>, now_millis: i64, remaining_millis: u64 },
    Transport { failure: Failure, remaining_millis: u64 },
    Processing { elapsed_millis: u64, remaining_millis: u64 },
}

/// Polling a server-accepted job never issues, settles or replays a credit.
/// Two transient retries belong to the whole job, including processing replies.
pub(crate) fn transition(state: Option<State>, event: Event) -> Result<Value> {
    transition_for(state, event, false)
}

/// An accepted note keeps its server capability through transient status outages, within its original deadline.
pub(crate) fn transition_note(state: Option<State>, event: Event) -> Result<Value> {
    transition_for(state, event, true)
}

fn transition_for(state: Option<State>, event: Event, note: bool) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    let maximum = if note { 5 } else { 2 };
    if state.retries > maximum || state.phase == Phase::Done { return Err(Error::InvalidRequest); }
    let mut delay = 0;
    let mut timeout = 0;
    let action = match event {
        Event::Begin { remaining_millis } => {
            if state.phase != Phase::Ready { return Err(Error::InvalidRequest); }
            if remaining_millis == 0 { state.phase = Phase::Done; "stop" }
            else {
                state.phase = Phase::AwaitingResponse;
                timeout = remaining_millis.min(crate::dictation_timing::STATUS_REQUEST_MILLIS);
                "attempt"
            }
        }
        Event::Response { status, retry_after, now_millis, remaining_millis } => {
            if state.phase != Phase::AwaitingResponse || !(100..=599).contains(&status) {
                return Err(Error::InvalidRequest);
            }
            if (200..=299).contains(&status) { state.phase = Phase::Received; "read" }
            else {
                let retryable = matches!(status, 408 | 429 | 502 | 503 | 504);
                delay = retry_after_delay(retry_after.as_deref(), now_millis)
                    .map(|v| v.max(200)).unwrap_or(200 << state.retries);
                retry(&mut state, retryable, &mut delay, remaining_millis, note)
            }
        }
        Event::Transport { failure, remaining_millis } => {
            if state.phase != Phase::AwaitingResponse { return Err(Error::InvalidRequest); }
            delay = 200 << state.retries;
            retry(&mut state, matches!(failure, Failure::Network), &mut delay, remaining_millis, note)
        }
        Event::Processing { elapsed_millis, remaining_millis } => {
            if state.phase != Phase::Received { return Err(Error::InvalidRequest); }
            delay = 100_u64.saturating_sub(elapsed_millis);
            if delay >= remaining_millis { state.phase = Phase::Done; delay = 0; "stop" }
            else { state.phase = Phase::Ready; "wait" }
        }
    };
    Ok(json!({"state":state,"action":action,"delayMillis":delay,"timeoutMillis":timeout}))
}

fn retry(state: &mut State, allowed: bool, delay: &mut u64, remaining: u64, note: bool) -> &'static str {
    if allowed && (note || state.retries < 2) && *delay < remaining {
        // Notes back off up to 6.4s; ordinary dictation retains its two transient retries.
        state.retries = (state.retries + 1).min(if note { 5 } else { 2 });
        state.phase = Phase::Ready;
        "retry"
    } else {
        state.phase = Phase::Done;
        *delay = 0;
        "stop"
    }
}
