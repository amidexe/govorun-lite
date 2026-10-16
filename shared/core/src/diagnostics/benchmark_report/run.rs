//! Sequential benchmark transitions. Physical discovery, decoding, cancellation
//! and the exclusive recording lease belong to the host/OS adapter.
use super::{Device, Facts, Measurement, SystemFacts, FAILURES, STAGES};
pub struct Run {
    facts: Facts,
    generation: u64,
    owned: bool,
}
impl Run {
    pub fn new(system: SystemFacts) -> Result<Self, &'static str> {
        let facts = Facts {
            state: "idle".into(),
            stage: "devices".into(),
            devices: Vec::new(),
            active_device_id: None,
            failure_code: None,
            system,
        };
        if !facts.checked() {
            return Err("benchmark_invalid_facts");
        }
        Ok(Self {
            facts,
            generation: 0,
            owned: false,
        })
    }
    /// Every published snapshot is an independent value, including device rows.
    pub fn facts(&self) -> Facts {
        self.facts.clone()
    }
    pub fn running(&self) -> bool {
        self.owned
    }
    pub fn begin(&mut self, available: bool) -> Result<u64, &'static str> {
        if self.owned || !available {
            return Err("benchmark_running");
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("benchmark_generation_exhausted")?;
        self.owned = true;
        self.facts.state = "running".into();
        self.facts.stage = "devices".into();
        self.facts.devices.clear();
        self.facts.active_device_id = None;
        self.facts.failure_code = None;
        Ok(self.generation)
    }
    fn current(&self, generation: u64) -> bool {
        self.owned && self.generation == generation && self.facts.state == "running"
    }
    pub fn discovered(
        &mut self,
        generation: u64,
        devices: Vec<Device>,
    ) -> Result<bool, &'static str> {
        if !self.current(generation) {
            return Ok(false);
        }
        if !self.facts.devices.is_empty()
            || devices.is_empty()
            || devices.iter().any(|d| {
                d.state != "pending"
                    || d.stage != "fixtures"
                    || d.result.is_some()
                    || d.failure_code.is_some()
            })
        {
            return Err("benchmark_invalid_devices");
        }
        let mut candidate = self.facts.clone();
        candidate.devices = devices;
        if !candidate.checked() {
            return Err("benchmark_invalid_devices");
        }
        self.facts = candidate;
        Ok(true)
    }
    pub fn select(&mut self, generation: u64, index: usize) -> Result<bool, &'static str> {
        if !self.current(generation) {
            return Ok(false);
        }
        if self.facts.active_device_id.is_some()
            || self
                .facts
                .devices
                .iter()
                .take(index)
                .any(|d| !matches!(d.state.as_str(), "done" | "failed"))
        {
            return Err("benchmark_invalid_transition");
        }
        let device = self
            .facts
            .devices
            .get_mut(index)
            .ok_or("benchmark_invalid_transition")?;
        if device.state != "pending" {
            return Err("benchmark_invalid_transition");
        }
        device.state = "running".into();
        self.facts.stage = device.stage.clone();
        self.facts.active_device_id = Some(device.id.clone());
        Ok(true)
    }
    pub fn progress(
        &mut self,
        generation: u64,
        id: &str,
        stage: &str,
    ) -> Result<bool, &'static str> {
        if !self.current(generation) || self.facts.active_device_id.as_deref() != Some(id) {
            return Ok(false);
        }
        if !STAGES.contains(&stage) {
            return Err("benchmark_invalid_stage");
        }
        let device = self
            .facts
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("benchmark_invalid_transition")?;
        device.stage = stage.into();
        self.facts.stage = stage.into();
        Ok(true)
    }
    pub fn measured(
        &mut self,
        generation: u64,
        id: &str,
        result: Measurement,
    ) -> Result<bool, &'static str> {
        if !self.current(generation) || self.facts.active_device_id.as_deref() != Some(id) {
            return Ok(false);
        }
        let device = self
            .facts
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("benchmark_invalid_transition")?;
        if !result.checked(&device.backend) {
            return Err("benchmark_invalid_measurement");
        }
        device.result = Some(result);
        device.state = "done".into();
        device.stage = "long".into();
        self.facts.stage = "long".into();
        self.facts.active_device_id = None;
        Ok(true)
    }
    /// Failure accepts only a public code; private exception bodies cannot enter facts.
    pub fn failed(&mut self, generation: u64, code: &str) -> Result<bool, &'static str> {
        if !self.current(generation) {
            return Ok(false);
        }
        if !FAILURES.contains(&code) {
            return Err("benchmark_invalid_failure");
        }
        if let Some(id) = self.facts.active_device_id.take() {
            let device = self
                .facts
                .devices
                .iter_mut()
                .find(|d| d.id == id)
                .ok_or("benchmark_invalid_transition")?;
            device.state = "failed".into();
            device.failure_code = Some(code.into());
        } else {
            self.facts.state = "failed".into();
            self.facts.failure_code = Some(code.into());
        }
        Ok(true)
    }
    pub fn cancelled(&mut self, generation: u64) -> bool {
        if !self.current(generation) {
            return false;
        }
        if let Some(id) = self.facts.active_device_id.take() {
            if let Some(device) = self.facts.devices.iter_mut().find(|d| d.id == id) {
                device.state = "cancelled".into();
            }
        }
        self.facts.state = "cancelled".into();
        true
    }
    /// Exactly one host lease is released, including failure and cancellation.
    pub fn finish(&mut self, generation: u64) -> Result<bool, &'static str> {
        if !self.owned || self.generation != generation {
            return Ok(false);
        }
        if self.facts.active_device_id.is_some() {
            return Err("benchmark_invalid_transition");
        }
        if self.facts.state == "running" {
            if self.facts.devices.is_empty()
                || self
                    .facts
                    .devices
                    .iter()
                    .any(|d| !matches!(d.state.as_str(), "done" | "failed"))
            {
                return Err("benchmark_invalid_transition");
            }
            self.facts.failure_code = self
                .facts
                .devices
                .iter()
                .find_map(|d| d.failure_code.clone());
            self.facts.state = if self.facts.failure_code.is_some() {
                "failed"
            } else {
                "done"
            }
            .into();
        }
        if !self.facts.checked() {
            return Err("benchmark_invalid_facts");
        }
        self.owned = false;
        Ok(true)
    }
}

/// Normalize an adapter failure into the closed report vocabulary. Unknown
/// exception bodies are classified by the measured stage and never returned.
pub fn public_failure(stage: &str, code: &str) -> &'static str {
    match code {
        "samples_unavailable" => "samples_unavailable",
        "model_missing" => "model_missing",
        "runtime_incompatible" => "runtime_incompatible",
        "initialization_failed" => "initialization_failed",
        "recognition_failed" => "recognition_failed",
        _ if stage == "fixtures" => "samples_unavailable",
        _ if matches!(stage, "devices" | "model") => "initialization_failed",
        _ => "recognition_failed",
    }
}
