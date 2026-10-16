use crate::{statistics_ledger_model::{account_key, Account, State},
    statistics_sync::{date, DeviceDay, Snapshot, Total, RETENTION_DAYS}, Error, Result};
use chrono::Days;
use serde::Deserialize;
use std::collections::BTreeMap;

/// The numeric counters of the immediately preceding v3 client. Storage adapters
/// read these once; the journal and this marker are committed atomically.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Previous {
    today: String,
    total: Option<Total>,
    days: Vec<DeviceDay>,
    account: Option<PreviousAccount>,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviousAccount {
    id: String,
    generation: i64,
    cloud: Option<Snapshot>,
    acknowledged_total: Total,
    acknowledged_days: Vec<DeviceDay>,
    reset_pending: bool,
}

fn recent(days: Vec<DeviceDay>, start: &str, end: &str) -> Result<BTreeMap<String, DeviceDay>> {
    let mut result = BTreeMap::new();
    if days.len() > 10_000 { return Err(Error::InvalidRequest); }
    for day in days {
        day.validate(&day.date)?;
        if day.date.as_str() >= start && day.date.as_str() <= end
            && result.insert(day.date.clone(), day).is_some() { return Err(Error::InvalidRequest); }
    }
    Ok(result)
}

pub(crate) fn restore(state: &mut State, previous: Previous) -> Result<()> {
    let end = date(&previous.today)?;
    let start = end.checked_sub_days(Days::new(RETENTION_DAYS - 1))
        .ok_or(Error::InvalidRequest)?.to_string();
    // An explicit local/remote reset in the new journal always wins.
    if state.epoch > 0 { state.continuity_imported = true; return Ok(()); }
    state.days.retain(|day, _| day >= &start && day <= &previous.today);
    let previous_total = match previous.total {
        Some(total) => total,
        None => {
            let mut total = Total::default();
            for day in &previous.days { total = total.overlay(&day.total(), &Total::default())?; }
            total
        }
    };
    previous_total.validate()?;
    if !state.continuity_imported {
        let total = previous_total.clone();
        let mut days = recent(previous.days, &start, &previous.today)?;
        for day in days.values_mut() {
            day.revision = day.revision.max(crate::statistics_sync::wire_revision(day));
        }
        let old = Account { carried_total: total, carried_days: days, ..Default::default() };
        let (total, days) = old.upload(&state.total, &state.days)?;
        state.total = total;
        state.days = days;
        state.continuity_imported = true;
    }
    if let Some(previous) = previous.account {
        account_key(&previous.id)?;
        let account = state.accounts.entry(previous.id).or_default();
        if !account.reset_pending && !previous.reset_pending {
            // The old day file may omit lifetime usage already acknowledged by
            // this account. Carry only that missing baseline, so new dictations
            // are counted immediately and retries do not add it again.
            previous.acknowledged_total.validate()?;
            let missing = Total::default().overlay(&previous.acknowledged_total, &previous_total)?;
            account.carried_total = account.carried_total.acknowledged(&missing);
        }
        if !account.continuity_imported {
            if account.cloud.generation == 0 && !account.reset_pending {
                if !(0..=1_000_000_000).contains(&previous.generation) { return Err(Error::InvalidRequest); }
                previous.acknowledged_total.validate()?;
                account.cloud = previous.cloud.unwrap_or(Snapshot { generation: previous.generation, ..Default::default() });
                account.acknowledged_total = previous.acknowledged_total;
                account.acknowledged_days = recent(previous.acknowledged_days, &start, &end.to_string())?;
                account.reset_pending = previous.reset_pending;
            }
            account.continuity_imported = true;
        }
    }
    state.validate()
}

/// Older imported ledgers may still lack the acknowledged lifetime baseline.
/// Reuse the validated, idempotent import repair; unreadable preceding bytes or
/// an explicit reset cannot authorize changing an already-valid current ledger.
pub(crate) fn repair_needed(state: &serde_json::Value, previous: &serde_json::Value) -> bool {
    let Ok(current) = serde_json::from_value::<State>(state.clone()) else {return false;};
    let Ok(previous) = serde_json::from_value::<Previous>(previous.clone()) else {return false;};
    let mut repaired = current.clone();
    restore(&mut repaired, previous).is_ok() && repaired.accounts.iter().any(|(key, account)| {
        current.accounts.get(key).is_some_and(|old| old.carried_total != account.carried_total)
    })
}
