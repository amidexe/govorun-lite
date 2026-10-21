use crate::{Error, Result};
use serde::{Deserialize, Serialize};

const RETRY_DELAYS: [u64; 4] = [1_000, 3_000, 10_000, 30_000];
// Consecutive exhausted bursts wait longer, from 30 s up to 4 h. Only Ready
// resets the step; demand and reconnect still wake a local wait at once.
const FAILURE_COOLDOWNS: [u64; 6] = [30_000, 120_000, 600_000, 1_800_000, 3_600_000, 14_400_000];

/// An opaque, in-memory continuation. Native executors retain it unchanged;
/// only this reducer owns budgets, coalescing, wakeups and completion ordering.
#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    epoch: u64,
    clock: u64,
    ticket: u64,
    phase: Phase,
    attempts: usize,
    #[serde(default)]
    failures: usize,
    requested: bool,
    fresh_followup: bool,
    pending_demand: bool,
    explicit_demand: bool,
    local_not_before: u64,
    server_not_before: u64,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Phase { #[default] Idle, Running, Waiting }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Trigger { Maintenance, Reconnect, Demand }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Outcome { Ready, Retry, Terminal, Offline, Blocked, Cancelled, HttpFailure, Progress }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Request { trigger: Trigger, join: bool },
    Complete { ticket: u64, outcome: Outcome, retry_after_millis: Option<u64>, status: Option<u16> },
    Tick { ticket: u64 },
    Cancel,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum Action {
    None,
    Idle { failure_reason: Option<&'static str> },
    Run { ticket: u64, explicit_demand: bool },
    Wait { ticket: u64, milliseconds: u64, wakeable: bool, cycle_complete: bool,
        failure_reason: Option<&'static str> },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Transition { state: State, idle: bool, action: Action }

pub(crate) fn transition(state: Option<State>, event: Event, epoch: u64,
    now_millis: u64, allowed: bool) -> Result<Transition> {
    // Preserve ticket identity when account scope changes: a late native
    // completion must never collide with the first operation of the new scope.
    let ticket = state.as_ref().map_or(0, |s| s.ticket);
    let mut state = state.filter(|s| s.epoch == epoch).unwrap_or(State { epoch, ticket, ..State::default() });
    if epoch == 0 && allowed || now_millis < state.clock || state.attempts > RETRY_DELAYS.len() + 1
        || state.failures > FAILURE_COOLDOWNS.len()
        || state.phase != Phase::Idle && state.ticket == 0
        || state.phase == Phase::Running && state.attempts == 0 {
        return Err(Error::InvalidRequest);
    }
    state.clock = now_millis;
    let action = if !allowed { state.stop() } else { state.apply(event)? };
    Ok(Transition { idle: state.phase == Phase::Idle, state, action })
}

impl State {
    fn apply(&mut self, event: Event) -> Result<Action> {
        match event {
            Event::Cancel => Ok(self.stop()),
            Event::Request { trigger, join } => {
                let demand = matches!(trigger, Trigger::Demand);
                let fresh = !matches!(trigger, Trigger::Maintenance);
                let active = self.phase != Phase::Idle;
                // Joining work already admitted for explicit demand does not
                // manufacture an additional cycle. Upgrading background work does.
                if !active || !join || demand && !self.explicit_demand {
                    self.requested = true;
                    self.fresh_followup |= fresh;
                    self.pending_demand |= demand;
                }
                if !active { return self.begin_cycle(fresh); }
                if fresh && self.phase == Phase::Waiting {
                    self.local_not_before = self.clock;
                    // A connectivity signal cannot shorten a server deadline.
                    return self.advance();
                }
                Ok(Action::None)
            }
            Event::Tick { ticket } => {
                if self.phase != Phase::Waiting || ticket != self.ticket { return Ok(Action::None); }
                self.advance()
            }
            Event::Complete { ticket, outcome, retry_after_millis, status } => {
                if self.phase != Phase::Running || ticket != self.ticket { return Ok(Action::None); }
                let outcome = match outcome {
                    Outcome::HttpFailure => match status {
                        Some(408 | 429 | 500..=599) => Outcome::Retry,
                        Some(400..=499) => Outcome::Terminal,
                        _ => return Err(Error::InvalidRequest),
                    },
                    other => other,
                };
                if let Some(delay) = retry_after_millis.filter(|_| matches!(&outcome, Outcome::Retry | Outcome::Terminal)) {
                    self.server_not_before = self.server_not_before.max(
                        self.clock.checked_add(delay).ok_or(Error::InvalidRequest)?);
                }
                match outcome {
                    Outcome::Ready => {
                        self.failures = 0;
                        self.local_not_before = self.clock;
                        if self.requested { self.begin_cycle(true) } else { Ok(self.stop()) }
                    }
                    Outcome::Progress => {
                        // Successful replay can leave expired passes to retire.
                        // Continue that work without resetting the wire budget.
                        if self.attempts > RETRY_DELAYS.len() {
                            return self.schedule_followup(Some("style_pass_issue_failed"));
                        }
                        self.local_not_before = self.clock;
                        self.advance()
                    }
                    Outcome::Retry => {
                        if self.attempts > RETRY_DELAYS.len() {
                            if self.fresh_followup {
                                self.pending_demand |= self.explicit_demand;
                                return self.begin_cycle(true);
                            }
                            return self.schedule_followup(Some("style_pass_issue_failed"));
                        }
                        // The ongoing retry covers ordinary duplicate requests.
                        self.requested = false;
                        self.local_not_before = self.clock.saturating_add(RETRY_DELAYS[self.attempts - 1]);
                        self.advance()
                    }
                    Outcome::Terminal => Ok(self.fail("account_state_changed")),
                    // The cancelled effect belongs to the old request. A newer
                    // explicit preparation still needs its own bounded cycle.
                    Outcome::Cancelled | Outcome::Offline | Outcome::Blocked if self.fresh_followup => {
                        self.pending_demand |= self.explicit_demand;
                        self.begin_cycle(true)
                    }
                    Outcome::Blocked => self.schedule_followup(None),
                    Outcome::Offline => {
                        let demand = self.explicit_demand || self.pending_demand;
                        let action = self.stop();
                        self.pending_demand = demand;
                        Ok(action)
                    }
                    Outcome::Cancelled => Ok(self.stop()),
                    Outcome::HttpFailure => unreachable!("HTTP outcome normalized above"),
                }
            }
        }
    }

    fn begin_cycle(&mut self, fresh: bool) -> Result<Action> {
        self.attempts = 0;
        self.explicit_demand = self.pending_demand;
        self.pending_demand = false;
        self.requested = false;
        self.fresh_followup = false;
        if fresh { self.local_not_before = self.clock; }
        self.advance()
    }

    fn schedule_followup(&mut self, reason: Option<&'static str>) -> Result<Action> {
        // Exhaustion bounds one wire burst, not the lifetime of recovery.
        // Keep a cancellable timer when the server recovers without a path
        // change. Reconnect can wake it; server Retry-After still wins.
        self.attempts = 0;
        self.requested = false;
        self.fresh_followup = false;
        let step = self.failures.min(FAILURE_COOLDOWNS.len() - 1);
        self.failures = (step + 1).min(FAILURE_COOLDOWNS.len());
        self.local_not_before = self.clock.saturating_add(FAILURE_COOLDOWNS[step]);
        let mut action = self.advance()?;
        if let Action::Wait { failure_reason, .. } = &mut action { *failure_reason = reason; }
        Ok(action)
    }

    fn next_ticket(&mut self) -> Result<u64> {
        self.ticket = self.ticket.checked_add(1).ok_or(Error::InvalidRequest)?;
        Ok(self.ticket)
    }

    fn advance(&mut self) -> Result<Action> {
        let deadline = self.local_not_before.max(self.server_not_before);
        if self.clock < deadline {
            self.phase = Phase::Waiting;
            return Ok(Action::Wait { ticket: self.next_ticket()?,
                milliseconds: (deadline - self.clock).min(60_000),
                wakeable: self.clock >= self.server_not_before,
                cycle_complete: self.attempts == 0, failure_reason: None });
        }
        self.run()
    }

    fn run(&mut self) -> Result<Action> {
        self.explicit_demand |= self.pending_demand;
        self.pending_demand = false;
        self.requested = false;
        self.phase = Phase::Running;
        self.attempts += 1;
        Ok(Action::Run { ticket: self.next_ticket()?, explicit_demand: self.explicit_demand })
    }

    fn stop(&mut self) -> Action {
        self.phase = Phase::Idle;
        self.requested = false;
        self.pending_demand = false;
        self.explicit_demand = false;
        self.fresh_followup = false;
        Action::Idle { failure_reason: None }
    }

    fn fail(&mut self, reason: &'static str) -> Action {
        self.stop();
        Action::Idle { failure_reason: Some(reason) }
    }
}
