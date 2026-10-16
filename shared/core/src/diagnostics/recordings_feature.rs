//! Core owns confirmation/view; the Debug-only port owns bounded local audio IO.
use crate::agent::AgentClient;
use govorun_core::recording_support::{Command, Facts};
use serde_json::{json, Value};
use std::time::Duration;

async fn call(agent: &AgentClient, method: &str, params: Value) -> Result<Value, &'static str> {
    agent.call_named(method, params, Duration::from_secs(20)).await
}

async fn read(agent: &AgentClient) -> Result<Facts, &'static str> {
    serde_json::from_value(call(agent, "recordingDiagnostics.localFacts", json!({})).await?)
        .map_err(|_| "recording_diagnostics_invalid_facts")
}

pub async fn invoke(
    agent: &AgentClient,
    method: &str,
    params: Value,
) -> Result<Value, &'static str> {
    if method == "recordingDiagnostics.snapshot" {
        if !params.as_object().is_some_and(|params| params.is_empty()) {
            return Err("invalid_command");
        }
    } else if method == "recordingDiagnostics.action" {
        let command = serde_json::from_value::<Command>(params).map_err(|_| "invalid_command")?;
        let effect = read(agent).await?.effect(command)?;
        let params = serde_json::to_value(effect).map_err(|_| "invalid_command")?;
        if call(agent, "recordingDiagnostics.localEffect", params).await?
            != json!({"accepted":true})
        {
            return Err("recording_diagnostics_effect_failed");
        }
    } else {
        return Err("unsupported_command");
    }
    serde_json::to_value(read(agent).await?.snapshot())
        .map_err(|_| "recording_diagnostics_invalid_facts")
}
