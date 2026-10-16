use super::*;
use chrono::NaiveDate;
use std::collections::BTreeSet;

pub(super) fn scope_id(value: &str) -> Result<()> {
    if crate::request::uuid(value) { Ok(()) } else { Err(Error::InvalidRequest) }
}

pub(crate) fn day(value: &str) -> Result<()> {
    if value.len() == 10 && NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .is_ok_and(|date| date.format("%Y-%m-%d").to_string() == value) {
        Ok(())
    } else { Err(Error::InvalidRequest) }
}

pub(super) fn lease_id(value: &str) -> Result<()> {
    if !value.is_empty() && value.len() <= 128 { Ok(()) } else { Err(Error::InvalidRequest) }
}

pub(super) fn days(values: &Days) -> Result<()> {
    if values.len() > 3660 { return Err(Error::InvalidRequest); }
    for (date, count) in values {
        day(date)?;
        if *count > 1_000_000 { return Err(Error::InvalidRequest); }
    }
    Ok(())
}

pub(super) fn requests(values: &BTreeMap<String, RequestCount>, totals: &Days) -> Result<()> {
    if values.len() > 3660 { return Err(Error::InvalidRequest); }
    for (date, value) in values {
        day(date)?;
        if value.count > 1_000_000 || value.credits < value.count || value.credits > 1_000_000
            || value.credits > value.count * crate::style_inventory::BATCH as u32 || value.credits > *totals.get(date).unwrap_or(&0) {
            return Err(Error::InvalidRequest);
        }
    }
    Ok(())
}

pub(super) fn scope(value: &Scope) -> Result<()> {
    scope_id(&value.scope_id)?;
    day(&value.day)?;
    aliases(&value.scope_id,&value.settlement_scope_ids,&value.daily_settlement_scope_ids)
}
pub(super) fn aliases(primary:&str,full:&[String],daily:&[String])->Result<()> {
    let aliases:Vec<_>=full.iter().chain(daily).collect();
    let distinct:BTreeSet<_>=aliases.iter().map(|id|id.to_ascii_lowercase()).collect();
    if aliases.len()>127 || distinct.len()!=aliases.len() || distinct.contains(&primary.to_ascii_lowercase()) {return Err(Error::InvalidRequest);}
    for alias in aliases {scope_id(alias)?;}Ok(())
}

pub(super) fn ledger(value: &Ledger) -> Result<()> {
    for map in [&value.totals, &value.acknowledged] {
        if map.len() > 128 { return Err(Error::InvalidRequest); }
        for (id, counts) in map { scope_id(id)?; days(counts)?; }
    }
    for (id, counts) in &value.acknowledged {
        for (date, count) in counts {
            if *count > *value.totals.get(id).and_then(|days| days.get(date)).unwrap_or(&0) {
                return Err(Error::InvalidRequest);
            }
        }
    }
    if value.requests.len() > 128 || value.pending.len() > 256 { return Err(Error::InvalidRequest); }
    for (id, counts) in &value.requests {
        scope_id(id)?;
        requests(counts, value.totals.get(id).unwrap_or(&Days::new()))?;
    }
    for (id, reservation) in &value.pending {
        lease_id(id)?;
        day(&reservation.day)?;
        if reservation.credits_by_scope.is_empty() || reservation.credits_by_scope.len() > crate::style_inventory::BATCH {
            return Err(Error::InvalidRequest);
        }
        let mut sum = 0;
        for (scope, count) in &reservation.credits_by_scope {
            scope_id(scope)?;
            if !(1..=crate::style_inventory::BATCH as u32).contains(count) { return Err(Error::InvalidRequest); }
            sum += count;
        }
        if sum > crate::style_inventory::BATCH as u32 || reservation.request_credits.is_some_and(|credits|
            reservation.credits_by_scope.len() != 1 || credits != sum) { return Err(Error::InvalidRequest); }
    }
    Ok(())
}

pub(crate) fn report(value:&Report)->Result<()> {
    scope_id(&value.scope_id)?;
    let totals:Days=value.days.iter().map(|d|(d.date.clone(),d.consumed)).collect();
    if totals.len()!=value.days.len() {return Err(Error::InvalidRequest);}
    days(&totals)?;requests(&value.requests,&totals)
}
