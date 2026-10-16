use crate::{Error, Result};
use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) const MAX_TOTAL: i64 = 9_000_000_000_000_000;
const MAX_GENERATION: i64 = 1_000_000_000;
const RESPONSE_DAYS: u64 = 16;
pub(crate) const RETENTION_DAYS: u64 = 35;

#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Total {
    pub(crate) words: i64,
    pub(crate) speech_seconds: i64,
    pub(crate) recognitions: i64,
}
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Day {
    pub(crate) date: String,
    pub(crate) words: i64,
    pub(crate) speech_seconds: i64,
    pub(crate) recognitions: i64,
}
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeviceDay {
    pub(crate) date: String,
    pub(crate) revision: i64,
    pub(crate) words: i64,
    pub(crate) speech_seconds: i64,
    pub(crate) recognitions: i64,
    pub(crate) recognition_rtf_buckets: Vec<i64>,
    pub(crate) recognition_errors: i64,
    #[serde(default)]
    pub(crate) style_attempts: i64,
    #[serde(default)]
    pub(crate) style_failures: i64,
    #[serde(default)]
    pub(crate) style_slow: i64,
}
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub(crate) generation: i64,
    pub(crate) cursor: i64,
    pub(crate) total: Option<Total>,
    pub(crate) days: Vec<Day>,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Response {
    schema_version: u32,
    generation: i64,
    cursor: i64,
    snapshot: bool,
    day_window_start: String,
    total: Total,
    days: Vec<Day>,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Command {
    Accept {
        current: Snapshot,
        response: Response,
    },
    Prepare {
        today: String,
        local_days: Vec<DeviceDay>,
        revisions: BTreeMap<String, i64>,
        acknowledged: BTreeMap<String, i64>,
    },
}

pub(crate) fn date(value: &str) -> Result<NaiveDate> {
    if value.len() != 10
        || !value.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        return Err(Error::InvalidRequest);
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| Error::InvalidRequest)
}
fn counters(values: &[i64]) -> Result<()> {
    if values.iter().all(|v| (0..=MAX_TOTAL).contains(v)) {
        Ok(())
    } else {
        Err(Error::InvalidRequest)
    }
}

/// Admission for the exact numeric upload produced by the statistics ledger.
pub(crate) fn validate_upload(value: Value) -> Result<()> {
    #[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Upload { schema_version: u32, generation: i64, cursor: i64,
        total: Total, days: Vec<DeviceDay> }
    let upload: Upload = serde_json::from_value(value).map_err(|_| Error::InvalidRequest)?;
    if upload.schema_version != 3 || !(0..=MAX_GENERATION).contains(&upload.generation)
        || upload.cursor < 0 || upload.days.len() > 36 {
        return Err(Error::InvalidRequest);
    }
    counters(&[upload.total.words, upload.total.speech_seconds, upload.total.recognitions])?;
    let mut dates = std::collections::HashSet::new();
    for day in upload.days {
        date(&day.date)?;
        counters(&[day.words, day.speech_seconds, day.recognitions, day.recognition_errors, day.style_attempts, day.style_failures, day.style_slow])?;
        counters(&day.recognition_rtf_buckets)?;
        if day.style_failures > day.style_attempts || day.style_slow > day.style_attempts - day.style_failures || day.recognition_rtf_buckets.len() != 5 || day.revision != wire_revision(&day)
            || !dates.insert(day.date) {
            return Err(Error::InvalidRequest);
        }
    }
    Ok(())
}
fn validate_days(days: &[Day], total: &Total, start: NaiveDate, end: NaiveDate) -> Result<()> {
    counters(&[total.words, total.speech_seconds, total.recognitions])?;
    let mut remaining = [total.words, total.speech_seconds, total.recognitions];
    let mut dates = std::collections::HashSet::new();
    if days.len() > RESPONSE_DAYS as usize {
        return Err(Error::InvalidRequest);
    }
    for day in days {
        let parsed = date(&day.date)?;
        if parsed < start || parsed > end || !dates.insert(&day.date) {
            return Err(Error::InvalidRequest);
        }
        for (available, value) in
            remaining
                .iter_mut()
                .zip([day.words, day.speech_seconds, day.recognitions])
        {
            if value < 0 || value > *available {
                return Err(Error::InvalidRequest);
            }
            *available -= value;
        }
    }
    Ok(())
}

