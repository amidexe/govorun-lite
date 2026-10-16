//! Compose processing settings from the durable Rust document, never from
//! native preference mirrors. Unknown configuration suspends background work.
use super::NativeDocuments;
use crate::features::stylization::{commands::random_id, Stylization};
use govorun_core::{stylization::reporting::Settings, Error, Result};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) struct Metadata {
    enabled: bool,
    accepted_terms: u32,
}
impl NativeDocuments {
    pub fn attach_stylization(self: &Arc<Self>, owner: Arc<Stylization>) -> Result<()> {
        let weak = Arc::downgrade(self);
        owner.set_timing_sink(Arc::new(move |timing| {
            if let Some(ports) = weak.upgrade() {
                let mut tasks = ports.diagnostic_tasks.lock().unwrap();
                tasks.retain(|task| !task.is_finished());
                let ports = ports.clone();
                tasks.push(tokio::spawn(async move {
                    let _ = ports
                        .call(
                            "data.diagnostic",
                            json!({"event":"stylization_timing", "timing":timing}),
                        )
                        .await;
                }));
            }
        }))?;
        self.stylization
            .set(owner)
            .map_err(|_| Error::InvalidRequest)?;
        if let Some(self_host) = self.self_host.get() {
            let weak = Arc::downgrade(self);
            self_host.attach_route_changed(Arc::new(move |route| {
                if let Some(ports) = weak.upgrade() {
                    let _ = ports.configure_route(route);
                }
            }));
        }
        Ok(())
    }
    fn configure_route(&self, route: Option<bool>) -> Result<()> {
        let metadata = *self
            .style_metadata
            .lock()
            .map_err(|_| Error::InvalidResponse)?;
        if let Some(owner) = self.stylization.get() {
            owner.configure_reporting(metadata.zip(route).map(|(metadata, self_hosted)| {
                Settings {
                    official_enabled: metadata.enabled,
                    accepted_terms: metadata.accepted_terms,
                    self_hosted,
                }
            }))?;
        }
        Ok(())
    }
    pub async fn style_settings(&self) -> Result<Option<Settings>> {
        let metadata = *self
            .style_metadata
            .lock()
            .map_err(|_| Error::InvalidResponse)?;
        let Some(metadata) = metadata else {
            return Ok(None);
        };
        let self_hosted = if let Some(owner) = self.self_host.get() {
            owner
                .route_enabled()
                .await
                .map_err(|_| Error::StorageUnavailable)?
        } else {
            false
        };
        Ok(Some(Settings {
            official_enabled: metadata.enabled,
            accepted_terms: metadata.accepted_terms,
            self_hosted,
        }))
    }
    pub async fn network_observation(&self) -> Result<Option<bool>> {
        let facts: super::DeviceFacts = serde_json::from_value(
            self.call("data.localFacts", json!({}))
                .await
                .map_err(|_| Error::NetworkUnavailable)?,
        )
        .map_err(|_| Error::InvalidResponse)?;
        Ok(facts.network_available)
    }
    pub async fn network_available(&self) -> Result<bool> {
        // Unknown path status is not confirmed offline.
        Ok(self.network_observation().await? != Some(false))
    }
    pub async fn stylization_enabled(&self) -> Result<bool> {
        let settings = self.style_settings().await;
        if let Some(owner) = self.stylization.get() {
            owner.configure_reporting(settings.as_ref().ok().copied().flatten())?;
        }
        Ok(settings?.is_some_and(|v| v.official_selected(self.core.snapshot().access.client_terms_version)))
    }
    pub(super) async fn configure_stylization(
        &self,
        value: &Value,
    ) -> std::result::Result<(), &'static str> {
        let metadata = &value["metadata"];
        let decoded = (|| {
            Ok(Metadata {
                enabled: metadata["enabled"]
                    .as_bool()
                    .ok_or("styles_invalid_response")?,
                accepted_terms: metadata["localTermsVersion"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("styles_invalid_response")?,
            })
        })();
        *self
            .style_metadata
            .lock()
            .map_err(|_| "styles_invalid_response")? = decoded.ok();
        // A failed/unknown configuration must never authorize a worker.
        self.stylization_enabled()
            .await
            .map_err(|error| error.code())?;
        decoded.map(|_| ())
    }
    pub(super) async fn prepare_stylization(
        &self,
        generation: u64,
    ) -> std::result::Result<Value, &'static str> {
        self.current(generation)?;
        let owner = self.stylization.get().ok_or("stylization_unavailable")?;
        if self
            .stylization_enabled()
            .await
            .map_err(|error| error.code())?
        {
            owner
                .prepare(
                    generation,
                    true,
                    true,
                    1,
                    random_id().map_err(|e| e.code())?,
                )
                .await
                .map_err(|error| error.code())?;
        }
        self.current(generation)?;
        Ok(Value::Null)
    }
}
