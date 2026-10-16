//! Feature events follow observable snapshot changes, not watch notifications.
use serde_json::Value;
use std::sync::Mutex;

#[derive(Default)]
pub struct SnapshotGate(Mutex<Option<Value>>);
impl SnapshotGate {
    pub fn changed(&self, next: &Value) -> bool {
        let mut previous = self.0.lock().unwrap_or_else(|poison| poison.into_inner());
        if previous.as_ref() == Some(next) {
            return false;
        }
        *previous = Some(next.clone());
        true
    }
}
