//! Display of measured ASR facts. Execution and recorded fixtures stay in OS adapters.
pub mod run;
mod report;
mod system;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub use system::{Adapter, SystemFacts};
const STAGES: &[&str] = &["fixtures", "model", "cold", "warm", "long"];
const FAILURES: &[&str] = &[
    "samples_unavailable",
    "model_missing",
    "runtime_incompatible",
    "initialization_failed",
    "recognition_failed",
];
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "BenchmarkTiming"))]
pub struct Timing {
    pub milliseconds: f64,
    pub backend: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "BenchmarkResult"))]
pub struct Measurement {
    pub suite: String,
    pub platform: String,
    pub model: String,
    pub requested_backend: String,
    pub short_audio_ms: f64,
    pub long_audio_ms: f64,
    pub load_ms: f64,
    pub cold: Timing,
    pub warm: Timing,
    pub longer: Timing,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "BenchmarkDevice"))]
pub struct Device {
    pub id: String,
    pub name: String,
    pub backend: String,
    pub state: String,
    pub stage: String,
    pub result: Option<Measurement>,
    pub failure_code: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Facts {
    pub state: String,
    pub stage: String,
    pub devices: Vec<Device>,
    pub active_device_id: Option<String>,
    pub failure_code: Option<String>,
    pub system: SystemFacts,
}
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", schemars(rename = "BenchmarkSnapshot"))]
pub struct Snapshot {
    pub state: String,
    pub stage: String,
    pub devices: Vec<Device>,
    pub active_device_id: Option<String>,
    pub failure_code: Option<String>,
    pub report: String,
}
impl Facts {
    pub fn checked(&self) -> bool {
        if !["idle", "running", "done", "failed", "cancelled"].contains(&self.state.as_str())
            || !(self.stage == "devices" || STAGES.contains(&self.stage.as_str()))
            || !failure(&self.failure_code, &self.state)
            || self.devices.len() > 64
            || !self.system.checked()
        {
            return false;
        }
        let mut ids = BTreeSet::new();
        let mut running = 0;
        for device in &self.devices {
            if !ids.insert(&device.id)
                || !(device.id == "cpu"
                    || device.id.strip_prefix("gpu-").is_some_and(|id| {
                        !id.starts_with('0') && id.parse::<u32>().ok().is_some_and(|n| n > 0)
                    }))
                || !text(&device.name, 160)
                || device.name.is_empty()
                || !["cpu", "metal", "vulkan"].contains(&device.backend.as_str())
                || (device.id == "cpu") != (device.backend == "cpu")
                || !["pending", "running", "done", "failed", "cancelled"]
                    .contains(&device.state.as_str())
                || !STAGES.contains(&device.stage.as_str())
                || !failure(&device.failure_code, &device.state)
            {
                return false;
            }
            if device.state == "done" {
                if device.stage != "long"
                    || !device
                        .result
                        .as_ref()
                        .is_some_and(|result| result.checked(&device.backend))
                {
                    return false;
                }
            } else if device.result.is_some() {
                return false;
            }
            if device.state == "running" {
                running += 1;
                if self.state != "running" || self.active_device_id.as_ref() != Some(&device.id) {
                    return false;
                }
            }
            if device.state == "cancelled" && self.state != "cancelled" {
                return false;
            }
        }
        running <= 1
            && (self.state == "running" || self.active_device_id.is_none())
            && self
                .active_device_id
                .as_ref()
                .is_none_or(|id| ids.contains(id))
            && (self.state != "idle" || self.devices.is_empty())
            && (self.state != "done"
                || !self.devices.is_empty()
                    && self.devices.iter().all(|device| device.state == "done"))
            && (self.state != "failed"
                || self.devices.iter().all(|device| {
                    !["pending", "running", "cancelled"].contains(&device.state.as_str())
                }))
    }
    pub fn snapshot(self, journal: &str) -> Result<Snapshot, &'static str> {
        if !self.checked() {
            return Err("benchmark_invalid_facts");
        }
        let report = report::suite(&self, journal);
        if report.len() > 131_072 {
            return Err("benchmark_report_too_large");
        }
        Ok(Snapshot {
            state: self.state,
            stage: self.stage,
            devices: self.devices,
            active_device_id: self.active_device_id,
            failure_code: self.failure_code,
            report,
        })
    }
}
impl Measurement {
    pub fn checked(&self, backend: &str) -> bool {
        self.suite == "gigaam-samples-v1"
            && ["macos", "windows"].contains(&self.platform.as_str())
            && self.requested_backend == if backend == "cpu" { "cpu" } else { "automatic" }
            && (backend != "metal" || self.platform == "macos")
            && (backend != "vulkan" || self.platform == "windows")
            && text(&self.model, 160)
            && self.short_audio_ms == 11290.0
            && self.long_audio_ms == 71250.0
            && self.load_ms.is_finite()
            && self.load_ms >= 0.0
            && [&self.cold, &self.warm, &self.longer].iter().all(|timing| {
                timing.milliseconds.is_finite()
                    && timing.milliseconds > 0.0
                    && timing.backend == backend
            })
    }
    pub fn export(&self) -> String {
        report::measurement(self)
    }
}
fn failure(value: &Option<String>, state: &str) -> bool {
    match value {
        Some(value) => state == "failed" && FAILURES.contains(&value.as_str()),
        None => state != "failed",
    }
}
fn text(value: &str, maximum: usize) -> bool {
    value.encode_utf16().count() <= maximum && !value.chars().any(char::is_control)
}
