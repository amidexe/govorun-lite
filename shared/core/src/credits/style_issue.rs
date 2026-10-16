use crate::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Action {
    PreserveJournal,
    DiscardAndRefreshQuota,
    DiscardAndRefreshAccount,
    DiscardAndStop,
}

/// Only an Accounts issuance response can establish a pre-commit rejection.
/// A transport failure or malformed success never permits replacing its key.
#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Decision {
    action: Action,
    pub(crate) discard_journal: bool,
    pub(crate) refresh_account: bool,
    pub(crate) discard_replacement_receipts: bool,
}

/// Only exact Accounts errors and validated quota snapshots influence stored pass state.
/// Every other JSON shape is an unrecognized response; its payload is discarded.
#[derive(Clone, Copy)]
pub(crate) enum FailureBody { Quota, InvalidReturnReceipt, ReturnAlreadyUsed, Unrecognized }

impl FailureBody {
    pub(crate) fn from_value(body: &Value) -> Self {
        if quota_limit(body).is_some() { return Self::Quota; }
        let code = body.as_object().filter(|object| object.len() == 1)
            .and_then(|object| object.get("error")).and_then(Value::as_str);
        match code {
            Some("style_pass_limit") => Self::Quota,
            Some("invalid_credit_return_receipt") => Self::InvalidReturnReceipt,
            Some("credit_return_already_used") => Self::ReturnAlreadyUsed,
            _ => Self::Unrecognized,
        }
    }
    fn code(self) -> Option<&'static str> {
        match self {
            Self::Quota => Some("style_pass_limit"),
            Self::InvalidReturnReceipt => Some("invalid_credit_return_receipt"),
            Self::ReturnAlreadyUsed => Some("credit_return_already_used"),
            Self::Unrecognized => None,
        }
    }
}

/// A quota response can only publish a confirmed zero for the named limit.
/// The full access object is still validated by AccountSession before storage.
pub(crate) fn quota_limit(body: &Value) -> Option<crate::stylization::AccessFailure> {
    use crate::stylization::AccessFailure;
    let object = body.as_object()?;
    if object.len() != 4 || body["error"] != "style_pass_limit" || body["remaining"] != 0 {
        return None;
    }
    let (limit, field) = match body["limitKind"].as_str()? {
        "daily" => (AccessFailure::DailyLimit, "remainingToday"),
        "period" => (AccessFailure::PeriodLimit, "remainingTotal"),
        _ => return None,
    };
    let access = body["styleAccess"].as_object()?;
    let balance = access.get("usageBalance").unwrap_or(&body["styleAccess"]);
    (balance[field] == 0).then_some(limit)
}
impl<'de> Deserialize<'de> for FailureBody {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        Ok(Self::from_value(&Value::deserialize(decoder)?))
    }
}

pub(crate) fn failure(status: Option<u16>, body: FailureBody) -> Result<Decision> {
    if status.is_some_and(|value| !(100..=599).contains(&value)) {
        return Err(Error::InvalidRequest);
    }
    let code = body.code();
    let action = match status {
        Some(429) if code == Some("style_pass_limit") => Action::DiscardAndRefreshQuota,
        Some(401 | 403 | 428) => Action::DiscardAndRefreshAccount,
        Some(400..=499) if status != Some(408) => Action::DiscardAndStop,
        _ => Action::PreserveJournal,
    };
    Ok(Decision {
        action,
        discard_journal: !matches!(action, Action::PreserveJournal),
        refresh_account: matches!(
            action,
            Action::DiscardAndRefreshQuota | Action::DiscardAndRefreshAccount
        ),
        discard_replacement_receipts: matches!(
            (status, code),
            (Some(400), Some("invalid_credit_return_receipt"))
                | (Some(409), Some("credit_return_already_used"))
        ),
    })
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for FailureBody {
    fn schema_name() -> String { "IssueFailureBody".into() }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Unrecognized response shapes are deliberately retained as a conservative outcome.
        schemars::schema::Schema::Bool(true)
    }
}
