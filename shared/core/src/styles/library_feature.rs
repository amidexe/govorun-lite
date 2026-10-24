//! Shared catalog owner used by the style screen, activation and content sync.
//! The containing style feature publishes its complete public snapshot.
use super::dictionary::Configure;
use govorun_core::{
    style_repository::{StyleDocumentError, StyleRepository},
    DocumentStore,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;

pub struct StyleLibraryFeature {
    repository: StyleRepository,
    configure: Configure,
    serial: Mutex<()>,
    pending_local: AtomicBool,
    pending_content: AtomicBool,
    pending_preferences: AtomicBool,
}
impl StyleLibraryFeature {
    pub async fn resolve_dictation(
        &self,
        application: Option<&str>,
        identity: crate::style_repository::ApplicationIdentity,
    ) -> Result<govorun_core::style_repository::DictationStyle, &'static str> {
        let snapshot = self
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        let configuration = &snapshot["configuration"];
        govorun_core::style_repository::resolve_dictation(
            configuration["context"].clone(),
            configuration["applicationRules"].clone(),
            application,
            identity,
        )
        .map_err(|_| "styles_invalid_response")
    }
    pub fn new(store: Arc<dyn DocumentStore>, configure: Configure) -> Arc<Self> {
        Arc::new(Self {
            repository: StyleRepository::new(store),
            configure,
            serial: Mutex::new(()),
            pending_local: AtomicBool::new(false),
            pending_content: AtomicBool::new(false),
            pending_preferences: AtomicBool::new(false),
        })
    }

    /// Configuration ACK remains inside the write boundary. A lost ACK does
    /// not roll back storage or repeat the edit; the next read restores cache
    /// and retries the unacknowledged content/preference notifications.
    pub async fn execute(
        &self,
        action: Value,
        revision: Option<String>,
        local_change: bool,
    ) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let result = self
            .repository
            .execute(action, revision)
            .await
            .map_err(code)?;
        if local_change && result.changed {
            self.pending_local.store(true, Ordering::SeqCst);
            if result.mark_dirty {
                self.pending_content.store(true, Ordering::SeqCst);
            }
            if result.preferences_dirty {
                self.pending_preferences.store(true, Ordering::SeqCst);
            }
        }
        let configuration = json!({"context":result.context,"applicationRules":result.application_rules,"metadata":result.metadata,
            "revision":result.revision,"changed":result.changed,"skipped":result.skipped,
            "localChange":self.pending_local.load(Ordering::SeqCst),
            "contentDirty":self.pending_content.load(Ordering::SeqCst),
            "preferencesDirty":self.pending_preferences.load(Ordering::SeqCst)});
        (self.configure)(configuration.clone()).await?;
        self.pending_local.store(false, Ordering::SeqCst);
        self.pending_content.store(false, Ordering::SeqCst);
        self.pending_preferences.store(false, Ordering::SeqCst);
        Ok(
            json!({"configuration":configuration,"projection":result.projection,
            "storedOptionsPresent":result.stored_options_present,"accepted":result.accepted,"reason":result.reason}),
        )
    }
}
fn code(error: StyleDocumentError) -> &'static str {
    match error {
        StyleDocumentError::InvalidAction => "styles_invalid_request",
        StyleDocumentError::InvalidStorage => "styles_storage_invalid",
        StyleDocumentError::StorageUnavailable => "styles_storage_unavailable",
        StyleDocumentError::Conflict => "styles_conflict",
    }
}
