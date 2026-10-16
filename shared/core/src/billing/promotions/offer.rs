use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub(super) fn timestamp(value: &Value) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.as_str().ok_or(Error::InvalidResponse)?)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|_| Error::InvalidResponse)
}

pub(crate) fn uuid(value: &Value) -> bool {
    value.as_str().is_some_and(|value| {
        value.len() == 36
            && value.bytes().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            })
    })
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.trim().is_empty())
}

pub(super) fn free_reset(offer: &Value) -> bool {
    offer.get("accessDays").is_none()
        && offer["grantsAccessOnConfirmation"] == true
        && offer["createsSubscription"] == false
        && offer["checkoutRequired"] == false
        && omitted_or_zero(offer, "followingPriceKopeks")
        && omitted_or_zero(offer, "followingPeriodDays")
        && offer["bonus"].is_null()
        && empty_periods(offer)
        && text(&offer["offerId"])
        && text(&offer["planCode"])
}

fn omitted_or_zero(offer: &Value, key: &str) -> bool {
    offer.get(key).is_none_or(|value| value.as_u64() == Some(0))
}

fn empty_periods(offer: &Value) -> bool {
    offer["billingPeriods"].is_null()
        || offer["billingPeriods"]
            .as_array()
            .is_some_and(Vec::is_empty)
}

pub(super) fn needs_catalog(offer: &Value) -> bool {
    offer.get("accessDays").is_none() && !free_reset(offer)
}

pub(crate) fn inspect(offer: &Value) -> Result<bool> {
    if offer["currency"] != "RUB"
        || !text(&offer["confirmationTitle"])
        || !text(&offer["confirmationBody"])
        || !text(&offer["offerId"])
        || !offer["checkoutRequired"].is_boolean()
        || !offer["createsSubscription"].is_boolean()
        || !offer["grantsAccessOnConfirmation"].is_boolean()
    {
        return Err(Error::InvalidResponse);
    }
    // Presence matters: a declared null or malformed day grant is never a discount.
    if let Some(days) = offer.get("accessDays") {
        let fields = ["days", "planCode", "planName", "startsAt", "endsAt"];
        if !days.as_object().is_some_and(|v| {
            v.len() == fields.len() && fields.iter().all(|key| v.contains_key(*key))
        }) || !days["days"]
            .as_u64()
            .is_some_and(|n| (1..=366).contains(&n))
            || !matches!(days["planCode"].as_str(), Some("base" | "pro"))
            || !text(&days["planName"])
            || days["planCode"] != offer["planCode"]
            || days["planName"] != offer["planName"]
            || timestamp(&days["startsAt"])?.timestamp() <= 0
            || timestamp(&days["endsAt"])? <= timestamp(&days["startsAt"])?
            || offer["createsSubscription"] != false
            || offer["checkoutRequired"] != false
            || offer["grantsAccessOnConfirmation"] != true
            || !offer["bonus"].is_null()
            || !empty_periods(offer)
        {
            return Err(Error::InvalidResponse);
        }
        return Ok(false);
    }
    if free_reset(offer) {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn validate(
    offer: &Value,
    catalog: Option<&Value>,
    requested: Option<&str>,
) -> Result<()> {
    if !inspect(offer)? {
        return Ok(());
    }
    let catalog = catalog.ok_or(Error::InvalidResponse)?;
    let offers = catalog["offers"].as_array().ok_or(Error::InvalidResponse)?;
    let mut selected = offers
        .iter()
        .filter(|offer| offer["offerId"].as_str() == requested);
    let selected_offer = selected.next().ok_or(Error::InvalidResponse)?;
    if selected.next().is_some()
        || !text(&catalog["catalogVersion"])
        || offer["catalogVersion"] != catalog["catalogVersion"]
        || offer["offerId"] != selected_offer["offerId"]
        || offer["planCode"] != selected_offer["planCode"]
        || catalog["currency"] != "RUB"
        || !offer["followingPriceKopeks"]
            .as_u64()
            .is_some_and(|n| n > 0)
        || !offer["followingPeriodDays"].as_u64().is_some_and(|n| n > 0)
        || offer["followingPriceKopeks"] != selected_offer["priceKopeks"]
        || offer["followingPeriodDays"] != selected_offer["periodDays"]
        || offer["checkoutRequired"] != offer["createsSubscription"]
        || (offer["checkoutRequired"] == true
            && (!offer["billingPeriods"]
                .as_array()
                .is_some_and(|values| (1..=36).contains(&values.len()))
                || offer["grantsAccessOnConfirmation"] != false))
    {
        return Err(Error::InvalidResponse);
    }
    Ok(())
}
