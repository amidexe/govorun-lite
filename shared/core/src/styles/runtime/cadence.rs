//! Reserve opportunities only; no text, paid objects or network I/O is retained.
//! The reserve scheduler owns retries. Only startup, connectivity, scope changes
//! and jittered post-use opportunities enqueue work; UTC boundaries do not.
use super::{reporting::Online, workers::Enabled, Stylization};
use crate::{features::cancellation::Cancellation, stylization::scheduling::Trigger};
use std::{
    sync::{Arc, Mutex, Weak},
};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{Duration, Instant},
};

const MINUTE: u64 = 60_000;
const MAX_DELAY: u64 = 20 * MINUTE;
enum Signal {
    Used(u64),
    Immediate(u64),
}
pub(super) struct Worker {
    sender: mpsc::Sender<Signal>,
    cancel: Cancellation,
    task: Mutex<Option<JoinHandle<()>>>,
}
impl Worker {
    pub(super) fn start(owner: &Arc<Stylization>, enabled: Enabled, online: Online) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let cancel = Cancellation::default();
        let task = tokio::spawn(run(
            Arc::downgrade(owner),
            receiver,
            cancel.clone(),
            enabled,
            online,
        ));
        Self {
            sender,
            cancel,
            task: Mutex::new(Some(task)),
        }
    }
    pub(super) fn used(&self, epoch: u64) {
        let _ = self.sender.try_send(Signal::Used(epoch));
    }
    pub(super) fn wake(&self, epoch: u64) {
        let _ = self.sender.try_send(Signal::Immediate(epoch));
    }
    pub(super) async fn stop(&self) {
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
fn enqueue(pending: &mut Option<(u64, Instant)>, epoch: u64, delay: u64) {
    let due = Instant::now() + Duration::from_millis(delay);
    if pending.is_none_or(|(saved, at)| saved != epoch || due < at) {
        *pending = Some((epoch, due));
    }
}
fn clock_delay(owner: &Stylization) -> u64 {
    MINUTE + (owner.account.inner.entropy)() % (MAX_DELAY - MINUTE + 1)
}
async fn run(
    owner: Weak<Stylization>,
    mut signals: mpsc::Receiver<Signal>,
    cancel: Cancellation,
    enabled: Enabled,
    online: Online,
) {
    let Some(initial) = owner.upgrade() else {
        return;
    };
    let mut changes = initial.account.subscribe();
    drop(initial);
    let mut pending: Option<(u64, Instant)> = None;
    let mut active = None;
    loop {
        let signal = tokio::select! {
            biased;
            _=cancel.cancelled()=>break,
            signal=signals.recv()=>{let Some(signal)=signal else {break};Some(signal)},
            changed=changes.changed()=>{if changed.is_err() {break;}None},
            _=async {tokio::time::sleep_until(pending.expect("enabled timer").1).await},if pending.is_some()=>None,
        };
        let Some(owner) = owner.upgrade() else { break };
        let now = (owner.clock)();
        let snapshot = owner.account.snapshot();
        let epoch = snapshot.session_generation;
        let available = owner.current(epoch).is_ok()
            && owner.account.style_authority().authorized
            && owner.account.style_access_current(now)
            && online().await
            && enabled().await.unwrap_or(false);
        if !available {
            active = None;
            pending = None;
            continue;
        }
        let scope = owner
            .account
            .profile()
            .map(|profile| profile.wire()["styleAccess"]["usageBalance"]["scopeId"].clone());
        if active.as_ref() != Some(&(epoch, scope.clone())) {
            active = Some((epoch, scope));
            pending = None;
            let _ = owner.maintain(epoch, Trigger::Reconnect, true);
        }
        let Ok(_) = owner.observation(false, true).await else {
            pending = None;
            continue;
        };
        match signal {
            Some(Signal::Used(saved)) if saved == epoch => {
                let delay = clock_delay(&owner);
                enqueue(&mut pending, epoch, delay);
            }
            Some(Signal::Immediate(saved)) if saved == epoch => enqueue(&mut pending, epoch, 0),
            _ => {}
        }
        if pending.is_some_and(|(saved, at)| saved == epoch && at <= Instant::now()) {
            pending = None;
            let _ = owner.maintain(epoch, Trigger::Maintenance, true);
        }
    }
}