/// Produces a complete commit decision before any native reset or persistence effect.
fn accept(current: Snapshot, response: Response) -> Result<Value> {
    if response.schema_version != 3
        || !(1..=MAX_GENERATION).contains(&response.generation)
        || response.cursor <= 0
        || !(0..=MAX_GENERATION).contains(&current.generation)
        || current.cursor < 0
    {
        return Err(Error::InvalidRequest);
    }
    let start = date(&response.day_window_start)?;
    let end = start
        .checked_add_days(Days::new(RESPONSE_DAYS - 1))
        .ok_or(Error::InvalidRequest)?;
    validate_days(&response.days, &response.total, start, end)?;
    let reset = current.generation > 0 && current.generation != response.generation;
    if ((current.cursor == 0 || current.generation == 0 || reset) && !response.snapshot)
        || (!reset && response.cursor < current.cursor)
    {
        return Err(Error::InvalidRequest);
    }
    let mut days = BTreeMap::new();
    if !response.snapshot {
        let total = current.total.as_ref().ok_or(Error::InvalidRequest)?;
        // Cached dates outside the new server window expire; never retain future dates.
        for day in current.days {
            let parsed = date(&day.date)?;
            if parsed >= start && parsed <= end && days.insert(day.date.clone(), day).is_some() {
                return Err(Error::InvalidRequest);
            }
        }
        validate_days(
            &days.values().cloned().collect::<Vec<_>>(),
            total,
            start,
            end,
        )?;
    }
    for day in response.days {
        days.insert(day.date.clone(), day);
    }
    let days: Vec<_> = days.into_values().collect();
    validate_days(&days, &response.total, start, end)?;
    Ok(json!({"resetLocal":reset,"snapshot":Snapshot {
        generation:response.generation, cursor:response.cursor,
        total:Some(response.total), days,
    }}))
}

pub(crate) fn wire_revision(day: &DeviceDay) -> i64 {
    [day.words, day.speech_seconds, day.recognitions, day.recognition_errors, day.style_attempts, day.style_failures, day.style_slow]
        .iter().chain(&day.recognition_rtf_buckets)
        .fold(1_i64, |sum, value| sum.saturating_add(*value).min(MAX_GENERATION))
}

fn prepare(
    today: String,
    local_days: Vec<DeviceDay>,
    revisions: BTreeMap<String, i64>,
    acknowledged: BTreeMap<String, i64>,
) -> Result<Value> {
    let end = date(&today)?;
    let start = end
        .checked_sub_days(Days::new(RETENTION_DAYS - 1))
        .ok_or(Error::InvalidRequest)?;
    if local_days.len() > 10_000 || revisions.len() > 10_000 || acknowledged.len() > 10_000 {
        return Err(Error::InvalidRequest);
    }
    for (day, revision) in revisions.iter().chain(&acknowledged) {
        date(day)?;
        if *revision < 0 {
            return Err(Error::InvalidRequest);
        }
    }
    let mut local = BTreeMap::new();
    for mut day in local_days {
        let parsed = date(&day.date)?;
        counters(&[
            day.words,
            day.speech_seconds,
            day.recognitions,
            day.recognition_errors, day.style_attempts, day.style_failures, day.style_slow,
        ])?;
        counters(&day.recognition_rtf_buckets)?;
        if day.style_failures > day.style_attempts || day.style_slow > day.style_attempts - day.style_failures || day.recognition_rtf_buckets.len() != 5 {
            return Err(Error::InvalidRequest);
        }
        day.revision = wire_revision(&day);
        if parsed >= start && parsed <= end && local.insert(day.date.clone(), day).is_some() {
            return Err(Error::InvalidRequest);
        }
    }
    let sent: BTreeMap<_, _> = revisions
        .into_iter()
        .filter(|(day, revision)| {
            local.contains_key(day) && *revision > acknowledged.get(day).copied().unwrap_or(0)
        })
        .take(36)
        .collect();
    let days: Vec<_> = sent.keys().filter_map(|day| local.get(day)).map(|day| {
        let mut wire = json!(day);
        if day.style_attempts == 0 {
            if let Some(fields) = wire.as_object_mut() {
                fields.remove("styleAttempts"); fields.remove("styleFailures");
                fields.remove("styleSlow");
            }
        }
        wire
    }).collect();
    Ok(json!({"retentionStart":start.to_string(),"retentionEnd":today,"sent":sent,"days":days}))
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::Accept { current, response } => accept(current, response),
        Command::Prepare {
            today,
            local_days,
            revisions,
            acknowledged,
        } => prepare(today, local_days, revisions, acknowledged),
    }
}
