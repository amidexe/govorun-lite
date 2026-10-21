//! Native journals provide facts and OS effects; the common core prepares every row/export.
use crate::agent::AgentClient;
use govorun_core::journal_report::Journal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub maximum_events: usize, pub events: Vec<crate::journal_report::Row>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<Vec<crate::journal_report::Row>>"))]
    pub extended_events: Option<Vec<crate::journal_report::Row>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<bool>"))]
    pub extended_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<bool>"))]
    pub native_available: Option<bool>,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
enum Command {
    Copy {},
    Clear { confirmed: bool },
}
async fn call(agent: &AgentClient, method: &str, params: Value) -> Result<Value, &'static str> {
    agent.call_named(method, params, Duration::from_secs(20)).await
}
async fn read(agent: &AgentClient) -> Result<Journal, &'static str> {
    let value = call(agent, "diagnostics.localFacts", json!({})).await?;
    let journal =
        serde_json::from_value::<Journal>(value).map_err(|_| "diagnostics_invalid_facts")?;
    let count = match &journal {
        Journal::Macos(entries) => entries.len(),
        Journal::Windows(entries) => entries.len(),
    };
    if count > govorun_core::journal_report::MAXIMUM {
        return Err("diagnostics_invalid_facts");
    }
    Ok(journal)
}
async fn effect(agent: &AgentClient, params: Value) -> Result<(), &'static str> {
    if call(agent, "diagnostics.localEffect", params).await? == json!({"accepted":true}) {
        Ok(())
    } else {
        Err("diagnostics_effect_failed")
    }
}
pub async fn copy(agent: &AgentClient, text: String) -> Result<Value, &'static str> {
    if text.len() > crate::journal_report::EXPORT_LIMIT {
        return Err("diagnostics_report_too_large");
    }
    effect(agent, json!({"action":"copy","text":text})).await?;
    Ok(Value::Null)
}
pub async fn invoke(
    agent: &AgentClient,
    method: &str,
    params: Value,
) -> Result<Value, &'static str> {
    if method == "diagnostics.snapshot" {
        if !params.as_object().is_some_and(|p| p.is_empty()) {
            return Err("invalid_command");
        }
    } else if method == "diagnostics.action" {
        match serde_json::from_value::<Command>(params).map_err(|_| "invalid_command")? {
            Command::Copy {} => return copy(agent, read(agent).await?.export()).await,
            Command::Clear { confirmed: true } => {
                effect(agent, json!({"action":"clear","confirmed":true})).await?
            }
            Command::Clear { confirmed: false } => return Err("invalid_command"),
        }
    } else {
        return Err("unsupported_command");
    }
    serde_json::to_value(DiagnosticsSnapshot { maximum_events: crate::journal_report::MAXIMUM, events: read(agent).await?.rows(), extended_events: None, extended_enabled: None, native_available: None }).map_err(|_| "diagnostics_invalid_facts")
}
