use crate::Result;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    Proof { status: u16, body: String, date: Option<String>, modified: Option<String> },
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Failure {
    kind: FailureKind,
    status: Option<u16>,
    code: Option<String>,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum FailureKind { Ignored, Cancelled, Security, Offline, Transport, Deadline, Http, Service }

impl Failure {
    pub(crate) fn transient(&self) -> bool {
        match self.kind {
            FailureKind::Transport | FailureKind::Deadline => true,
            FailureKind::Http => self.status.is_some_and(transient_http),
            FailureKind::Service => matches!(self.code.as_deref(), Some("busy" | "timeout" | "unavailable")),
            _ => false,
        }
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(crate) struct RetryHint { status: u16, milliseconds: i64 }

pub(crate) fn retry_after(direct: i64, hints: &[RetryHint]) -> i64 {
    hints.iter().filter(|hint| transient_http(hint.status))
        .map(|hint| hint.milliseconds).fold(direct.max(0), i64::max)
}

fn transient_http(status: u16) -> bool { matches!(status, 408 | 429 | 500 | 502 | 503 | 504) }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Proof { status: String }

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::Proof { status, body, date, modified } => {
            let age = sample_age(status, &body, date.as_deref(), modified.as_deref());
            Ok(json!({"healthy":age.is_some(),"sampleAge":age}))
        }
    }
}

fn sample_age(status: u16, body: &str, date: Option<&str>, modified: Option<&str>) -> Option<i64> {
    if status != 200 || body.len() > 128 { return None; }
    let payload: Proof = serde_json::from_str(body).ok()?;
    if payload.status != "ok" { return None; }
    let served = chrono::DateTime::parse_from_rfc2822(date?).ok()?;
    let sampled = chrono::DateTime::parse_from_rfc2822(modified?).ok()?;
    let age = served.timestamp_millis().checked_sub(sampled.timestamp_millis())?;
    (0..=180_000).contains(&age).then_some(age)
}
