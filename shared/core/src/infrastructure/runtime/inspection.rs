//! Stateless package probe. Uses the production lifetime without opening stores,
//! registering input hooks, accessing audio or starting another process.
use super::session::{facts, Session};
use serde_json::{json, Value};

pub async fn report(platform: &str, identity: &str, name: &str, version: &str, build: &str) -> Result<Value, &'static str> {
    let lifetime = Session::local(facts(platform, identity, name, version, build))?;
    let mut snapshot = lifetime.snapshot().await?;
    // Inspection advertises only its stateless lifecycle, never live OS ports.
    snapshot["capabilities"] = json!([
        "runtime.snapshot", "runtime.shutdown", "runtime.shutdownWhenIdle", "recording.cancel"
    ]);
    let maintenance = lifetime.invoke("runtime.prepareMaintenance", json!({})).await?;
    lifetime.shutdown_when_idle().await?;
    Ok(json!({"schemaVersion":1,"snapshot":snapshot,"maintenance":maintenance,
        "inspectionOnly":true,"cleanExit":lifetime.is_closed()}))
}
