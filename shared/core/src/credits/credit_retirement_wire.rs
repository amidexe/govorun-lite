use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipts { replacement_receipts: Vec<String> }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Failure { error: String }

/// Only a complete, exact server response can replace or destroy a local pass.
/// Unreadable/extended/partial responses leave the original retirement pending.
pub(crate) fn decide(status: Option<u16>, body: &str, expected_count: usize,
    retry_after: Option<&str>, now_millis: i64) -> Result<Value> {
    if !(1..=crate::style_inventory::BATCH).contains(&expected_count) { return Err(Error::InvalidRequest); }
    if status == Some(200) && body.len() <= 65_536 {
        if let Ok(response) = serde_json::from_str::<Receipts>(body) {
            let receipts = response.replacement_receipts;
            if receipts.len() == expected_count
                && receipts.iter().collect::<HashSet<_>>().len() == receipts.len()
                && receipts.iter().all(|r| (64..=4_096).contains(&r.len()) && !r.trim().is_empty()) {
                return Ok(json!({"state":"ready","replacementReceipts":receipts,"retryAfterMillis":null}));
            }
        }
    }
    let failure = if body.len() <= 4_096 { serde_json::from_str::<Failure>(body).ok() } else { None };
    let discard = matches!((status, failure.as_ref().map(|f| f.error.as_str())),
        (Some(400), Some("invalid_text_credits")) | (Some(409), Some("text_credit_already_used")));
    let delay = (!discard).then(|| crate::style_start::retry_after_delay(retry_after, now_millis)).flatten();
    Ok(json!({"state":if discard { "discard" } else { "retry" },
        "replacementReceipts":[],"retryAfterMillis":delay}))
}
