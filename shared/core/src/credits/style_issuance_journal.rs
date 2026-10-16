use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

const MAX_DATE_MILLIS: u64 = 253_402_300_799_999;
const MAX_BATCH: usize = crate::style_inventory::BATCH;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Batch { count: usize },
    Operation { id: String },
    Lease { count: usize, scope: Option<String>, day: Option<String> },
    Replay {
        now_millis: u64,
        replay_until_millis: u64,
    },
    Response {
        expected_key_id: String,
        key_id: String,
        request_count: usize,
        signature_count: usize,
    },
}

/// Replay uses the deadline written by the common issuance preparation. Pass
/// expiry is separate: an exact replay may return expired passes for retirement.
/// Missing deadlines are invalid current-format journals, never legacy imports.
pub(crate) fn evaluate(event: Event) -> Result<Value> {
    match event {
        Event::Batch { count } if count <= MAX_BATCH => Ok(json!({})),
        Event::Operation { id } if crate::request::uuid(&id) && !id.bytes().any(|b| b.is_ascii_uppercase()) => Ok(json!({})),
        Event::Lease { count, scope, day } if (1..=MAX_BATCH).contains(&count)
            && scope.as_ref().is_none_or(|id| crate::request::uuid(id) && day.is_some()) => Ok(json!({})),
        Event::Batch { .. } | Event::Operation { .. } | Event::Lease { .. } => Err(Error::InvalidRequest),
        Event::Replay { now_millis, replay_until_millis } => {
            if now_millis > MAX_DATE_MILLIS
                || replay_until_millis == 0
                || replay_until_millis > MAX_DATE_MILLIS
            {
                return Err(Error::InvalidRequest);
            }
            Ok(json!({"replay": now_millis < replay_until_millis}))
        }
        Event::Response { expected_key_id, key_id, request_count, signature_count } => {
            if expected_key_id.is_empty()
                || key_id != expected_key_id
                || !(1..=MAX_BATCH).contains(&request_count)
                || signature_count != request_count
            {
                return Err(Error::InvalidResponse);
            }
            Ok(json!({"valid": true}))
        }
    }
}
