use crate::{catalog, Error, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

fn require(value: bool) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::InvalidResponse)
    }
}

fn bounded(value: &Value, minimum: u64, maximum: u64) -> bool {
    value
        .as_u64()
        .is_some_and(|value| (minimum..=maximum).contains(&value))
}

fn token(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|s| (32..=2048).contains(&s.len()))
}

fn empty(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(|s| s.is_empty())
}

fn date(value: &Value) -> Result<Option<DateTime<Utc>>> {
    if value.is_null() {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(value.as_str().ok_or(Error::InvalidResponse)?)
        .map(|date| Some(date.with_timezone(&Utc)))
        .map_err(|_| Error::InvalidResponse)
}

/// Admits the server's disclosure before any client can ask for confirmation.
/// A catalog is required at the preparation boundary; value-only DTO decoders
/// may reuse the wire checks without owning a second pricing policy.
pub fn validate_preview(value: &Value, selected_catalog: Option<(&Value, &str)>) -> Result<()> {
    require(
        catalog::identifier(&value["catalogVersion"], 1, false)
            && catalog::identifier(&value["offerId"], 3, true)
            && matches!(value["planCode"].as_str(), Some("base" | "pro"))
            && bounded(&value["priceKopeks"], 1, 100_000_000)
            && bounded(&value["periodDays"], 1, 366)
            && value["currency"] == "RUB"
            && value["serverConfirmationRequired"] == true
            && bounded(&value["amountDueNowKopeks"], 0, 100_000_000)
            && bounded(&value["firstChargeAmountKopeks"], 1, 100_000_000)
            && bounded(&value["renewalAmountKopeks"], 1, 100_000_000)
            && value["automaticRenewal"] == true
            && value["cancellationMethod"] == "native_account_subscription"
            && value["subscriptionConsentInitiallyAccepted"] == false
            && token(&value["subscriptionConsentToken"]),
    )?;
    require(
        value
            .get("paymentMethodKind")
            .is_none_or(|v| matches!(v.as_str(), Some("card" | "sbp"))),
    )?;
    if let Some(methods) = value
        .get("availablePaymentMethods")
        .filter(|v| !v.is_null())
    {
        let methods = methods.as_array().ok_or(Error::InvalidResponse)?;
        require(
            methods.len() <= 2
                && methods
                    .iter()
                    .all(|v| matches!(v.as_str(), Some("card" | "sbp")))
                && (methods.len() != 2 || methods[0] != methods[1]),
        )?;
    }
    require(
        value
            .get("checkoutAvailable")
            .is_none_or(|v| v.is_boolean()),
    )?;
    let first = date(&value["firstChargeAt"])?;
    let free_end = date(&value["freeAccessEndsAt"])?;
    let renewal = date(&value["renewalChargeAt"])?;
    let service_end = date(&value["servicePeriodEndsAt"])?;
    require(free_end == first)?;
    if value["freeAccessSource"] == "binding_first" {
        let delay = value["firstChargeDelaySeconds"]
            .as_u64()
            .ok_or(Error::InvalidResponse)?;
        require(
            (60..=86_400).contains(&delay)
                && delay % 60 == 0
                && first.is_none()
                && renewal.is_none()
                && service_end.is_none()
                && value["firstChargeAmountKopeks"] == value["priceKopeks"],
        )?;
    } else {
        require(value["firstChargeDelaySeconds"].is_null() && first.is_some() && renewal > first)?;
    }
    let source = value["freeAccessSource"]
        .as_str()
        .ok_or(Error::InvalidResponse)?;
    let kind = value["purchaseKind"]
        .as_str()
        .ok_or(Error::InvalidResponse)?;
    require(match kind {
        "subscription_start" => {
            empty(&value["fromPlanCode"])
                && empty(&value["checkoutIntentToken"])
                && (service_end.is_none() || (source == "none" && service_end == renewal))
                && match source {
                    "card_trial" => bounded(&value["trialDays"], 0, 366),
                    "promo" | "none" | "current_period" | "binding_first" => {
                        value["trialDays"] == 0
                    }
                    _ => false,
                }
                && if source == "none" {
                    value["amountDueNowKopeks"] == value["priceKopeks"]
                } else {
                    value["amountDueNowKopeks"] == 0
                }
        }
        "trial_plan_upgrade" => {
            value["planCode"] == "pro"
                && value["fromPlanCode"] == "base"
                && source == "card_trial"
                && value["trialDays"] == 0
                && value["amountDueNowKopeks"] == 0
                && empty(&value["checkoutIntentToken"])
                && service_end == first
        }
        "plan_upgrade" => {
            value["planCode"] == "pro"
                && value["fromPlanCode"] == "base"
                && source == "none"
                && value["trialDays"] == 0
                && value["amountDueNowKopeks"] == value["priceKopeks"]
                && token(&value["checkoutIntentToken"])
                && service_end == renewal
        }
        "scheduled_offer_change" => {
            matches!(value["fromPlanCode"].as_str(), Some("base" | "pro"))
                && source == "current_period"
                && value["trialDays"] == 0
                && value["amountDueNowKopeks"] == 0
                && empty(&value["checkoutIntentToken"])
                && service_end == first
        }
        _ => false,
    })?;
    receipt_contact(value)?;
    for name in [
        "subscriptionTerms",
        "userAgreement",
        "refundPolicy",
        "privacyNotice",
        "receiptDataConsent",
    ] {
        let document = &value["legalDocuments"][name];
        require(
            document["version"]
                .as_str()
                .is_some_and(|v| !v.trim().is_empty() && v.len() <= 64),
        )?;
        let url = url::Url::parse(document["url"].as_str().ok_or(Error::InvalidResponse)?)
            .map_err(|_| Error::InvalidResponse)?;
        require(url.scheme() == "https" && url.host_str().is_some())?;
    }
    if let Some((catalog, requested_offer)) = selected_catalog {
        crate::validate_catalog(catalog)?;
        let offer = catalog["offers"]
            .as_array()
            .ok_or(Error::InvalidResponse)?
            .iter()
            .find(|offer| offer["offerId"] == requested_offer)
            .ok_or(Error::InvalidResponse)?;
        require(
            value["catalogVersion"] == catalog["catalogVersion"]
                && value["offerId"] == offer["offerId"]
                && value["planCode"] == offer["planCode"]
                && value["periodDays"] == offer["periodDays"],
        )?;
        if !value["promotion"].is_null() {
            promotion(value, offer)?;
        } else {
            require(
                value["renewalAmountKopeks"] == offer["priceKopeks"]
                    && if kind == "plan_upgrade" {
                        value["priceKopeks"].as_u64() < offer["priceKopeks"].as_u64()
                    } else {
                        value["priceKopeks"] == offer["priceKopeks"]
                    },
            )?;
        }
    }
    Ok(())
}

fn receipt_contact(value: &Value) -> Result<()> {
    let masked = value["receiptEmailMasked"].as_str().unwrap_or("");
    require(match value["receiptContactState"].as_str() {
        Some("oauth_email_available") => {
            token(&value["receiptContactToken"])
                && masked.len() <= 64
                && !masked.contains(['\r', '\n'])
                && masked
                    .split_once("***@")
                    .is_some_and(|(before, after)| !before.is_empty() && !after.is_empty() && !after.contains("***@"))
        }
        Some("email_required") => masked.trim().is_empty() && empty(&value["receiptContactToken"]),
        _ => false,
    })
}

fn promotion(value: &Value, offer: &Value) -> Result<()> {
    let promo = &value["promotion"];
    require(
        value["purchaseKind"] == "subscription_start"
            && promo["currency"] == "RUB"
            && promo["catalogVersion"] == value["catalogVersion"]
            && promo["offerId"] == offer["offerId"]
            && promo["planCode"] == offer["planCode"]
            && promo["followingPriceKopeks"] == offer["priceKopeks"]
            && promo["followingPeriodDays"] == offer["periodDays"],
    )?;
    let periods = promo["billingPeriods"]
        .as_array()
        .ok_or(Error::InvalidResponse)?;
    require((1..=36).contains(&periods.len()))?;
    for (index, period) in periods.iter().enumerate() {
        let payable = period["payablePriceKopeks"]
            .as_u64()
            .ok_or(Error::InvalidResponse)?;
        let standard = offer["priceKopeks"]
            .as_u64()
            .ok_or(Error::InvalidResponse)?;
        require(
            period["number"].as_u64() == Some(index as u64 + 1)
                && period["standardPriceKopeks"] == offer["priceKopeks"]
                && payable > 0
                && payable < standard
                && period["discountAmountKopeks"].as_u64() == Some(standard - payable),
        )?;
    }
    require(
        value["priceKopeks"] == periods[0]["payablePriceKopeks"]
            && value["firstChargeAmountKopeks"] == periods[0]["payablePriceKopeks"]
            && &value["renewalAmountKopeks"]
                == if periods.len() > 1 {
                    &periods[1]["payablePriceKopeks"]
                } else {
                    &promo["followingPriceKopeks"]
                },
    )
}
