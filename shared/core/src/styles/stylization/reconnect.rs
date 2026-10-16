//! Process-local reconnect continuation. All adapters execute the same common
//! reconcile → refresh → warm policy with at most one sequential follow-up.
use crate::{AccountSession, Error, Result};
use serde_json::json;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Reconcile,
    Refresh,
    Warm,
    FollowUp,
    FinishSatisfied,
    RetainForLater,
}
struct State {
    epoch: u64,
    generation: u64,
    captured: u64,
    phase: &'static str,
    completed: bool,
    fresh: bool,
    followup_used: bool,
    running: bool,
    ready: bool,
}
impl State {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            generation: 0,
            captured: 0,
            phase: "begin",
            completed: true,
            fresh: false,
            followup_used: false,
            running: false,
            ready: true,
        }
    }
    fn begin(&mut self) {
        self.phase = "begin";
        self.completed = true;
        self.fresh = false;
        self.captured = self.generation;
    }
}
pub struct Reconnect {
    account: AccountSession,
    state: Mutex<State>,
}
impl Reconnect {
    pub fn new(account: AccountSession) -> Self {
        Self {
            state: Mutex::new(State::new(account.snapshot().session_generation)),
            account,
        }
    }
    fn state(&self, epoch: u64) -> Result<std::sync::MutexGuard<'_, State>> {
        self.account.current(epoch)?;
        let mut state = self.state.lock().map_err(|_| Error::InvalidResponse)?;
        if state.epoch != epoch {
            *state = State::new(epoch);
        }
        Ok(state)
    }
    pub fn unavailable(&self, epoch: u64) -> Result<()> {
        let mut state = self.state(epoch)?;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(Error::InvalidRequest)?;
        state.ready = false;
        Ok(())
    }
    /// True admits a runner. False joins existing work or a satisfied demand.
    pub fn request(&self, epoch: u64, user_demand: bool) -> Result<bool> {
        let mut state = self.state(epoch)?;
        if user_demand && state.ready {
            return Ok(false);
        }
        if state.running {
            state.fresh = true;
            return Ok(false);
        }
        if state.ready {
            state.generation = state
                .generation
                .checked_add(1)
                .ok_or(Error::InvalidRequest)?;
            state.ready = false;
        }
        state.running = true;
        state.followup_used = false;
        state.begin();
        Ok(true)
    }
    pub fn next(&self, epoch: u64, can_warm: bool, online: bool) -> Result<Step> {
        let mut state = self.state(epoch)?;
        if !state.running {
            return Err(Error::InvalidRequest);
        }
        let snapshot = self.account.snapshot();
        let facts = serde_json::from_value(json!({"phase":state.phase,"completed":state.completed,
            "freshDemandPending":state.fresh,"observationChanged":state.captured!=state.generation,
            "followupUsed":state.followup_used,"canWarm":can_warm,
            "allowed":snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending && online}))
        .map_err(|_| Error::InvalidRequest)?;
        let action = crate::reconnect_recovery::action(facts);
        let step = match action {
            "reconcile" => {
                state.phase = "reconcile";
                Step::Reconcile
            }
            "refresh" => {
                state.phase = "refresh";
                Step::Refresh
            }
            "warm" => {
                state.phase = "warm";
                Step::Warm
            }
            "runSequentialFollowUp" => {
                state.followup_used = true;
                state.begin();
                Step::FollowUp
            }
            "finishSatisfied" => {
                if state.captured == state.generation {
                    state.ready = true;
                }
                state.running = false;
                Step::FinishSatisfied
            }
            "retainForLaterTrigger" => {
                state.running = false;
                Step::RetainForLater
            }
            _ => return Err(Error::InvalidResponse),
        };
        self.account.current(epoch)?;
        Ok(step)
    }
    pub fn complete(&self, epoch: u64, completed: bool) -> Result<()> {
        let mut state = self.state(epoch)?;
        if !state.running {
            return Err(Error::InvalidRequest);
        }
        state.completed = completed;
        Ok(())
    }
    /// One atomic readiness observation: ready, running, or retained failure.
    pub fn readiness(&self, epoch: u64) -> Result<Option<bool>> {
        let state = self.state(epoch)?;
        Ok(if state.ready {
            Some(true)
        } else if state.running {
            None
        } else {
            Some(false)
        })
    }
    pub fn ready(&self, epoch: u64) -> Result<bool> {
        Ok(self.state(epoch)?.ready)
    }
    pub fn cancel(&self, epoch: u64) -> Result<()> {
        let mut state = self.state(epoch)?;
        state.running = false;
        state.fresh = false;
        Ok(())
    }
}
