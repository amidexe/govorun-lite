#[path="statistics_persistence.rs"]
pub(crate) mod persistence;
use crate::{
    statistics_ledger_model::{account_key, add, Account, Receipt, State},
    statistics_sync::{self, date, Day, DeviceDay, Response, Snapshot, Total, RETENTION_DAYS},
    Error, Result,
};
use chrono::Days;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Event {
    StyleProcessing { account: String, failed: bool, #[serde(default)] slow: bool },
    Words {
        count: i64,
    },
    Seconds {
        seconds: i64,
    },
    Performance {
        speech_milliseconds: i64,
        processing_milliseconds: i64,
        success: bool,
    },
    Session {
        text: String,
        speech_seconds: i64,
    },
    SessionFacts {
        words: i64,
        speech_seconds: i64,
    },
    RecognitionFacts {
        words: i64,
        speech_milliseconds: i64,
        processing_milliseconds: i64,
        success: bool,
    },
    Recognition {
        text: String,
        speech_milliseconds: i64,
        processing_milliseconds: i64,
        success: bool,
    },
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Action {
    Inspect,
    CountWords {
        text: String,
    },
    Record {
        today: String,
        event: Event,
    },
    Reset {
        account: Option<String>,
    },
    Prepare {
        account: String,
        today: String,
    },
    DeleteAccepted {
        account: String,
        epoch: i64,
    },
    Accept {
        account: String,
        receipt: Receipt,
        response: Response,
    },
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Command {
    state: Option<Value>,
    continuity: Option<Value>,
    #[serde(default)]
    recover_storage: bool,
    event: Action,
}

fn prune(state: &mut State, today: &str) -> Result<()> {
    let end = date(today)?;
    let start = end
        .checked_sub_days(Days::new(RETENTION_DAYS - 1))
        .ok_or(Error::InvalidRequest)?
        .to_string();
    state
        .days
        .retain(|day, _| day.as_str() >= start.as_str() && day.as_str() <= today);
    for account in state.accounts.values_mut() {
        account
            .carried_days
            .retain(|day, _| day.as_str() >= start.as_str() && day.as_str() <= today);
        account
            .acknowledged_days
            .retain(|day, _| day.as_str() >= start.as_str() && day.as_str() <= today);
    }
    Ok(())
}
fn performance(day: &mut DeviceDay, speech: i64, processing: i64, success: bool) -> Result<()> {
    if !success || speech <= 0 || processing < 0 {
        day.recognition_errors = add(day.recognition_errors, 1)?;
    } else {
        let (speech, processing) = (i128::from(speech), i128::from(processing));
        let bucket = if processing * 4 < speech {
            0
        } else if processing * 2 < speech {
            1
        } else if processing < speech {
            2
        } else if processing < speech * 2 {
            3
        } else {
            4
        };
        day.recognition_rtf_buckets[bucket] = add(day.recognition_rtf_buckets[bucket], 1)?;
    }
    Ok(())
}
fn increment(day: &mut DeviceDay, count: i64, seconds: i64) -> Result<()> {
    day.words = add(day.words, count.max(0))?;
    day.speech_seconds = add(day.speech_seconds, seconds.max(0))?;
    if count > 0 {
        day.recognitions = add(day.recognitions, 1)?;
    }
    Ok(())
}
fn recognition(day: &mut DeviceDay, words: i64, speech: i64, processing: i64, success: bool) -> Result<()> {
    if words < 0 { return Err(Error::InvalidRequest); }
    performance(day, speech, processing, success)?;
    increment(day, words, speech_seconds(speech)?)
}
fn record(state: &mut State, today: String, event: Event) -> Result<()> {
    prune(state, &today)?;
    let previous = state.days.get(&today).cloned().unwrap_or(DeviceDay {
        date: today.clone(),
        recognition_rtf_buckets: vec![0; 5],
        ..Default::default()
    });
    let mut next = previous.clone();
    match event {
        Event::StyleProcessing { account, failed, slow } => {
            if failed && slow { return Err(Error::InvalidRequest); }
            account_key(&account)?;
            // Reuse account-scoped day counters: copying local word history
            // to another login must not attribute this account's failures to it.
            let owner = state.accounts.entry(account).or_default();
            if owner.reset_pending { return Ok(()); }
            let day = owner.carried_days.entry(today.clone()).or_insert_with(|| DeviceDay {
                date: today.clone(), recognition_rtf_buckets: vec![0; 5], ..Default::default()
            });
            day.style_attempts = add(day.style_attempts, 1)?;
            if failed { day.style_failures = add(day.style_failures, 1)?; }
            if slow { day.style_slow = add(day.style_slow, 1)?; }
            day.revision = add(day.revision, 1)?;
            return Ok(());
        }
        Event::Words { count } => increment(&mut next, count, 0)?,
        Event::Seconds { seconds } => increment(&mut next, 0, seconds)?,
        Event::Session {
            text,
            speech_seconds,
        } => increment(
            &mut next,
            count_words(&text) as i64,
            speech_seconds,
        )?,
        Event::SessionFacts { words, speech_seconds } => {
            if words < 0 { return Err(Error::InvalidRequest); }
            increment(&mut next, words, speech_seconds)?;
        }
        Event::RecognitionFacts { words, speech_milliseconds, processing_milliseconds, success } =>
            recognition(&mut next, words, speech_milliseconds, processing_milliseconds, success)?,
        Event::Performance {
            speech_milliseconds,
            processing_milliseconds,
            success,
        } => performance(
            &mut next,
            speech_milliseconds,
            processing_milliseconds,
            success,
        )?,
        Event::Recognition {
            text,
            speech_milliseconds,
            processing_milliseconds,
            success,
        } => {
            recognition(&mut next, count_words(&text) as i64,
                speech_milliseconds, processing_milliseconds, success)?;
        }
    }
    if next != previous {
        next.revision = add(previous.revision, 1)?;
        state.total.words = add(state.total.words, next.words - previous.words)?;
        state.total.speech_seconds = add(
            state.total.speech_seconds,
            next.speech_seconds - previous.speech_seconds,
        )?;
        state.total.recognitions = add(
            state.total.recognitions,
            next.recognitions - previous.recognitions,
        )?;
        state.days.insert(today, next);
    }
    Ok(())
}
fn pending(state: &State, account: &Account) -> Result<bool> {
    let (total, days) = account.upload(&state.total, &state.days)?;
    Ok(account.reset_pending
        || total != account.acknowledged_total
        || days.iter().any(|(key, day)| {
            account
                .acknowledged_days
                .get(key)
                .map_or(true, |ack| day.revision > ack.revision)
        }))
}
fn projection(state: &State, account: Option<&Account>) -> Result<Value> {
    let mut days: BTreeMap<String, Day> = BTreeMap::new();
    let total = if let Some(account) = account.filter(|a| a.cloud.total.is_some()) {
        for day in &account.cloud.days {
            days.insert(day.date.clone(), day.clone());
        }
        let (uploaded_total, uploaded_days) = account.upload(&state.total, &state.days)?;
        for (key, local) in &uploaded_days {
            let remote = days.get(key).cloned().unwrap_or_default();
            let ack = account
                .acknowledged_days
                .get(key)
                .map(DeviceDay::total)
                .unwrap_or_default();
            let merged = Total {
                words: remote.words,
                speech_seconds: remote.speech_seconds,
                recognitions: remote.recognitions,
            }
            .overlay(&local.total(), &ack)?;
            days.insert(
                key.clone(),
                Day {
                    date: key.clone(),
                    words: merged.words,
                    speech_seconds: merged.speech_seconds,
                    recognitions: merged.recognitions,
                },
            );
        }
        account
            .cloud
            .total
            .as_ref()
            .unwrap()
            .overlay(&uploaded_total, &account.acknowledged_total)?
    } else {
        days = state
            .days
            .iter()
            .map(|(key, day)| (key.clone(), day.display()))
            .collect();
        state.total.clone()
    };
    let has_pending = match account {
        Some(account) => pending(state, account)?,
        None => state.total != Total::default() || !state.days.is_empty(),
    };
    Ok(
        json!({"total":total,"days":days.into_values().collect::<Vec<_>>(),
        "hasCloud":account.is_some_and(|a| a.cloud.total.is_some()),
        "hasPending":has_pending}),
    )
}
fn reset_local(state: &mut State) -> Result<()> {
    state.epoch = add(state.epoch, 1)?;
    for account in state.accounts.values_mut() {
        let (total, days) = account.upload(&state.total, &state.days)?;
        account.carried_total = total;
        account.carried_days = days;
    }
    state.days.clear();
    state.total = Total::default();
    Ok(())
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let mut state = crate::statistics_recovery::load(command.state.map(persistence::decode), command.continuity, command.recover_storage)?;
    let mut result = json!({"action":"complete"});
    match command.event {
        Action::Inspect => (),
        Action::CountWords { text } => {
            result["count"] = json!(count_words(&text));
        }
        Action::Record { today, event } => record(&mut state, today, event)?,
        Action::Reset { account } => {
            reset_local(&mut state)?;
            if let Some(key) = account {
                account_key(&key)?;
                state.accounts.insert(
                    key,
                    Account {
                        reset_pending: true,
                        ..Default::default()
                    },
                );
            }
        }
        Action::DeleteAccepted { account, epoch } => {
            if epoch != state.epoch {
                return Err(Error::InvalidRequest);
            }
            account_key(&account)?;
            let current = state
                .accounts
                .get_mut(&account)
                .ok_or(Error::InvalidRequest)?;
            if !current.reset_pending {
                return Err(Error::InvalidRequest);
            }
            *current = Account::default();
        }
        Action::Prepare { account, today } => {
            account_key(&account)?;
            prune(&mut state, &today)?;
            let account_state = state.accounts.entry(account.clone()).or_default();
            if account_state.reset_pending {
                result = json!({"action":"delete","epoch":state.epoch});
            } else {
                let (uploaded_total, uploaded_days) =
                    account_state.upload(&state.total, &state.days)?;
                let prepared = statistics_sync::evaluate(statistics_sync::Command::Prepare {
                    today,
                    local_days: uploaded_days.values().cloned().collect(),
                    revisions: uploaded_days
                        .iter()
                        .map(|(key, day)| (key.clone(), day.revision))
                        .collect(),
                    acknowledged: account_state
                        .acknowledged_days
                        .iter()
                        .map(|(key, day)| (key.clone(), day.revision))
                        .collect(),
                })?;
                let sent: BTreeMap<String, i64> = serde_json::from_value(prepared["sent"].clone())
                    .map_err(|_| Error::InvalidResponse)?;
                let receipt = Receipt {
                    account,
                    epoch: state.epoch,
                    total: uploaded_total.clone(),
                    days: sent
                        .keys()
                        .filter_map(|key| {
                            uploaded_days.get(key).map(|day| (key.clone(), day.clone()))
                        })
                        .collect(),
                };
                result = json!({"action":"sync","receipt":receipt,"request":{"schemaVersion":3,
                    "generation":account_state.cloud.generation,"cursor":account_state.cloud.cursor,
                    "total":uploaded_total,"days":prepared["days"]}});
            }
        }
        Action::Accept {
            account,
            receipt,
            response,
        } => {
            account_key(&account)?;
            let current = state
                .accounts
                .get(&account)
                .ok_or(Error::InvalidRequest)?
                .clone();
            let (uploaded_total, uploaded_days) = current.upload(&state.total, &state.days)?;
            if receipt.account != account
                || receipt.epoch != state.epoch
                || !uploaded_total.covers(&receipt.total)
            {
                return Err(Error::InvalidRequest);
            }
            receipt.total.validate()?;
            if receipt.days.len() > 35 {
                return Err(Error::InvalidRequest);
            }
            for (key, day) in &receipt.days {
                day.validate(key)?;
                if uploaded_days
                    .get(key)
                    .is_some_and(|current| !current.covers(day))
                {
                    return Err(Error::InvalidRequest);
                }
            }

            if current.reset_pending {
                return Err(Error::InvalidRequest);
            }
            let accepted = statistics_sync::evaluate(statistics_sync::Command::Accept {
                current: current.cloud,
                response,
            })?;
            let snapshot: Snapshot = serde_json::from_value(accepted["snapshot"].clone())
                .map_err(|_| Error::InvalidResponse)?;
            if accepted["resetLocal"] == true {
                reset_local(&mut state)?;
                state.accounts.insert(
                    account,
                    Account {
                        cloud: snapshot,
                        ..Default::default()
                    },
                );
            } else {
                let current = state
                    .accounts
                    .get_mut(&account)
                    .ok_or(Error::InvalidRequest)?;
                current.cloud = snapshot;
                current.acknowledged_total =
                    current.acknowledged_total.acknowledged(&receipt.total);
                for (key, day) in receipt.days {
                    if uploaded_days.contains_key(&key)
                        && current
                            .acknowledged_days
                            .get(&key)
                            .map_or(true, |ack| day.revision > ack.revision)
                    {
                        current.acknowledged_days.insert(key, day);
                    }
                }
            }
        }
    }
    state.validate()?;
    let mut accounts = BTreeMap::new();
    for (key, account) in &state.accounts {
        accounts.insert(key.clone(), projection(&state, Some(account))?);
    }
    result["projection"] = json!({"local":projection(&state, None)?,"accounts":accounts,
        "storageRecovered":state.storage_recovered});
    let mut stored = serde_json::to_value(state).map_err(|_| Error::InvalidResponse)?;
    let mut view = result["projection"].take();
    persistence::encode(&mut stored, &mut view);
    result["state"] = stored; result["projection"] = view;
    Ok(result)
}

pub(crate) fn count_words(text: &str) -> usize { text.split_whitespace().count() }

pub(crate) fn speech_seconds(milliseconds: i64) -> Result<i64> {
    Ok((milliseconds.max(0).checked_add(500).ok_or(Error::InvalidRequest)? / 1000).max(1))
}
