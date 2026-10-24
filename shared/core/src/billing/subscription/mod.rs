//! Subscription interaction owns sequencing; native windows format admitted data
//! and execute one-shot OS effects. Financial attempts remain in CheckoutService.
mod checkout;
mod client;
mod promotion;
mod refresh;
mod state;
pub mod overview;
mod overview_notice;

use crate::{AccountSession, BillingManagementCommand, Error, Result};
use serde_json::{json, Value};
pub(crate) use state::SubscriptionController;
use state::Operation;

impl AccountSession {
    pub async fn subscription_action(&self, epoch: u64, command: Value) -> Result<Value> {
        self.subscription_action_reporting(epoch, command, &|_| {}).await
    }
    /// A refused action shows one generic line; `refused` receives its cause for the journal.
    pub async fn subscription_action_reporting(&self, epoch: u64, command: Value,
        refused: &(dyn Fn(Error) + Sync)) -> Result<Value> {
        self.current(epoch)?;
        let action = command["action"].as_str().ok_or(Error::InvalidRequest)?;
        if action == "dismissPromo" {
            self.inner.promotion_window.execute(self, epoch, &self.promotion_scope(epoch), &command).await?;
            self.inner.subscription.dismiss(epoch);
            self.subscription_changed();
            return Ok(json!({"effect":null}));
        }
        if action == "dismissPreview" {
            self.inner.subscription.dismiss(epoch);
            self.subscription_changed();
            return Ok(json!({"effect":null}));
        }
        if !self.snapshot().signed_in {
            if action == "recover" { return Ok(json!({"effect":null})); }
            if action != "refresh" { return Err(Error::SignedOut); }
        }
        if action == "resumeCheckout" {
            if command["userInitiated"] != true { return Err(Error::ConsentUnconfirmed); }
            let checkout = self.inner.checkout.snapshot(epoch);
            let view = self.inner.subscription.snapshot(epoch, &checkout, &self.inner.billing.snapshot(epoch),
                &self.inner.catalog.snapshot(epoch));
            if view["canResume"] != true { return Err(Error::InvalidRequest); }
            return Ok(json!({"effect":{"kind":"openUrl","url":view["checkoutUrl"]}}));
        }
        if matches!(action, "refresh" | "recover") && self.inner.subscription.queue_refresh(epoch, action == "refresh") {
            return Ok(json!({"effect":null}));
        }
        if !crate::PromotionController::handles(action) && !matches!(action, "refresh" | "recover" | "restartBinding" | "previewCheckout" |
            "choosePaymentMethod" | "confirmCheckout" | "resumeCheckout" |
            "cancelRenewal" | "cancelPlanChange" | "removeCard" | "requestRefund" | "activateStarter") {
            return Err(Error::InvalidRequest);
        }
        if matches!(action, "previewCheckout" | "choosePaymentMethod" | "resumeCheckout" | "restartBinding")
            && command["userInitiated"] != true {
            return Err(Error::ConsentUnconfirmed);
        }
        let operation = self.inner.subscription.begin(epoch,
            matches!(action, "previewCheckout" | "choosePaymentMethod" | "previewPromo"),
            !matches!(action, "refresh" | "recover") || command["userInitiated"] == true)?;
        let _guard = Completion { session: self, operation };
        self.subscription_changed();
        let mut result = match action {
            action if crate::PromotionController::handles(action) => self.subscription_promote(operation, &command).await,
            "previewCheckout" | "choosePaymentMethod" => self.subscription_prepare(operation, &command).await,
            "confirmCheckout" => self.subscription_confirm(operation, &command).await,
            "restartBinding" => self.subscription_restart(operation).await,
            "refresh" | "recover" => self.subscription_refresh(operation, action == "refresh").await,
            "activateStarter" => self.subscription_activate(epoch, &command).await,
            _ => self.subscription_manage(epoch, action, &command).await,
        };
        self.current(epoch)?;
        self.inner.subscription.require(operation)?;
        if let Some(full) = self.inner.subscription.take_refresh(operation)? {
            let recovery = self.subscription_refresh(operation, full).await;
            self.current(epoch)?;
            self.inner.subscription.require(operation)?;
            result = match result {
                // A foreground status check may suppress opening the bank again,
                // but cannot discard settings after access was already granted.
                Ok(effect) if matches!(effect["kind"].as_str(), Some("activateStyle" | "promotionAccess")) => {
                    if let Err(error) = recovery { self.inner.subscription.fail(operation, &error)?; }
                    Ok(effect)
                }
                Err(error) => Err(error),
                Ok(_) => recovery,
            };
        }
        match result {
            Ok(effect) => Ok(json!({"effect":effect})),
            Err(error @ (Error::SessionChanged | Error::SignedOut)) => Err(error),
            Err(error) => {
                self.inner.subscription.fail(operation, &error)?;
                refused(error);
                Ok(json!({"effect":null, "failed":true}))
            }
        }
    }

    async fn subscription_manage(&self, epoch: u64, action: &str, command: &Value) -> Result<Value> {
        let management = match action {
            "cancelRenewal" => BillingManagementCommand::CancelRenewal,
            "cancelPlanChange" => BillingManagementCommand::CancelScheduledPlanChange,
            "requestRefund" => serde_json::from_value(json!({"requestRefund": command["reasonCode"]}))
                .map_err(|_| Error::InvalidRequest)?,
            _ => BillingManagementCommand::RemoveCard,
        };
        self.manage_billing(epoch, management, command["confirmed"] == true).await?;
        // Access is independent: a read failure cannot undo an accepted mutation.
        let _ = self.refresh_profile_at(epoch).await;
        self.current(epoch)?;
        Ok(Value::Null)
    }

    fn subscription_changed(&self) {
        // Native hosts admit views by the account revision, not only by
        // the watch notification. Window-only changes must advance it as well.
        drop(self.mutate());
        self.changed();
    }

    async fn subscription_activate(&self, epoch: u64, command: &Value) -> Result<Value> {
        let selection = command["selection"].as_str().ok_or(Error::InvalidRequest)?;
        if !matches!(selection, "formal" | "light" | "concise") ||
            command["termsVersion"].as_u64() != Some(u64::from(self.inner.terms)) {
            return Err(Error::InvalidRequest);
        }
        if let Some(options) = command.get("activationOptions").filter(|value| !value.is_null()) {
            if !matches!(options["emojiMode"].as_str(), Some("off" | "restrained" | "lively" | "bright")) ||
                !options["reducedPunctuation"].is_boolean() || !options["profanityFilter"].is_boolean() {
                return Err(Error::InvalidRequest);
            }
        }
        self.activate_free_access(epoch, crate::FreeAccessActivation {
            confirmed: command["confirmed"] == true, terms_version: Some(self.inner.terms),
            preferred_style: selection.to_owned(),
            expected_program_version: command["programVersion"].as_str().map(str::to_owned),
        }).await?;
        Ok(json!({"kind":"activateStyle", "selection":selection, "termsVersion":self.inner.terms,
            "activationOptions":command["activationOptions"]}))
    }
}

struct Completion<'a> { session: &'a AccountSession, operation: Operation }
impl Drop for Completion<'_> {
    fn drop(&mut self) {
        if self.session.current(self.operation.epoch).is_ok()
            && self.session.inner.subscription.finish(self.operation)
        {
            self.session.subscription_changed();
        }
    }
}
