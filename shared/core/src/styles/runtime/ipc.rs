//! Lifetimes on the private dictation pipe. This keeps neither text nor passes;
//! cancel and progress belong to one requesting adapter and one account epoch.
use super::{commands::Command, Stylization};
use crate::features::cancellation::Cancellation;
use govorun_core::{
    stylization::{foreground::Stage, Failure},
    Error, Future, Result,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::mpsc;

pub type Deliver = Arc<dyn Fn(Value) -> Future<'static, Result<()>> + Send + Sync>;
#[derive(Default)]
struct State {
    closed: bool,
    active: HashMap<String, (u64, Cancellation)>,
}
#[derive(Default)]
pub struct Requests(Mutex<State>);
pub struct Guard {
    owner: Arc<Requests>,
    id: String,
    cancel: Cancellation,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "operation", deny_unknown_fields)]
pub enum Cancel {
    #[serde(rename = "stylizationCancel")]
    Request {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}
impl Requests {
    pub fn register(self: &Arc<Self>, id: &str, epoch: u64) -> Result<Guard> {
        if id.is_empty() || id.len() > 64 || id.chars().any(char::is_control) {
            return Err(Error::InvalidRequest);
        }
        let mut state = self.0.lock().map_err(|_| Error::InvalidResponse)?;
        if state.closed {
            return Err(Error::SessionChanged);
        }
        if state.active.len() >= 32 || state.active.contains_key(id) {
            return Err(Error::InvalidRequest);
        }
        let cancel = Cancellation::default();
        state.active.insert(id.to_owned(), (epoch, cancel.clone()));
        Ok(Guard {
            owner: self.clone(),
            id: id.into(),
            cancel,
        })
    }
    pub fn cancel(&self, params: Value) -> Result<bool> {
        let Cancel::Request {
            request_id,
            session_generation,
        } = serde_json::from_value(params).map_err(|_| Error::InvalidRequest)?;
        if request_id.is_empty()
            || request_id.len() > 64
            || request_id.chars().any(char::is_control)
        {
            return Err(Error::InvalidRequest);
        }
        let state = self.0.lock().map_err(|_| Error::InvalidResponse)?;
        if let Some((_, cancel)) = state
            .active
            .get(&request_id)
            .filter(|v| v.0 == session_generation)
        {
            cancel.cancel();
            return Ok(true);
        }
        Ok(false)
    }
    pub fn close(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.closed = true;
            for (_, cancel) in state.active.values() {
                cancel.cancel();
            }
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.owner.0.lock() {
            state.active.remove(&self.id);
        }
    }
}
impl Guard {
    pub async fn run_value(
        &self,
        operation: impl std::future::Future<Output = Result<Value>>,
    ) -> Result<Value> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(Error::StyleTransport(Failure::Cancelled)),
            result = operation => result,
        }
    }
    /// Three fixed states at most. The adapter acknowledges them before the
    /// final reply; repeated poll observations cannot grow an event queue.
    pub async fn run(
        &self,
        owner: &Stylization,
        command: Command,
        deliver: Deliver,
    ) -> Result<Value> {
        self.run_inner(owner, command, deliver, None).await
    }
    pub async fn run_configured(
        &self, owner: &Stylization, command: Command, deliver: Deliver,
        selected: bool, online: bool,
    ) -> Result<Value> {
        self.run_inner(owner, command, deliver, Some((selected, online))).await
    }
    async fn run_inner(
        &self, owner: &Stylization, command: Command, deliver: Deliver,
        configuration: Option<(bool, bool)>,
    ) -> Result<Value> {
        let (sender, mut receiver) = mpsc::channel(3);
        let stage = AtomicU8::new(0);
        let dispatch = async {
            let result = {
                let progress = |value| {
                    let (order, label) = match value {
                        Stage::Checking => (1, "checking_connection"),
                        Stage::Accepted => (2, "accepted"),
                        Stage::Processing => (3, "processing"),
                    };
                    if order > stage.fetch_max(order, Ordering::AcqRel) {
                        let _ = sender.try_send(label);
                    }
                };
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => Err(Error::StyleTransport(Failure::Cancelled)),
                    result = async {
                        if let Some((selected, online)) = configuration {
                            owner.dispatch_configured(command, selected, online, &progress).await
                        } else { owner.dispatch(command, &progress).await }
                    } => result,
                }
            };
            drop(sender);
            result
        };
        let delivery = async {
            while let Some(stage) = receiver.recv().await {
                let result = tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Ok(()),
                    result = deliver(json!({"channel":"accountProgress","id":self.id,"stage":stage})) => result,
                };
                if result.is_err() {
                    self.cancel.cancel();
                    return Err(Error::NetworkUnavailable);
                }
            }
            Ok(())
        };
        let (result, delivered) = tokio::join!(dispatch, delivery);
        delivered?;
        result
    }
}
