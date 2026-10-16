use super::*;
use std::collections::BTreeSet;

pub(super) fn reservation(day: Option<String>, scopes: Vec<Option<String>>, admitted: Option<String>)
    -> Result<Option<Reservation>> {
    if scopes.is_empty() || scopes.len() > crate::style_inventory::BATCH { return Err(Error::InvalidRequest); }
    let Some(day) = day else { return Ok(None); };
    validation::day(&day)?;
    let size = scopes.len() as u32;
    let mut counts = BTreeMap::new();
    if let Some(scope) = admitted {
        validation::scope_id(&scope)?;
        counts.insert(scope, size);
    } else {
        for scope in scopes.into_iter().flatten() {
            validation::scope_id(&scope)?;
            *counts.entry(scope).or_insert(0) += 1;
        }
    }
    if counts.is_empty() { return Ok(None); }
    let request_credits = (counts.len() == 1 && counts.values().sum::<u32>() == size).then_some(size);
    Ok(Some(Reservation { day, credits_by_scope: counts, request_credits }))
}

pub(super) fn record(ledger: &mut Ledger, reservation: Reservation) -> Result<()> {
    for (scope, credits) in reservation.credits_by_scope {
        let total = ledger.totals.entry(scope.clone()).or_default().entry(reservation.day.clone()).or_default();
        *total = total.checked_add(credits).ok_or(Error::InvalidRequest)?;
        if let Some(credits) = reservation.request_credits {
            let count = ledger.requests.entry(scope).or_default().entry(reservation.day.clone()).or_default();
            count.count = count.count.checked_add(1).ok_or(Error::InvalidRequest)?;
            count.credits = count.credits.checked_add(credits).ok_or(Error::InvalidRequest)?;
        }
    }
    Ok(())
}

fn merge(ledger: &mut Ledger, scope: String, days: Days) {
    for (date, count) in days {
        for map in [&mut ledger.totals, &mut ledger.acknowledged] {
            let old = map.entry(scope.clone()).or_default().entry(date.clone()).or_default();
            *old = (*old).max(count);
        }
    }
}

pub(super) fn observe(ledger: &mut Ledger, scope: &Scope, days: Days,
    counts: BTreeMap<String, RequestCount>, external: Vec<String>) -> Result<()> {
    validation::scope(scope)?;
    validation::days(&days)?;
    validation::requests(&counts, &days)?;
    // A protected lease set may span more scopes than the retained usage map.
    if external.len() > 256 * crate::style_inventory::BATCH { return Err(Error::InvalidRequest); }
    for id in &external { validation::scope_id(id)?; }
    let protected: BTreeSet<_> = external.iter().chain(std::iter::once(&scope.scope_id))
        .chain(&scope.settlement_scope_ids).chain(&scope.daily_settlement_scope_ids)
        .chain(ledger.pending.values().flat_map(|value| value.credits_by_scope.keys())).cloned().collect();
    let retired: Vec<_> = ledger.totals.keys().chain(ledger.acknowledged.keys())
        .filter(|id| !protected.contains(*id) && debt(ledger, id, None) == 0).cloned().collect();
    for id in retired { ledger.totals.remove(&id); ledger.acknowledged.remove(&id); ledger.requests.remove(&id); }
    for (day, count) in counts {
        let old = ledger.requests.entry(scope.scope_id.clone()).or_default().entry(day).or_default();
        old.count = old.count.max(count.count);
        old.credits = old.credits.max(count.credits);
    }
    merge(ledger, scope.scope_id.clone(), days);
    Ok(())
}

fn as_days(values: Vec<Day>) -> Result<Days> {
    let mut days = Days::new();
    if values.len() > 36 { return Err(Error::InvalidRequest); }
    for value in values {
        if days.insert(value.date, value.consumed).is_some() { return Err(Error::InvalidRequest); }
    }
    validation::days(&days)?;
    Ok(days)
}

pub(super) fn acknowledge(ledger: &mut Ledger, report: Report, received: Vec<Day>) -> Result<()> {
    validation::scope_id(&report.scope_id)?;
    let sent = as_days(report.days)?;
    validation::requests(&report.requests, &sent)?;
    let days = as_days(received)?;
    if sent.is_empty() || sent.keys().ne(days.keys()) || sent.iter().any(|(date, count)| days[date] < *count) {
        return Err(Error::InvalidRequest);
    }
    merge(ledger, report.scope_id, days);
    Ok(())
}

pub(super) fn debt(ledger: &Ledger, scope: &str, day: Option<&str>) -> u32 {
    ledger.totals.get(scope).into_iter().flat_map(|days| days.iter())
        .filter(|(date, _)| day.is_none_or(|day| *date == day))
        .map(|(date, count)| count.saturating_sub(*ledger.acknowledged.get(scope)
            .and_then(|days| days.get(date)).unwrap_or(&0)) as u64)
        .sum::<u64>().min(i32::MAX as u64) as u32
}

pub(super) fn outstanding(ledger: &Ledger, scope: &Scope) -> (Option<u32>, Option<u32>) {
    let full = std::iter::once(&scope.scope_id).chain(&scope.settlement_scope_ids);
    let total = full.clone().map(|id| u64::from(debt(ledger, id, None))).sum::<u64>();
    let today = full.chain(&scope.daily_settlement_scope_ids)
        .map(|id| u64::from(debt(ledger, id, Some(&scope.day)))).sum::<u64>();
    (Some(today.min(i32::MAX as u64) as u32), Some(total.min(i32::MAX as u64) as u32))
}

pub(super) fn reports(ledger: &Ledger) -> Vec<Report> {
    ledger.totals.iter().filter_map(|(scope, days)| {
        let days: Vec<_> = days.iter().filter(|(day, count)| **count > *ledger.acknowledged
            .get(scope).and_then(|values| values.get(*day)).unwrap_or(&0)).take(36)
            .map(|(date, consumed)| Day { date: date.clone(), consumed: *consumed }).collect();
        if days.is_empty() { return None; }
        let requests = ledger.requests.get(scope).into_iter().flat_map(|values| values.iter())
            .filter(|(date, _)| days.iter().any(|day| &day.date == *date))
            .map(|(date, count)| (date.clone(), count.clone())).collect();
        Some(Report { scope_id: scope.clone(), days, requests })
    }).collect()
}
