//! An event-driven executor with one pending-batch deadline, no delivery retry. The
//! common reporter owns consent, the interval and atomic counter detachment.
use crate::features::cancellation::Cancellation;
use govorun_core::{stylization::reporting::Reporting, AccountSession, Future};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub type Online = Arc<dyn Fn() -> Future<'static, bool> + Send + Sync>;
pub struct Reporter {
    cancel: Cancellation,
    task: Mutex<Option<JoinHandle<()>>>,
    connectivity: watch::Sender<u64>,
}
impl Reporter {
    pub fn start(account: AccountSession, owner: Arc<Reporting>, online: Online) -> Self {
        let cancel = Cancellation::default();
        let cancelled = cancel.clone();
        let (connectivity, mut network) = watch::channel(0u64);
        let mut counters = owner.subscribe();
        let mut account_changes = account.subscribe();
        let mut boundary = account.subscribe_boundary();
        let task = tokio::spawn(async move {
            // Also consider counts accumulated before composition. Unknown
            // settings suspend them; no startup event grants consent.
            loop {
                let epoch = account.snapshot().session_generation;
                let mut deadline = None;
                tokio::select! {
                    biased;
                    _ = cancelled.cancelled() => break,
                    _ = async {
                        let available = online().await;
                        let _ = owner.flush(epoch, &|| available).await;
                        deadline = owner.pending_deadline(available);
                    } => {},
                }
                tokio::select! {
                    biased;
                    _ = cancelled.cancelled() => break,
                    _ = async {
                        match deadline {
                            Some(at) => tokio::time::sleep_until(at).await,
                            None => std::future::pending().await,
                        }
                    } => {},
                    result = boundary.changed() => {
                        if result.is_err() { break; }
                    },
                    result = account_changes.changed() => {
                        if result.is_err() { break; }
                    },
                    result = counters.changed() => {
                        if result.is_err() { break; }
                    },
                    result = network.changed() => {
                        if result.is_err() { break; }
                    },
                }
            }
        });
        Self {
            cancel,
            task: Mutex::new(Some(task)),
            connectivity,
        }
    }
    pub fn connectivity_changed(&self) {
        self.connectivity
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
    pub async fn stop(&self) {
        self.cancel.cancel();
        let task = self.task.lock().ok().and_then(|mut task| task.take());
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
impl Drop for Reporter {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
