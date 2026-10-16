use super::{BillingStatusService, BillingStatusSnapshot, BillingStatusSource};
use super::refund::{self, BillingRefundReason};
use crate::{AccountRequest, AccountSession, CheckoutHost, CheckoutService, Error, Method, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BillingManagementCommand {
    CancelRenewal, RemoveCard, RequestRefund(BillingRefundReason), CancelScheduledPlanChange,
}

impl BillingManagementCommand {
    pub fn owns(request: &AccountRequest) -> bool {
        request.method() == Method::Post && matches!(request.path(),
            "/v1/billing/cancel-renewal" | "/v1/billing/payment-method/remove" |
            "/v1/billing/refund-requests" | "/v1/billing/change")
    }
    fn path(self) -> &'static str {
        match self {
            Self::CancelRenewal => "/v1/billing/cancel-renewal",
            Self::RemoveCard => "/v1/billing/payment-method/remove",
            Self::RequestRefund(_) => "/v1/billing/refund-requests",
            Self::CancelScheduledPlanChange => "/v1/billing/change",
        }
    }
    fn body(self, subscription: &Value) -> Result<Value> {
        Ok(match self {
            Self::RequestRefund(reason) => json!({"reasonCode":reason}),
            Self::CancelScheduledPlanChange => {
                if !crate::catalog::identifier(&subscription["offerId"], 3, true) ||
                    !crate::catalog::identifier(&subscription["pendingOfferId"], 3, true) {
                    return Err(Error::InvalidRequest);
                }
                json!({"targetOfferId":subscription["offerId"]})
            }
            _ => json!({}),
        })
    }
    fn checking(self) -> &'static str {
        match self {
            Self::CancelRenewal => "Проверяем отключение автопродления. Обновите статус подписки",
            Self::RemoveCard => "Проверяем отвязку карты. Обновите статус подписки",
            Self::RequestRefund(_) => "Проверяем получение запроса на возврат. Обновите статус подписки",
            Self::CancelScheduledPlanChange => "Проверяем отмену смены тарифа. Обновите статус подписки",
        }
    }
}

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BillingManagementSnapshot {
    pub command: BillingManagementCommand,
    pub running: bool,
    pub needs_check: bool,
    pub message: Option<String>,
    pub acknowledged: bool,
    #[serde(skip)]
    target_offer_id: Option<String>,
}

impl BillingManagementSnapshot {
    fn checking(command: BillingManagementCommand) -> Self {
        Self { command, running: false, needs_check: true, message: Some(command.checking().into()), acknowledged: false,
            target_offer_id: None }
    }
    fn refund_acknowledged(command: BillingManagementCommand) -> Self {
        Self { command, running: false, needs_check: false, message: Some(refund::ACKNOWLEDGED.into()), acknowledged: true,
            target_offer_id: None }
    }
    fn resolved(previous: &Self, status: &Value) -> Self {
        let command = previous.command;
        let subscription = &status["subscription"];
        let stopped = subscription["renewsAutomatically"] == false;
        let (needs_check, message) = match command {
            BillingManagementCommand::CancelRenewal if stopped =>
                (false, "Автопродление отключено. Доступ сохранён до конца периода."),
            BillingManagementCommand::CancelRenewal =>
                (false, "Автопродление остаётся включённым. Проверьте статус подписки перед повтором."),
            BillingManagementCommand::RemoveCard => match subscription["paymentMethod"]["state"].as_str() {
                Some("none") if stopped => (false,
                    "Карта отвязана. Новых списаний не будет. Оплаченный доступ сохранён до конца периода."),
                Some("removal_pending") => (true, "Отвязываем карту. Проверьте состояние позже."),
                Some("active") => (false, "Карта пока привязана. Проверьте состояние ещё раз."),
                _ => (true, "Результат отвязки пока не подтверждён. Проверьте состояние карты."),
            },
            BillingManagementCommand::RequestRefund(_) => {
                if refund::validate(&subscription["refundRequest"]).is_ok() {
                    return Self::refund_acknowledged(command);
                }
                (false, "Запрос на возврат пока не подтверждён. При необходимости обратитесь в поддержку.")
            }
            BillingManagementCommand::CancelScheduledPlanChange => {
                let pending = &subscription["pendingOfferId"];
                if !crate::catalog::identifier(&subscription["offerId"], 3, true) {
                    (true, command.checking())
                } else if subscription["offerId"].as_str() != previous.target_offer_id.as_deref() {
                    (false, "Тариф изменился. Проверьте подписку перед повтором.")
                } else if pending.is_null() || pending.as_str() == Some("") {
                    (false, "Смена тарифа отменена. Текущий тариф сохранён.")
                } else if crate::catalog::identifier(pending, 3, true) {
                    (false, "Смена тарифа всё ещё запланирована. Проверьте статус перед повтором.")
                } else { (true, command.checking()) }
            }
        };
        Self { command, running: false, needs_check, message: Some(message.into()), acknowledged: false,
            target_offer_id: previous.target_offer_id.clone() }
    }
    fn failed(command: BillingManagementCommand, error: Error) -> Self {
        let rejected = matches!(error, Error::Http(status) if (400..500).contains(&status) && !matches!(status, 408 | 429));
        if !rejected { return Self::checking(command); }
        Self { command, running: false, needs_check: false, acknowledged: false, target_offer_id: None, message: Some(match error {
            Error::Http(401) => "Войдите в аккаунт Говоруна",
            _ => "Не удалось выполнить действие. Обновите статус и попробуйте ещё раз",
        }.into()) }
    }
}

