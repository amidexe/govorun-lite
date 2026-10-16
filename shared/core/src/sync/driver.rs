//! Desktop execution of the common synchronization workflow and retry policy.
use crate::features::cancellation::Cancellation;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::{watch, Notify};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub account: String,
    pub generation: u64,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Content,
    Preferences,
    Statistics,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Preferences => "preferences",
            Self::Statistics => "statistics",
        }
    }
    fn diagnostic(self) -> &'static str {
        match self {
            Self::Content => "content_sync_failed",
            Self::Preferences => "preferences_sync_failed",
            Self::Statistics => "stats_sync_failed",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub code: &'static str,
    pub status: Option<u16>,
}
impl From<&'static str> for Failure {
    fn from(code: &'static str) -> Self {
        Self { code, status: None }
    }
}
impl Failure {
    pub fn http(code: &'static str, response: &Value) -> Self {
        Self {
            code,
            status: response["status"]
                .as_u64()
                .filter(|n| (100..=599).contains(n))
                .map(|n| n as u16),
        }
    }
}
pub type StageFuture = Pin<Box<dyn Future<Output = Result<(), Failure>> + Send>>;
pub type Stages = Arc<dyn Fn(Stage, Scope, Cancellation) -> StageFuture + Send + Sync>;
pub type Context = Arc<dyn Fn() -> Option<Scope> + Send + Sync>;
pub type Pending = Arc<dyn Fn(&Scope) -> bool + Send + Sync>;
pub type Diagnostics = Arc<dyn Fn(&'static str, Failure) + Send + Sync>;
pub type Jitter = Arc<dyn Fn() -> f64 + Send + Sync>;

struct Active {
    id: u64,
    scope: Scope,
    cancellation: Cancellation,
}
#[derive(Default)]
struct State {
    serial: u64,
    active: Option<Active>,
    requested: bool,
    immediate: bool,
    stopped: bool,
    paused: Option<Scope>,
}
pub struct SyncDriver {
    context: Context,
    stages: Stages,
    pending: Pending,
    diagnostics: Diagnostics,
    jitter: Jitter,
    state: Mutex<State>,
    finished: Notify,
    wake: watch::Sender<()>,
}
impl SyncDriver {
    pub fn new(
        context: Context,
        stages: Stages,
        pending: Pending,
        diagnostics: Diagnostics,
        jitter: Jitter,
    ) -> Arc<Self> {
        Arc::new(Self {
            context,
            stages,
            pending,
            diagnostics,
            jitter,
            state: Mutex::new(State::default()),
            finished: Notify::new(),
            wake: watch::channel(()).0,
        })
    }
    /// Context returns None unless the current account has synchronization
    /// enabled. An ordinary login never implicitly enables data transfer.
    pub fn request(self: &Arc<Self>, immediate: bool) {
        let scope = (self.context)();
        let mut state = self.state.lock().unwrap();
        if state.stopped {
            return;
        }
        if let Some(active) = &state.active {
            if scope.as_ref() != Some(&active.scope) {
                active.cancellation.cancel();
            }
        }
        let Some(scope) = scope else {
            state.requested = false;
            return;
        };
        if state.paused.as_ref() == Some(&scope) && !immediate {
            return;
        }
        state.paused = None;
        state.requested = true;
        state.immediate |= immediate;
        if state.active.is_some() {
            if immediate {
                self.wake.send_replace(());
            }
            return;
        }
        state.serial += 1;
        let id = state.serial;
        let cancellation = Cancellation::default();
        state.active = Some(Active {
            id,
            scope: scope.clone(),
            cancellation: cancellation.clone(),
        });
        let immediate = state.immediate;
        state.immediate = false;
        state.requested = false;
        let driver = self.clone();
        tokio::spawn(async move {
            let execution = driver.clone();
            let outcome =
                tokio::spawn(async move { execution.run(&scope, &cancellation, immediate).await })
                    .await;
            let error = match outcome {
                Ok(Err("sync_cancelled")) | Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error),
                Err(_) => Some("sync_stage_panicked"),
            };
            if let Some(error) = error {
                (driver.diagnostics)("account_sync_driver_failed", error.into());
            }
            driver.finish(id);
        });
    }
    pub async fn stop(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.stopped = true;
            state.requested = false;
            if let Some(active) = &state.active {
                active.cancellation.cancel();
            }
        }
        loop {
            let notified = self.finished.notified();
            if self.state.lock().unwrap().active.is_none() {
                return;
            }
            notified.await;
        }
    }
    fn finish(self: &Arc<Self>, id: u64) {
        let next = {
            let mut state = self.state.lock().unwrap();
            if state.active.as_ref().is_none_or(|active| active.id != id) {
                return;
            }
            state.active = None;
            if state.stopped || !state.requested {
                None
            } else {
                Some(state.immediate)
            }
        };
        self.finished.notify_waiters();
        if let Some(immediate) = next {
            self.request(immediate);
        }
    }
    fn check(&self, scope: &Scope, cancellation: &Cancellation) -> Result<(), &'static str> {
        if cancellation.is_cancelled() || (self.context)().as_ref() != Some(scope) {
            Err("sync_cancelled")
        } else {
            Ok(())
        }
    }
    async fn run(
        &self,
        scope: &Scope,
        cancellation: &Cancellation,
        mut immediate: bool,
    ) -> Result<(), &'static str> {
        let mut schedule = call(
            json!({"kind":"schedule","failures":0,"succeeded":true,"pending":true,"jitter":0.0}),
        )?;
        let mut failures = 0;
        loop {
            self.check(scope, cancellation)?;
            if !immediate {
                let mut wake = self.wake.subscribe();
                let millis = schedule["delayMillis"]
                    .as_u64()
                    .ok_or("sync_invalid_response")?;
                if !self.state.lock().unwrap().immediate {
                    tokio::select! {
                        _=tokio::time::sleep(std::time::Duration::from_millis(millis))=>(),
                        _=wake.changed()=>(),
                        _=cancellation.cancelled()=>return Err("sync_cancelled"),
                    }
                }
            }
            self.check(scope, cancellation)?;
            {
                let mut state = self.state.lock().unwrap();
                state.requested = false;
                state.immediate = false;
            }
            let mut decision = call(json!({"kind":"start","scope":scope}))?;
            while decision["action"] != "complete" {
                self.check(scope, cancellation)?;
                let stage: Stage = serde_json::from_value(decision["action"].clone())
                    .map_err(|_| "sync_invalid_response")?;
                // Do not drop a stage during a durable write. The stage receives
                // cancellation and must fence account effects at its IO boundary.
                let result = (self.stages)(stage, scope.clone(), cancellation.clone()).await;
                self.check(scope, cancellation)?;
                if let Err(error) = result {
                    (self.diagnostics)(stage.diagnostic(), error);
                }
                decision = call(
                    json!({"kind":"advance","flow":decision["flow"],"scope":scope,
                    "stage":stage.name(),"succeeded":result.is_ok()}),
                )?;
            }
            let requested = {
                let mut state = self.state.lock().unwrap();
                let requested = state.requested;
                state.requested = false;
                immediate = state.immediate;
                state.immediate = false;
                requested
            };
            schedule = call(
                json!({"kind":"schedule","failures":failures,"succeeded":decision["succeeded"],
                "pending":requested || (self.pending)(scope),"jitter":(self.jitter)()}),
            )?;
            self.check(scope, cancellation)?;
            failures = schedule["failures"]
                .as_u64()
                .ok_or("sync_invalid_response")?;
            if schedule["delayMillis"].is_null() {
                if failures > 0 { self.state.lock().unwrap().paused = Some(scope.clone()); }
                return Ok(());
            }
        }
    }
}
fn call(command: Value) -> Result<Value, &'static str> {
    let reply = govorun_core::evaluate_business_command(
        &json!({"operation":"synchronization","command":command}).to_string(),
    );
    let reply: Value = serde_json::from_str(&reply).map_err(|_| "sync_invalid_response")?;
    reply.get("result").cloned().ok_or("sync_invalid_request")
}
