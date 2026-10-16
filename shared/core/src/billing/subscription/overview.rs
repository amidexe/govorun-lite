//! A display-only subscription slice. Fresh server facts choose the meaning;
//! native bindings only format the admitted amount/date and render the copy.
use crate::{BillingStatusSnapshot, CatalogSnapshot, CheckoutSnapshot, Error, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
pub use super::overview_notice::RenewalNotice;

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverviewFacts {
    pub epoch: u64,
    pub signed_in: bool,
    pub status_fresh: bool,
    pub checkout_pending: bool,
    pub payment_updating: bool,
    pub status_updating: bool,
    pub subscription: Option<SubscriptionFacts>,
    pub catalog_trial_days: Option<u32>,
    pub now: String,
}

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionFacts {
    pub state: String,
    pub renews_automatically: bool,
    pub payment_method: Option<PaymentMethodFacts>,
    pub next_charge: Option<NextChargeFacts>,
    pub current_period_ends_at: Option<String>,
    pub renewal_issue: Option<RenewalIssueFacts>,
}

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PaymentMethodFacts { pub available: Option<bool> }

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct NextChargeFacts {
    pub amount_kopeks: Option<i64>,
    pub currency: Option<String>,
    pub charge_at: Option<String>,
    pub notice_at: Option<String>,
}

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RenewalIssueFacts {
    pub state: String,
    pub resolution_kind: Option<String>,
    pub payment_state: Option<String>,
    pub amount_kopeks: Option<i64>,
    pub currency: Option<String>,
}

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PaymentSummary {
    Message { message: &'static str },
    NextCharge { prefix: &'static str, amount_kopeks: i64, charge_at: String, suffix: &'static str },
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PlanActionKind { ChangePlan, TryTrial, Subscribe, ChoosePlan }

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PlanAction { pub kind: PlanActionKind, pub title: &'static str }

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionOverview {
    pub epoch: u64,
    pub confirmed_status: bool,
    pub payment_method_unavailable: bool,
    pub payment_summary: PaymentSummary,
    pub status_title: &'static str,
    pub plan_action: PlanAction,
    pub period_end: Option<String>,
    pub period_label: &'static str,
    pub renewal_notice: Option<RenewalNotice>,
}

impl SubscriptionOverview {
    pub fn project(facts: OverviewFacts) -> Result<Self> {
        let now = DateTime::parse_from_rfc3339(&facts.now).map_err(|_| Error::InvalidRequest)?.with_timezone(&Utc);
        let subscription = facts.subscription.as_ref();
        let confirmed = facts.signed_in && facts.status_fresh && !facts.checkout_pending && subscription.is_some();
        let unavailable = confirmed && subscription.and_then(|s| s.payment_method.as_ref())
            .is_some_and(|method| method.available == Some(false));
        let renewal = confirmed && subscription.is_some_and(|s| s.renews_automatically);
        let payment = if !confirmed {
            message(if facts.payment_updating { "Обновляем информацию о подписке…" }
                else { "Не удалось проверить списания. Обновите состояние подписки." })
        } else if unavailable {
            message("Списания приостановлены. Выберите другой способ оплаты.")
        } else if !renewal {
            message("Автопродление отключено. Списаний не будет.")
        } else {
            subscription.and_then(|s| s.next_charge.as_ref()).and_then(|next| {
                let amount = next.amount_kopeks.filter(|amount| *amount > 0)?;
                if next.currency.as_deref() != Some("RUB") { return None; }
                let date = DateTime::parse_from_rfc3339(next.charge_at.as_deref()?).ok()?.with_timezone(&Utc);
                (date > now).then_some(PaymentSummary::NextCharge {
                    prefix: "Автопродление включено. Следующее списание — ", amount_kopeks: amount,
                    charge_at: date.to_rfc3339_opts(SecondsFormat::AutoSi, true), suffix: ".",
                })
            }).unwrap_or_else(|| message("Автопродление включено. Дата следующего списания уточняется."))
        };
        let status_title = if !confirmed {
            if facts.status_updating { "Проверяем состояние подписки" } else { "Состояние списаний не проверено" }
        } else {
            match subscription.map(|s| s.state.as_str()) {
                Some("refunded") => "Подписка завершена после возврата",
                Some("pending") => "Запланирована",
                Some("trialing") => "Пробный период",
                Some("active") => "Подключена",
                Some("grace") => "Нужна оплата",
                _ => "Неактивна",
            }
        };
        let (kind, title) = if renewal { (PlanActionKind::ChangePlan, "Сменить тариф") }
            else if facts.catalog_trial_days.is_some_and(|days| days > 0) { (PlanActionKind::TryTrial, "Попробовать бесплатно") }
            else if facts.catalog_trial_days.is_some() { (PlanActionKind::Subscribe, "Оформить подписку") }
            else { (PlanActionKind::ChoosePlan, "Выбрать тариф") };
        let period_end = subscription.filter(|_| confirmed)
            .and_then(|value| super::overview_notice::period_end(value, now));
        let period_label = super::overview_notice::period_label(subscription.filter(|_| confirmed));
        let renewal_notice = subscription.filter(|_| confirmed && !unavailable)
            .and_then(|value| super::overview_notice::renewal_notice(value, now));
        Ok(Self { epoch: facts.epoch, confirmed_status: confirmed, payment_method_unavailable: unavailable,
            payment_summary: payment, status_title, plan_action: PlanAction { kind, title },
            period_end, period_label, renewal_notice })
    }

    pub(crate) fn from_snapshots(epoch: u64, signed_in: bool, billing: &BillingStatusSnapshot,
        checkout: &CheckoutSnapshot, catalog: &CatalogSnapshot, window_busy: bool) -> Self {
        let subscription = billing.status.as_ref().and_then(|status|
            serde_json::from_value(status["subscription"].clone()).ok());
        let trial_days = catalog.catalog.as_ref().filter(|value| catalog.state == "loaded" &&
            value["offers"].as_array().is_some_and(|offers| !offers.is_empty()))
            .and_then(|value| value["clientTrialDays"].as_u64()).and_then(|days| u32::try_from(days).ok());
        Self::project(OverviewFacts { epoch, signed_in, status_fresh: billing.fresh,
            checkout_pending: checkout.pending, payment_updating: billing.refreshing || checkout.checking,
            status_updating: billing.refreshing || window_busy, subscription, catalog_trial_days: trial_days,
            now: Utc::now().to_rfc3339() }).expect("current subscription overview time")
    }
}

fn message(message: &'static str) -> PaymentSummary { PaymentSummary::Message { message } }
