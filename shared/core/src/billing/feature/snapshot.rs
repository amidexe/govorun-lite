//! Format an admitted core view. No transport, charge selection or storage.
use super::{
    checkout_view,
    dto::*,
    format::{money, Dates},
};
use serde_json::{json, Value};

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn optional(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn payment(summary: &Value, dates: Dates) -> Result<String, &'static str> {
    match text(&summary["kind"]) {
        "message" => Ok(text(&summary["message"]).into()),
        "nextCharge" => {
            let amount = summary["amountKopeks"]
                .as_i64()
                .filter(|v| *v > 0)
                .ok_or("invalid_billing")?;
            let date = dates
                .date(summary["chargeAt"].as_str(), true, true, false)
                .ok_or("invalid_billing")?;
            let amount = money(amount);
            let amount = amount
                .strip_suffix("0 ₽")
                .filter(|_| amount.contains(','))
                .map(|prefix| format!("{prefix} ₽"))
                .unwrap_or(amount);
            Ok(format!(
                "{}{amount}, {date}{}",
                text(&summary["prefix"]),
                text(&summary["suffix"])
            ))
        }
        _ => Err("invalid_billing"),
    }
}

fn renewal(notice: &Value) -> Result<Option<String>, &'static str> {
    if notice.is_null() {
        return Ok(None);
    }
    match text(&notice["kind"]) {
        "message" => Ok(optional(&notice["message"])),
        "upcoming" => {
            let amount = notice["amountKopeks"]
                .as_i64()
                .filter(|v| *v > 0)
                .ok_or("invalid_billing")?;
            Ok(Some(format!(
                "{}{}{}",
                text(&notice["prefix"]),
                money(amount),
                text(&notice["suffix"])
            )))
        }
        _ => Err("invalid_billing"),
    }
}

fn card_last_four(method: &Value) -> Option<&str> {
    method["lastFour"].as_str().filter(|s| s.len() == 4 && s.bytes().all(|c| c.is_ascii_digit()))
}

fn card(method: &Value) -> Option<String> {
    if !method.is_object() {
        return None;
    }
    Some(match text(&method["state"]) {
        "none" => "Карта не привязана".into(),
        "active" if method["available"] == false => {
            "Способ оплаты временно недоступен. Новых списаний не будет — выберите другой способ."
                .into()
        }
        "active" if method["kind"] == "sbp" => "Счёт СБП подключён".into(),
        "active" => match card_last_four(method)
        {
            Some(last) => format!("Карта •••• {last}"),
            None => "Карта привязана".into(),
        },
        "removal_pending" => "Отвязываем карту".into(),
        _ => "Проверяем состояние карты".into(),
    })
}

/// Android's existing card copy; the same card formatter owns masking on every host.
pub(crate) fn payment_method_view(method: &Value, status_available: bool) -> Value {
    let last_four = card_last_four(method);
    let known = matches!(method["state"].as_str(), Some("active" | "none"));
    let message = if !status_available || (!known && method["state"] != "removal_pending") {
        "Проверяем способ оплаты…".into()
    } else { card(method).unwrap_or_else(|| "Проверяем способ оплаты…".into()) };
    json!({"lastFour":last_four, "text":message, "needsRefresh":!status_available || !known})
}

fn balances(quota: &Value, dates: Dates) -> (Vec<BillingBalance>, Option<String>) {
    let balance = |title: &str, key: &str, detail: Option<String>| {
        let percent = quota[key].as_u64();
        BillingBalance {
            title: title.into(),
            value: percent
                .map(|n| format!("Осталось {n}%"))
                .unwrap_or("Обновляем остаток".into()),
            detail,
            percent,
        }
    };
    let status = &quota["periodStatus"];
    let prefix = text(&status["prefix"]);
    let detail = if prefix.is_empty() {
        None
    } else if status["timestamp"].is_null() {
        Some(prefix.into())
    } else {
        dates
            .date(
                status["timestamp"].as_str(),
                status["includeTime"] != true,
                status["includeTime"] == true,
                false,
            )
            .map(|date| format!("{prefix}{date}"))
    };
    let items = match text(&quota["kind"]) {
        "singleGrant" => vec![balance("Бесплатный лимит", "periodPercent", detail)],
        "singleBonus" => vec![balance("Бонусный лимит", "periodPercent", detail)],
        "period" => vec![
            balance(
                "Сегодня",
                "todayPercent",
                dates
                    .date(quota["todayReset"].as_str(), false, true, false)
                    .map(|date| format!("Обновится {date}")),
            ),
            balance("Общий лимит", "periodPercent", detail),
        ],
        _ => vec![],
    };
    let period = dates
        .date(quota["periodStart"].as_str(), true, false, false)
        .zip(dates.date(quota["periodEnd"].as_str(), true, false, false))
        .map(|(start, end)| format!("Период: {start} — {end}"));
    (items, period)
}

