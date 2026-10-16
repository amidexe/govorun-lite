//! Display decisions use admitted billing facts. Bindings format only the slots.
use super::overview::SubscriptionFacts;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RenewalNotice {
    Message { message: &'static str },
    Upcoming { prefix: &'static str, amount_kopeks: i64, suffix: &'static str },
}

pub(super) fn period_end(subscription: &SubscriptionFacts, now: DateTime<Utc>) -> Option<String> {
    if !matches!(subscription.state.as_str(), "active" | "trialing" | "grace" | "cancelled") {
        return None;
    }
    let end = timestamp(subscription.current_period_ends_at.as_deref())?;
    (end > now).then(|| end.to_rfc3339_opts(SecondsFormat::AutoSi, true))
}

pub(super) fn period_label(subscription: Option<&SubscriptionFacts>) -> &'static str {
    match subscription.map(|value| value.state.as_str()) {
        Some("pending") => "Первое списание",
        Some("trialing") => "Пробный доступ до",
        _ => "Доступ до",
    }
}

pub(super) fn renewal_notice(subscription: &SubscriptionFacts, now: DateTime<Utc>) -> Option<RenewalNotice> {
    if let Some(issue) = &subscription.renewal_issue {
        // A declared retry is the server decision; an optional retry timestamp
        // must not cause two platforms to promise different outcomes.
        if issue.state == "retry_scheduled" {
            return message("Оплата не прошла. Деньги не списаны. Повторим оплату автоматически.");
        }
        if issue.payment_state.as_deref() == Some("not_charged") &&
            issue.currency.as_deref() == Some("RUB") && issue.amount_kopeks.is_some_and(|amount| amount > 0) {
            return message(if matches!(issue.state.as_str(), "grace_ending" | "stopped") &&
                issue.resolution_kind.as_deref() == Some("customer_action_required") {
                "Оплата не прошла. Деньги не списаны. Откройте управление подпиской."
            } else { "Оплата не прошла. Деньги не списаны. Мы уже проверяем проблему." });
        }
        return match issue.state.as_str() {
            "grace_ending" => message("Оплата не прошла. Деньги не списаны. Мы уже проверяем проблему."),
            "stopped" => message("Подписка остановлена. Возобновить её можно вручную."),
            // An unknown issue cannot be presented as an upcoming successful renewal.
            _ => None,
        };
    }
    if !subscription.renews_automatically { return None; }
    let next = subscription.next_charge.as_ref()?;
    let amount = next.amount_kopeks.filter(|amount| *amount > 0)?;
    if next.currency.as_deref() != Some("RUB") { return None; }
    let notice_at = timestamp(next.notice_at.as_deref())?;
    let charge_at = timestamp(next.charge_at.as_deref())?;
    (now >= notice_at && now < charge_at).then_some(RenewalNotice::Upcoming {
        prefix: "Завтра продлится подписка — ", amount_kopeks: amount, suffix: ".",
    })
}

fn timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value?).ok().map(|date| date.with_timezone(&Utc))
}

fn message(message: &'static str) -> Option<RenewalNotice> { Some(RenewalNotice::Message { message }) }
