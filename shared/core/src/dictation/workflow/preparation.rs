//! Coalesced background reserve work. Foreground never waits for this worker.
use super::Failure;
use serde::Deserialize;
use serde_json::Value;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::watch;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Start,
    Automatic,
    Maintenance,
}
impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Automatic => "automatic",
            Self::Maintenance => "maintenance",
        }
    }
}
pub trait Ports: Send + Sync + 'static {
    fn epoch(&self) -> u64;
    fn plan(
        &self,
        epoch: u64,
        phase: &'static str,
        credits: usize,
    ) -> impl Future<Output = Result<Value, Failure>> + Send;
    fn refresh(&self, epoch: u64) -> impl Future<Output = Result<(), Failure>> + Send;
    fn warm(&self, epoch: u64) -> impl Future<Output = Result<(), Failure>> + Send;
}
#[derive(Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Action {
    None,
    RefreshProfile,
    PrepareReserve,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Instruction {
    action: Action,
    maximum_join_milliseconds: u64,
}
impl Instruction {
    fn read(value: Value) -> Result<Self, Failure> {
        let instruction: Self = serde_json::from_value(value).map_err(|_| invalid())?;
        if instruction.maximum_join_milliseconds != 0 {
            return Err(invalid());
        }
        Ok(instruction)
    }
}
fn invalid() -> Failure {
    Failure::Effect("dictation_invalid_preparation")
}
type Signal = watch::Receiver<Option<Result<(), Failure>>>;
struct Pending {
    id: u64,
    epoch: u64,
    signal: Signal,
    cancelled: Arc<AtomicBool>,
}
#[derive(Default)]
struct State {
    next: u64,
    pending: Option<Pending>,
    followup: Option<Phase>,
}
pub struct Preparation<P> {
    ports: Arc<P>,
    state: Mutex<State>,
}
impl<P: Ports> Preparation<P> {
    pub fn new(ports: Arc<P>) -> Arc<Self> {
        Arc::new(Self {
            ports,
            state: Mutex::new(State::default()),
        })
    }
    pub async fn prepare(self: &Arc<Self>, epoch: u64, phase: Phase) -> Result<(), Failure> {
        self.current(epoch, None)?;
        let signal = {
            let mut state = self.state.lock().unwrap();
            if state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.epoch == epoch)
            {
                if phase == Phase::Automatic {
                    state.followup = Some(phase);
                }
                state.pending.as_ref().unwrap().signal.clone()
            } else {
                if let Some(old) = state.pending.take() {
                    old.cancelled.store(true, Ordering::Release);
                }
                state.next = state.next.checked_add(1).ok_or(invalid())?;
                let id = state.next;
                let cancelled = Arc::new(AtomicBool::new(false));
                let (send, signal) = watch::channel(None);
                state.followup = None;
                state.pending = Some(Pending {
                    id,
                    epoch,
                    signal: signal.clone(),
                    cancelled: cancelled.clone(),
                });
                let owner = self.clone();
                tokio::spawn(async move {
                    let mut phase = phase;
                    loop {
                        let result = owner.run(epoch, phase, &cancelled).await;
                        let next = {
                            let mut state = owner.state.lock().unwrap();
                            if state
                                .pending
                                .as_ref()
                                .is_some_and(|pending| pending.id == id)
                            {
                                let next = state.followup.take();
                                if next.is_none() {
                                    state.pending = None;
                                }
                                next
                            } else {
                                None
                            }
                        };
                        if let Some(next) = next {
                            phase = next;
                            continue;
                        }
                        // A failed maintenance request must not erase balances
                        // or replay text. Ordinary preflight remains authoritative.
                        send.send_replace(Some(if result == Err(Failure::Cancelled) {
                            result
                        } else {
                            Ok(())
                        }));
                        break;
                    }
                });
                signal
            }
        };
        wait(signal).await
    }
    async fn run(&self, epoch: u64, phase: Phase, cancelled: &AtomicBool) -> Result<(), Failure> {
        self.current(epoch, Some(cancelled))?;
        let mut instruction = Instruction::read(self.ports.plan(epoch, phase.name(), 1).await?)?;
        self.current(epoch, Some(cancelled))?;
        if instruction.action == Action::RefreshProfile {
            self.ports.refresh(epoch).await?;
            self.current(epoch, Some(cancelled))?;
            instruction = Instruction::read(self.ports.plan(epoch, "refreshed", 1).await?)?;
            self.current(epoch, Some(cancelled))?;
        }
        if instruction.action == Action::PrepareReserve {
            self.ports.warm(epoch).await?;
            self.current(epoch, Some(cancelled))?;
        }
        Ok(())
    }
    /// Foreground owns its issuance deadline; background work is never joined.
    pub async fn join(&self, _epoch: u64, _credits: usize) -> u64 { 0 }
    pub fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(pending) = state.pending.take() {
            pending.cancelled.store(true, Ordering::Release);
        }
        state.followup = None;
    }
    fn current(&self, epoch: u64, cancelled: Option<&AtomicBool>) -> Result<(), Failure> {
        if self.ports.epoch() != epoch || cancelled.is_some_and(|flag| flag.load(Ordering::Acquire))
        {
            Err(Failure::Cancelled)
        } else {
            Ok(())
        }
    }
}
async fn wait(mut signal: Signal) -> Result<(), Failure> {
    loop {
        if let Some(result) = *signal.borrow_and_update() {
            return result;
        }
        signal
            .changed()
            .await
            .map_err(|_| Failure::Effect("dictation_preparation_stopped"))?;
    }
}
