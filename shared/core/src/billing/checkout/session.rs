use crate::{Error, Result};
use chrono::DateTime;
use serde_json::Value;

/// Admission of a creation result against the exact disclosure the user confirmed.
pub(super) fn matches_confirmation(value: &Value, preview: &Value) -> Result<()> {
    if super::plan_change::mode(preview).is_some() {
        return super::plan_change::validate(value, preview, false);
    }
    for key in [
        "catalogVersion",
        "offerId",
        "planCode",
        "purchaseKind",
        "trialDays",
        "freeAccessSource",
        "priceKopeks",
        "periodDays",
        "currency",
    ] {
        require(value[key] == preview[key])?;
    }
    require(
        value["paymentMethodKind"].as_str().unwrap_or("card")
            == preview["paymentMethodKind"].as_str().unwrap_or("card"),
    )?;
    require(
        crate::promotions::valid_preview_id(&value["checkoutSessionId"])
            && value["browserReturnGrantsAccess"] == false
            && value["serverConfirmationRequired"] == true,
    )?;
    let state = value["state"].as_str().ok_or(Error::InvalidResponse)?;
    require(matches!(
        state,
        "created"
            | "add_card_pending"
            | "payment_pending"
            | "payment_authorized"
            | "trial_active"
            | "subscription_active"
            | "subscription_scheduled"
            | "cancelled"
            | "failed"
            | "expired"
    ))?;
    let first = date(&value["firstChargeAt"])?;
    require(first == date(&value["freeAccessEndsAt"])?)?;
    if preview["freeAccessSource"] == "binding_first" {
        require(value["firstChargeDelaySeconds"] == preview["firstChargeDelaySeconds"])?;
        require(
            first.is_some()
                || matches!(
                    state,
                    "created" | "add_card_pending" | "cancelled" | "failed" | "expired"
                ),
        )?;
    } else {
        require(value["firstChargeDelaySeconds"].is_null())?;
        let first = first.ok_or(Error::InvalidResponse)?;
        let expected = date(&preview["firstChargeAt"])?.ok_or(Error::InvalidResponse)?;
        require(
            if matches!(
                preview["freeAccessSource"].as_str(),
                Some("promo" | "current_period")
            ) {
                first == expected
            } else {
                (0..=31 * 60 * 1000).contains(&(first - expected))
            },
        )?;
    }
    if let Some(url) = value["paymentUrl"].as_str().filter(|s| !s.is_empty()) {
        let parsed = url::Url::parse(url).map_err(|_| Error::InvalidResponse)?;
        require(
            matches!(state, "add_card_pending" | "payment_pending")
                && parsed.scheme() == "https"
                && parsed.host_str().is_some()
                && parsed.username().is_empty()
                && parsed.password().is_none(),
        )?;
    } else {
        require(value["paymentUrl"].is_null() || value["paymentUrl"] == "")?;
    }
    Ok(())
}

/// Admit server-discovered status without reconstructing a purchase request.
pub(super) fn validate_recovery(value: &Value) -> Result<&'static str> {
    use crate::catalog::identifier;
    let integer = |key: &str, min, max| {
        value[key]
            .as_u64()
            .is_some_and(|n| (min..=max).contains(&n))
    };
    require(
        identifier(&value["catalogVersion"], 1, false)
            && identifier(&value["offerId"], 3, true)
            && matches!(value["planCode"].as_str(), Some("base" | "pro"))
            && integer("priceKopeks", 1, 100_000_000)
            && integer("periodDays", 1, 366)
            && value["currency"] == "RUB"
            && matches!(
                value["paymentMethodKind"].as_str().unwrap_or("card"),
                "card" | "sbp"
            ),
    )?;
    let source = value["freeAccessSource"]
        .as_str()
        .ok_or(Error::InvalidResponse)?;
    require(match value["purchaseKind"].as_str() {
        Some("plan_upgrade") => {
            value["planCode"] == "pro" && source == "none" && value["trialDays"] == 0
        }
        Some("subscription_start") => match source {
            "card_trial" => integer("trialDays", 0, 366),
            "promo" | "none" | "current_period" | "binding_first" => value["trialDays"] == 0,
            _ => false,
        },
        _ => false,
    })?;
    matches_confirmation(value, value)?;
    let state = value["state"].as_str().ok_or(Error::InvalidResponse)?;
    if source == "binding_first" {
        let first = date(&value["firstChargeAt"])?;
        if value["firstChargeDelaySeconds"].is_null() {
            require(
                first.is_some_and(|v| v > 0)
                    && matches!(
                        state,
                        "subscription_scheduled"
                            | "subscription_active"
                            | "cancelled"
                            | "failed"
                            | "expired"
                    ),
            )?;
        } else {
            require(
                integer("firstChargeDelaySeconds", 60, 86_400)
                    && value["firstChargeDelaySeconds"]
                        .as_u64()
                        .is_some_and(|v| v % 60 == 0),
            )?;
            if let Some(first) = first {
                require(
                    first > 0
                        && matches!(
                            state,
                            "subscription_scheduled"
                                | "subscription_active"
                                | "payment_pending"
                                | "payment_authorized"
                                | "cancelled"
                                | "failed"
                                | "expired"
                        ),
                )?;
            }
        }
    }
    if value["paymentMethodKind"] == "sbp" {
        if let Some(raw) = value["paymentUrl"].as_str().filter(|s| !s.is_empty()) {
            let url = url::Url::parse(raw).map_err(|_| Error::InvalidResponse)?;
            require(
                raw.len() <= 2048
                    && raw.starts_with("https://sub.nspk.ru/")
                    && url.fragment().is_none()
                    && url.path().len() > 1
                    && match url.query() {
                        None => {
                            url.path().len() == 33
                                && url.path()[1..].bytes().all(|b| b.is_ascii_alphanumeric())
                        }
                        Some(query) => query.split('&').filter(|v| *v == "type=03").count() == 1,
                    },
            )?;
        }
    }
    for key in ["cancellationPending", "canRestartBinding"] {
        require(value.get(key).is_none_or(|v| v.is_null() || v.is_boolean()))?;
    }
    require(
        value
            .get("expiresInSeconds")
            .is_none_or(|v| v.is_null() || v.as_u64().is_some_and(|n| n <= 86_400)),
    )?;
    disposition(value)
}

pub(super) fn disposition(value: &Value) -> Result<&'static str> {
    match value["state"].as_str() {
        Some("created" | "add_card_pending" | "payment_pending" | "payment_authorized") => {
            Ok("pending")
        }
        Some("trial_active" | "subscription_active") => Ok("connected"),
        Some("subscription_scheduled") => Ok("scheduled"),
        Some("cancelled" | "failed" | "expired") => Ok("terminal"),
        _ => Err(Error::InvalidResponse),
    }
}

pub(super) fn can_cancel(value: &Value) -> bool {
    value["canRestartBinding"] == true
        && value["cancellationPending"] != true
        && value["purchaseKind"] == "subscription_start"
        && matches!(
            value["state"].as_str(),
            Some("add_card_pending" | "payment_pending")
        )
}

fn require(value: bool) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::InvalidResponse)
    }
}
fn date(value: &Value) -> Result<Option<i64>> {
    if value.is_null() {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(value.as_str().ok_or(Error::InvalidResponse)?)
        .map(|v| Some(v.timestamp_millis()))
        .map_err(|_| Error::InvalidResponse)
}
