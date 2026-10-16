//! Physical ACK and recovery complete before updating the active settings.
#[path = "resources.rs"]
mod resources;
use super::{Runtime, Settings};
use govorun_os_runtime::capture::owner::Preferences;
use std::sync::Arc;
impl resources::Ports for Runtime {
    fn input<'a>(&'a self, input: &'a Preferences) -> resources::Reply<'a> {
        Box::pin(async move {
            let done = self.executor.configure(input.clone(), false)?;
            tokio::task::spawn_blocking(move || {
                done.recv().map_err(|_| "microphone_executor_unavailable")?
            })
            .await
            .map_err(|_| "microphone_executor_unavailable")?
        })
    }
    fn model<'a>(&'a self, config: &'a resources::Config) -> resources::Reply<'a> {
        Box::pin(async move {
            let model = self.model.clone();
            let busy = self.busy();
            let compute = config.compute;
            let keep = config.keep_model;
            tokio::task::spawn_blocking(move || model.configure(compute, keep, busy))
                .await
                .map_err(|_| "recognition_model_unavailable")?
        })
    }
}
impl Runtime {
    pub async fn configure(self: &Arc<Self>, next: Settings) -> Result<(), &'static str> {
        let previous = self
            .settings
            .read()
            .map_err(|_| "dictation_settings_unavailable")?
            .clone();
        resources::apply(self.as_ref(), &previous.resources(), &next.resources()).await?;
        *self
            .settings
            .write()
            .map_err(|_| "dictation_settings_unavailable")? = next;
        self.processing_changed();
        Ok(())
    }
    pub async fn prepare(self: &Arc<Self>) -> Result<(), &'static str> {
        if self.busy() {
            return Err("dictation_prepare_unavailable");
        }
        let model = self.model.clone();
        let result = tokio::task::spawn_blocking(move || model.prepare())
            .await
            .map_err(|_| "recognition_model_unavailable")?;
        self.processing_changed();
        result
    }
}
