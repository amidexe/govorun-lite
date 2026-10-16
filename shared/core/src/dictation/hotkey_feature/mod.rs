//! One assignment writer and capture lifetime; OS agents never decide conflicts.
mod commands;
mod ports;
use super::{dictionary::Events, snapshot_gate::SnapshotGate};
use crate::agent::{storage::AgentStore, AgentClient};
use govorun_core::{hotkeys::{settings, workflow::Workflow, Facts}, Document, DocumentStore};
use serde_json::Value;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::time::Instant;

struct State { flow: Workflow, facts: Facts, document: Document }
pub struct Hotkeys {
    agent: AgentClient,
    physical: std::sync::RwLock<Option<Arc<dyn crate::agent::AgentTransport>>>,
    store: AgentStore,
    serial: tokio::sync::Mutex<Option<State>>,
    gate: SnapshotGate,
    events: Events,
    started: Instant,
    next_id: AtomicU64,
    instance: u64,
}
impl Hotkeys {
    pub fn new(agent: AgentClient, events: Events) -> Arc<Self> {
        static INSTANCES: AtomicU64 = AtomicU64::new(0);
        Arc::new(Self { store: AgentStore::new(agent.clone()), agent, physical: Default::default(), serial: Default::default(),
            gate: Default::default(), events, started: Instant::now(), next_id: AtomicU64::new(0), instance: INSTANCES.fetch_add(1, Ordering::Relaxed) })
    }
    pub fn attach_physical(&self, ports: Arc<dyn crate::agent::AgentTransport>) {
        if let Ok(mut physical) = self.physical.write() { *physical = Some(ports); }
    }
    fn now(&self) -> u64 { self.started.elapsed().as_millis().min(u64::MAX as u128) as u64 }
    async fn refresh(&self, state: &mut Option<State>) -> Result<(), &'static str> {
        let document = self.store.read_document(settings::KEY.into()).await.map_err(ports::storage_error)?
            .ok_or("hotkeys_storage_unavailable")?;
        settings::values(&document.value)?;
        let facts = serde_json::from_value::<Facts>(self.call("hotkeys.localFacts", serde_json::json!({"storage":document.value})).await?)
            .map_err(|_| "hotkeys_invalid_facts")?.normalized();
        if !facts.checked() { return Err("hotkeys_invalid_facts"); }
        if let Some(state) = state { state.facts = facts; state.document = document; }
        else { *state = Some(State { flow: Workflow::default(), facts, document }); }
        Ok(())
    }
    fn publish(&self, state: &State) -> Value {
        let value = state.flow.snapshot(&state.facts);
        if self.gate.changed(&value) { (self.events)("hotkeys.changed", value.clone()); }
        value
    }
    pub async fn stop(&self) {
        let mut state = self.serial.lock().await;
        if let Some(state) = state.as_mut() {
            let id = state.flow.capture().map(|c| c.id.clone()); state.flow.stop();
            state.facts.permission_ready = false;
            if let Some(id) = id { let _ = self.effect(serde_json::json!({"action":"stop","captureID":id})).await; }
            self.publish(state);
        }
    }
}
