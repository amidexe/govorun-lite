//! A rejected issuance must observe fresh account facts before issuing again.
//! Keep only its safe outcome across transient refresh failures or cancellation.
use super::{Preparation, Stylization};
use govorun_core::{
    style_pass_vault::issuance::Outcome,
    stylization::scheduling::{Completion, Outcome as Scheduled},
    Error, Result,
};

pub(super) struct PendingRefresh {
    pub epoch: u64,
    pub outcome: Outcome,
}
impl Stylization {
    pub(super) async fn resume_refresh(
        &self,
        epoch: u64,
        pending: &mut Option<PendingRefresh>,
    ) -> Result<Option<Preparation>> {
        let Some(saved) = pending.as_ref() else {
            return Ok(None);
        };
        if saved.epoch != epoch {
            *pending = None;
            return Ok(None);
        }
        let outcome = saved.outcome;
        match self
            .account
            .refresh_for_activity(std::time::Duration::ZERO)
            .await
        {
            Ok(_) => {
                self.current(epoch)?;
                self.bind_profile().await?;
                *pending = None;
                Ok(None)
            }
            Err(
                error @ (Error::SessionChanged
                | Error::SignedOut
                | Error::StorageUnavailable
                | Error::StorageInvalid),
            ) => Err(error),
            Err(_) => {
                self.current(epoch)?;
                // No quota from the failed observation authorizes another POST.
                Ok(Some(Preparation {
                    ready: false,
                    retry_after_millis: outcome.retry_after_millis,
                    blocked_reason: None,
                    requires_recovery: false,
                    maintenance: Some(Completion {
                        outcome: Scheduled::Retry,
                        ..outcome.maintenance(true)
                    }),
                }))
            }
        }
    }
    pub(super) async fn rejected_preparation(
        &self,
        outcome: Outcome,
        explicit: bool,
        background: bool,
        required: usize,
    ) -> Result<Preparation> {
        let view = self.observation(explicit, background).await?;
        let completion = outcome.maintenance(view.refill["maintenanceRequired"] == true);
        Ok(Preparation {
            ready: view.ready >= required,
            retry_after_millis: completion.retry_after_millis,
            blocked_reason: None,
            requires_recovery: false,
            maintenance: Some(completion),
        })
    }
}
