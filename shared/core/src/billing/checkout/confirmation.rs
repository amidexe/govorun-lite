use crate::{Error, Result};
use serde_json::{json, Value};

/// Native callers return only the shown disclosure identity and explicit input.
/// The admitted preview owns endpoint, receipt token and bank intent selection.
pub(super) fn prepared(preview: &Value, input: &Value) -> Result<(&'static str, Value)> {
    let fields = ["offerId", "subscriptionConsentToken", "receiptEmail",
        "subscriptionConsentAccepted", "paymentServiceNoticeAccepted"];
    if !input.as_object().is_some_and(|o| o.keys().all(|k| fields.contains(&k.as_str())))
        || input["offerId"] != preview["offerId"]
        || input["subscriptionConsentToken"] != preview["subscriptionConsentToken"]
        || preview["checkoutAvailable"] == false
        || input["subscriptionConsentAccepted"] != true
        || input["paymentServiceNoticeAccepted"] != true {
        return Err(Error::InvalidRequest);
    }
    let email = text(input, "receiptEmail")?.trim();
    let path = super::preview_action::confirmation_path(preview);
    let mut request = json!({"subscriptionConsentToken":preview["subscriptionConsentToken"],
        "subscriptionConsentAccepted":true, "paymentServiceNoticeAccepted":true});
    if path == super::plan_change::PATH {
        request["targetOfferId"] = preview["offerId"].clone();
    } else {
        if (!email.is_empty() && !plausible_email(email))
            || (email.is_empty() && text(preview, "receiptContactToken")?.is_empty()) {
            return Err(Error::Checkout(super::CheckoutFailure::invalid_contact()));
        }
        request["offerId"] = preview["offerId"].clone();
        if email.is_empty() { request["receiptContactToken"] = preview["receiptContactToken"].clone(); }
        else { request["receiptEmail"] = json!(email); }
        if let Some(intent) = preview.get("checkoutIntentToken") {
            request["checkoutIntentToken"] = intent.clone();
        }
    }
    // Reuse the same admission as durable/replayed confirmations.
    Ok((path, confirmation(preview, path, &request)?))
}

/// Builds the only permitted mutation from an admitted disclosure and explicit consent.
/// Prices, account IDs, provider results and caller-generated retry keys never enter it.
pub(super) fn confirmation(preview: &Value, path: &str, input: &Value) -> Result<Value> {
    let expected_path = super::preview_action::confirmation_path(preview);
    let no_charge = expected_path == "/v1/billing/change";
    let fields: &[&str] = if no_charge {
        &[
            "targetOfferId",
            "subscriptionConsentToken",
            "subscriptionConsentAccepted",
            "paymentServiceNoticeAccepted",
        ]
    } else {
        &[
            "offerId",
            "receiptEmail",
            "receiptContactToken",
            "checkoutIntentToken",
            "subscriptionConsentToken",
            "subscriptionConsentAccepted",
            "paymentServiceNoticeAccepted",
        ]
    };
    if path != expected_path
        || !input
            .as_object()
            .is_some_and(|o| o.keys().all(|k| fields.contains(&k.as_str())))
        || input[if no_charge {
            "targetOfferId"
        } else {
            "offerId"
        }] != preview["offerId"]
        || input["subscriptionConsentToken"] != preview["subscriptionConsentToken"]
        || input["subscriptionConsentAccepted"] != true
        || input["paymentServiceNoticeAccepted"] != true
        || preview["checkoutAvailable"] == false
    {
        return Err(Error::InvalidRequest);
    }
    let mut body = json!({"subscriptionConsentToken":preview["subscriptionConsentToken"],
        "subscriptionConsentAccepted":true, "paymentServiceNoticeAccepted":true});
    if no_charge {
        body["targetOfferId"] = preview["offerId"].clone();
        return Ok(body);
    }
    body["offerId"] = preview["offerId"].clone();
    let email = text(input, "receiptEmail")?.trim();
    let token = text(input, "receiptContactToken")?;
    if !email.is_empty() {
        if !token.is_empty() || !plausible_email(email) {
            return Err(Error::InvalidRequest);
        }
        body["receiptEmail"] = json!(email);
    } else if token.is_empty() || input["receiptContactToken"] != preview["receiptContactToken"] {
        return Err(Error::InvalidRequest);
    } else {
        body["receiptContactToken"] = json!(token);
    }
    let intent = text(input, "checkoutIntentToken")?;
    if intent != text(preview, "checkoutIntentToken")? {
        return Err(Error::InvalidRequest);
    }
    if !intent.is_empty() {
        body["checkoutIntentToken"] = json!(intent);
    }
    Ok(body)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    match &value[key] {
        Value::Null => Ok(""),
        Value::String(v) => Ok(v),
        _ => Err(Error::InvalidRequest),
    }
}

fn plausible_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    value.len() <= 254
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

pub(super) fn new_key() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::NetworkUnavailable)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}
