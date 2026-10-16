use crate::{Error, ProfileRefreshCadence, ProfileRefreshReason, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProfileAdmission {
    Current,
    Superseded,
    Join { ticket: u64 },
    Start { ticket: u64, ancillary: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ProfileNextStep {
    Complete,
    Ancillary,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Flight {
    ticket: u64,
    cadence_ticket: u64,
    ancillary: bool,
    profile_loaded: bool,
}

/// One admission owner for profile work. Task handles remain runtime ports;
/// neither Kotlin nor the desktop host chooses coalescing or follow-up work.
#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileActivity {
    generation: Option<u64>,
    sequence: u64,
    cadence: ProfileRefreshCadence,
    flight: Option<Flight>,
}

impl ProfileActivity {
    pub fn begin_scope(&mut self, epoch: u64, now_ms: u64, jitter_ms: u64) {
        if self.generation.is_some_and(|current| epoch < current) {
            return;
        }
        if self.generation != Some(epoch) {
            self.generation = Some(epoch);
            self.flight = None;
        }
        self.cadence.begin_scope(epoch, now_ms, jitter_ms);
    }

    pub fn needs_refresh(&self, epoch: u64, now_ms: u64, max_age: Duration) -> bool {
        self.cadence.needs_refresh(epoch, now_ms, max_age)
    }

    pub fn next_delay(&self, epoch: Option<u64>, now_ms: u64, jitter_ms: u64) -> Duration {
        self.cadence.next_delay(epoch, now_ms, jitter_ms)
    }

    pub fn observe(&mut self, epoch: u64, now_ms: u64, jitter_ms: u64, succeeded: bool) -> bool {
        if self.generation.is_some_and(|current| epoch < current) {
            return false;
        }
        self.begin_scope(epoch, now_ms, jitter_ms);
        self.cadence.observe(epoch, now_ms, jitter_ms, succeeded);
        true
    }

    pub fn request(
        &mut self,
        epoch: u64,
        now_ms: u64,
        max_age: Duration,
        ancillary: bool,
        jitter_ms: u64,
    ) -> Result<ProfileAdmission> {
        if self.generation.is_some_and(|current| epoch < current) {
            return Ok(ProfileAdmission::Superseded);
        }
        self.begin_scope(epoch, now_ms, jitter_ms);
        if let Some(flight) = &mut self.flight {
            flight.ancillary |= ancillary;
            return Ok(ProfileAdmission::Join {
                ticket: flight.ticket,
            });
        }
        if !self.needs_refresh(epoch, now_ms, max_age) {
            return Ok(ProfileAdmission::Current);
        }
        self.sequence = self.sequence.checked_add(1).ok_or(Error::InvalidRequest)?;
        let cadence_ticket = self.cadence.started(epoch, now_ms, jitter_ms);
        self.flight = Some(Flight {
            ticket: self.sequence,
            cadence_ticket,
            ancillary,
            profile_loaded: false,
        });
        Ok(ProfileAdmission::Start {
            ticket: self.sequence,
            ancillary,
        })
    }

    /// Closing the profile-only flight here makes a later ancillary demand a
    /// new operation, rather than joining a task that already skipped its work.
    pub fn profile_loaded(&mut self, epoch: u64, ticket: u64) -> Result<ProfileNextStep> {
        if self.generation != Some(epoch) {
            return Err(Error::SessionChanged);
        }
        let flight = self
            .flight
            .as_mut()
            .filter(|flight| flight.ticket == ticket)
            .ok_or(Error::SessionChanged)?;
        if flight.profile_loaded {
            return Err(Error::InvalidRequest);
        }
        flight.profile_loaded = true;
        if flight.ancillary {
            Ok(ProfileNextStep::Ancillary)
        } else {
            self.flight = None;
            Ok(ProfileNextStep::Complete)
        }
    }

    pub fn finished(
        &mut self,
        epoch: u64,
        ticket: u64,
        now_ms: u64,
        jitter_ms: u64,
        succeeded: bool,
    ) -> bool {
        if self.generation != Some(epoch) {
            return false;
        }
        let Some(flight) = self
            .flight
            .as_ref()
            .filter(|flight| flight.ticket == ticket)
        else {
            return false;
        };
        // An authoritative observation may already have recorded this result.
        self.cadence
            .finished(epoch, flight.cadence_ticket, now_ms, jitter_ms, succeeded);
        self.flight = None;
        true
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ProfileActivityEvent {
    Inspect {
        reason: ProfileRefreshReason,
    },
    Request {
        reason: ProfileRefreshReason,
        ancillary: bool,
    },
    Confirmed,
    ProfileLoaded {
        ticket: u64,
    },
    Finished {
        ticket: u64,
        succeeded: bool,
    },
}

pub(crate) fn evaluate(
    mut state: ProfileActivity,
    event: ProfileActivityEvent,
    epoch: Option<u64>,
    now_ms: u64,
    jitter_ms: Option<u64>,
) -> Result<Value> {
    state.cadence.validate()?;
    if state.sequence == u64::MAX
        || state.flight.as_ref().is_some_and(|flight| {
            state.generation.is_none() || flight.ticket == 0 || flight.ticket > state.sequence
        })
    {
        return Err(Error::InvalidRequest);
    }
    let jitter = jitter_ms.unwrap_or(0);
    let mut admission = None;
    let mut next_step = None;
    let mut changed = false;
    let reason = match event {
        ProfileActivityEvent::Inspect { reason } => reason,
        event => {
            let epoch = epoch.ok_or(Error::InvalidRequest)?;
            match event {
                ProfileActivityEvent::Request { reason, ancillary } => {
                    admission =
                        Some(state.request(epoch, now_ms, reason.max_age(), ancillary, jitter)?);
                    changed = true;
                }
                ProfileActivityEvent::Confirmed => {
                    changed = state.observe(epoch, now_ms, jitter, true);
                }
                ProfileActivityEvent::ProfileLoaded { ticket } => {
                    next_step = Some(state.profile_loaded(epoch, ticket)?);
                    changed = true;
                }
                ProfileActivityEvent::Finished { ticket, succeeded } => {
                    changed = state.finished(epoch, ticket, now_ms, jitter, succeeded);
                }
                ProfileActivityEvent::Inspect { .. } => unreachable!(),
            }
            ProfileRefreshReason::Explicit
        }
    };
    Ok(
        json!({"admission":admission, "nextStep":next_step, "changed":changed,
        "nextDelayMillis":state.next_delay(epoch, now_ms, jitter).as_millis() as u64,
        "needsRefresh":epoch.is_some_and(|epoch| state.needs_refresh(epoch, now_ms, reason.max_age())),
        "state":state}),
    )
}
