//! Local OS observations; no application usage, titles or catalog leave the device.
use govorun_os_runtime::{
    applications::{self, Choice},
    network::PathMonitor,
};
use serde_json::{json, Value};
use std::sync::Mutex;

pub struct Observations {
    network: Option<PathMonitor>,
    applications: Mutex<Vec<Choice>>,
}
impl Observations {
    pub fn new() -> Self {
        Self {
            network: PathMonitor::new().ok(),
            applications: Mutex::new(vec![]),
        }
    }
    pub fn bind(&self, handler: crate::agent::Connectivity) -> Result<(), &'static str> {
        if let Some(network) = &self.network {
            network.bind(std::sync::Arc::new(move |event| {
                handler(event.online, event.revision, event.recovery_revision)
            }))?;
        }
        Ok(())
    }
    pub fn facts(&self, params: &Value) -> Result<Value, &'static str> {
        if !params.as_object().is_some_and(|fields| fields.is_empty()) {
            return Err("invalid_parameters");
        }
        let applications = self.applications.lock().map_err(|_| "styles_unavailable")?;
        // Readiness, selection and pause are replaced by their authoritative
        // host owners. Unknown OS network availability is not confirmed offline.
        Ok(json!({"permissionsReady":false,"paused":false,
            "networkAvailable":self.network.as_ref().and_then(PathMonitor::available),
            "selfHosted":false,"selfHostedReady":false,"selfHostedEnabled":false,"selfHostedTitle":"",
            "applications":applications.iter().map(|v|json!({"id":v.id,"name":v.name})).collect::<Vec<_>>()}))
    }
    pub fn load(&self, saved: &[Choice]) -> Result<Value, &'static str> {
        let applications = applications::discover(saved);
        *self.applications.lock().map_err(|_| "styles_unavailable")? = applications;
        Ok(Value::Null)
    }
}
#[cfg(target_os = "macos")]
pub fn saved(domain: &impl super::defaults_documents::Domain) -> Vec<Choice> {
    use super::defaults_documents::Cell;
    let Ok(snapshot) = domain.read() else {
        return vec![];
    };
    let Some(Cell::Data(bytes)) = snapshot.get("govorunApplicationStyleRulesV1") else {
        return vec![];
    };
    let Ok(Value::Array(rules)) = serde_json::from_slice::<Value>(bytes) else {
        return vec![];
    };
    rules
        .iter()
        .filter_map(|rule| {
            Some(Choice {
                id: rule["bundleID"].as_str()?.into(),
                name: rule["applicationName"].as_str()?.into(),
            })
        })
        .collect()
}
