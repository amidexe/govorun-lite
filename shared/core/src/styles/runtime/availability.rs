//! Only executes the shared circuit's bounded probe opportunities. A new event
//! can resume its remaining budget; no host retry count or backoff exists.
use super::workers::Enabled;
use crate::features::cancellation::Cancellation;
use govorun_core::{stylization::outage::Outage, AccountSession, Future, Result};
use serde_json::Value;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{Duration, Sleep},
};

pub type Probe = Arc<dyn Fn() -> Future<'static, Result<Value>> + Send + Sync>;
pub type Changed = Arc<dyn Fn(bool) + Send + Sync>;
pub struct Worker {
    cancel: Cancellation,
    task: Mutex<Option<JoinHandle<()>>>,
    suspend: watch::Sender<u64>,
}
impl Worker {
    pub fn start(
        account: AccountSession,
        owner: Arc<Outage>,
        enabled: Enabled,
        probe: Probe,
        changed: Changed,
    ) -> Self {
        let cancel = Cancellation::default();
        let (suspend, suspended) = watch::channel(0);
        let events = owner.subscribe();
        let published = owner.paused().unwrap_or(false);
        let task = tokio::spawn(run(
            account,
            owner,
            enabled,
            probe,
            changed,
            cancel.clone(),
            suspended,
            events,
            published,
        ));
        Self {
            cancel,
            task: Mutex::new(Some(task)),
            suspend,
        }
    }
    pub fn suspend(&self) {
        self.suspend
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
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
async fn run(
    account: AccountSession,
    owner: Arc<Outage>,
    enabled: Enabled,
    probe: Probe,
    changed: Changed,
    cancel: Cancellation,
    mut suspended: watch::Receiver<u64>,
    mut events: watch::Receiver<u64>,
    mut published: bool,
) {
    let mut changes = account.subscribe();
    let mut boundary = account.subscribe_boundary();
    let mut effect: Option<Future<'static, Result<Value>>> = None;
    let mut timer: Option<Pin<Box<Sleep>>> = None;
    // Recover a circuit opened immediately before composition as well.
    let mut scheduled = published;
    loop {
        if scheduled
            && effect.is_none()
            && timer.is_none()
            && account.snapshot().authority.processing_ready
        {
            let enabled = enabled.clone();
            let probe = probe.clone();
            effect = Some(Box::pin(async move {
                if enabled().await? {
                    probe().await
                } else {
                    Ok(serde_json::json!({"action":"stop"}))
                }
            }));
        }
        scheduled = false;
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            result = suspended.changed() => {
                if result.is_err() { break; }
                effect = None;
                timer = None;
            },
            result = boundary.changed() => {
                if result.is_err() { break; }
                effect = None;
                timer = None;
            },
            result = changes.changed() => {
                if result.is_err() { break; }
                if !account.snapshot().authority.processing_ready {
                    effect = None;
                    timer = None;
                } else { scheduled = owner.paused().unwrap_or(false); }
            },
            result = events.changed() => {
                if result.is_err() { break; }
                let paused = owner.paused().unwrap_or(false);
                if paused != published { published = paused; changed(paused); }
                timer = None;
                scheduled = paused;
            },
            result = async { effect.as_mut().expect("enabled probe").await }, if effect.is_some() => {
                effect = None;
                if let Ok(result) = result {
                    match result["action"].as_str() {
                        Some("wait") => {
                            if let Some(delay) = result["delayMillis"].as_u64() {
                                timer = Some(Box::pin(tokio::time::sleep(Duration::from_millis(delay))));
                            }
                        },
                        Some("stop") => {},
                        _ => scheduled = result["paused"] == true,
                    }
                }
            },
            _ = async { timer.as_mut().expect("enabled wait").await }, if timer.is_some() => {
                timer = None;
                scheduled = true;
            },
        }
    }
}
