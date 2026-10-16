use super::{session, CheckoutHost, CheckoutService};
use crate::{AccountRequest, Error, HttpResponse, Method, Result};
use serde_json::json;

impl CheckoutService {
    /// Cancel one explicitly selected checkout. The server owns the race with
    /// bank confirmation; neither a lost reply nor a conflict proves cancellation.
    pub async fn cancel_binding(
        &self,
        host: &dyn CheckoutHost,
        epoch: u64,
        requested: &str,
    ) -> Result<HttpResponse> {
        host.current(epoch)?;
        let _operation = self.operation.lock().await;
        host.current(epoch)?;
        let identity = self.scope(epoch);
        let _activity = self.begin_activity(host, epoch, identity, super::lifecycle::Activity::Cancel)?;
        let result = self.cancel_binding_locked(host, epoch, requested).await;
        host.current(epoch)?;
        self.recovery_result(epoch, identity, requested == "latest", true, result)
    }

    async fn cancel_binding_locked(&self, host: &dyn CheckoutHost, epoch: u64,
        requested: &str) -> Result<HttpResponse> {
        if !crate::promotions::valid_preview_id(&json!(requested)) {
            return Err(Error::InvalidRequest);
        }
        let (identity, key) = self.restore(host, epoch).await?;
        let target = self.recovery_target(epoch, identity, key, requested)?;
        if target.journaled || target.observed {
            // Resolve an unknown ID through explicit recovery first. Guessing a
            // different ID must never erase or replace the uncertain purchase.
            if target.exact_id.as_deref() != Some(requested) {
                return Err(Error::InvalidRequest);
            }
            let saved = {
                let state = self.checked(epoch, identity)?;
                if target.observed {
                    state.observed.as_ref().map(|v| v.response.clone()).map(|body| HttpResponse {
                        status: 200, body, etag: None, retry_after_seconds: None })
                } else {
                    Some(state.attempt.as_ref().and_then(|attempt| attempt.response.clone())
                        .ok_or(Error::StorageInvalid)?)
                }
            };
            if let Some(saved) = saved {
                if session::validate_recovery(&saved.body)? != "pending"
                    || saved.body["cancellationPending"] == true {
                    return self.accept_recovery(host, epoch, &target, Ok(saved)).await;
                }
                if !session::can_cancel(&saved.body) { return Err(Error::InvalidRequest); }
            }
        }
        // Do not preflight with GET: ordinary reconciliation can advance a
        // previously confirmed payment. Cancellation has its own server boundary.
        let path = format!("/v1/billing/tbank/checkout/{requested}/cancel-binding");
        let mut result = host
            .request(epoch, AccountRequest::internal(Method::Post, &path, None))
            .await;
        host.current(epoch)?;
        drop(self.checked(epoch, identity)?);
        if result.as_ref().is_ok_and(|reply| {
            reply.status == 409 && reply.body["error"] == "checkout_requires_reconciliation"
        }) {
            result = host
                .request(
                    epoch,
                    AccountRequest::internal(Method::Get, &target.path, None),
                )
                .await;
        }
        self.accept_recovery(host, epoch, &target, result).await
    }
}
