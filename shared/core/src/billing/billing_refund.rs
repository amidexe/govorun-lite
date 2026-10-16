use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BillingRefundReason {
    ServiceNotWorking, AccessNotGranted, ChargedAfterCancellation,
    DuplicateCharge, SubscriptionNotNeeded, Other,
}

pub(super) const ACKNOWLEDGED: &str = "Запрос на возврат получен";

pub(super) fn validate(value: &Value) -> Result<()> {
    let valid_text = |key: &str| value[key].as_str().is_some_and(|s|
        !s.trim().is_empty() && s.len() <= 128 && !s.chars().any(char::is_control));
    let valid_date = |value: &Value| value.as_str()
        .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok());
    if !valid_text("id") || !matches!(value["state"].as_str(),
        Some("requested" | "reviewing" | "approved" | "rejected" | "executing" | "refunded" | "failed")) ||
        serde_json::from_value::<BillingRefundReason>(value["reasonCode"].clone()).is_err() ||
        !valid_date(&value["requestedAt"]) || !value["renewalStopped"].is_boolean() ||
        value.get("currentAccessEndsAt").is_some_and(|v| !v.is_null() && !valid_date(v)) {
        return Err(Error::InvalidResponse);
    }
    Ok(())
}
