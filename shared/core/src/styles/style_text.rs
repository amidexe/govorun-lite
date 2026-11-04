use crate::{Error, Result};
use serde_json::{json, Value};

/// Pure local admission. Text is neither retained nor included in an account
/// request, view or diagnostic. UTF-16 units preserve invalid surrogates
/// across JNI/JSON so all native transports reject the same malformed input.
pub(crate) fn admit(units: Vec<u16>) -> Result<Value> {
    admit_class(units, None)
}

/// The request class "digest" (Говорун.Эхо's итоги of a long note) has its own length and price: the model reads the
/// whole note and writes one page, so a credit covers more characters than an improvement's. Any other class keeps
/// the improvement's rule.
pub(crate) fn admit_class(units: Vec<u16>, class: Option<&str>) -> Result<Value> {
    if class == Some(DIGEST) {
        let price = digest_price(&units)?;
        if price["allowed"] != true { return Err(Error::InvalidRequest); }
        return Ok(price);
    }
    let characters = count(&units)?;
    if characters == 0 || characters > crate::dictation_plan::MAXIMUM { return Err(Error::InvalidRequest); }
    Ok(json!({"characters": characters, "requiredCredits": characters.div_ceil(crate::credit_limits::CHARACTERS_PER_CREDIT),
        "deadlineMillis": crate::dictation_timing::deadline(characters),
        "admissionBudgetMillis": crate::dictation_timing::ADMISSION_MILLIS}))
}

pub(crate) const DIGEST: &str = "digest";

/// Generic operations use the existing pricing classes; their exact class is checked again by the server catalog.
pub(crate) fn digest_payload(payload: &Value) -> bool {
    payload.get("textOperation").is_some() && payload["requestClass"] == DIGEST
}

pub(crate) fn valid_operation(name: &str) -> bool {
    let bytes = name.as_bytes();
    (2..=64).contains(&bytes.len()) && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(b))
}

/// Dictation's label: a request Govorun's entry put no product on is Govorun's own dictation.
pub(crate) const DICTATION: &str = "dictation";

/// The product a request is for: the label Govorun's entry put on it ("echo" for Говорун.Эхо's notes), dictation when
/// none. A text operation with no product, or a malformed label, is refused, so another product's request never counts as
/// a dictation.
pub(crate) fn product(payload: &Value) -> Result<&str> {
    let label = match payload.get("product") {
        None => DICTATION,
        Some(value) => value.as_str().filter(|label| valid_product(label)).ok_or(Error::InvalidRequest)?,
    };
    if label == DICTATION && payload.get("textOperation").is_some() {
        return Err(Error::InvalidRequest);
    }
    Ok(label)
}

/// A product label as Accounts stores it: a lowercase letter, then lowercase letters, digits or «_», 2–32 in all.
pub(crate) fn valid_product(label: &str) -> bool {
    let bytes = label.as_bytes();
    (2..=32).contains(&bytes.len()) && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}

/// What итоги of this text cost before anything is sent: the credits, whether they cost more than the largest
/// improvement (Echo asks the person first), or why the text cannot have итоги.
pub(crate) fn digest_price(units: &[u16]) -> Result<Value> {
    use crate::credit_limits::*;
    let characters = count(units)?;
    let credits = characters.div_ceil(DIGEST_CHARACTERS_PER_CREDIT);
    let reason = if characters < DIGEST_MINIMUM_CHARACTERS { Some("too_short") }
        else if characters > DIGEST_MAXIMUM_CHARACTERS { Some("too_long") } else { None };
    Ok(json!({"characters": characters, "requiredCredits": credits, "allowed": reason.is_none(), "reason": reason,
        "minimumCharacters": DIGEST_MINIMUM_CHARACTERS, "maximumCharacters": DIGEST_MAXIMUM_CHARACTERS,
        "confirm": credits > MAXIMUM_CHARACTERS.div_ceil(CHARACTERS_PER_CREDIT),
        "deadlineMillis": crate::dictation_timing::NOTE_MILLIS,
        "admissionBudgetMillis": crate::dictation_timing::ADMISSION_MILLIS}))
}

pub(crate) fn count(units: &[u16]) -> Result<usize> {
    let mut characters = 0_usize;
    let mut trailing = 0_usize;
    for scalar in char::decode_utf16(units.iter().copied()) {
        let scalar = scalar.map_err(|_| Error::InvalidRequest)?;
        if scalar.is_whitespace() {
            if characters > 0 { trailing += 1; }
        } else {
            characters = characters.checked_add(trailing + 1).ok_or(Error::InvalidRequest)?;
            trailing = 0;
        }
    }
    Ok(characters)
}
