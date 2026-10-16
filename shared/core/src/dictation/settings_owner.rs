//! One CAS writer for recording settings. The OS port never persists choices.
//! Slow driver/model changes rebase on the current document before committing;
//! a failed commit restores the durable configuration without retrying a write.
use super::{dictionary::Events, snapshot_gate::SnapshotGate};
use govorun_core::{
    permission_setup::{settings, Command, Facts, Field, Snapshot},
    Document, DocumentStore, Future, StoreError,
};
use serde_json::Value;
use std::sync::Arc;
pub trait Ports: Send + Sync {
    fn facts(&self) -> Future<'_, Result<Snapshot, &'static str>>;
    fn busy(&self) -> bool;
    fn execute(&self, command: Command) -> Future<'_, Result<(), &'static str>>;
    fn configure<'a>(
        &'a self,
        field: &'a Field,
        previous: &'a Snapshot,
        next: &'a Snapshot,
    ) -> Future<'a, Result<(), &'static str>>;
}
pub struct Owner {
    store: Arc<dyn DocumentStore>,
    ports: Arc<dyn Ports>,
    serial: tokio::sync::Mutex<()>,
    gate: SnapshotGate,
    events: Events,
}
fn storage(error: StoreError) -> &'static str {
    match error {
        StoreError::Invalid => "dictation_storage_invalid",
        StoreError::Conflict => "dictation_storage_conflict",
        StoreError::Unavailable => "dictation_storage_unavailable",
    }
}
impl Owner {
    pub fn new(store: Arc<dyn DocumentStore>, ports: Arc<dyn Ports>, events: Events) -> Arc<Self> {
        Arc::new(Self {
            store,
            ports,
            serial: Default::default(),
            gate: Default::default(),
            events,
        })
    }
    async fn read(&self) -> Result<(Document, Snapshot), &'static str> {
        let document = self
            .store
            .read_document(settings::KEY.into())
            .await
            .map_err(storage)?
            .ok_or("dictation_storage_unavailable")?;
        let observed = self.ports.facts().await?;
        let snapshot = settings::overlay(&document.value, observed)?;
        Ok((document, snapshot))
    }
    fn publish(&self, snapshot: Snapshot) -> Result<Value, &'static str> {
        let value = serde_json::to_value(snapshot).map_err(|_| "dictation_invalid_facts")?;
        if self.gate.changed(&value) {
            (self.events)("dictation.changed", value.clone());
        }
        Ok(value)
    }
    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let action = match method {
            "dictation.snapshot" if params.as_object().is_some_and(|value| value.is_empty()) => {
                None
            }
            "dictation.action" => {
                Some(serde_json::from_value::<Command>(params).map_err(|_| "invalid_command")?)
            }
            _ => return Err("invalid_command"),
        };
        let _serial = self.serial.lock().await;
        let (document, previous) = self.read().await?;
        if action.is_none() {
            return self.publish(previous);
        }
        if let Some(action) = action {
            let action = Facts {
                can_prepare: !self.ports.busy(),
                settings: previous.clone(),
            }
            .effect(action)?;
            if let Command::Set { field, value } = action {
                if matches!(field, Field::LaunchAtLogin) {
                    self.ports.execute(Command::Set { field, value }).await?;
                } else {
                    self.set(document, previous, field, value).await?;
                }
            } else {
                self.ports.execute(action).await?;
            }
        }
        let (_, snapshot) = self.read().await?;
        self.publish(snapshot)
    }
    /// Reconcile physical resources after OS permission changes. The serialized
    /// read prevents this refresh from rolling back an uncommitted settings edit.
    pub async fn refresh_execution(&self) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let (_, snapshot) = self.read().await?;
        self.ports
            .configure(&Field::AlwaysOnMicrophone, &snapshot, &snapshot)
            .await?;
        self.publish(snapshot)
    }
    async fn set(
        &self,
        document: Document,
        previous: Snapshot,
        field: Field,
        value: Value,
    ) -> Result<(), &'static str> {
        let patch = settings::assign(
            &document.value,
            &field,
            &value,
            &previous,
            self.ports.busy(),
        )?;
        let next = settings::overlay(&patch, previous.clone())?;
        if patch == document.value {
            return Ok(());
        }
        if let Err(error) = self.ports.configure(&field, &previous, &next).await {
            if self
                .ports
                .configure(&field, &next, &previous)
                .await
                .is_err()
            {
                return Err("dictation_configuration_recovery_failed");
            }
            return Err(error);
        }
        let saved = async {
            // Never overwrite account/style changes made during OS work.
            let (current, observed) = self.read().await?;
            let patch =
                settings::assign(&current.value, &field, &value, &observed, self.ports.busy())?;
            let written = if patch == current.value {
                current
            } else {
                self.store
                    .write_document(settings::KEY.into(), Some(current.revision), patch.clone())
                    .await
                    .map_err(storage)?
            };
            if written.value != patch {
                return Err("dictation_storage_invalid");
            }
            Ok(())
        }
        .await;
        if let Err(error) = saved {
            // Roll back to the current durable value, including a concurrent
            // successful settings write; do not roll back unrelated data.
            let durable = self
                .read()
                .await
                .map(|(_, snapshot)| snapshot)
                .unwrap_or(previous);
            if self.ports.configure(&field, &next, &durable).await.is_err() {
                return Err("dictation_configuration_recovery_failed");
            }
            return Err(error);
        }
        Ok(())
    }
}
