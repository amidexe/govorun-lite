//! One document owner for UI, synchronization and agent configuration.
use crate::features::{dto::dto, snapshot_gate::SnapshotGate};
use govorun_core::{
    dictionary_repository::{DictionaryDocumentError, DictionaryRepository},
    DocumentStore,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

dto!(DictionaryEntry {
    id: String,
    from: String,
    to: String
});
dto!(DictionarySnapshot { enabled: bool, entries: Vec<DictionaryEntry>, revision: String });
dto!(DictionaryToggle {
    enabled: bool,
    revision: String
});
pub type Configure = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send>> + Send + Sync,
>;
pub type Events = Arc<dyn Fn(&str, Value) + Send + Sync>;

pub struct DictionaryFeature {
    repository: DictionaryRepository,
    configure: Configure,
    events: Events,
    gate: SnapshotGate,
    serial: Mutex<()>,
    pending_local: AtomicBool,
}
impl DictionaryFeature {
    pub async fn apply_text(&self, text: &str) -> Result<String, &'static str> {
        // Reuse normal admission/configuration recovery so a lost durable ACK
        // is repaired before a recording observes the changed dictionary.
        let configuration = self
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        let storage = &configuration["storage"];
        let raw = storage["text"]
            .as_str()
            .ok_or("dictionary_invalid_response")?;
        let enabled = storage["enabled"]
            .as_bool()
            .ok_or("dictionary_invalid_response")?;
        govorun_core::dictionary_repository::apply(raw, text, enabled)
            .map_err(|_| "dictionary_invalid_response")
    }
    pub fn new(store: Arc<dyn DocumentStore>, configure: Configure, events: Events) -> Arc<Self> {
        Arc::new(Self {
            repository: DictionaryRepository::new(store, "dictionary.document".into()),
            configure,
            events,
            gate: SnapshotGate::default(),
            serial: Mutex::new(()),
            pending_local: AtomicBool::new(false),
        })
    }

    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let (command, revision) = match method {
            "dictionary.snapshot" if params.as_object().is_some_and(|value| value.is_empty()) => {
                (json!({"kind":"snapshot"}), None)
            }
            "dictionary.replace" => {
                let input: DictionarySnapshot =
                    serde_json::from_value(params).map_err(|_| "dictionary_invalid_request")?;
                (
                    json!({"kind":"replace","entries":input.entries,"enabled":input.enabled}),
                    Some(input.revision),
                )
            }
            "dictionary.setEnabled" => {
                let input: DictionaryToggle =
                    serde_json::from_value(params).map_err(|_| "dictionary_invalid_request")?;
                (
                    json!({"kind":"setEnabled","enabled":input.enabled}),
                    Some(input.revision),
                )
            }
            _ => return Err("dictionary_invalid_request"),
        };
        let result = self.execute(command, revision, true).await?;
        projection(&result)
    }

    /// Private callers get portable text; only the public view is emitted.
    /// Configuration acknowledgement stays in the serialized write boundary, so
    /// an older operation cannot configure the agent after a newer mutation.
    pub async fn execute(
        &self,
        command: Value,
        revision: Option<String>,
        local_change: bool,
    ) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let result = self
            .repository
            .execute(command, revision)
            .await
            .map_err(code)?;
        if local_change && result.changed {
            self.pending_local.store(true, Ordering::SeqCst);
        }
        let skipped = result.storage["text"]
            .as_str()
            .is_some_and(govorun_core::dictionary_document::has_unusable);
        let configuration = json!({"storage":result.storage,"revision":result.revision,
            "changed":result.changed,"skipped":skipped,"localChange":self.pending_local.load(Ordering::SeqCst)});
        (self.configure)(configuration.clone()).await?;
        self.pending_local.store(false, Ordering::SeqCst);
        let snapshot = projection(&configuration)?;
        if self.gate.changed(&snapshot) {
            (self.events)("dictionary.changed", snapshot);
        }
        Ok(configuration)
    }
}
fn projection(value: &Value) -> Result<Value, &'static str> {
    let snapshot: DictionarySnapshot =
        serde_json::from_value(json!({"enabled":value["storage"]["enabled"],
        "entries":value["storage"]["entries"],"revision":value["revision"]}))
        .map_err(|_| "dictionary_invalid_response")?;
    serde_json::to_value(snapshot).map_err(|_| "dictionary_invalid_response")
}
fn code(error: DictionaryDocumentError) -> &'static str {
    match error {
        DictionaryDocumentError::InvalidAction => "dictionary_invalid_request",
        DictionaryDocumentError::InvalidStorage => "dictionary_storage_invalid",
        DictionaryDocumentError::StorageUnavailable => "dictionary_storage_unavailable",
        DictionaryDocumentError::Conflict => "dictionary_conflict",
    }
}
