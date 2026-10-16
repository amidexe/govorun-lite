//! Existing diagnosticsEvents Data, including Foundation's 2001 date epoch.
use crate::persistence::defaults_documents::{Cell, Domain};
use chrono::{DateTime, SecondsFormat, Utc};
use govorun_core::journal_report::{admission, MacEvent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
const KEY: &str = "diagnosticsEvents";
const EPOCH: i64 = 978_307_200;
const LIMIT: usize = 8 * 1_048_576;
#[derive(Clone, Serialize, Deserialize)]
struct Stored {
    id: String,
    date: f64,
    level: String,
    category: String,
    message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timing: Option<String>,
}
fn date(value: f64) -> Option<DateTime<Utc>> {
    if !value.is_finite() {
        return None;
    }
    let whole = value.floor();
    let seconds = (whole as i64).checked_add(EPOCH)?;
    let nanos = ((value - whole) * 1_000_000_000.0).round() as u32;
    if nanos == 1_000_000_000 {
        DateTime::from_timestamp(seconds.checked_add(1)?, 0)
    } else {
        DateTime::from_timestamp(seconds, nanos)
    }
}
fn timestamp(value: &str) -> Option<f64> {
    let date = DateTime::parse_from_rfc3339(value).ok()?;
    Some(
        date.timestamp().checked_sub(EPOCH)? as f64
            + f64::from(date.timestamp_subsec_nanos()) / 1_000_000_000.0,
    )
}
impl Stored {
    fn event(&self, precision: SecondsFormat) -> Option<MacEvent> {
        let id = uuid::Uuid::parse_str(&self.id)
            .ok()?
            .hyphenated()
            .to_string()
            .to_uppercase();
        admission::macos(MacEvent {
            id,
            date: date(self.date)?.to_rfc3339_opts(precision, true),
            level: self.level.clone(),
            category: self.category.clone(),
            message: self.message.clone(),
            timing: self.timing.clone(),
        })
    }
}
fn stored(domain: &impl Domain) -> Result<Vec<Stored>, &'static str> {
    let snapshot = domain.read().map_err(|_| "storage_unavailable")?;
    let Some(Cell::Data(bytes)) = snapshot.get(KEY) else {
        return Ok(vec![]);
    };
    if bytes.len() > LIMIT {
        return Err("storage_unavailable");
    }
    let records: Vec<Stored> = serde_json::from_slice(bytes).unwrap_or_default();
    // Swift's Codable array rejects the whole array on a bad required UUID/level.
    if records.iter().any(|record| {
        uuid::Uuid::parse_str(&record.id).is_err()
            || !["info", "warning", "error"].contains(&record.level.as_str())
    }) {
        return Ok(vec![]);
    }
    Ok(records)
}
pub fn read(domain: &impl Domain) -> Result<Vec<MacEvent>, &'static str> {
    let mut events: Vec<_> = stored(domain)?
        .iter()
        .filter_map(|record| record.event(SecondsFormat::Secs))
        .collect();
    if events.len() > 200 {
        events.drain(..events.len() - 200);
    }
    Ok(events)
}
pub fn append(
    domain: &impl Domain,
    event: MacEvent,
    deduplicate: bool,
) -> Result<(), &'static str> {
    let mut event = admission::macos(event).ok_or("invalid_parameters")?;
    event.id = uuid::Uuid::parse_str(&event.id)
        .map_err(|_| "invalid_parameters")?
        .hyphenated()
        .to_string()
        .to_uppercase();
    let records = stored(domain)?;
    let mut original_dates: HashMap<(String, String), VecDeque<f64>> = HashMap::new();
    let events = records
        .iter()
        .filter_map(|record| {
            let event = record.event(SecondsFormat::Nanos)?;
            original_dates
                .entry((event.id.clone(), event.date.clone()))
                .or_default()
                .push_back(record.date);
            Some(event)
        })
        .collect();
    let events = admission::append_macos(events, event, deduplicate);
    let records: Result<Vec<_>, &'static str> = events
        .into_iter()
        .map(|event| {
            // Keep each original Foundation date bit-for-bit; only new records need conversion.
            let original = original_dates
                .get_mut(&(event.id.clone(), event.date.clone()))
                .and_then(VecDeque::pop_front);
            let date = original
                .or_else(|| timestamp(&event.date))
                .ok_or("invalid_parameters")?;
            Ok(Stored {
                id: event.id,
                date,
                level: event.level,
                category: event.category,
                message: event.message,
                timing: event.timing,
            })
        })
        .collect();
    let bytes = serde_json::to_vec(&records?).map_err(|_| "storage_unavailable")?;
    domain
        .write(BTreeMap::from([(KEY.into(), Some(Cell::Data(bytes)))]))
        .map_err(|_| "storage_unavailable")
}
pub fn clear(domain: &impl Domain) -> Result<(), &'static str> {
    domain
        .write(BTreeMap::from([(KEY.into(), None)]))
        .map_err(|_| "storage_unavailable")
}
