use crate::{Error, Result};
use chrono::DateTime;
use serde_json::Value;

pub(super) const PATH: &str = "/v1/billing/change";

pub(super) fn mode(preview: &Value) -> Option<&'static str> {
    match preview["purchaseKind"].as_str() {
        Some("trial_plan_upgrade") => Some("changed_now_without_charge"),
        Some("scheduled_offer_change") => Some("scheduled_at_period_end"),
        _ => None,
    }
}

/// The creation reply predates the exact receipt endpoint. Validate every
/// disclosed term it returns; an exact recovery receipt must return all terms.
pub(super) fn validate(value: &Value, preview: &Value, receipt: bool) -> Result<()> {
    let expected = mode(preview).ok_or(Error::InvalidResponse)?;
    require(value["mode"] == expected && value["targetOfferId"] == preview["offerId"])?;
    require(value["targetPlanCode"] == preview["planCode"] && value["amountDueNowKopeks"] == 0)?;
    require(value["firstChargeAmountKopeks"] == preview["firstChargeAmountKopeks"])?;
    if receipt || value.get("catalogVersion").is_some() {
        require(value["catalogVersion"] == preview["catalogVersion"])?;
    }
    let expected_first = date(&preview["firstChargeAt"])?;
    if receipt || expected == "changed_now_without_charge" || value.get("firstChargeAt").is_some() {
        require(date(&value["firstChargeAt"])? == expected_first)?;
    }
    let effective = date(&value["effectiveAt"])?;
    if expected == "scheduled_at_period_end" { require(effective == expected_first)?; }
    Ok(())
}

fn date(value: &Value) -> Result<i64> {
    DateTime::parse_from_rfc3339(value.as_str().ok_or(Error::InvalidResponse)?)
        .map(|v| v.timestamp_millis()).map_err(|_| Error::InvalidResponse)
}
fn require(value: bool) -> Result<()> {
    if value { Ok(()) } else { Err(Error::InvalidResponse) }
}
