use crate::{Error, Result};
use serde_json::{json, Value};

/// Pure local admission. Text is neither retained nor included in an account
/// request, view or diagnostic. UTF-16 units preserve invalid surrogates
/// across JNI/JSON so all native transports reject the same malformed input.
pub(crate) fn admit(units: Vec<u16>) -> Result<Value> {
    let characters = count(&units)?;
    if characters == 0 || characters > crate::dictation_plan::MAXIMUM { return Err(Error::InvalidRequest); }
    Ok(json!({"characters": characters, "requiredCredits": characters.div_ceil(crate::credit_limits::CHARACTERS_PER_CREDIT),
        "deadlineMillis": crate::dictation_timing::deadline(characters),
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
