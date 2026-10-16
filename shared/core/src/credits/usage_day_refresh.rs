use crate::Profile;
use chrono::{DateTime, NaiveDate, Utc};
use std::time::Duration;
use tokio::time::Instant;

/// A server UTC-day boundary invalidates freshness, never schedules network I/O.
/// The existing jittered profile poll (or user activity) observes the new day.
/// One attempt per observed bucket prevents an unchanged response retry loop.
#[derive(Default)]
pub(crate) struct UsageDayRefresh {
    bucket: Option<(String, NaiveDate)>,
    due: Option<Instant>,
    attempted: bool,
}

impl UsageDayRefresh {
    pub(crate) fn observe(&mut self, profile: &Profile) {
        let Some((bucket, delay)) = boundary(profile) else {
            *self = Self::default();
            return;
        };
        self.started();
        if self.bucket.as_ref() != Some(&bucket) {
            self.bucket = Some(bucket);
            self.attempted = false;
        }
        self.due = (!self.attempted).then(|| Instant::now() + delay);
    }

    pub(crate) fn delay(&self) -> Option<Duration> {
        self.due.map(|due| due.saturating_duration_since(Instant::now()))
    }

    pub(crate) fn is_due(&self) -> bool {
        self.delay().is_some_and(|delay| delay.is_zero())
    }

    pub(crate) fn fresh(&self) -> bool {
        self.bucket.is_some() && !self.attempted && self.delay().is_some_and(|delay| !delay.is_zero())
    }

    pub(crate) fn started(&mut self) {
        if self.is_due() {
            self.attempted = true;
            self.due = None;
        }
    }

    pub(crate) fn failed(&mut self) {
        self.started();
        self.due = None; // The common failure backoff owns the next attempt.
    }
}

fn boundary(profile: &Profile) -> Option<((String, NaiveDate), Duration)> {
    let balance = &profile.wire()["styleAccess"]["usageBalance"];
    if balance["version"].as_u64()? != 1 { return None; }
    let consumed = |name| match balance.get(name) {
        None => Some(0),
        Some(value) => value.as_u64(),
    };
    if consumed("consumedTotal")? < consumed("consumedToday")?
        && !balance["dailySettlementScopeIds"].as_array().is_some_and(|ids| !ids.is_empty()) {
        return None;
    }
    let scope = balance["scopeId"].as_str()?.to_owned();
    if scope.is_empty() { return None; }
    let day = NaiveDate::parse_from_str(balance["day"].as_str()?, "%Y-%m-%d").ok()?;
    let server = DateTime::parse_from_rfc3339(balance["serverTime"].as_str()?).ok()?.with_timezone(&Utc);
    if server.date_naive() != day { return None; }
    let midnight = day.succ_opt()?.and_hms_opt(0, 0, 0)?.and_utc();
    Some(((scope, day), (midnight - server).to_std().ok()?))
}
