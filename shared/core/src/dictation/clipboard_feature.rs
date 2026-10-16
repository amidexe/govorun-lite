//! One serialized preference owner; the agent only stores raw Defaults values.
use super::{dictionary::Events, snapshot_gate::SnapshotGate};
use govorun_core::{clipboard_settings::ClipboardSettings, DocumentStore};
use serde_json::Value;
use std::sync::Arc;

pub struct Clipboard {
    settings: ClipboardSettings,
    serial: tokio::sync::Mutex<()>,
    gate: SnapshotGate,
    events: Events,
}
impl Clipboard {
    pub fn new(store: Arc<dyn DocumentStore>, events: Events) -> Arc<Self> {
        Arc::new(Self { settings: ClipboardSettings::new(store), serial: Default::default(),
            gate: Default::default(), events })
    }
    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let value = self.settings.invoke(method, params).await?;
        if self.gate.changed(&value) { (self.events)("clipboard.changed", value.clone()); }
        Ok(value)
    }
}
