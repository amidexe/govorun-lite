use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// After a successful read only. Recording start, a rejected issuance, reconnect,
/// focus and statistics sync each refresh on demand, so a quiet session needs no
/// tighter poll; the server marks device activity at most hourly anyway.
pub const PROFILE_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);
/// Failed reads and an unavailable account keep the earlier short cadence.
pub const PROFILE_RETRY_INTERVAL: Duration = Duration::from_secs(45);
pub const PROFILE_REFRESH_JITTER: Duration = Duration::from_secs(15);
pub const PROFILE_REFRESH_MAX_BACKOFF: Duration = Duration::from_secs(300);
pub const PROFILE_FOCUS_MAX_AGE: Duration = Duration::from_secs(15);
pub const PROFILE_SYNC_MAX_AGE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ProfileRefreshReason {
    Explicit,
    Background,
    Focus,
    Sync,
}

impl ProfileRefreshReason {
    pub fn max_age(self) -> Duration {
        match self {
            Self::Explicit => Duration::ZERO,
            Self::Background => PROFILE_REFRESH_INTERVAL,
            Self::Focus => PROFILE_FOCUS_MAX_AGE,
            Self::Sync => PROFILE_SYNC_MAX_AGE,
        }
    }
}

/// Volatile scheduling state, never an entitlement or persisted tariff clock.
/// Hosts supply monotonic elapsed time. A ticket fences completion against a
/// newer read or authoritative publication, even within the same account.
#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileRefreshCadence {
    generation: Option<u64>,
    revision: u64,
    observed_at_ms: Option<u64>,
    failures: u8,
    due_at_ms: Option<u64>,
}

impl ProfileRefreshCadence {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.failures > 3 || self.revision == u64::MAX {
            Err(Error::InvalidRequest)
        } else {
            Ok(())
        }
    }

    pub fn begin_scope(&mut self, generation: u64, now_ms: u64, jitter_ms: u64) {
        if self.generation != Some(generation) {
            *self = Self {
                generation: Some(generation),
                ..Self::default()
            };
            self.reschedule(now_ms, jitter_ms);
        }
    }

    pub fn needs_refresh(&self, generation: u64, now_ms: u64, max_age: Duration) -> bool {
        self.generation != Some(generation)
            || self.observed_at_ms.map_or(true, |at| {
                now_ms < at || Duration::from_millis(now_ms - at) >= max_age
            })
    }

    pub fn started(&mut self, generation: u64, now_ms: u64, jitter_ms: u64) -> u64 {
        self.begin_scope(generation, now_ms, jitter_ms);
        self.revision = self.revision.saturating_add(1);
        self.observed_at_ms = Some(now_ms);
        self.reschedule(now_ms, jitter_ms);
        self.revision
    }

    /// Called only for an accepted, current authoritative observation.
    pub fn observe(&mut self, generation: u64, now_ms: u64, jitter_ms: u64, succeeded: bool) {
        self.begin_scope(generation, now_ms, jitter_ms);
        self.revision = self.revision.saturating_add(1);
        self.failures = if succeeded {
            0
        } else {
            self.failures.saturating_add(1).min(3)
        };
        self.observed_at_ms = Some(now_ms);
        self.reschedule(now_ms, jitter_ms);
    }

    pub fn finished(
        &mut self,
        generation: u64,
        ticket: u64,
        now_ms: u64,
        jitter_ms: u64,
        succeeded: bool,
    ) -> bool {
        if self.generation != Some(generation) || self.revision != ticket {
            return false;
        }
        self.observe(generation, now_ms, jitter_ms, succeeded);
        true
    }

    pub fn next_delay(&self, generation: Option<u64>, now_ms: u64, jitter_ms: u64) -> Duration {
        let delay = if generation.is_some() && self.generation == generation {
            self.due_at_ms.map(|due| due.saturating_sub(now_ms))
        } else {
            None
        };
        Duration::from_millis(delay.unwrap_or_else(|| retry_ms(0, jitter_ms)).max(1))
    }

    fn reschedule(&mut self, now_ms: u64, jitter_ms: u64) {
        self.due_at_ms = Some(now_ms.saturating_add(interval_ms(self.failures, jitter_ms)));
    }
}

fn interval_ms(failures: u8, jitter_ms: u64) -> u64 {
    if failures == 0 {
        PROFILE_REFRESH_INTERVAL.as_millis() as u64 + jitter(jitter_ms)
    } else {
        retry_ms(failures, jitter_ms)
    }
}

fn retry_ms(failures: u8, jitter_ms: u64) -> u64 {
    let minimum =
        (PROFILE_RETRY_INTERVAL * (1 << failures.min(3))).min(PROFILE_REFRESH_MAX_BACKOFF);
    minimum.as_millis() as u64 + jitter(jitter_ms)
}

fn jitter(jitter_ms: u64) -> u64 {
    jitter_ms.min(PROFILE_REFRESH_JITTER.as_millis() as u64)
}

pub(crate) fn profile_refresh_jitter(entropy: &std::sync::Arc<dyn Fn() -> u64 + Send + Sync>) -> u64 {
    let maximum = PROFILE_REFRESH_JITTER.as_millis() as u64;
    entropy() % (maximum + 1)
}
