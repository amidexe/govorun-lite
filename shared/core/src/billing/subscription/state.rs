use crate::{Error, Result};
use serde_json::{json, Value};
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Operation { pub epoch: u64, pub serial: u64, pub foreground: bool }

#[derive(Clone)]
pub(super) struct Preview {
    pub id: String,
    pub wire: Value,
    pub selection: Value,
    pub catalog: Value,
}

#[derive(Default)]
pub(super) struct State {
    epoch: u64,
    serial: u64,
    active: Option<Operation>,
    preparing: bool,
    refresh_queued: Option<bool>,
    pub browser_sbp: bool,
    pub preview: Option<Preview>,
    pub message: Option<String>,
    pub failure_message: Option<String>,
    failed: bool,
    pub outcome: Option<Value>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Default)]
pub(crate) struct SubscriptionController { state: Mutex<State> }

impl SubscriptionController {
    fn state(&self) -> MutexGuard<'_, State> { self.state.lock().expect("subscription window lock") }

    pub(super) fn begin(&self, epoch: u64, preparing: bool, foreground: bool) -> Result<Operation> {
        let mut state = self.state();
        if state.epoch != epoch { *state = State { epoch, ..Default::default() }; }
        if state.active.is_some() { return Err(Error::InvalidRequest); }
        state.serial += 1;
        let operation = Operation { epoch, serial: state.serial, foreground };
        state.active = Some(operation);
        state.preparing = preparing;
        state.refresh_queued = None;
        if foreground {
            state.message = None;
            state.failure_message = None;
            state.failed = false;
        }
        Ok(operation)
    }

    pub(super) fn checked(&self, operation: Operation) -> Result<MutexGuard<'_, State>> {
        let state = self.state();
        if state.epoch != operation.epoch || state.active != Some(operation) {
            Err(Error::SessionChanged)
        } else { Ok(state) }
    }

    pub(super) fn require(&self, operation: Operation) -> Result<()> {
        drop(self.checked(operation)?);
        Ok(())
    }

    pub(super) fn finish(&self, operation: Operation) -> bool {
        if let Ok(mut state) = self.checked(operation) {
            state.active = None;
            state.preparing = false;
            state.refresh_queued = None;
            true
        } else {
            false
        }
    }

    pub(super) fn dismiss(&self, epoch: u64) {
        let mut state = self.state();
        if state.epoch != epoch || state.active.is_some() && !state.preparing { return; }
        state.active = None;
        state.refresh_queued = None;
        state.preview = None;
        state.message = None;
        state.failed = false;
    }

    pub(super) fn queue_refresh(&self, epoch: u64, full: bool) -> bool {
        let mut state = self.state();
        if state.epoch != epoch || state.active.is_none() { return false; }
        state.refresh_queued = Some(state.refresh_queued.unwrap_or(false) || full);
        true
    }

    pub(super) fn take_refresh(&self, operation: Operation) -> Result<Option<bool>> {
        let mut state = self.checked(operation)?;
        Ok(state.refresh_queued.take())
    }

    pub(crate) fn catalog_changed(&self, epoch: u64, catalog: Option<&Value>) {
        let mut state = self.state();
        if state.epoch == epoch && state.preview.as_ref().is_some_and(|preview|
            !crate::checkout::same_published_catalog(&preview.catalog, catalog.unwrap_or(&Value::Null))) {
            state.preview = None;
        }
    }

    pub(super) fn fail(&self, operation: Operation, error: &Error) -> Result<()> {
        let mut state = self.checked(operation)?;
        if !operation.foreground { return Ok(()); }
        state.failed = true;
        if let Some(message) = state.failure_message.take() {
            state.message = Some(message);
            return Ok(());
        }
        let message = match error {
            Error::Checkout(failure) => {
                if failure.discard_confirmation { state.preview = None; }
                failure.message
            }
            Error::CheckoutRecovery(failure) => failure.message.unwrap_or("Не удалось проверить состояние покупки."),
            _ => "Не удалось выполнить действие. Проверьте связь и попробуйте ещё раз.",
        };
        state.message = Some(message.to_owned());
        Ok(())
    }

    pub(crate) fn snapshot(&self, epoch: u64, checkout: &crate::CheckoutSnapshot,
        billing: &crate::BillingStatusSnapshot, catalog: &crate::CatalogSnapshot) -> Value {
        let state = self.state();
        let checkout_unavailable = catalog.state == "loaded" && catalog.catalog.as_ref().is_some_and(|value|
            value["storefront"]["state"] != "unavailable" &&
            value["channels"]["tbankPreview"] != true);
        let needs_recovery = checkout.pending || billing.status.as_ref().is_some_and(|value| value["subscription"]["state"] == "pending");
        if state.epoch != epoch {
            return json!({"epoch":epoch,"busy":false,"previewId":null,"preview":null,
                "message":null,"failed":false,"outcome":null,"needsRecovery":needs_recovery,"expiresAt":null,"canResume":false,"canCancel":false,
                "bindingUrl":null,"checkoutUrl":null,"checkoutUnavailable":checkout_unavailable});
        }
        let outcome = state.outcome.as_ref().unwrap_or(&Value::Null);
        let matches = checkout.pending && checkout.session_id.as_deref()
            .is_some_and(|id| outcome["checkoutSessionId"] == id);
        let can_resume = matches && outcome["clientCheckout"]["launchAction"] == "open_bank"
            && state.expires_at.is_none_or(|time| time > chrono::Utc::now());
        let url = if can_resume { &outcome["clientCheckout"]["resumeUrl"] } else { &Value::Null };
        let update = if outcome["clientCheckout"]["disposition"] == "scheduled" {
            &billing.checkout.scheduled
        } else { &billing.checkout.settled };
        let message = if !state.failed && state.preview.is_none() && state.outcome.is_some()
            && !checkout.pending && update.replace { &update.message } else { &state.message };
        json!({"epoch":epoch,"busy":state.active.is_some(),
            "previewId":state.preview.as_ref().map(|p| &p.id),
            "preview":state.preview.as_ref().map(|p| &p.wire),
            "message":message,"failed":state.failed,"outcome":state.outcome,"needsRecovery":needs_recovery || state.failed,"checkoutUnavailable":checkout_unavailable,
            "expiresAt":state.expires_at.filter(|_| matches).map(|value| value.to_rfc3339()),
            "canResume":can_resume,"canCancel":matches && outcome["clientCheckout"]["canCancel"] == true,
            "checkoutUrl":url,"bindingUrl":if outcome["paymentMethodKind"] == "sbp" { url } else { &Value::Null }})
    }
}
