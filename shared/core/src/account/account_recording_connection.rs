//! Optional connection preparation uses the same rules as the Android boundary.
use super::AccountHost;
use crate::{features::stylization::commands::Command, Error};

impl AccountHost {
    pub async fn begin_styles_recording(&self) -> Result<Option<String>, Error> {
        let core = self.core.get().ok_or(Error::StorageUnavailable)?;
        let ports = self.document_ports.get().ok_or(Error::StorageUnavailable)?;
        let owner = self.stylization.get().ok_or(Error::StorageUnavailable)?;
        let epoch = core.snapshot().session_generation;
        let selected = ports.stylization_enabled().await?;
        let online = ports.network_available().await?;
        core.current(epoch)?;
        let result = owner.dispatch_configured(
            Command::BeginRecording { session_generation: epoch }, selected, online, &|_| {},
        ).await?;
        Ok(result["requestId"].as_str().map(str::to_owned))
    }
    pub async fn end_styles_recording(&self, request_id: String) {
        if let Some(owner) = self.stylization.get() {
            let _ = owner.dispatch(Command::EndRecording { request_id }, &|_| {}).await;
        }
    }
}
