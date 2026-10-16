use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    attempts: u8,
    ambiguous: bool,
    awaiting_response: bool,
    done: bool,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Begin { remaining_millis: u64 },
    Response { status: u16, body: String, retry_after: Option<String>, now_millis: i64,
        remaining_millis: u64 },
    Transport { failure: Failure, remaining_millis: u64 },
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Failure { Network, Offline, Security, Cancelled, Deadline, Other }

/// One acceptance replay owner. The caller retains the same body, request ID
/// and lease, and provides time/transport facts without making retry decisions.
pub(crate) fn transition(state: Option<State>, event: Event) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    if state.attempts > 2 || state.done { return Err(Error::InvalidRequest); }
    let action;
    let mut delay = 0;
    let mut timeout = 0;
    match event {
        Event::Begin { remaining_millis } => {
            if state.awaiting_response { return Err(Error::InvalidRequest); }
            if state.attempts >= 2 || remaining_millis == 0 {
                state.done = true;
                action = "stop";
            } else {
                state.attempts += 1;
                state.awaiting_response = true;
                timeout = remaining_millis.min(crate::dictation_timing::START_ATTEMPT_MILLIS);
                action = "attempt";
            }
        }
        Event::Response { status, body, retry_after, now_millis, remaining_millis } => {
            if !state.awaiting_response || !(100..=599).contains(&status) { return Err(Error::InvalidRequest); }
            state.awaiting_response = false;
            if status == 202 { action = "accept"; state.done = true; }
            else if !state.ambiguous && status == 401 && invalid_pass(&body) {
                action = "discard"; state.done = true;
            } else if !state.ambiguous && status == 429 {
                action = "restore"; state.done = true;
            } else {
                state.ambiguous = true;
                let retryable = matches!(status, 408 | 429 | 502 | 503 | 504);
                delay = retry_after_delay(retry_after.as_deref(), now_millis).unwrap_or(0).max(200);
                if retryable && state.attempts < 2 && delay < remaining_millis { action = "retry"; }
                else { action = "reconcile"; state.done = true; delay = 0; }
            }
        }
        Event::Transport { failure, remaining_millis } => {
            if !state.awaiting_response { return Err(Error::InvalidRequest); }
            state.awaiting_response = false;
            state.ambiguous = true;
            if matches!(failure, Failure::Network) && state.attempts < 2 && remaining_millis > 200 {
                action = "retry"; delay = 200;
            } else { action = "reconcile"; state.done = true; }
        }
    }
    Ok(json!({"state":state,"action":action,"delayMillis":delay,"timeoutMillis":timeout}))
}

fn invalid_pass(body: &str) -> bool {
    body.len() <= 256 && serde_json::from_str::<Value>(body).ok()
        .and_then(|v| v.as_object().cloned())
        .is_some_and(|v| v.len() == 1 && v.get("error") == Some(&json!("invalid_style_pass")))
}

pub(crate) fn retry_after_delay(value: Option<&str>, now_millis: i64) -> Option<u64> {
    let value = value.filter(|v| v.len() <= 128)?.trim();
    if !value.is_empty() && value.bytes().all(|v| v.is_ascii_digit()) {
        // An overflowing server minimum forbids retry within a finite budget.
        return Some(value.parse::<u64>().unwrap_or(u64::MAX).saturating_mul(1_000));
    }
    let target = chrono::DateTime::parse_from_rfc2822(value).ok()?.timestamp_millis();
    Some(target.saturating_sub(now_millis).max(0) as u64)
}