impl BillingStatusSnapshot {
    pub(super) fn acknowledge_refund(&mut self) {
        if let Some(value) = &mut self.management {
            if matches!(value.command, BillingManagementCommand::RequestRefund(_)) { value.acknowledged = true; }
        }
    }
    pub(super) fn reconcile_management(&mut self) {
        if let Some(current) = &self.management {
            if current.needs_check && !current.running {
                if let Some(status) = &self.status {
                    let resolved = BillingManagementSnapshot::resolved(current, status);
                    self.message = resolved.message.clone();
                    self.management = Some(resolved);
                }
            }
        }
    }
}


// Check before any discovery I/O, then check again under the state lock before
// committing a command: discovery may refresh or invalidate the billing status.
fn management_body(state: &super::State, epoch: u64, command: BillingManagementCommand) -> Result<Value> {
    if state.snapshot.epoch != epoch { return Err(Error::SessionChanged); }
    if !state.snapshot.fresh || state.management_id.is_some() || !state.writes.is_empty() ||
        state.snapshot.management.as_ref().is_some_and(|m| m.needs_check) {
        return Err(Error::InvalidRequest);
    }
    let subscription = &state.snapshot.status.as_ref().ok_or(Error::InvalidRequest)?["subscription"];
    if command == BillingManagementCommand::RemoveCard &&
        (subscription["paymentMethod"]["state"] != "active" || subscription["paymentMethod"]["canRemove"] != true) {
        return Err(Error::InvalidRequest);
    }
    command.body(subscription)
}

impl BillingStatusService {
    /// The one command owner: confirmation, checkout exclusion, admission,
    /// dispatch, uncertainty and user-visible result. Native callers apply its
    /// view; expected HTTP/network failures are represented there.
    pub async fn manage<H: BillingStatusSource + CheckoutHost>(&self, source: &H,
        checkout: &CheckoutService, epoch: u64, command: BillingManagementCommand,
        confirmed: bool) -> Result<()> {
        if !confirmed { return Err(Error::ConsentUnconfirmed); }
        BillingStatusSource::current(source, epoch)?;
        management_body(&self.state.lock().expect("billing status lock"), epoch, command)?;
        let _checkout = checkout.management_scope(source, epoch).await?;
        let (id, snapshot, body) = {
            let mut state = self.state.lock().expect("billing status lock");
            let body = management_body(&state, epoch, command)?;
            state.sequence += 1;
            let id = state.sequence;
            state.management_id = Some(id);
            state.snapshot.management = Some(BillingManagementSnapshot {
                command, running: true, needs_check: false, message: None, acknowledged: false,
                target_offer_id: body["targetOfferId"].as_str().map(str::to_owned),
            });
            state.snapshot.message = None;
            (id, state.changed(), body)
        };
        let mut pending = ManagementPending { service: self, source, epoch, id, command, finished: false };
        source.billing_changed(&snapshot)?;
        let response = self.request(source, epoch, AccountRequest::new(Method::Post,
            command.path(), Some(body))?).await.and_then(|r| r.success());
        BillingStatusSource::current(source, epoch)?;
        let result = match response {
            Ok(_) => {
                let current = self.snapshot(epoch);
                if current.management.as_ref().is_some_and(|m| m.acknowledged) {
                    BillingManagementSnapshot::refund_acknowledged(command)
                } else if current.fresh {
                    BillingManagementSnapshot::resolved(current.management.as_ref().ok_or(Error::InvalidResponse)?,
                        current.status.as_ref().ok_or(Error::InvalidResponse)?)
                } else { BillingManagementSnapshot::checking(command) }
            }
            Err(Error::SessionChanged | Error::SignedOut) => return Err(Error::SessionChanged),
            Err(error) => BillingManagementSnapshot::failed(command, error),
        };
        let snapshot = self.finish_management(epoch, id, result).ok_or(Error::SessionChanged)?;
        pending.finished = true;
        source.billing_changed(&snapshot)
    }

    fn finish_management(&self, epoch: u64, id: u64, mut result: BillingManagementSnapshot) -> Option<BillingStatusSnapshot> {
        let mut state = self.state.lock().expect("billing status lock");
        if state.snapshot.epoch != epoch || state.management_id != Some(id) { return None; }
        result.target_offer_id = state.snapshot.management.as_ref().and_then(|m| m.target_offer_id.clone());
        state.management_id = None;
        state.snapshot.message = result.message.clone();
        state.snapshot.management = Some(result);
        Some(state.changed())
    }
}

struct ManagementPending<'a, H: BillingStatusSource> {
    service: &'a BillingStatusService,
    source: &'a H,
    epoch: u64,
    id: u64,
    command: BillingManagementCommand,
    finished: bool,
}
impl<H: BillingStatusSource> Drop for ManagementPending<'_, H> {
    fn drop(&mut self) {
        if !self.finished {
            let acknowledged = self.service.snapshot(self.epoch).management.as_ref().is_some_and(|m| m.acknowledged);
            if let Some(snapshot) = self.service.finish_management(self.epoch, self.id,
                if acknowledged { BillingManagementSnapshot::refund_acknowledged(self.command) }
                else { BillingManagementSnapshot::checking(self.command) }) {
                if self.source.current(self.epoch).is_ok() { let _ = self.source.billing_changed(&snapshot); }
            }
        }
    }
}

impl AccountSession {
    pub async fn manage_billing(&self, epoch: u64, command: BillingManagementCommand, confirmed: bool) -> Result<()> {
        self.inner.billing.manage(self, &self.inner.checkout, epoch, command, confirmed).await
    }
}
/// Show payment management only for a current subscription or a linked method.
pub(crate) fn visible(subscription: &serde_json::Value) -> bool {
    subscription["renewsAutomatically"] == true ||
        matches!(subscription["state"].as_str(), Some("trialing" | "active" | "grace")) ||
        matches!(subscription["paymentMethod"]["state"].as_str(), Some("active" | "removal_pending"))
}
