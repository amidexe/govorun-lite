//! One cumulative number per unreported scope/day, plus existing phrase counts.
//! No pass, lease, request identifier or acknowledgement history reaches disk.
use super::Envelope;
use crate::{usage_ledger as usage, StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
struct Counter {
    scope_id: String,
    day: String,
    consumed: u32,
    #[serde(default, skip_serializing_if="Option::is_none")]
    requests: Option<usage::RequestCount>,
}
pub(super) fn pending(value: &Envelope) -> StoreResult<Value> {
    let reports = usage::apply(value.usage().map_err(|_|StoreError::Invalid)?, usage::Event::Validate)
        .map_err(|_|StoreError::Invalid)?.reports;
    let counters: Vec<_> = reports.into_iter().flat_map(|report| {
        report.days.into_iter().map(move |day| Counter {
            scope_id: report.scope_id.clone(), requests: report.requests.get(&day.date).cloned(),
            day: day.date, consumed: day.consumed,
        })
    }).collect();
    serde_json::to_value(counters).map_err(|_|StoreError::Invalid)
}
pub(super) fn restore(value: &Value) -> StoreResult<Value> {
    let counters: Vec<Counter> = serde_json::from_value(value.clone()).map_err(|_|StoreError::Invalid)?;
    let mut ledger = usage::Ledger::default();
    for counter in counters {
        let days = ledger.totals.entry(counter.scope_id.clone()).or_default();
        if days.insert(counter.day.clone(), counter.consumed).is_some() {return Err(StoreError::Invalid);}
        if let Some(requests) = counter.requests {
            ledger.requests.entry(counter.scope_id).or_default().insert(counter.day,requests);
        }
    }
    let validated = usage::apply(ledger,usage::Event::Validate).map_err(|_|StoreError::Invalid)?;
    serde_json::to_value(validated.ledger).map_err(|_|StoreError::Invalid)
}
