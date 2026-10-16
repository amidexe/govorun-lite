use crate::{
    statistics_sync::{date, Day, DeviceDay, Snapshot, Total, MAX_TOTAL},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Account {
    #[serde(default)]
    pub continuity_imported: bool,
    pub cloud: Snapshot,
    pub acknowledged_total: Total,
    pub acknowledged_days: BTreeMap<String, DeviceDay>,
    pub carried_total: Total,
    pub carried_days: BTreeMap<String, DeviceDay>,
    pub reset_pending: bool,
}

impl Account {
    /// A reset in another account cannot lower this account's installation counters.
    pub(crate) fn upload(
        &self,
        total: &Total,
        local: &BTreeMap<String, DeviceDay>,
    ) -> Result<(Total, BTreeMap<String, DeviceDay>)> {
        let total = self.carried_total.overlay(total, &Total::default())?;
        let mut days = self.carried_days.clone();
        for (key, value) in local {
            if let Some(day) = days.get_mut(key) {
                day.revision = add(day.revision, value.revision)?;
                day.words = add(day.words, value.words)?;
                day.speech_seconds = add(day.speech_seconds, value.speech_seconds)?;
                day.recognitions = add(day.recognitions, value.recognitions)?;
                day.recognition_errors = add(day.recognition_errors, value.recognition_errors)?;
                day.style_attempts = add(day.style_attempts, value.style_attempts)?;
                day.style_failures = add(day.style_failures, value.style_failures)?;
                day.style_slow = add(day.style_slow, value.style_slow)?;
                for (bucket, increment) in day
                    .recognition_rtf_buckets
                    .iter_mut()
                    .zip(&value.recognition_rtf_buckets)
                {
                    *bucket = add(*bucket, *increment)?;
                }
            } else {
                days.insert(key.clone(), value.clone());
            }
        }
        Ok((total, days))
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    #[serde(default)]
    pub storage_recovered: bool,
    #[serde(default)]
    pub continuity_imported: bool,
    pub schema_version: u32,
    pub epoch: i64,
    pub total: Total,
    pub days: BTreeMap<String, DeviceDay>,
    pub accounts: BTreeMap<String, Account>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            storage_recovered: false,
            continuity_imported: false,
            schema_version: 1,
            epoch: 0,
            total: Total::default(),
            days: BTreeMap::new(),
            accounts: BTreeMap::new(),
        }
    }
}

pub(crate) fn account_key(key: &str) -> Result<()> {
    if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}
pub(crate) fn counter(value: i64) -> Result<()> {
    if !(0..=MAX_TOTAL).contains(&value) {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}
pub(crate) fn add(left: i64, right: i64) -> Result<i64> {
    let sum = left.checked_add(right).ok_or(Error::InvalidRequest)?;
    counter(sum)?;
    Ok(sum)
}
impl Total {
    pub(crate) fn validate(&self) -> Result<()> {
        for value in [self.words, self.speech_seconds, self.recognitions] {
            counter(value)?;
        }
        Ok(())
    }
    pub(crate) fn covers(&self, other: &Self) -> bool {
        self.words >= other.words
            && self.speech_seconds >= other.speech_seconds
            && self.recognitions >= other.recognitions
    }
    pub(crate) fn acknowledged(&self, other: &Self) -> Self {
        Self {
            words: self.words.max(other.words),
            speech_seconds: self.speech_seconds.max(other.speech_seconds),
            recognitions: self.recognitions.max(other.recognitions),
        }
    }
    pub(crate) fn overlay(&self, local: &Self, acknowledged: &Self) -> Result<Self> {
        Ok(Self {
            words: add(self.words, (local.words - acknowledged.words).max(0))?,
            speech_seconds: add(
                self.speech_seconds,
                (local.speech_seconds - acknowledged.speech_seconds).max(0),
            )?,
            recognitions: add(
                self.recognitions,
                (local.recognitions - acknowledged.recognitions).max(0),
            )?,
        })
    }
}
impl DeviceDay {
    pub(crate) fn total(&self) -> Total {
        Total {
            words: self.words,
            speech_seconds: self.speech_seconds,
            recognitions: self.recognitions,
        }
    }
    pub(crate) fn display(&self) -> Day {
        Day {
            date: self.date.clone(),
            words: self.words,
            speech_seconds: self.speech_seconds,
            recognitions: self.recognitions,
        }
    }
    pub(crate) fn covers(&self, other: &Self) -> bool {
        self.revision >= other.revision
            && self.total().covers(&other.total())
            && self.recognition_errors >= other.recognition_errors
            && self.style_attempts >= other.style_attempts
            && self.style_failures >= other.style_failures
            && self.style_slow >= other.style_slow
            && self
                .recognition_rtf_buckets
                .iter()
                .zip(&other.recognition_rtf_buckets)
                .all(|(a, b)| a >= b)
    }
    pub(crate) fn validate(&self, key: &str) -> Result<()> {
        date(key)?;
        if key != self.date || self.recognition_rtf_buckets.len() != 5 {
            return Err(Error::InvalidRequest);
        }
        self.total().validate()?;
        counter(self.revision)?;
        counter(self.recognition_errors)?;
        counter(self.style_attempts)?; counter(self.style_failures)?; counter(self.style_slow)?;
        if self.style_failures > self.style_attempts || self.style_slow > self.style_attempts - self.style_failures { return Err(Error::InvalidRequest); }
        for value in &self.recognition_rtf_buckets {
            counter(*value)?;
        }
        Ok(())
    }
}
impl State {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.accounts.len() > 64 || self.days.len() > 35 {
            return Err(Error::InvalidRequest);
        }
        counter(self.epoch)?;
        self.total.validate()?;
        let mut total = Total::default();
        for (key, day) in &self.days {
            day.validate(key)?;
            total.words = add(total.words, day.words)?;
            total.speech_seconds = add(total.speech_seconds, day.speech_seconds)?;
            total.recognitions = add(total.recognitions, day.recognitions)?;
        }
        if !self.total.covers(&total) {
            return Err(Error::InvalidRequest);
        }
        for (key, account) in &self.accounts {
            account_key(key)?;
            account.acknowledged_total.validate()?;
            account.carried_total.validate()?;
            if account.acknowledged_days.len() > 35
                || account.carried_days.len() > 35
                || account.cloud.days.len() > 16
                || account.cloud.generation < 0
                || account.cloud.cursor < 0
            {
                return Err(Error::InvalidRequest);
            }
            for (key, day) in &account.acknowledged_days {
                day.validate(key)?;
            }
            for (key, day) in &account.carried_days {
                day.validate(key)?;
            }
            account.upload(&self.total, &self.days)?;
            if let Some(total) = &account.cloud.total {
                total.validate()?;
            }
            for day in &account.cloud.days {
                date(&day.date)?;
                Total {
                    words: day.words,
                    speech_seconds: day.speech_seconds,
                    recognitions: day.recognitions,
                }
                .validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Receipt {
    pub account: String,
    pub epoch: i64,
    pub total: Total,
    pub days: BTreeMap<String, DeviceDay>,
}
