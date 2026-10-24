use crate::profile_cadence::profile_refresh_jitter;
use crate::{
    AccountSession, Error, Profile, ProfileActivity, ProfileAdmission, Result, PROFILE_REFRESH_INTERVAL,
    PROFILE_SYNC_MAX_AGE,
};
use std::time::Duration;
use tokio::{sync::watch, time::Instant};

/// One owner for freshness, in-flight reads and retry cadence. Stored under the
/// account state lock: a session boundary and its observations change atomically.
pub(crate) struct ActivityRefresh {
    entropy: std::sync::Arc<dyn Fn() -> u64 + Send + Sync>,
    generation: u64,
    observation: Option<Result<()>>,
    clock: Instant,
    owner: ProfileActivity,
    usage_day: crate::usage_day_refresh::UsageDayRefresh,
    pending: Option<(u64, watch::Receiver<Option<Result<Profile>>>)>,
}

impl ActivityRefresh {
    pub(crate) fn new(entropy: std::sync::Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        let mut owner = ProfileActivity::default();
        owner.begin_scope(0, 0, profile_refresh_jitter(&entropy));
        Self {
            entropy,
            generation: 0,
            observation: None,
            clock: Instant::now(),
            owner,
            usage_day: Default::default(),
            pending: None,
        }
    }
}

impl ActivityRefresh {
    fn scope(&mut self, generation: u64) {
        if self.generation != generation {
            *self = Self {
                generation,
                ..Self::new(self.entropy.clone())
            };
            self.owner
                .begin_scope(generation, 0, profile_refresh_jitter(&self.entropy));
        }
    }

    fn now_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }

    pub(crate) fn style_usage_fresh(&self, profile_fresh: bool) -> bool {
        profile_fresh && self.usage_day.fresh()
    }

    /// Full authoritative profile responses, including explicit mutations, all
    /// reset the same cadence. Superseded responses must never call this method.
    pub(crate) fn observe(&mut self, generation: u64, result: Result<()>) {
        self.scope(generation);
        if result.is_err() { self.usage_day.failed(); }
        self.owner.observe(
            generation,
            self.now_ms(),
            profile_refresh_jitter(&self.entropy),
            result.is_ok(),
        );
        self.observation = Some(result);
    }

    pub(crate) fn profile_published(&mut self, generation: u64, profile: &Profile, full: bool) {
        self.scope(generation);
        self.usage_day.observe(profile);
        if full { self.observe(generation, Ok(())); }
    }
}

impl AccountSession {
    /// The host only drives this coordinator. It owns no second retry policy.
    pub fn activity_refresh_delay(&self) -> Duration {
        let mut state = self.state();
        let epoch = state.epoch;
        state.activity.scope(epoch);
        if state.tokens.is_none() {
            // Nothing to read until sign-in, and sign-in wakes the host loop itself.
            return PROFILE_REFRESH_INTERVAL
                + Duration::from_millis(profile_refresh_jitter(&self.inner.entropy));
        }
        if !state.storage_available {
            return state.activity.owner.next_delay(
                None,
                state.activity.now_ms(),
                profile_refresh_jitter(&self.inner.entropy),
            );
        }
        state.activity.owner.next_delay(
            Some(epoch),
            state.activity.now_ms(),
            profile_refresh_jitter(&self.inner.entropy),
        ) // UTC changes freshness, never the network poll schedule.
    }

    /// Part of the existing maintenance RPC so shutdown drains its storage work.
    /// Profile failure must not discard the successful statistics acknowledgement.
    pub async fn refresh_after_sync(&self, epoch: u64) -> Result<()> {
        self.refresh_for_activity_at(epoch, PROFILE_SYNC_MAX_AGE)
            .await
            .map(|_| ())
    }

    /// Lifecycle, keyboard demand and screen refresh share a single profile read.
    /// Explicit mutations retain their own ordered authoritative responses.
    /// The task belongs to the session: closing a screen never cancels refresh.
    pub async fn refresh_for_activity(&self, max_age: Duration) -> Result<Profile> {
        let epoch = self.lease()?.epoch;
        self.refresh_for_activity_at(epoch, max_age).await
    }

    pub async fn refresh_for_activity_at(&self, epoch: u64, max_age: Duration) -> Result<Profile> {
        let mut result = {
            let mut state = self.state();
            if state.epoch != epoch {
                return Err(Error::SessionChanged);
            }
            if state.tokens.is_none() {
                return Err(Error::SignedOut);
            }
            state.activity.scope(epoch);
            let now = state.activity.now_ms();
            let max_age = if state.activity.usage_day.is_due() { Duration::ZERO } else { max_age };
            // The common owner chooses join/start. Cache interpretation remains
            // in this account owner because it holds the authoritative profile.
            let admission = state.activity.owner.request(
                epoch,
                now,
                max_age,
                false,
                profile_refresh_jitter(&self.inner.entropy),
            )?;
            let admission = if admission == ProfileAdmission::Current {
                if let Some(outcome) = state.activity.observation {
                    outcome?;
                    if state.profile_fresh {
                        return state.profile.clone().ok_or(Error::InvalidResponse);
                    }
                }
                state.activity.owner.request(
                    epoch,
                    now,
                    Duration::ZERO,
                    false,
                    profile_refresh_jitter(&self.inner.entropy),
                )?
            } else {
                admission
            };
            match admission {
                ProfileAdmission::Join { ticket } => state
                    .activity
                    .pending
                    .as_ref()
                    .filter(|(pending, _)| *pending == ticket)
                    .map(|(_, receiver)| receiver.clone())
                    .ok_or(Error::InvalidResponse)?,
                ProfileAdmission::Start { ticket, .. } => {
                    state.activity.usage_day.started();
                    let (sender, receiver) = watch::channel(None);
                    state.activity.pending = Some((ticket, receiver.clone()));
                    let session = self.clone();
                    tokio::spawn(async move {
                        let outcome = session.refresh_profile_at(epoch).await;
                        let mut state = session.state();
                        if state.epoch == epoch {
                            let now = state.activity.now_ms();
                            if state.activity.owner.finished(
                                epoch,
                                ticket,
                                now,
                                profile_refresh_jitter(&session.inner.entropy),
                                outcome.is_ok(),
                            ) {
                                state.activity.pending = None;
                            }
                        }
                        sender.send_replace(Some(outcome));
                    });
                    receiver
                }
                ProfileAdmission::Current => return Err(Error::InvalidResponse),
                ProfileAdmission::Superseded => return Err(Error::SessionChanged),
            }
        };
        loop {
            let completed = result.borrow().clone();
            {
                let state = self.state();
                if state.epoch != epoch {
                    return Err(Error::SessionChanged);
                }
                if let Some(completed) = completed {
                    // A newer authoritative success wins over an older failed read.
                    if state.profile_fresh {
                        return state.profile.clone().ok_or(Error::InvalidResponse);
                    }
                    completed?;
                    if let Some(outcome) = state.activity.observation {
                        outcome?;
                    }
                    return Err(Error::InvalidResponse);
                }
            }
            result.changed().await.map_err(|_| Error::SessionChanged)?;
        }
    }
}
