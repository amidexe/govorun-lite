//! Commands and report view live here; the agent only runs recorded ASR checks.
pub use crate::features::benchmark_runner as runner;
use crate::agent::{AgentClient, Cancellation, Capability, Request};
use govorun_core::benchmark_report::{Facts, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
enum Command {
    Start {},
    Cancel {},
    Copy {},
}
async fn call(agent: &AgentClient, method: &str, params: Value) -> Result<Value, &'static str> {
    agent
        .call(
            Request {
                capability: Capability::named(method)?,
                params,
            },
            Duration::from_secs(20),
            &Cancellation::default(),
        )
        .await
}
fn project(facts: Value) -> Result<Snapshot, &'static str> {
    serde_json::from_value::<Facts>(facts).map_err(|_| "benchmark_invalid_facts")?.snapshot()
}
pub async fn invoke(
    agent: &AgentClient,
    method: &str,
    params: Value,
) -> Result<Value, &'static str> {
    let raw = if method == "benchmark.snapshot" {
        if !params.as_object().is_some_and(|params| params.is_empty()) {
            return Err("invalid_command");
        }
        call(agent, "benchmark.localFacts", json!({})).await?
    } else if method == "benchmark.action" {
        let command = serde_json::from_value::<Command>(params).map_err(|_| "invalid_command")?;
        match command {
            Command::Copy {} => {
                let snapshot = project(call(agent, "benchmark.localFacts", json!({})).await?)?;
                if snapshot.state == "running" {
                    return Err("benchmark_running");
                }
                super::diagnostics::copy(agent, snapshot.report.clone()).await?;
                return serde_json::to_value(snapshot).map_err(|_| "benchmark_invalid_facts");
            }
            Command::Start {} => {
                call(agent, "benchmark.localEffect", json!({"action":"start"})).await?
            }
            Command::Cancel {} => {
                call(agent, "benchmark.localEffect", json!({"action":"cancel"})).await?
            }
        }
    } else {
        return Err("unsupported_command");
    };
    serde_json::to_value(project(raw)?).map_err(|_| "benchmark_invalid_facts")
}
