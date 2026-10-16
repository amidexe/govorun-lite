use super::session;
use crate::{HttpResponse, Result};
use serde_json::{json, Value};

/// Derive client decisions from an admitted server session. These fields are
/// never authority for payment or entitlement, and never accepted from wire.
pub(crate) fn projection(value: &Value) -> Result<Value> {
    let disposition = session::validate_recovery(value)?;
    let pending = disposition == "pending";
    let url = value["paymentUrl"].as_str().filter(|v| !v.is_empty());
    let resume = pending && value["cancellationPending"] != true
        && matches!(
            value["state"].as_str(),
            Some("add_card_pending" | "payment_pending")
        );
    let deadline = pending
        && value["cancellationPending"] != true
        && value["state"] != "payment_authorized"
        && (value["state"] != "payment_pending" || url.is_some());
    Ok(json!({
        "disposition": disposition,
        "canCancel": pending && session::can_cancel(value),
        "resumeUrl": if resume { url } else { None },
        "expiresInSeconds": if deadline { value["expiresInSeconds"].as_u64() } else { None },
        "message": message(value),
        "launchAction": if !pending { "complete" } else if resume && url.is_some() { "open_bank" } else { "reconcile" },
    }))
}

pub(super) fn response(path: &str, mut response: HttpResponse) -> Result<HttpResponse> {
    if path == "/v1/billing/tbank/checkout" && (200..300).contains(&response.status) {
        response.body["clientCheckout"] = projection(&response.body)?;
    }
    Ok(response)
}

fn message(value: &Value) -> String {
    let state = value["state"].as_str().unwrap_or_default();
    let sbp = value["paymentMethodKind"] == "sbp";
    let source = value["freeAccessSource"].as_str().unwrap_or_default();
    if value["cancellationPending"] == true {
        return "Проверяем отмену оформления… Статус обновится автоматически.".into();
    }
    if value["purchaseKind"] == "plan_upgrade" && state != "subscription_active" {
        return match state {
            "failed" => "Оплата смены тарифа не прошла. Текущий тариф сохранён.",
            "cancelled" => "Смена тарифа отменена. Текущий тариф сохранён.",
            "expired" => "Смена тарифа не завершена. Текущий тариф сохранён.",
            _ => "Проверяем оплату смены тарифа… Статус обновится автоматически.",
        }
        .into();
    }
    if state == "cancelled" {
        return "Попытка отменена.".into();
    }
    if matches!(state, "payment_pending" | "payment_authorized") {
        return if value["paymentUrl"].as_str().is_none_or(str::is_empty) {
            "Проверяем оплату… Статус обновится автоматически."
        } else {
            "Ожидаем результат оплаты. Можно вернуться на страницу банка и продолжить оплату."
        }
        .into();
    }
    if source == "none" && matches!(state, "failed" | "expired") {
        return "Оплата не прошла. Можно попробовать ещё раз или выбрать другой способ оплаты."
            .into();
    }
    let amount = value["priceKopeks"].as_u64().expect("admitted amount");
    let amount = if amount % 100 == 0 {
        format!("{} ₽", amount / 100)
    } else {
        format!("{},{:02} ₽", amount / 100, amount % 100)
    };
    if state == "add_card_pending" && source == "none" && !sbp {
        return format!(
            "Подтвердите привязку карты за 0 ₽ в банке. Затем спишем {amount} за подписку."
        );
    }
    let verified = if sbp {
        "Счёт СБП подтверждён."
    } else {
        "Карта подтверждена."
    };
    let plan = if value["planCode"] == "pro" {
        "Про"
    } else {
        "Базовая"
    };
    // Locale/timezone formatting stays in presentation adapters. The common
    // owner chooses the sentence and amount; the placeholders are not wire text.
    let first = format!("Первое списание {{firstChargeDate}} в {{firstChargeTime}} — {amount}.");
    match state {
        "trial_active" => format!("{verified} Пробный период «{plan}» начался. {first}"),
        "subscription_scheduled" if source == "binding_first" => format!("{verified} Сейчас — 0 ₽. {first}"),
        "subscription_scheduled" => format!("{verified} Подписка начнётся после текущего бесплатного доступа. {first}"),
        "subscription_active" => format!("Подписка активна. Тариф «{plan}»."),
        "expired" => "Подключение не завершено. Списание не назначено. Можно подключить подписку заново.".into(),
        "failed" if sbp => "СБП не подключён. Попробуйте ещё раз.".into(),
        "failed" => "Карта не подтверждена. Деньги не списаны. Попробуйте ещё раз.".into(),
        _ if sbp => "Подтвердите подключение СБП в приложении банка. Статус обновится автоматически.".into(),
        _ => "Подтвердите карту на странице банка. После подтверждения статус обновится автоматически.".into(),
    }
}
