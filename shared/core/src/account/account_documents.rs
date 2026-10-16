//! Runtime ownership for documents. No renderer request can reach native CRUD.
use super::AccountHost;
use crate::{
    agent::AgentClient,
    features::documents::{native::NativeDocuments, DocumentsHost},
};
use govorun_core::AccountSession;
use crate::features::ports::Reply;
use serde_json::Value;
use std::sync::{atomic::Ordering, Arc};

impl AccountHost {
    pub(super) fn initialize_documents(
        &self,
        core: AccountSession,
        agent: AgentClient,
    ) -> Result<(), &'static str> {
        let ports = NativeDocuments::new(core, agent, self.open.clone());
        if let Some(owner) = self.self_host() {
            ports.attach_self_host(owner);
        }
        let documents = ports.build(self.events.clone(), ports.diagnostics())?;
        self.document_ports
            .set(ports)
            .map_err(|_| "documents_already_initialized")?;
        self.account_feature
            .get()
            .ok_or("account_storage_unavailable")?
            .attach_documents(documents.clone())?;
        self.billing
            .get()
            .ok_or("account_storage_unavailable")?
            .attach_documents(documents.clone())?;
        self.documents.send_replace(Some(documents));
        Ok(())
    }
    pub(super) fn documents_owner(&self) -> Result<Arc<DocumentsHost>, &'static str> {
        self.documents
            .borrow()
            .clone()
            .ok_or("documents_unavailable")
    }
    pub async fn document(&self, method: &str, params: Value) -> Reply {
        if self.stopping.load(Ordering::SeqCst) {
            return Err("native_closing");
        }
        self.documents_owner()?.invoke(method, params).await
    }
    pub async fn publish_documents(&self, identity_changed: bool) {
        let Ok(documents) = self.documents_owner() else {
            return;
        };
        if self.stopping.load(Ordering::SeqCst) {
            return;
        }
        if identity_changed {
            if self.core.get().is_some_and(|core| {
                let snapshot = core.snapshot();
                snapshot.storage_available && !snapshot.signed_in
            }) {
                let _ = documents.disable_official_selection().await;
            }
            let remote = self
                .core
                .get()
                .and_then(|core| core.profile())
                .and_then(|profile| profile.wire()["hasSyncedData"].as_bool());
            // A denied/corrupt local document does not close offline recognition.
            // Its feature reports failure/read-only rather than writing defaults.
            if let Err(code) = documents.identity_changed(remote).await {
                if let Some(ports) = self
                    .document_ports
                    .get()
                    .filter(|_| !matches!(code, "documents_stopped" | "account_session_changed"))
                {
                    (ports.diagnostics())("account_sync_driver_failed", code.into());
                }
            }
        }
        let _ = documents.publish_styles().await;
    }
    // Capture events now come from the in-process Rust owner, never worker RPC.
    pub async fn record_statistics(&self, event: Value) -> Reply {
        let mut ready = self.documents.subscribe();
        loop {
            if self.stopping.load(Ordering::Acquire) {
                return Err("native_closing");
            }
            let documents = ready.borrow().clone();
            if let Some(documents) = documents {
                documents.record(event).await?;
                return Ok(Value::Null);
            }
            ready.changed().await.map_err(|_| "documents_stopped")?;
        }
    }
    pub async fn dismiss_starter_guidance(&self, generation: u64) -> Reply {
        if self.stopping.load(Ordering::Acquire) {
            return Err("native_closing");
        }
        let core = self.core.get().ok_or("account_storage_unavailable")?;
        if core.snapshot().session_generation != generation {
            return Err("account_session_changed");
        }
        self.documents_owner()?
            .dismiss_starter_guidance(|| core.current(generation).is_ok())
            .await?;
        Ok(Value::Null)
    }
}
