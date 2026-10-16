use super::{attempt::Attempt, CheckoutHost, CheckoutService};
use crate::{Error, Result};
use serde::Serialize;

/// UI admission and the current server session come from the checkout owner.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CheckoutSnapshot {
    pub epoch: u64,
    pub revision: u64,
    pub restored: bool,
    pub pending: bool,
    /// Display-only evidence of an actual unresolved attempt. Discovery still blocks admission.
    pub purchase_pending: bool,
    pub working: bool,
    pub checking: bool,
    pub session_id: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) enum Activity { Prepare, Confirm, Reconcile, Cancel }

impl CheckoutService {
    pub fn snapshot(&self, epoch: u64) -> CheckoutSnapshot {
        let state = self.state.lock().expect("checkout state lock");
        let current = state.epoch == epoch;
        let restored = current && state.account.is_some();
        let session_id = if !restored { None } else {
            state.observed.as_ref().map(|value| value.session_id.clone()).or_else(|| {
                state.attempt.as_ref().filter(|attempt| attempt.unresolved())
                    .and_then(|attempt| attempt.response.as_ref())
                    .and_then(|reply| reply.body["checkoutSessionId"].as_str()).map(str::to_owned)
            })
        };
        CheckoutSnapshot {
            epoch, revision: state.revision, restored,
            pending: !restored || state.discovery != super::discovery::Discovery::Verified || state.observed.is_some()
                || state.attempt.as_ref().is_some_and(Attempt::unresolved),
            purchase_pending: current && (state.observed.is_some()
                || state.attempt.as_ref().is_some_and(Attempt::unresolved)),
            working: current && state.activity.is_some(),
            checking: current && matches!(state.activity, Some(Activity::Reconcile | Activity::Cancel)),
            session_id,
        }
    }

    pub(super) fn begin_activity<'a>(&'a self, host: &'a dyn CheckoutHost,
        epoch: u64, identity: u64, activity: Activity) -> Result<ActivityGuard<'a>> {
        let mut state = self.checked(epoch, identity)?;
        state.revision += 1;
        state.activity = Some(activity);
        // Cancellation or a dropped future cannot prove that discovery was empty.
        if matches!(activity, Activity::Reconcile | Activity::Cancel) { state.discovery = super::discovery::Discovery::Inconclusive; }
        drop(state);
        let guard = ActivityGuard { service: self, host, epoch, identity };
        host.checkout_changed(&self.snapshot(epoch))?;
        Ok(guard)
    }

    pub(super) fn require_confirmable(&self, epoch: u64, identity: u64) -> Result<()> {
        let state = self.checked(epoch, identity)?;
        if state.discovery != super::discovery::Discovery::Verified && !state.attempt.as_ref().is_some_and(Attempt::unresolved) {
            Err(Error::InvalidRequest)
        } else { Ok(()) }
    }
}

pub(super) struct ActivityGuard<'a> {
    service: &'a CheckoutService, host: &'a dyn CheckoutHost, epoch: u64, identity: u64,
}
impl Drop for ActivityGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.service.checked(self.epoch, self.identity) {
            state.activity = None;
            state.revision += 1;
            drop(state);
            let _ = self.host.checkout_changed(&self.service.snapshot(self.epoch));
        }
    }
}

pub(super) struct SessionPublication<'a> {
    pub(super) session: &'a crate::AccountSession,
    pub(super) epoch: u64,
}
impl Drop for SessionPublication<'_> {
    fn drop(&mut self) { self.session.publish_checkout_state(self.epoch); }
}
