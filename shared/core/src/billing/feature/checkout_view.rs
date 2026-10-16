//! The former macOS confirmation copy, using only an admitted core preview.
use super::{
    dto::BillingCheckout,
    format::{delay, money, Dates},
};
use serde_json::Value;

// Text and consent are preserved verbatim from SubscriptionCheckoutFormatter.
pub const RECEIPT_PURPOSE: &str = "Для чеков и обязательных уведомлений об оплате подписки.";
pub const RECURRING: &str =
    "Подписка продлевается автоматически. Автопродление можно отключить в любой момент.";
pub const CONSENT: &str = "Подтверждаю показанные условия подписки и автопродление. Автопродление можно отключить в любой момент. Разрешаю передать подтверждённый email ООО «КЛАУДКАССИР» и ОФД для отправки чеков, а Говоруну — присылать обязательные уведомления о состоянии оплаты.";
pub const SBP_HANDOFF: &str =
    "Подтвердите привязку счёта в приложении вашего банка. Данные карты вводить не нужно.";

pub fn checkout(
    window: &Value,
    catalog: &Value,
    dates: Dates,
) -> Result<Option<BillingCheckout>, &'static str> {
    let wire = &window["preview"];
    let Some(id) = window["previewId"].as_str() else {
        return Ok(None);
    };
    if !wire.is_object() {
        return Err("invalid_billing");
    }
    let Some(plan) = catalog["offers"].as_array().and_then(|offers| {
        offers
            .iter()
            .find(|offer| offer["offerId"] == wire["offerId"])
    }) else {
        return Ok(None);
    };
    let action = &wire["clientCheckoutPreview"];
    let action_enabled = action["canConfirm"].as_bool().ok_or("invalid_billing")?;
    let receipt_contact_needed = action["receiptContactRequired"].as_bool().ok_or("invalid_billing")?;
    let plan_name = plan["planName"].as_str().ok_or("invalid_billing")?;
    let plan_title = match plan["clientMultiplierCopy"].as_str() {
        Some(multiplier) => format!("{plan_name} {multiplier}"),
        None => plan_name.to_owned(),
    };
    let first_date = dates
        .date(wire["firstChargeAt"].as_str(), true, false, true)
        .unwrap_or_default();
    let first_moment = dates
        .date(wire["firstChargeAt"].as_str(), true, true, true)
        .unwrap_or_default();
    let renewal_date = dates
        .date(wire["renewalChargeAt"].as_str(), true, false, true)
        .unwrap_or_default();
    let period_end = dates.date(wire["servicePeriodEndsAt"].as_str(), true, false, true);
    let amount = |key: &str| {
        wire[key]
            .as_i64()
            .filter(|amount| *amount >= 0)
            .ok_or("invalid_billing")
    };
    let due = amount("amountDueNowKopeks")?;
    let due_copy = money(due);
    let first_copy = money(amount("firstChargeAmountKopeks")?);
    let renewal_copy = money(amount("renewalAmountKopeks")?);
    let period_days = wire["periodDays"].as_u64().ok_or("invalid_billing")?;
    let trial_days = wire["trialDays"].as_u64().ok_or("invalid_billing")?;
    let upgrade = wire["purchaseKind"] == "plan_upgrade";
    let trial_upgrade = wire["purchaseKind"] == "trial_plan_upgrade";
    let scheduled = wire["purchaseKind"] == "scheduled_offer_change";
    let binding_first = wire["freeAccessSource"] == "binding_first";
    let card_test = wire["freeAccessSource"] == "card_trial" && trial_days == 0;
    let sbp = wire["paymentMethodKind"] == "sbp";
    let headline = if upgrade {
        period_end
            .as_ref()
            .map(|date| format!("Про до {date}"))
            .unwrap_or("Переход на Про".into())
    } else if trial_upgrade {
        format!("Про бесплатно до {first_date}")
    } else if scheduled {
        period_end
            .map(|date| format!("Текущий вариант действует до {date}"))
            .unwrap_or("Смена при следующем продлении".into())
    } else {
        match wire["freeAccessSource"].as_str() {
            Some("binding_first") => "Привязка карты".into(),
            Some("promo") => format!("Промо действует до {first_date}"),
            Some("none") => "Без пробного периода".into(),
            Some("card_trial") if card_test => "Проверка карты без списания".into(),
            Some("card_trial") => format!("Первые {trial_days} дней бесплатно"),
            Some("current_period") => "До конца текущего периода".into(),
            _ => return Err("invalid_billing"),
        }
    };
    let first_charge = if scheduled {
        format!("Первое списание {first_date} — {first_copy}")
    } else if upgrade || due > 0 {
        format!("Следующее списание {renewal_date} — {renewal_copy}")
    } else if binding_first {
        format!(
            "Первое списание {first_copy} через {} после привязки карты",
            delay(
                wire["firstChargeDelaySeconds"]
                    .as_u64()
                    .ok_or("invalid_billing")?
            )
        )
    } else if card_test {
        format!("Первое списание {first_moment} — {first_copy}")
    } else {
        format!("Первое списание {first_date} — {first_copy}")
    };
    let mut action_title = if upgrade {
        format!("Доплатить {due_copy}")
    } else if trial_upgrade {
        "Перейти на Про".into()
    } else if scheduled {
        "Сменить тариф".into()
    } else if due > 0 {
        format!("Оплатить {due_copy}")
    } else {
        "Привязать карту".into()
    };
    if sbp && action_title == "Привязать карту" {
        action_title = "Подключить СБП".into();
    }
    let handoff = if sbp {
        SBP_HANDOFF.into()
    } else if upgrade {
        "Доплата будет списана с уже привязанной карты. Если банк потребует повторную проверку, мы отдельно откроем его защищённую страницу.".into()
    } else if due == 0 {
        String::new()
    } else {
        format!("Сначала привяжите карту за 0 ₽ в банке. Затем спишем {due_copy} за подписку.")
    };
    let methods = match &wire["availablePaymentMethods"] {
        Value::Null => vec![],
        Value::Array(methods) => methods
            .iter()
            .map(|value| value.as_str().map(str::to_owned).ok_or("invalid_billing"))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("invalid_billing"),
    };
    Ok(Some(BillingCheckout {
        id: id.into(),
        title: "Подключение подписки".into(),
        plan_title,
        message: window["message"].as_str().map(str::to_owned),
        payment_method_kind: wire["paymentMethodKind"].as_str().unwrap_or("card").into(),
        available_payment_methods: methods,
        trial_headline: headline,
        amount_due_now: if upgrade {
            format!("Доплата сейчас — {due_copy}")
        } else {
            format!("Сейчас — {due_copy}")
        },
        first_charge,
        renewal: format!("Далее {renewal_copy} / {period_days} дней"),
        handoff,
        email_masked: if wire["receiptContactState"] == "oauth_email_available" {
            wire["receiptEmailMasked"]
                .as_str()
                .map(|s| s.trim().to_owned())
        } else {
            None
        },
        email: wire["receiptEmail"].as_str().map(str::to_owned),
        email_required: receipt_contact_needed
            && wire["receiptContactState"] == "email_required",
        receipt_contact_needed,
        receipt_purpose: RECEIPT_PURPOSE.into(),
        action_title,
        recurring_disclosure: RECURRING.into(),
        consent: CONSENT.into(),
        action_enabled,
        has_legal_documents: wire["legalDocuments"].is_object(),
    }))
}
