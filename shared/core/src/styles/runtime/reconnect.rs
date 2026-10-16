//! Bounded executor for the shared reconnect continuation. Duplicate triggers
//! join that continuation; they never spawn additional operations or timers.
use crate::features::cancellation::Cancellation;
use govorun_core::{
    stylization::reconnect::{Reconnect, Step},
    AccountSession, Error, Future, Result,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

pub type Operation = Arc<dyn Fn(u64, Step) -> Future<'static, bool> + Send + Sync>;
pub type CanWarm = Arc<dyn Fn() -> bool + Send + Sync>;
pub struct Worker {
    account: AccountSession,
    owner: Arc<Reconnect>,
    sender: mpsc::Sender<u64>,
    online: Arc<AtomicBool>,
    network: watch::Sender<u64>,
    finished: watch::Sender<u64>,
    cancel: Cancellation,
    task: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub fn start(
        account: AccountSession,
        operation: Operation,
        can_warm: CanWarm,
        online: bool,
    ) -> Self {
        let owner = Arc::new(Reconnect::new(account.clone()));
        let (sender, receiver) = mpsc::channel(32);
        let (network, changes) = watch::channel(0);
        let (finished, _) = watch::channel(0);
        let online = Arc::new(AtomicBool::new(online));
        let cancel = Cancellation::default();
        let task = tokio::spawn(run(
            account.clone(),
            owner.clone(),
            receiver,
            online.clone(),
            changes,
            cancel.clone(),
            operation,
            can_warm,
            finished.clone(),
        ));
        Self {
            account,
            owner,
            sender,
            online,
            network,
            finished,
            cancel,
            task: Mutex::new(Some(task)),
        }
    }
    pub fn request(&self, epoch: u64, user_demand: bool) -> Result<bool> {
        self.account.current(epoch)?;
        if self.cancel.is_cancelled() {
            return Err(Error::SessionChanged);
        }
        let admitted = self.owner.request(epoch, user_demand)?;
        if admitted {
            if self.sender.try_send(epoch).is_err() {
                let _ = self.owner.cancel(epoch);
                self.finished
                    .send_modify(|revision| *revision = revision.wrapping_add(1));
                return Err(Error::InvalidRequest);
            }
        }
        Ok(admitted)
    }
    /// Join the existing shared continuation; caller admission owns the timeout.
    pub async fn wait_ready(&self, epoch: u64) -> Result<()> {
        let mut finished = self.finished.subscribe();
        let mut boundary = self.account.subscribe_boundary();
        self.account.current(epoch)?;
        if !self.online.load(Ordering::Acquire) {
            return Err(Error::StyleTransport(
                govorun_core::stylization::Failure::Offline,
            ));
        }
        self.request(epoch, true)?;
        loop {
            self.account.current(epoch)?;
            if self.cancel.is_cancelled() {
                return Err(Error::SessionChanged);
            }
            if !self.online.load(Ordering::Acquire) {
                return Err(Error::StyleTransport(
                    govorun_core::stylization::Failure::Offline,
                ));
            }
            match self.owner.readiness(epoch)? {
                // Readiness describes background recovery, not permission to
                // improve. A retained failure must leave foreground free to
                // use independent passes or attempt its own exact issuance.
                Some(_) => return Ok(()),
                None => {}
            }
            tokio::select! {
                _ = self.cancel.cancelled() => return Err(Error::SessionChanged),
                result = boundary.changed() => { result.map_err(|_| Error::SessionChanged)?; },
                result = finished.changed() => { result.map_err(|_| Error::SessionChanged)?; },
            }
        }
    }
    pub fn connectivity(&self, online: bool) -> Result<()> {
        if self.cancel.is_cancelled() {
            return Err(Error::SessionChanged);
        }
        self.online.store(online, Ordering::Release);
        let epoch = self.account.snapshot().session_generation;
        if !online {
            self.owner.unavailable(epoch)?;
        }
        self.network
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        if online {
            self.request(epoch, false)?;
        }
        Ok(())
    }
    pub fn ready(&self, epoch: u64) -> Result<bool> {
        self.owner.ready(epoch)
    }
    pub async fn stop(&self) {
        self.cancel.cancel();
        let task = self.task.lock().ok().and_then(|mut task| task.take());
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
    owner: Arc<Reconnect>,
    mut mailbox: mpsc::Receiver<u64>,
    online: Arc<AtomicBool>,
    mut network: watch::Receiver<u64>,
    cancel: Cancellation,
    operation: Operation,
    can_warm: CanWarm,
    finished: watch::Sender<u64>,
) {
    let mut changes = account.subscribe();
    let mut boundary = account.subscribe_boundary();
    let mut active: Option<u64> = None;
    let mut effect: Option<Future<'static, bool>> = None;
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            changed = boundary.changed() => {
                if changed.is_err() { break; }
                if let Some(epoch) = active.take() { let _ = owner.cancel(epoch); }
                effect = None;
                continue;
            },
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let snapshot = account.snapshot();
                if snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending { continue; }
                if let Some(epoch) = active.take() { let _ = owner.cancel(epoch); }
                effect = None;
                continue;
            },
            changed = network.changed() => {
                if changed.is_err() { break; }
                if online.load(Ordering::Acquire) { continue; }
                if let Some(epoch) = active.take() { let _ = owner.cancel(epoch); }
                effect = None;
                continue;
            },
            request = mailbox.recv() => {
                let Some(epoch) = request else { break; };
                if account.current(epoch).is_err() { continue; }
                active = Some(epoch);
            },
            completed = async { effect.as_mut().expect("enabled operation").await }, if effect.is_some() => {
                effect = None;
                if let Some(epoch) = active {
                    if owner.complete(epoch, completed).is_err() { active = None; }
                }
            },
        }
        let Some(epoch) = active else {
            continue;
        };
        loop {
            match owner.next(epoch, can_warm(), online.load(Ordering::Acquire)) {
                Ok(Step::FollowUp) => continue,
                Ok(Step::FinishSatisfied | Step::RetainForLater) | Err(_) => {
                    finished.send_modify(|revision| *revision = revision.wrapping_add(1));
                    active = None;
                    break;
                }
                Ok(step) => {
                    effect = Some(operation(epoch, step));
                    break;
                }
            }
        }
    }
    if let Some(epoch) = active {
        let _ = owner.cancel(epoch);
    }
    // No detached effect survives stop, a network loss or a session boundary.
}
