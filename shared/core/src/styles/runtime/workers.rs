//! Effect composition for the common reserve/credit schedulers. Configuration
//! is an authoritative host fact; retry rules are never reimplemented here.
use super::{
    commands::random_id,
    scheduler::{Failure, Scheduler},
    Stylization,
};
use govorun_core::{
    stylization::scheduling::{Completion, Kind, Outcome, Trigger},
    Error, Future, Result,
};
use std::sync::Arc;
pub type Enabled = Arc<dyn Fn() -> Future<'static, Result<bool>> + Send + Sync>;
pub struct Workers {
    reserve: Arc<Scheduler>,
    credit_completed: tokio::sync::watch::Sender<u64>,
}
impl Workers {
    pub fn new(
        owner: &Arc<Stylization>,
        enabled: Enabled,
        online: super::reporting::Online,
    ) -> Self {
        let weak = Arc::downgrade(owner);
        let failure: Failure = Arc::new(move |reason| {
            if let Some(owner) = weak.upgrade() {
                let _ = owner.foreground.reporting().record(reason);
            }
        });
        let credit_completed = tokio::sync::watch::channel(0u64).0;
        let weak = Arc::downgrade(owner);
        let reserve = Scheduler::start(
            owner.account.clone(),
            Kind::Reserve,
            Arc::new(move |epoch, explicit| {
                let weak = weak.clone();
                let enabled = enabled.clone();
                let online = online.clone();
                Box::pin(async move {
                    let Some(owner) = weak.upgrade() else {
                        return completion(Outcome::Cancelled);
                    };
                    if owner.current(epoch).is_err() {
                        return completion(Outcome::Cancelled);
                    }
                    if !online().await {
                        return completion(Outcome::Offline);
                    }
                    if owner.current(epoch).is_err() {
                        return completion(Outcome::Cancelled);
                    }
                    let allowed = enabled().await;
                    match allowed {
                        Ok(true) => {}
                        Ok(false) => return completion(Outcome::Cancelled),
                        Err(error) => return error_completion(error, Kind::Reserve),
                    }
                    if owner.current(epoch).is_err() {
                        return completion(Outcome::Cancelled);
                    }
                    let id = match random_id() {
                        Ok(id) => id,
                        Err(error) => return error_completion(error, Kind::Reserve),
                    };
                    match owner.prepare(epoch, explicit, true, 1, id).await {
                        Ok(prepared) => {
                            if let Some(completion) = prepared.maintenance {
                                return completion;
                            }
                            if let Some(delay) = prepared.retry_after_millis {
                                return Completion {
                                    retry_after_millis: Some(delay),
                                    ..completion(Outcome::Retry)
                                };
                            }
                            match owner.observation(explicit, true).await {
                                Ok(view) => {
                                    completion(if view.refill["maintenanceRequired"] == true {
                                        Outcome::Progress
                                    } else {
                                        Outcome::Ready
                                    })
                                }
                                Err(error) => error_completion(error, Kind::Reserve),
                            }
                        }
                        Err(error) => error_completion(error, Kind::Reserve),
                    }
                })
            }),
            failure,
        );
        Self {
            reserve,
            credit_completed,
        }
    }
    pub fn reserve(&self, epoch: u64, trigger: Trigger, join: bool) -> Result<()> {
        self.reserve.request(epoch, trigger, join, 0)
    }
    pub fn credit(&self, epoch: u64, trigger: Trigger, join: bool, delay: u64) -> Result<()> {
        let _ = (epoch, trigger, join, delay);
        Ok(())
    }
    pub fn subscribe_credit(&self) -> tokio::sync::watch::Receiver<u64> {
        self.credit_completed.subscribe()
    }
    pub async fn stop(&self) {
        self.reserve.stop().await;
    }
}
pub(super) fn completion(outcome: Outcome) -> Completion {
    Completion {
        outcome,
        retry_after_millis: None,
        status: None,
        wire_attempted: false,
    }
}
pub(super) fn error_completion(error: Error, kind: Kind) -> Completion {
    let mut result = completion(match error {
        Error::SessionChanged | Error::SignedOut | Error::StyleAccess(govorun_core::stylization::AccessFailure::NoAccess) => Outcome::Cancelled,
        Error::StyleTransport(govorun_core::stylization::Failure::Offline) => {
            Outcome::Offline
        }
        Error::Http(_) if matches!(kind, Kind::Reserve) => Outcome::HttpFailure,
        _ if matches!(kind, Kind::Credit) => Outcome::Failed,
        _ => Outcome::Retry,
    });
    if let Error::Http(status) = error {
        result.status = Some(status);
    }
    result
}
