//! Foreground facts come from the last confirmed session profile (H5).
//! Device observations cannot grant access or substitute another generation.
use crate::{AccountSession, Error, Result};
use serde_json::{json, Value};
impl AccountSession {
    pub fn dictation_access_facts(
        &self,
        epoch: u64,
        selected: bool,
        configured: bool,
        network_available: Option<bool>,
    ) -> Result<Value> {
        let snapshot = self.snapshot();
        let profile = self.profile();
        let service_available = profile
            .as_ref()
            .is_some_and(|profile| profile.wire()["onlineService"]["state"] == "available");
        if self.snapshot().revision != snapshot.revision {
            return Err(Error::SessionChanged);
        }
        Ok(json!({"selected":selected,
            "sessionCurrent":epoch == snapshot.session_generation,
            "entitlementActive":snapshot.access.entitlement_active,
            "processingReady":snapshot.access.processing_ready,
            "termsRequired":snapshot.access.needs_terms_acceptance,
            "serviceAvailable":service_available,
            "storageAvailable":snapshot.storage_available,
            "configured":configured,"networkAvailable":network_available}))
    }
}
