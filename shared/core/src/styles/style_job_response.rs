//! Official job wire interpretation. Only a validated delivered result can
//! record successful usage; ambiguous or malformed responses retain the lease
//! for reconciliation. Hosts execute decisions within the original session.
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

const MAX_BODY_BYTES: usize = 196_608;
const MAX_OUTPUT_SCALARS: usize = 24_000;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Phase {
    Start,
    Status,
}

// Parse the original wire, never a native DTO or Value: derived Deserialize
// rejects duplicate known fields before any usage confirmation can be authorized.
// Unknown fields remain forward compatible. Optional diagnostics and delays
// may be malformed without invalidating an otherwise authoritative result.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct Wire {
    state: String,
    text: Option<String>,
    job_token: Option<String>,
    failure: Option<String>,
    credit_recovery: Option<String>,
    credit_recovery_after_ms: Option<Box<RawValue>>,
    #[serde(rename = "pollAfterMs")]
    _poll_after_ms: Option<Box<RawValue>>,
    server_elapsed_ms: Option<Box<RawValue>>,
    server_admission_ms: Option<Box<RawValue>>,
    server_queue_ms: Option<Box<RawValue>>,
    server_forward_ms: Option<Box<RawValue>>,
    server_finalize_ms: Option<Box<RawValue>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Decision {
    kind: &'static str,
    settlement: &'static str,
    text: Option<String>,
    job_token: Option<String>,
    failure: Option<&'static str>,
    retry_after_millis: Option<u64>,
    server_elapsed_ms: Option<u64>,
    server_admission_ms: Option<u64>,
    server_queue_ms: Option<u64>,
    server_forward_ms: Option<u64>,
    server_finalize_ms: Option<u64>,
}

impl Decision {
    fn invalid() -> Self {
        Self {
            kind: "invalid",
            settlement: "reconcile",
            text: None,
            job_token: None,
            failure: Some("invalid_response"),
            retry_after_millis: None,
            server_elapsed_ms: None,
            server_admission_ms: None,
            server_queue_ms: None,
            server_forward_ms: None,
            server_finalize_ms: None,
        }
    }
}

pub(crate) fn decide(phase: Phase, body: &str) -> Decision {
    if body.len() > MAX_BODY_BYTES {
        return Decision::invalid();
    }
    let Ok(wire) = serde_json::from_str::<Wire>(body) else {
        return Decision::invalid();
    };
    let mut decision = Decision::invalid();
    match wire.state.as_str() {
        "processing" => {
            if matches!(phase, Phase::Start) && wire.job_token.as_deref().is_none_or(str::is_empty)
            {
                return decision;
            }
            decision.kind = "processing";
            decision.settlement = "none";
            // A status response cannot replace the capability of the accepted job.
            decision.job_token = if matches!(phase, Phase::Start) {
                wire.job_token
            } else {
                None
            };
            decision.failure = None;
        }
        "completed" => {
            let Some(text) = wire.text.as_deref().map(str::trim) else {
                return decision;
            };
            // Same scalar limit as the authoritative Styles response validator.
            if text.is_empty()
                || text.chars().take(MAX_OUTPUT_SCALARS + 1).count() > MAX_OUTPUT_SCALARS
            {
                return decision;
            }
            decision.kind = "completed";
            decision.settlement = "consume";
            decision.text = Some(text.to_owned());
            decision.failure = None;
        }
        "failed" => {
            decision.kind = "failed";
            decision.failure = Some(match wire.failure.as_deref() {
                Some("busy") => "busy",
                Some("timeout") => "timeout",
                Some("unavailable") => "unavailable",
                _ => "invalid_response",
            });
            decision.settlement = match wire.credit_recovery.as_deref() {
                Some("ready") => "restore",
                Some("consumed") => "discard",
                Some("pending") => {
                    // Terminal pending starts from zero; the reconciliation
                    // protocol has its own independent minimum retry delay.
                    decision.retry_after_millis = Some(
                        optional_integer(wire.credit_recovery_after_ms)
                            .unwrap_or(0)
                            .min(50_000),
                    );
                    "defer"
                }
                _ => "reconcile",
            };
        }
        _ => return decision,
    }
    let metric = |value| optional_integer(value).filter(|value| *value <= 120_000);
    decision.server_elapsed_ms = metric(wire.server_elapsed_ms);
    decision.server_admission_ms = metric(wire.server_admission_ms);
    decision.server_queue_ms = metric(wire.server_queue_ms);
    decision.server_forward_ms = metric(wire.server_forward_ms);
    decision.server_finalize_ms = metric(wire.server_finalize_ms);
    decision
}

fn optional_integer(value: Option<Box<RawValue>>) -> Option<u64> {
    // Do not parse diagnostics into a general numeric Value first: a valid
    // JSON number outside f64/u64 range must not invalidate delivered text.
    serde_json::from_str(value?.get()).ok()
}
