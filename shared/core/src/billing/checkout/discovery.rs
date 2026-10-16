use super::{attempt::Attempt, CheckoutHost, CheckoutService};
use crate::Result;

#[derive(Default, PartialEq, Eq)]
pub(super) enum Discovery {
    #[default]
    Unseen,
    Inconclusive,
    Verified,
}

impl CheckoutService {
    /// A readable empty local journal cannot exclude a checkout on another
    /// device. Admission owns the first server check; the UI need not race it.
    pub(super) async fn discover_before_new_operation(&self, host: &dyn CheckoutHost,
        epoch: u64, identity: u64) -> Result<()> {
        {
            let mut state = self.checked(epoch, identity)?;
            if state.discovery != Discovery::Unseen || state.observed.is_some()
                || state.attempt.as_ref().is_some_and(Attempt::unresolved) {
                return Ok(());
            }
            // Mark before I/O. A dropped future requires explicit recovery,
            // never another purchase or an automatic mutation retry.
            state.discovery = Discovery::Inconclusive;
        }
        let result = self.reconcile_locked(host, epoch, "latest").await;
        host.current(epoch)?;
        let result = self.recovery_result(epoch, identity, true, false, result);
        // Both HTTP responses and transport errors can carry an explicit 404.
        // The recovery owner alone proves empty discovery after readable storage.
        if self.checked(epoch, identity)?.discovery == Discovery::Verified { Ok(()) }
        else { result.and_then(crate::HttpResponse::success).map(|_| ()) }
    }
}
