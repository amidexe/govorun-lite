//! Accounts-only cumulative numeric usage. The storage adapter commits a returned
//! ledger atomically with lease usage confirmation; this module never sends a report.
mod rules;
pub(crate) mod balance_binding;
pub(crate) mod validation;

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Days = BTreeMap<String, u32>;
pub type ScopeDays = BTreeMap<String, Days>;
pub type Requests = BTreeMap<String, BTreeMap<String, RequestCount>>;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ledger {
    pub totals: ScopeDays,
    pub acknowledged: ScopeDays,
    #[serde(default)]
    pub requests: Requests,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pending: BTreeMap<String, Reservation>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RequestCount {
    pub count: u32,
    pub credits: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reservation {
    pub day: String,
    pub credits_by_scope: BTreeMap<String, u32>,
    pub request_credits: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Day {
    pub date: String,
    pub consumed: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub scope_id: String,
    pub days: Vec<Day>,
    #[serde(default)]
    pub requests: BTreeMap<String, RequestCount>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scope {
    pub scope_id: String,
    pub day: String,
    pub settlement_scope_ids: Vec<String>,
    pub daily_settlement_scope_ids: Vec<String>,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Event {
    Validate,
    Record { day: Option<String>, scopes: Vec<Option<String>> },
    Begin { lease_id: String, day: Option<String>, scopes: Vec<Option<String>>, admitted_scope: Option<String> },
    Settle { lease_id: String, consumed: bool },
    Observe { scope: Scope, device_days: Days, device_requests: BTreeMap<String, RequestCount>,
        protected_scopes: Vec<String> },
    Acknowledge { report: Report, days: Vec<Day> },
    Inspect { scope: Scope },
    Outstanding { scope_id: String, day: Option<String> },
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub ledger: Ledger,
    pub reports: Vec<Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outstanding_today: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outstanding_total: Option<u32>,
}

/// A rejected command produces no candidate state. The caller must preserve its
/// current durable envelope on every error, cancellation or failed write.
pub fn apply(mut ledger: Ledger, event: Event) -> Result<Projection> {
    validation::ledger(&ledger)?;
    let mut today = None;
    let mut total = None;
    match event {
        Event::Validate => (),
        Event::Record { day, scopes } => {
            if let Some(reservation) = rules::reservation(day, scopes, None)? {
                rules::record(&mut ledger, reservation)?;
            }
        }
        Event::Begin { lease_id, day, scopes, admitted_scope } => {
            validation::lease_id(&lease_id)?;
            if ledger.pending.contains_key(&lease_id) { return Err(Error::InvalidRequest); }
            if let Some(reservation) = rules::reservation(day, scopes, admitted_scope)? {
                ledger.pending.insert(lease_id, reservation);
            }
        }
        Event::Settle { lease_id, consumed } => {
            validation::lease_id(&lease_id)?;
            if let Some(reservation) = ledger.pending.remove(&lease_id) {
                if consumed { rules::record(&mut ledger, reservation)?; }
            }
        }
        Event::Observe { scope, device_days, device_requests, protected_scopes } => {
            rules::observe(&mut ledger, &scope, device_days, device_requests, protected_scopes)?;
            (today, total) = rules::outstanding(&ledger, &scope);
        }
        Event::Acknowledge { report, days } => rules::acknowledge(&mut ledger, report, days)?,
        Event::Inspect { scope } => {
            validation::scope(&scope)?;
            (today, total) = rules::outstanding(&ledger, &scope);
        }
        Event::Outstanding { scope_id, day } => {
            validation::scope_id(&scope_id)?;
            if let Some(day) = &day { validation::day(day)?; }
            total = Some(rules::debt(&ledger, &scope_id, day.as_deref()));
        }
    }
    validation::ledger(&ledger)?;
    Ok(Projection { reports: rules::reports(&ledger), ledger,
        outstanding_today: today, outstanding_total: total })
}
