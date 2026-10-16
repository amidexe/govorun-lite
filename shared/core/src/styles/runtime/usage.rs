//! Event-driven I/O for the common durable usage owner. Duplicate signals fold
//! into one follow-up; no detached task, timer or retry of text exists here.
use super::reporting::Online;
use crate::features::cancellation::Cancellation;
use govorun_core::{AccountSession, Future, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::{sync::watch, task::JoinHandle};

pub type Operation = Arc<dyn Fn(u64) -> Future<'static, Result<usize>> + Send + Sync>;
pub struct Worker {
    cancel: Cancellation,
    requested: watch::Sender<u64>,
    network: watch::Sender<bool>,
    closed: AtomicBool,
    task: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub fn start(account: AccountSession, operation: Operation, online: Online) -> Self {
        let cancel = Cancellation::default();
        let (requested, requests) = watch::channel(0);
        let (network, connectivity) = watch::channel(true);
        let boundary = account.subscribe_boundary();
        let changes = account.subscribe();
        let task = tokio::spawn(run(
            account,
            operation,
            online,
            cancel.clone(),
            requests,
            connectivity,
            boundary,
            changes,
        ));
        Self {
            cancel,
            requested,
            network,
            closed: AtomicBool::new(false),
            task: Mutex::new(Some(task)),
        }
    }
    pub fn request(&self) {
        if !self.closed.load(Ordering::Acquire) {
            self.requested.send_modify(|v| *v = v.wrapping_add(1));
        }
    }
    pub fn connectivity_changed(&self, online: bool) {
        if !self.closed.load(Ordering::Acquire) {
            self.network.send_replace(online);
            if online {
                self.request();
            }
        }
    }
    pub async fn stop(&self) {
        self.closed.store(true, Ordering::Release);
        self.cancel.cancel();
        let task = self.task.lock().ok().and_then(|mut v| v.take());
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
async fn run(
    account: AccountSession,
    operation: Operation,
    online: Online,
    cancel: Cancellation,
    mut requests: watch::Receiver<u64>,
    mut network: watch::Receiver<bool>,
    mut boundary: watch::Receiver<u64>,
    mut changes: watch::Receiver<u64>,
) {
    let mut dirty = true;
    let mut effect: Option<Future<'static, Result<usize>>> = None;
    loop {
        if dirty && effect.is_none() {
            dirty = false;
            let snapshot = account.snapshot();
            if *network.borrow() && snapshot.authority.authorized && !snapshot.logout_pending {
                let epoch = snapshot.session_generation;
                let online = online.clone();
                let operation = operation.clone();
                let account = account.clone();
                effect = Some(Box::pin(async move {
                    if !online().await {
                        return Ok(0);
                    }
                    account.current(epoch)?;
                    operation(epoch).await
                }));
            }
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            result = boundary.changed() => {
                if result.is_err() { break; }
                effect = None;
                dirty = true;
            },
            result = changes.changed() => {
                if result.is_err() { break; }
                if !account.snapshot().authority.authorized { effect = None; }
                dirty = true;
            },
            result = network.changed() => {
                if result.is_err() { break; }
                if !*network.borrow_and_update() { effect = None; }
                dirty = true;
            },
            result = requests.changed() => {
                if result.is_err() { break; }
                dirty = true;
            },
            _ = async { effect.as_mut().expect("enabled usage effect").await }, if effect.is_some() => {
                effect = None;
                // A failed send is retained by UsageSync, with no automatic
                // retry loop. Only a later real event schedules another pass.
            },
        }
    }
    // Dropping the effect closes I/O before the protected storage pipe ends.
}
