//! Typed owner of the existing maintenance continuations. The host executes
//! Run/Wait; retry budgets, tickets and wakeups remain in the common reducers.
use crate::{AccountSession, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use tokio::time::Instant;

#[derive(Clone, Copy)]
pub enum Kind {
    Reserve,
    Credit,
}
#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Trigger {
    Maintenance,
    Reconnect,
    Demand,
}
#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Ready,
    Retry,
    Failed,
    Terminal,
    Offline,
    Blocked,
    Cancelled,
    HttpFailure,
    Progress,
}
pub struct Completion {
    pub outcome: Outcome,
    pub retry_after_millis: Option<u64>,
    pub status: Option<u16>,
    pub wire_attempted: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    None,
    Idle {
        failure_reason: Option<String>,
    },
    Run {
        ticket: u64,
        explicit_demand: bool,
    },
    Wait {
        ticket: u64,
        milliseconds: u64,
        wakeable: bool,
        #[serde(default)]
        cycle_complete: bool,
        failure_reason: Option<String>,
    },
}
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Frame {
    pub idle: bool,
    pub action: Action,
}
pub struct Scheduling {
    account: AccountSession,
    kind: Kind,
    state: Mutex<Value>,
    clock: Instant,
}
impl Scheduling {
    pub fn new(account: AccountSession, kind: Kind) -> Self {
        Self {
            account,
            kind,
            state: Mutex::new(Value::Null),
            clock: Instant::now(),
        }
    }
    fn event(&self, epoch: u64, event: Value) -> Result<Frame> {
        self.account.current(epoch)?;
        let snapshot = self.account.snapshot();
        let allowed = snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending;
        let now = self.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let mut state = self.state.lock().map_err(|_| Error::InvalidResponse)?;
        let value = match self.kind {
            Kind::Reserve => serde_json::to_value(crate::reserve_maintenance::transition(
                serde_json::from_value(state.clone()).map_err(|_| Error::InvalidRequest)?,
                serde_json::from_value(event).map_err(|_| Error::InvalidRequest)?,
                epoch,
                now,
                allowed,
            )?),
            Kind::Credit => serde_json::to_value(crate::credit_recovery::transition(
                serde_json::from_value(state.clone()).map_err(|_| Error::InvalidRequest)?,
                serde_json::from_value(event).map_err(|_| Error::InvalidRequest)?,
                epoch,
                now,
                allowed,
            )?),
        }
        .map_err(|_| Error::InvalidResponse)?;
        self.account.current(epoch)?;
        let frame = serde_json::from_value(value.clone()).map_err(|_| Error::InvalidResponse)?;
        *state = value["state"].clone();
        Ok(frame)
    }
    pub fn request(
        &self,
        epoch: u64,
        trigger: Trigger,
        join: bool,
        delay_millis: u64,
    ) -> Result<Frame> {
        let mut event = json!({"kind":"request","trigger":trigger,"join":join});
        if matches!(self.kind, Kind::Credit) {
            event["delayMillis"] = json!(delay_millis);
        }
        self.event(epoch, event)
    }
    pub fn complete(&self, epoch: u64, ticket: u64, completion: Completion) -> Result<Frame> {
        let mut event = json!({"kind":"complete","ticket":ticket,"outcome":completion.outcome,
            "retryAfterMillis":completion.retry_after_millis});
        match self.kind {
            Kind::Credit => event["wireAttempted"] = json!(completion.wire_attempted),
            Kind::Reserve => event["status"] = json!(completion.status),
        }
        let mut frame=self.event(epoch, event)?;
        let retry = matches!(self.kind,Kind::Reserve) && matches!(completion.outcome,
            Outcome::Retry | Outcome::HttpFailure | Outcome::Blocked | Outcome::Progress)
            || matches!(self.kind,Kind::Credit) && completion.wire_attempted
                && matches!(completion.outcome,Outcome::Retry | Outcome::Failed);
        if retry {
            if let Action::Wait {milliseconds,..}=&mut frame.action {
                // Positive jitter preserves the reducer's retry floor and any
                // server Retry-After. Only background timers use this port;
                // foreground deadlines and the five-attempt budget are unchanged.
                let spread=*milliseconds/4;
                *milliseconds=milliseconds.saturating_add((self.account.inner.entropy)()%(spread+1));
            }
        }
        Ok(frame)
    }
    pub fn tick(&self, epoch: u64, ticket: u64) -> Result<Frame> {
        self.event(epoch, json!({"kind":"tick","ticket":ticket}))
    }
    pub fn cancel(&self, epoch: u64) -> Result<Frame> {
        self.event(epoch, json!({"kind":"cancel"}))
    }
}
