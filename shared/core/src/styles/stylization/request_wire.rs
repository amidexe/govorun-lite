//! JNI wire view of the existing closed gateway requests; no credentials.
use super::{Kind, Request};
use crate::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Command {
    Availability {},
    Reliability { body: super::reliability::Batch },
    Start { body: Value, request_id: String },
    Status { token: String },
    Reconcile { passes: Vec<String> },
    Retire { passes: Vec<String> },
    Outcome { pass: String, outcome: String, latency_bucket: String },
}
pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let request = match command {
        Command::Availability {} => Request::availability()?,
        Command::Reliability { body } => Request::reliability(body)?,
        Command::Start { body, request_id } => Request::start(body, request_id, Duration::from_secs(1))?,
        Command::Status { token } => Request::status(token, Duration::from_secs(1))?,
        Command::Reconcile { passes } => Request::reconcile(passes)?,
        Command::Retire { passes } => Request::retire(passes)?,
        Command::Outcome { pass, outcome, latency_bucket } => Request::outcome(pass, &outcome, &latency_bucket)?,
    };
    let mut headers = json!({});
    if let Some(id) = request.request_id() { headers["X-Govorun-Request-ID"] = json!(id); }
    if let Some(token) = request.job_token() { headers["X-Govorun-Job-Token"] = json!(token); }
    if let Some(preference) = request.prefer_header() { headers["Prefer"] = json!(preference); }
    if let Some(pass) = request.outcome_pass() { headers["X-Govorun-Style-Pass"] = json!(pass); }
    Ok(json!({"path":request.path(),"method":if matches!(request.kind(),Kind::Status|Kind::Availability) {"GET"} else {"POST"},
        "body":request.body(),"headers":headers}))
}
