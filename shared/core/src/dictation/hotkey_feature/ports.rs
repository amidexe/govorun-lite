use super::{Hotkeys, State};
use crate::agent::{Cancellation, Capability, Request};
use govorun_core::{hotkeys::{settings, workflow::Assignment}, DocumentStore, StoreError};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

pub(super) fn storage_error(error: StoreError) -> &'static str {
    match error {
        StoreError::Invalid => "hotkeys_storage_invalid",
        StoreError::Conflict => "hotkeys_storage_conflict",
        StoreError::Unavailable => "hotkeys_storage_unavailable",
    }
}
impl Hotkeys {
    pub(super) async fn call(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let physical = self.physical.read().map_err(|_| "hotkeys_unavailable")?.clone();
        if let Some(ports) = physical { return ports.invoke(method, params).await; }
        self.agent.call(Request { capability: Capability::named(method)?, params }, Duration::from_secs(20), &Cancellation::default()).await
    }
    pub(super) async fn effect(&self, params: Value) -> Result<(), &'static str> {
        let reply = self.call("hotkeys.localEffect", params).await?;
        if reply == json!({"accepted":true}) { Ok(()) } else { Err("hotkeys_effect_failed") }
    }
    pub(super) async fn configure(&self, state: &State) -> Result<(), &'static str> {
        self.effect(json!({"action":"configure","dictation":state.facts.dictation.raw,
            "plain":state.facts.effective_plain().map(|g|g.raw.clone()),"cancel":state.facts.cancel.raw})).await
    }
    pub(super) async fn save(&self, state: &mut State, change: Assignment) -> Result<(), &'static str> {
        let old_id = state.flow.capture().map(|v|v.id.clone());
        let result = async {
            if let Some(id) = &old_id { self.effect(json!({"action":"stop","captureID":id})).await?; }
            let next = if change.reset { settings::reset(&state.document.value, change.target, change.gesture.as_ref())? }
                else { settings::assign(&state.document.value, change.target, change.gesture.as_ref())? };
            let written = if next == state.document.value { state.document.clone() } else {
                self.store.write_document(settings::KEY.into(), Some(state.document.revision.clone()), next.clone()).await.map_err(storage_error)?
            };
            if written.value != next { return Err("hotkeys_storage_invalid"); }
            let facts = serde_json::from_value::<govorun_core::hotkeys::Facts>(self.call("hotkeys.localFacts", json!({"storage":written.value})).await?)
                .map_err(|_| "hotkeys_invalid_facts")?.normalized();
            state.document = written; state.facts = facts;
            if !state.facts.checked() { return Err("hotkeys_invalid_facts"); }
            self.configure(state).await
        }.await;
        state.flow.saved(result.is_ok());
        result
    }
    pub(super) fn schedule_expiry(self: &Arc<Self>, id: String, deadline: u64) {
        let owner = Arc::downgrade(self);
        let deadline = self.started + Duration::from_millis(deadline);
        tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            let Some(owner) = owner.upgrade() else { return; };
            let mut state = owner.serial.lock().await;
            let Some(state) = state.as_mut() else { return; };
            if state.flow.expire(&id, owner.now()) {
                let _ = owner.effect(json!({"action":"stop","captureID":id})).await;
                owner.publish(state);
            }
        });
    }
}
