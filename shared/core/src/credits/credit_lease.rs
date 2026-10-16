use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    epoch: u64,
    clock: u64,
    version: u64,
    scope: Option<u64>,
    leases: BTreeMap<String, Lease>,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Lease {
    phase: Phase,
    version: u64,
    not_before: u64,
    owner: Option<String>,
}

#[derive(Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Phase { Starting, Accepted, Waiting }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Observation { at_millis: u64, event: Event }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Starting { id: String },
    Accepted { id: String },
    Ambiguous { id: String },
    Defer { id: String, milliseconds: i64 },
    Forget { id: String },
    Inspect { id: String, #[serde(default)] observe: bool, expected_version: Option<u64> },
    Select { ids: Vec<String> },
    Scope { epoch: u64, ids: Vec<String> },
    Current { id: String, version: u64 },
    Claim { id: String, version: u64, owner: String },
    Release { id: String, version: u64, owner: String },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub(crate) struct Transition { state: State, result: Value }

pub(crate) fn transition(state: Option<State>, observations: Vec<Observation>, epoch: u64,
    now_millis: u64, grace_millis: Option<u64>, allowed: bool) -> Result<Transition> {
    let grace_millis = grace_millis.unwrap_or(15_000);
    let version = state.as_ref().map_or(0, |s| s.version);
    let mut state = state.filter(|s| s.epoch == epoch)
        .unwrap_or(State { epoch, version, ..State::default() });
    if epoch == 0 && allowed || now_millis < state.clock || !(1..=50_000).contains(&grace_millis)
        || observations.is_empty() {
        return Err(Error::InvalidRequest);
    }
    state.clock = now_millis;
    if !allowed { state.leases.clear(); }
    let mut result = Value::Null;
    for observation in observations {
        if observation.at_millis > now_millis { return Err(Error::InvalidRequest); }
        result = state.apply(observation.event, observation.at_millis, grace_millis, allowed)?;
    }
    Ok(Transition { state, result })
}

impl State {
    fn put(&mut self, id: &str, phase: Phase, not_before: u64) -> Result<()> {
        if id.is_empty() || id.len() > 256 { return Err(Error::InvalidRequest); }
        self.version = self.version.checked_add(1).ok_or(Error::InvalidRequest)?;
        self.leases.insert(id.into(), Lease { phase, version: self.version, not_before, owner: None });
        Ok(())
    }

    fn wait(&mut self, id: &str, at: u64, delay: u64) -> Result<()> {
        let deadline = at.checked_add(delay).ok_or(Error::InvalidRequest)?;
        let previous = self.leases.get(id).filter(|l| l.phase == Phase::Waiting).map_or(0, |l| l.not_before);
        self.put(id, Phase::Waiting, deadline.max(previous))
    }

    fn facts(&self, id: &str, allowed: bool) -> Value {
        let lease = self.leases.get(id);
        let starting = lease.is_some_and(|l| l.phase == Phase::Starting);
        let accepted = lease.is_some_and(|l| l.phase == Phase::Accepted);
        json!({"starting":starting,"accepted":accepted,"blocksNewLease":!allowed || !accepted,
            "requiresRecovery":allowed && !starting && !accepted,
            "version":lease.map_or(0, |l| l.version),
            "delayMillis":lease.filter(|l| l.phase == Phase::Waiting).map(|l| l.not_before.saturating_sub(self.clock))})
    }

    fn apply(&mut self, event: Event, at: u64, grace: u64, allowed: bool) -> Result<Value> {
        match event {
            Event::Inspect { id, observe, expected_version } => {
                if allowed && observe && !self.leases.contains_key(&id) { self.wait(&id, at, grace)?; }
                let mut facts = self.facts(&id, allowed);
                if let Some(version) = expected_version {
                    facts["current"] = json!(allowed && self.leases.get(&id).is_some_and(|l| l.version == version));
                }
                Ok(facts)
            }
            Event::Select { ids } => {
                let mut eligible = Vec::new();
                let mut delay: Option<u64> = None;
                let mut recoverable = false;
                for id in ids {
                    if allowed && !self.leases.contains_key(&id) { self.wait(&id, at, grace)?; }
                    let facts = self.facts(&id, allowed);
                    recoverable |= facts["requiresRecovery"] == true;
                    if facts["requiresRecovery"] == true {
                        let remaining = facts["delayMillis"].as_u64().ok_or(Error::InvalidRequest)?;
                        if remaining == 0 {
                            if self.leases.get(&id).is_some_and(|lease|lease.owner.is_none()) {
                                eligible.push(json!({"id":id,"version":facts["version"]}));
                            }
                        }
                        else { delay = Some(delay.map_or(remaining, |old| old.min(remaining))); }
                    }
                }
                // A vault snapshot may race with a newly durable checkout.
                // Only terminal observations or a scope boundary remove entries.
                if !eligible.is_empty() { delay = Some(delay.map_or(250, |old| old.min(250))); }
                Ok(json!({"eligible":eligible,"retryAfterMillis":delay,
                    // Pending stored pass state already excludes these tokens from the
                    // available inventory. Its ambiguity cannot fence a different
                    // pass; Inspect still fences the individual lease/version.
                    "blocksNewLease":!allowed,"requiresRecovery":recoverable}))
            }
            _ if !allowed => Ok(json!(false)),
            Event::Starting { id } => { self.put(&id, Phase::Starting, 0)?; Ok(Value::Null) }
            Event::Accepted { id } => { self.put(&id, Phase::Accepted, 0)?; Ok(Value::Null) }
            Event::Ambiguous { id } => {
                if self.leases.get(&id).is_some_and(|l| l.phase == Phase::Waiting) { return Ok(self.facts(&id, allowed)); }
                let accepted = self.leases.get(&id).is_some_and(|l| l.phase == Phase::Accepted);
                self.wait(&id, at, if accepted { 0 } else { grace })?;
                Ok(self.facts(&id, allowed))
            }
            Event::Defer { id, milliseconds } => {
                self.wait(&id, at, milliseconds.clamp(0, 50_000) as u64)?;
                Ok(self.facts(&id, allowed))
            }
            Event::Forget { id } => Ok(json!(self.leases.remove(&id).is_some())),
            Event::Scope { epoch, ids } => {
                let changed = self.scope.is_some_and(|old| old != epoch);
                self.scope = Some(epoch);
                if changed {
                    self.leases.retain(|id, _| ids.contains(id));
                    for id in ids { self.wait(&id, at, grace)?; }
                }
                Ok(json!(changed))
            }
            Event::Current { id, version } =>
                Ok(json!(self.leases.get(&id).is_some_and(|l| l.version == version))),
            Event::Claim { id, version, owner } => {
                if owner.is_empty() { return Err(Error::InvalidRequest); }
                let lease = self.leases.get_mut(&id).filter(|l| l.phase == Phase::Waiting && l.version == version
                    && l.not_before <= self.clock && l.owner.is_none());
                if let Some(lease) = lease { lease.owner = Some(owner); Ok(json!(true)) }
                else { Ok(json!(false)) }
            }
            Event::Release { id, version, owner } => {
                if let Some(lease) = self.leases.get_mut(&id).filter(|l| l.version == version && l.owner.as_ref() == Some(&owner)) {
                    lease.owner = None;
                }
                Ok(Value::Null)
            }
        }
    }
}
