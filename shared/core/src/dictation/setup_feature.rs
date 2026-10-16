//! Setup commands route through the host; OS facts/effects stay in the adapter.
pub use govorun_core::features::permission_settings as settings;
use crate::agent::AgentClient;
use govorun_core::permission_setup::{Command, Facts, Snapshot};
use serde_json::{json, Value};
use std::time::Duration;

async fn call(agent: &AgentClient, method: &str, params: Value) -> Result<Value, &'static str> {
    agent.call_named(method, params, Duration::from_secs(20)).await
}
fn project(settings: Snapshot) -> Result<Value, &'static str> {
    if !settings.checked() {
        return Err("dictation_invalid_facts");
    }
    serde_json::to_value(settings).map_err(|_| "dictation_invalid_facts")
}
pub async fn invoke(
    agent: &AgentClient,
    method: &str,
    params: Value,
) -> Result<Value, &'static str> {
    let command = if method == "dictation.snapshot" {
        if !params.as_object().is_some_and(|params| params.is_empty()) {
            return Err("invalid_command");
        }
        None
    } else if method == "dictation.action" {
        Some(serde_json::from_value::<Command>(params).map_err(|_| "invalid_command")?)
    } else {
        return Err("unsupported_command");
    };
    let facts =
        serde_json::from_value::<Facts>(call(agent, "dictation.localFacts", json!({})).await?)
            .map_err(|_| "dictation_invalid_facts")?;
    if let Some(command) = command {
        let command = facts.effect(command)?;
        let params = serde_json::to_value(command).map_err(|_| "invalid_command")?;
        let value = call(agent, "dictation.localEffect", params).await?;
        project(serde_json::from_value(value).map_err(|_| "dictation_invalid_facts")?)
    } else {
        project(facts.settings)
    }
}
