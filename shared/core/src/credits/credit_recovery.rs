use crate::{Error, Result};
use serde::{Deserialize, Serialize};

const WIRE_BUDGET: u8 = 5;
const FAILURE_DELAY: u64 = 50_000;

/// Recovery counts network opportunities, not inspections of a lease which is
/// still waiting. The continuation is opaque to native I/O executors.
#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    epoch: u64,
    clock: u64,
    ticket: u64,
    phase: Phase,
    attempts: u8,
    requested: bool,
    fresh_followup: bool,
    followup_used: bool,
    not_before: u64,
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
pub(crate) enum Outcome { Ready, Retry, Failed, Offline, Cancelled }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Request { trigger: Trigger, join: bool, #[serde(default)] delay_millis: u64 },
    Complete { ticket: u64, outcome: Outcome, wire_attempted: bool, retry_after_millis: Option<u64> },
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
    Wait { ticket: u64, milliseconds: u64, wakeable: bool },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Transition { state: State, idle: bool, action: Action }

pub(crate) fn transition(state: Option<State>, event: Event, epoch: u64,
    now_millis: u64, allowed: bool) -> Result<Transition> {
    let ticket = state.as_ref().map_or(0, |s| s.ticket);
    let mut state = state.filter(|s| s.epoch == epoch)
        .unwrap_or(State { epoch, ticket, ..State::default() });
    if epoch == 0 && allowed || now_millis < state.clock || state.attempts > WIRE_BUDGET
        || state.phase != Phase::Idle && state.ticket == 0 {
        return Err(Error::InvalidRequest);
    }
    state.clock = now_millis;
    let action = if allowed { state.apply(event)? } else { state.stop() };
    Ok(Transition { idle: state.phase == Phase::Idle, state, action })
}

impl State {
    fn apply(&mut self, event: Event) -> Result<Action> {
        match event {
            Event::Cancel => Ok(self.stop()),
            Event::Request { trigger, join, delay_millis } => {
                if self.phase == Phase::Idle {
                    self.attempts = 0;
                    self.followup_used = false;
                    self.defer(delay_millis.min(FAILURE_DELAY))?;
                    return self.advance();
                }
                if !join {
                    if !matches!(trigger, Trigger::Maintenance) && !self.followup_used {
                        self.fresh_followup = true;
                    } else if matches!(trigger, Trigger::Maintenance) && self.phase == Phase::Running {
                        self.requested = true;
                    }
                }
                // Neither a duplicate nor fresh demand shortens an existing
                // ambiguity grace or server deadline.
                Ok(Action::None)
            }
            Event::Tick { ticket } => {
                if self.phase != Phase::Waiting || ticket != self.ticket { return Ok(Action::None); }
                self.advance()
            }
            Event::Complete { ticket, outcome, wire_attempted, retry_after_millis } => {
                if self.phase != Phase::Running || ticket != self.ticket { return Ok(Action::None); }
                if wire_attempted { self.attempts += 1; }
                match outcome {
                    Outcome::Offline | Outcome::Cancelled => Ok(self.stop()),
                    Outcome::Ready if !self.requested => Ok(self.stop()),
                    Outcome::Ready => self.continue_cycle(),
                    Outcome::Retry | Outcome::Failed => {
                        // A failed native step may not know whether bytes were
                        // sent. Bound such failures as opportunities too.
                        if matches!(outcome, Outcome::Failed) && !wire_attempted { self.attempts += 1; }
                        let delay = retry_after_millis.unwrap_or(
                            if wire_attempted || matches!(outcome, Outcome::Failed) { FAILURE_DELAY } else { 250 });
                        self.defer(delay.clamp(250, FAILURE_DELAY))?;
                        self.continue_cycle()
                    }
                }
            }
        }
    }

    fn continue_cycle(&mut self) -> Result<Action> {
        self.requested = false;
        if self.attempts >= WIRE_BUDGET {
            if self.fresh_followup && !self.followup_used {
                self.attempts = 0;
                self.followup_used = true;
                self.fresh_followup = false;
            } else { return Ok(self.stop()); }
        }
        self.advance()
    }

    fn defer(&mut self, delay: u64) -> Result<()> {
        self.not_before = self.not_before.max(self.clock.checked_add(delay).ok_or(Error::InvalidRequest)?);
        Ok(())
    }

    fn advance(&mut self) -> Result<Action> {
        self.ticket = self.ticket.checked_add(1).ok_or(Error::InvalidRequest)?;
        if self.clock < self.not_before {
            self.phase = Phase::Waiting;
            Ok(Action::Wait { ticket: self.ticket, milliseconds: (self.not_before - self.clock).min(50_000), wakeable: false })
        } else {
            self.phase = Phase::Running;
            Ok(Action::Run { ticket: self.ticket, explicit_demand: false })
        }
    }

    fn stop(&mut self) -> Action {
        self.phase = Phase::Idle;
        self.requested = false;
        self.fresh_followup = false;
        Action::Idle { failure_reason: None }
    }
}
