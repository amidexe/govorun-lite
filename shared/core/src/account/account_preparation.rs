//! Direct preparation ports. The core observes its own profile and vault;
//! no native authority view participates in foreground admission.
use super::AccountHost;
use crate::features::{
    dictation::{
        preparation::{Phase, Ports, Preparation},
        Failure,
    },
    stylization::commands::random_id,
};
use govorun_core::Error;
use serde_json::Value;
use std::{
    sync::{atomic::Ordering, Arc, Weak},
    time::Duration,
};

pub(super) struct AccountPreparation(Weak<AccountHost>);
fn failure(error: Error) -> Failure {
    if error == Error::SessionChanged {
        Failure::Cancelled
    } else {
        Failure::Effect(error.code())
    }
}
impl AccountPreparation {
    fn host(&self, epoch: u64) -> Result<Arc<AccountHost>, Failure> {
        let host = self.0.upgrade().ok_or(Failure::Cancelled)?;
        if host.stopping.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        host.core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?
            .current(epoch)
            .map_err(failure)?;
        Ok(host)
    }
}
impl Ports for AccountPreparation {
    fn epoch(&self) -> u64 {
        self.0
            .upgrade()
            .and_then(|host| {
                host.core
                    .get()
                    .map(|core| core.snapshot().session_generation)
            })
            .unwrap_or(0)
    }
    async fn plan(
        &self,
        epoch: u64,
        phase: &'static str,
        credits: usize,
    ) -> Result<Value, Failure> {
        let host = self.host(epoch)?;
        let _permit = host
            .requests
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Failure::Cancelled)?;
        self.host(epoch)?;
        let ports = host
            .document_ports
            .get()
            .ok_or(Failure::Effect("documents_unavailable"))?;
        let selected = ports.stylization_enabled().await.map_err(failure)?;
        let online = ports.network_available().await.map_err(failure)?;
        let owner = host
            .stylization
            .get()
            .ok_or(Failure::Effect("styles_unavailable"))?;
        let observation = owner.observation(false, false).await.map_err(failure)?;
        self.host(epoch)?;
        observation
            .preparation(phase, selected, online, credits)
            .map_err(failure)
    }
    async fn refresh(&self, epoch: u64) -> Result<(), Failure> {
        let host = self.host(epoch)?;
        let _permit = host
            .requests
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Failure::Cancelled)?;
        self.host(epoch)?;
        let core = host
            .core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?;
        core.refresh_for_activity_at(epoch, Duration::ZERO)
            .await
            .map_err(failure)?;
        self.host(epoch)?;
        // The durable reserve binding follows the newly published core profile.
        host.stylization
            .get()
            .ok_or(Failure::Effect("styles_unavailable"))?
            .bind_profile()
            .await
            .map_err(failure)?;
        self.host(epoch)?;
        Ok(())
    }
    async fn warm(&self, epoch: u64) -> Result<(), Failure> {
        let host = self.host(epoch)?;
        let _permit = host
            .requests
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Failure::Cancelled)?;
        self.host(epoch)?;
        let ports = host
            .document_ports
            .get()
            .ok_or(Failure::Effect("documents_unavailable"))?;
        if !ports.stylization_enabled().await.map_err(failure)? {
            return Ok(());
        }
        self.host(epoch)?;
        host.stylization
            .get()
            .ok_or(Failure::Effect("styles_unavailable"))?
            .prepare(epoch, true, true, 1, random_id().map_err(failure)?)
            .await
            .map_err(failure)?;
        self.host(epoch)?;
        Ok(())
    }
}
impl AccountHost {
    fn preparation(self: &Arc<Self>) -> Arc<Preparation<AccountPreparation>> {
        self.dictation_preparation
            .get_or_init(|| Preparation::new(Arc::new(AccountPreparation(Arc::downgrade(self)))))
            .clone()
    }
    pub async fn prepare_dictation(self: &Arc<Self>, phase: Phase) -> Result<(), Failure> {
        let epoch = self
            .core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?
            .snapshot()
            .session_generation;
        self.preparation().prepare(epoch, phase).await
    }
    pub async fn join_dictation_preparation(self: &Arc<Self>, epoch: u64, credits: usize) -> u64 {
        self.preparation().join(epoch, credits).await
    }
}
