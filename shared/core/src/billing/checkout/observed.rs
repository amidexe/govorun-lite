use super::{session, CheckoutService};
use crate::{Error, Result};
use serde_json::Value;

/// Current server state discovered without a locally confirmed request.
/// Kept only in this account session; a restart reads the server again.
/// Observation can never reconstruct consent or create another payment.
pub(super) struct ObservedCheckout {
    pub(super) session_id: String,
    pub(super) response: Value,
}

impl CheckoutService {
    pub(super) fn retain_observed_checkout(&self, epoch: u64,
        identity: u64, response: &Value) -> Result<()> {
        session::validate_recovery(response)?;
        let observed = ObservedCheckout {
            session_id: response["checkoutSessionId"].as_str().ok_or(Error::InvalidResponse)?.into(),
            response: response.clone(),
        };
        let mut state = self.checked(epoch, identity)?;
        state.observed = Some(observed);
        state.attempt = None;
        state.preview = None;
        state.selection = None;
        Ok(())
    }
}
