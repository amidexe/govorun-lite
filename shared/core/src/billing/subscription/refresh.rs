use super::state::Operation;
use crate::{AccountRequest, AccountSession, Error, Method, Result};
use serde_json::Value;

impl AccountSession {
    async fn subscription_reconcile(&self, operation: Operation) -> Result<()> {
        let reply = self.reconcile_checkout(operation.epoch, "latest").await?;
        self.inner.subscription.require(operation)?;
        if reply.status == 404 && reply.body["clientCheckoutRecoveryFailure"]["discoveryEmpty"] == true {
            let mut state = self.inner.subscription.checked(operation)?;
            state.outcome = None;
            state.expires_at = None;
            return Ok(());
        }
        self.subscription_outcome(operation, reply, false)?;
        Ok(())
    }

    pub(super) async fn subscription_refresh(&self, operation: Operation, full: bool) -> Result<Value> {
        if !self.snapshot().signed_in {
            self.refresh_catalog(operation.epoch).await?;
            return Ok(Value::Null);
        }
        // One failed read cannot skip the other independent sources. Each step
        // remains bound to the captured session; revocation stops the sequence.
        let mut failure = self.subscription_reconcile(operation).await.err();
        self.current(operation.epoch)?;
        self.inner.subscription.require(operation)?;
        let status = self.request_at(operation.epoch,
            AccountRequest::new(Method::Get, "/v1/billing/status", None)?).await
            .and_then(|reply| reply.success().map(|_| ()));
        if failure.is_none() { failure = status.err(); }
        self.current(operation.epoch)?;
        self.inner.subscription.require(operation)?;
        if full {
            let profile = self.refresh_profile_at(operation.epoch).await;
            if failure.is_none() { failure = profile.err(); }
            self.current(operation.epoch)?;
            self.inner.subscription.require(operation)?;
            let catalog = self.refresh_catalog(operation.epoch).await;
            if failure.is_none() { failure = catalog.err(); }
            self.current(operation.epoch)?;
        }
        match failure { Some(error) => Err(error), None => Ok(Value::Null) }
    }

    pub(super) async fn subscription_restart(&self, operation: Operation) -> Result<Value> {
        let id = {
            let state = self.inner.subscription.checked(operation)?;
            let outcome = state.outcome.as_ref().ok_or(Error::InvalidRequest)?;
            if outcome["clientCheckout"]["canCancel"] != true { return Err(Error::InvalidRequest); }
            outcome["checkoutSessionId"].as_str().ok_or(Error::InvalidResponse)?.to_owned()
        };
        let reply = self.cancel_checkout_binding(operation.epoch, &id).await?;
        self.subscription_outcome(operation, reply, false)?;
        self.subscription_refresh(operation, false).await
    }
}
