use super::state::{Operation, Preview};
use crate::{AccountSession, CheckoutInteraction, Error, HttpResponse, Result};
use serde_json::{json, Value};

impl AccountSession {
    pub(super) async fn subscription_prepare(&self, operation: Operation, command: &Value) -> Result<Value> {
        let client = super::client::Client::read(command)?;
        let mut selection = if command["action"] == "choosePaymentMethod" {
            let state = self.inner.subscription.checked(operation)?;
            let preview = state.preview.as_ref().ok_or(Error::InvalidRequest)?;
            if command["previewId"] != preview.id { return Err(Error::InvalidRequest); }
            let method = command["method"].as_str().ok_or(Error::InvalidRequest)?;
            if !preview.wire["availablePaymentMethods"].as_array()
                .is_some_and(|methods| methods.iter().any(|value| value == method)) {
                return Err(Error::InvalidRequest);
            }
            let mut selection = preview.selection.clone();
            selection["paymentMethodKind"] = json!(method);
            selection
        } else {
            let offer = command["offerId"].as_str().ok_or(Error::InvalidRequest)?;
            let mut selection = json!({"offerId":offer});
            if let Some(method) = client.preferred_method() {
                selection["preferredPaymentMethodKind"] = json!(method);
            }
            if let Some(promo) = command.get("promoPreviewId").filter(|value| !value.is_null()) {
                selection["promoPreviewId"] = promo.clone();
            }
            selection
        };
        // Normalize absent optional fields so adapters cannot create different
        // intents by serializing null instead of omitting a value.
        selection.as_object_mut().ok_or(Error::InvalidRequest)?.retain(|_, v| !v.is_null());
        {
            let mut state = self.inner.subscription.checked(operation)?;
            state.preview = None;
            state.browser_sbp = client.opens_sbp();
        }
        let displayed = command.get("displayedCatalog").filter(|value| !value.is_null());
        let result = self.begin_checkout_interaction(operation.epoch, selection.clone(), displayed, client.channel()).await?;
        self.inner.subscription.require(operation)?;
        match result {
            CheckoutInteraction::Confirmation(reply) => {
                self.subscription_admit_reply(operation, &reply)?;
                self.inner.subscription.checked(operation)?.preview = Some(Preview {
                    id: format!("{}:{}", operation.epoch, operation.serial), wire: reply.body, selection,
                    catalog: self.inner.catalog.snapshot(operation.epoch).catalog.unwrap_or(Value::Null),
                });
                Ok(Value::Null)
            }
            CheckoutInteraction::Outcome(reply) => self.subscription_outcome(operation, reply, true),
        }
    }

    pub(super) async fn subscription_confirm(&self, operation: Operation, command: &Value) -> Result<Value> {
        if command["confirmed"] != true { return Err(Error::ConsentUnconfirmed); }
        let preview = {
            let state = self.inner.subscription.checked(operation)?;
            let preview = state.preview.as_ref().ok_or(Error::InvalidRequest)?;
            if command["previewId"] != preview.id { return Err(Error::InvalidRequest); }
            preview.wire.clone()
        };
        let reply = self.confirm_prepared_checkout(operation.epoch, json!({
            "offerId":preview["offerId"], "subscriptionConsentToken":preview["subscriptionConsentToken"],
            "receiptEmail":command["email"], "subscriptionConsentAccepted":true,
            "paymentServiceNoticeAccepted":true,
        })).await?;
        self.subscription_outcome(operation, reply, true)
    }

    pub(super) fn subscription_outcome(&self, operation: Operation, reply: HttpResponse,
        open_bank: bool) -> Result<Value> {
        self.subscription_admit_reply(operation, &reply)?;
        let presentation = &reply.body["clientCheckout"];
        let action = presentation["launchAction"].as_str().ok_or(Error::InvalidResponse)?;
        if !matches!(action, "complete" | "open_bank" | "reconcile") { return Err(Error::InvalidResponse); }
        let mut state = self.inner.subscription.checked(operation)?;
        state.preview = None;
        state.message = presentation["message"].as_str().map(str::to_owned);
        let deadline = presentation["expiresInSeconds"].as_i64()
            .and_then(|seconds| chrono::Utc::now().checked_add_signed(chrono::Duration::seconds(seconds)));
        let same_session = state.outcome.as_ref().is_some_and(|old|
            old["checkoutSessionId"].is_string() && old["checkoutSessionId"] == reply.body["checkoutSessionId"]);
        state.expires_at = match (state.expires_at.filter(|_| same_session), deadline) {
            (Some(old), Some(next)) => Some(old.min(next)),
            (_, next) => next,
        };
        let effect = if open_bank && action == "open_bank" &&
            (state.browser_sbp || reply.body["paymentMethodKind"] != "sbp") {
            json!({"kind":"openUrl", "url":presentation["resumeUrl"]})
        } else { Value::Null };
        state.outcome = Some(reply.body);
        Ok(effect)
    }

}

impl AccountSession {
    fn subscription_admit_reply(&self, operation: Operation, reply: &HttpResponse) -> Result<()> {
        if (200..300).contains(&reply.status) { return Ok(()); }
        let mut state = self.inner.subscription.checked(operation)?;
        if let Some(directive) = reply.body.get("clientCheckoutFailure") {
            if directive["discardConfirmation"] == true { state.preview = None; }
            state.failure_message = directive["message"].as_str().map(str::to_owned);
        } else if let Some(directive) = reply.body.get("clientCheckoutRecoveryFailure") {
            state.failure_message = directive["message"].as_str().map(str::to_owned);
        }
        Err(Error::Http(reply.status))
    }
}
