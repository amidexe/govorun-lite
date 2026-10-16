//! Renderer capabilities come from the active host owners, not the OS agent.
pub use super::runtime_session as session;
#[path="runtime/inspection.rs"]
pub mod inspection;
use super::{routes, Route};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn project(mut snapshot: Value, documents_available: bool) -> Result<Value, &'static str> {
    let capabilities = snapshot["capabilities"]
        .as_array()
        .ok_or("native_invalid_capabilities")?;
    let mut public = BTreeSet::new();
    for capability in capabilities {
        let method = capability.as_str().ok_or("native_invalid_capabilities")?;
        if routes().get(method) == Some(&Route::Agent)
            && crate::features::capabilities::renderer_command_allowed(method)
        {
            public.insert(method.to_owned());
        }
    }
    for (method, route) in routes() {
        if *route != Route::Agent
            && crate::features::capabilities::renderer_command_allowed(method)
            && (if *route == Route::Clipboard {
                snapshot["platform"] == "macos"
                    && (capabilities.iter().any(|v| v == "clipboard.snapshot")
                        || ["storage.read", "storage.write"]
                            .iter()
                            .all(|name| capabilities.iter().any(|v| v == name)))
            } else if *route == Route::Runtime {
                capabilities.iter().any(|value| value == method)
            } else if *route == Route::PermissionSetup {
                capabilities.iter().any(|v| v == "dictation.snapshot")
                    || ["dictation.localFacts", "dictation.localEffect"].iter().all(|name| capabilities.iter().any(|v| v == name))
            } else if *route == Route::RecordingSupport {
                capabilities.iter().any(|v| v == "recordingDiagnostics.snapshot")
                    || ["recordingDiagnostics.localFacts", "recordingDiagnostics.localEffect"]
                        .iter().all(|name| capabilities.iter().any(|v| v == name))
            } else if *route == Route::Benchmark {
                capabilities.iter().any(|v| v == "benchmark.snapshot")
                    || [
                        "benchmark.localFacts",
                        "benchmark.localEffect",
                        "diagnostics.localEffect",
                    ]
                    .iter()
                    .all(|name| capabilities.iter().any(|v| v == name))
            } else if *route == Route::Diagnostics {
                capabilities.iter().any(|v| v == "diagnostics.snapshot")
                    || ["diagnostics.localFacts", "diagnostics.localEffect"]
                        .iter()
                        .all(|name| capabilities.iter().any(|v| v == name))
            } else if *route == Route::Hotkeys {
                capabilities.iter().any(|v| v == "hotkeys.snapshot")
                    || [
                        "hotkeys.localFacts",
                        "hotkeys.localEffect",
                        "storage.read",
                        "storage.write",
                    ]
                    .iter()
                    .all(|name| capabilities.iter().any(|v| v == name))
            } else {
                documents_available
                    || *route == Route::About
                        && capabilities.iter().any(|value| value == "about.localFacts")
                    || *route == Route::SelfHost
                        && capabilities.iter().any(|value| value == "secret.read")
            })
        {
            public.insert(method.clone());
        }
    }
    snapshot["capabilities"] = json!(public);
    Ok(snapshot)
}
