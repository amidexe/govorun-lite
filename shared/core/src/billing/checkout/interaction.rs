use super::{attempt::Attempt, CheckoutHost, CheckoutService};
use crate::{CatalogChannel, HttpResponse, Result};
use serde_json::Value;

/// A user action either needs a new disclosure or continues the confirmation
/// already owned by the protected journal. UI never stores a second attempt.
pub enum CheckoutInteraction {
    Confirmation(HttpResponse),
    Outcome(HttpResponse),
}

impl CheckoutService {
    pub async fn begin_interaction(
        &self, host: &dyn CheckoutHost, epoch: u64, selection: Value,
        displayed: Option<&Value>, channel: CatalogChannel,
    ) -> Result<CheckoutInteraction> {
        host.current(epoch)?;
        super::preparation::validate_request(&selection)?;
        let _operation = self.operation.lock().await;
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Prepare)?;
        // This also rejects a different selection while a confirmed attempt is
        // unresolved. Restore, decision and execution share one operation gate.
        let preview = self.prepare_locked(host, epoch, selection, displayed, channel).await?;
        if !(200..300).contains(&preview.status) {
            return Ok(CheckoutInteraction::Confirmation(preview));
        }
        let confirmed = {
            let state = self.checked(epoch, identity)?;
            state.attempt.as_ref().filter(|attempt| attempt.unresolved())
                .map(|Attempt { request, intent, .. }| (request.path().to_owned(), intent.clone()))
        };
        let Some((path, intent)) = confirmed else {
            return Ok(CheckoutInteraction::Confirmation(preview));
        };
        let response = self.confirm_with_presentation_locked(
            host, epoch, identity, Some(&path), intent,
        ).await?;
        Ok(CheckoutInteraction::Outcome(response))
    }
}