pub fn snapshot(
    projection: &Value,
    style_window: &Value,
    scope: String,
    terms_version: u32,
    message: Option<String>,
    dates: Dates,
) -> Result<BillingSnapshot, &'static str> {
    let account = &projection["snapshot"];
    let signed_in = account["signedIn"] == true;
    let access = &account["access"];
    let profile = &projection["profile"];
    let billing = &projection["billingStatus"];
    let subscription = &billing["status"]["subscription"];
    let window = &projection["subscriptionWindow"];
    let pending = signed_in && projection["checkout"]["pending"] == true;
    let confirmed = signed_in && billing["fresh"] == true && !pending;
    let busy = window["busy"] == true;
    let overview = &projection["subscriptionOverview"];
    let period_end = match &overview["periodEnd"] {
        Value::Null => None,
        Value::String(value) => Some(dates.date(Some(value), true, false, true)
            .ok_or("invalid_billing")?),
        _ => return Err("invalid_billing"),
    };
    let catalog_state = text(&projection["billingCatalog"]["state"]);
    let catalog = &projection["billingCatalog"]["catalog"];
    let offers = catalog["offers"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let trial = catalog["clientTrialDays"].as_u64().unwrap_or(0);
    let plans = offers
        .iter()
        .map(|offer| -> Result<BillingPlan, &'static str> {
            let amount = offer["priceKopeks"]
                .as_i64()
                .filter(|v| *v > 0)
                .ok_or("invalid_billing")?;
            Ok(BillingPlan {
                id: text(&offer["offerId"]).into(),
                title: text(&offer["planName"]).into(),
                multiplier: optional(&offer["clientMultiplierCopy"]),
                price: money(amount),
                period: format!("за {}", text(&offer["billingPeriodLabel"])),
                limit: text(&offer["clientLimitCopy"]).into(),
                trial: if trial > 0 {
                    format!("{trial} дней бесплатно")
                } else {
                    String::new()
                },
                current: confirmed
                    && subscription["renewsAutomatically"] == true
                    && subscription["paymentMethod"]["available"] != false
                    && matches!(text(&subscription["state"]), "active" | "trialing")
                    && subscription["offerId"] == offer["offerId"],
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (balances, period) = balances(&style_window["quota"], dates);
    let actions = account["sessionGeneration"].as_u64().filter(|_| signed_in)
        .map(|epoch| govorun_core::BillingActions::from_projection(projection, epoch))
        .unwrap_or_default();
    let has_access = access["displayEntitlementActive"] == true && style_window["starterLocallyExhausted"] != true;
    let payment_details = !pending && crate::billing_status::management_visible(subscription);
    Ok(BillingSnapshot {
        scope, signed_in, busy, message:message.or(dates.checkout_message(window)?).or_else(||optional(&billing["message"])),
        plan_title:style_window["quota"]["planTitle"].as_str().unwrap_or("Доступ").into(),
        has_access,
        show_plan_intro:!has_access && !pending && profile["promoProgram"]["clientVisible"] != true,
        show_plans:true,
        show_payment_details:payment_details,
        access_type:profile["styleAccess"]["type"].as_str().unwrap_or("none").into(),
        promo_program:program(&profile["promoProgram"])?,
        storefront:BillingStorefront { state:catalog["storefront"]["state"].as_str().unwrap_or("open").into(), message:text(&catalog["storefront"]["message"]).into() },
        refresh_after_seconds:catalog["refreshAfterSeconds"].as_u64().unwrap_or(60), balances, period,
        can_activate_starter:access["freeAccessAvailable"] == true, terms_version,
        status:BillingStatus {
            state:if confirmed {subscription["state"].as_str().unwrap_or("unknown")} else {"unknown"}.into(), fresh:confirmed,
            loading:billing["refreshing"] == true || busy, payment:payment(&overview["paymentSummary"],dates)?, title:text(&overview["statusTitle"]).into(),
            period_end, renewal_notice:renewal(&overview["renewalNotice"] )?,
            checkout_expires_at:optional(&window["expiresAt"]), card:if !confirmed {None} else if billing["management"]["needsCheck"] == true {Some("Проверяем состояние карты".into())} else {card(&subscription["paymentMethod"])},
            binding_url:optional(&window["bindingUrl"]), payment_method_unavailable:overview["paymentMethodUnavailable"] == true,
            trial_active:confirmed && profile["styleAccess"]["state"] == "active" && profile["styleAccess"]["type"] == "trial",
            pending_checkout:pending, can_resume_checkout:window["canResume"] == true, can_restart_binding:window["canCancel"] == true,
            can_cancel_renewal:actions.cancel_renewal, can_cancel_change:actions.cancel_plan_change, can_remove_card:actions.remove_card,
        },
        catalog_state:match catalog_state {"idle"|"loading"=>"loading", "loaded" if catalog.is_object()=>"loaded", _=>"error"}.into(),
        plan_action:text(&overview["planAction"]["title"]).into(),
        plan_disclosure:(!plans.is_empty()).then(|| if trial > 0 {format!("Первое списание — через {trial} дней, затем — автопродление. Отменить подписку можно в любое время. Если отменить до первого списания, оплаты не будет.")} else {"Отменить подписку можно в любое время.".into()}),
        can_preview_checkout:signed_in && !pending && !plans.is_empty() && catalog["channels"]["tbankPreview"] == true,
        plans, checkout:checkout_view::checkout(window,catalog,dates)?, promo:None, promo_access:None,
    })
}

fn program(value: &Value) -> Result<Option<PromoProgram>, &'static str> {
    if !value.is_object() {
        return Ok(None);
    }
    serde_json::from_value(
        serde_json::json!({"state":value["state"],"visible":value["clientVisible"] == true,
        "canActivate":value["clientCanActivate"] == true,"programVersion":text(&value["programVersion"]),
        "participated":value["participated"] == true,"enrollmentOpen":value["enrollmentOpen"] == true,
        "accessMode":value["accessMode"].as_str().unwrap_or("common_end"),"accessDays":value["accessDays"].as_u64().unwrap_or(0),
        "endsAt":value.get("endsAt").filter(|v| !v.is_null()).unwrap_or(&value["accessEndsAt"]),
        "dailyLimit":value["dailyLimit"].as_u64().unwrap_or(0),"periodLimit":value["periodLimit"].as_u64().unwrap_or(0),
        "periodDays":value["periodDays"].as_u64().unwrap_or(0),"message":text(&value["message"])}),
    ).map(Some).map_err(|_| "invalid_billing")
}
