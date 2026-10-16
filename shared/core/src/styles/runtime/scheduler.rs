//! Mailbox and timers only. Common Scheduling owns every retry, coalescing
//! decision and completion ticket; dropping an effect cancels its I/O future.
use crate::features::cancellation::Cancellation;
use govorun_core::{
    stylization::scheduling::{Action, Completion, Kind, Scheduling, Trigger},
    AccountSession, Error, Future, Result,
};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{Duration, Sleep},
};
pub type Operation = Arc<dyn Fn(u64, bool) -> Future<'static, Completion> + Send + Sync>;
pub type Failure = Arc<dyn Fn(&str) + Send + Sync>;
struct Request {
    epoch: u64,
    trigger: Trigger,
    join: bool,
    delay: u64,
}
pub struct Scheduler {
    account: AccountSession,
    sender: mpsc::Sender<Request>,
    cancel: Cancellation,
    task: Mutex<Option<JoinHandle<()>>>,
}
impl Scheduler {
    pub fn start(
        account: AccountSession,
        kind: Kind,
        operation: Operation,
        failure: Failure,
    ) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(32);
        let cancel = Cancellation::default();
        let task = tokio::spawn(run(
            account.clone(),
            kind,
            receiver,
            cancel.clone(),
            operation,
            failure,
        ));
        Arc::new(Self {
            account,
            sender,
            cancel,
            task: Mutex::new(Some(task)),
        })
    }
    pub fn request(&self, epoch: u64, trigger: Trigger, join: bool, delay: u64) -> Result<()> {
        self.account.current(epoch)?;
        if self.cancel.is_cancelled() {
            return Err(Error::SessionChanged);
        }
        self.sender
            .try_send(Request {
                epoch,
                trigger,
                join,
                delay,
            })
            .map_err(|_| Error::InvalidRequest)
    }
    pub async fn stop(&self) {
        self.cancel.cancel();
        let task = self.task.lock().ok().and_then(|mut task| task.take());
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
impl Drop for Scheduler {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn run(
    account: AccountSession,
    kind: Kind,
    mut mailbox: mpsc::Receiver<Request>,
    cancel: Cancellation,
    operation: Operation,
    failure: Failure,
) {
    let scheduling = Scheduling::new(account.clone(), kind);
    let mut epoch = account.snapshot().session_generation;
    let mut changes = account.subscribe();
    let mut boundary = account.subscribe_boundary();
    let mut effect: Option<Future<'static, Completion>> = None;
    let mut timer: Option<Pin<Box<Sleep>>> = None;
    let mut ticket = 0;
    loop {
        let frame = tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            changed = boundary.changed() => {
                if changed.is_err() { break; }
                epoch = account.snapshot().session_generation;
                scheduling.cancel(epoch)
            },
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let snapshot = account.snapshot();
                // Reserve maintenance owns profile recovery. Losing freshness
                // cannot cancel the very continuation that restores it. Actual
                // sign-out/storage boundaries still drop its in-flight effect.
                let allowed = account.style_authority().authorized
                    || matches!(kind, Kind::Reserve) && snapshot.signed_in
                        && snapshot.storage_available && !snapshot.logout_pending;
                let current = snapshot.session_generation;
                if current == epoch && allowed { continue; }
                epoch = current;
                scheduling.cancel(epoch)
            },
            request = mailbox.recv() => {
                let Some(request) = request else { break; };
                if account.current(request.epoch).is_err() { continue; }
                epoch = request.epoch;
                scheduling.request(epoch, request.trigger, request.join, request.delay)
            },
            completion = async { effect.as_mut().expect("enabled effect").await }, if effect.is_some() => {
                effect = None;
                scheduling.complete(epoch, ticket, completion)
            },
            _ = async { timer.as_mut().expect("enabled timer").await }, if timer.is_some() => {
                timer = None;
                scheduling.tick(epoch, ticket)
            },
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(Error::SessionChanged) => {
                effect = None;
                timer = None;
                continue;
            }
            Err(_) => {
                effect = None;
                timer = None;
                let _ = scheduling.cancel(epoch);
                failure("style_pass_issue_failed");
                continue;
            }
        };
        if matches!(frame.action, Action::None) {
            continue;
        }
        effect = None;
        timer = None;
        match frame.action {
            Action::Run {
                ticket: next,
                explicit_demand,
            } => {
                ticket = next;
                effect = Some(operation(epoch, explicit_demand));
            }
            Action::Wait {
                ticket: next,
                milliseconds,
                failure_reason,
                ..
            } => {
                ticket = next;
                timer = Some(Box::pin(tokio::time::sleep(Duration::from_millis(
                    milliseconds,
                ))));
                if let Some(reason) = failure_reason {
                    failure(&reason);
                }
            }
            Action::Idle { failure_reason } => {
                if let Some(reason) = failure_reason {
                    failure(&reason);
                }
            }
            Action::None => unreachable!(),
        }
    }
    // Drop pending effects before returning; no detached maintenance writer.
}
