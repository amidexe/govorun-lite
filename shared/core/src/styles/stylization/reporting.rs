//! One process-local owner shared by piggyback and standalone reports. A batch
//! is restored only before any possible send, never after ambiguous delivery.
use super::{
    reliability::{Counters, Lease},
    Gateway, Request,
};
use crate::{AccountSession, Error, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::{
    sync::{watch, Mutex as AsyncMutex},
    time::{Duration, Instant},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub official_enabled: bool,
    pub self_hosted: bool,
    pub accepted_terms: u32,
}

impl Settings {
    pub fn official_selected(self, terms: u32) -> bool {
        self.official_enabled && !self.self_hosted && self.accepted_terms == terms
    }
}

struct State {
    epoch: u64,
    counters: Counters,
    next_attempt: Instant,
    settings: Option<Settings>,
}
pub struct Reporting {
    account: AccountSession,
    gateway: Arc<dyn Gateway>,
    state: Mutex<State>,
    sending: AsyncMutex<()>,
    changed: watch::Sender<u64>,
    events: watch::Sender<u64>,
}
impl Reporting {
    pub fn new(account: AccountSession, gateway: Arc<dyn Gateway>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                epoch: account.snapshot().session_generation,
                counters: Counters::default(),
                next_attempt: Instant::now(),
                settings: None,
            }),
            account,
            gateway,
            sending: AsyncMutex::new(()),
            changed: watch::channel(0).0,
            events: watch::channel(0).0,
        })
    }
    fn state(&self) -> Result<std::sync::MutexGuard<'_, State>> {
        let mut state = self.state.lock().map_err(|_| Error::InvalidResponse)?;
        let epoch = self.account.snapshot().session_generation;
        if state.epoch != epoch {
            state.counters.end_session();
            state.epoch = epoch;
            state.next_attempt = Instant::now();
        }
        let snapshot = self.account.snapshot();
        if snapshot.logout_pending
            || !snapshot.signed_in && snapshot.storage_available
            || self.ended(&state)
        {
            state.counters.end_session();
        }
        Ok(state)
    }
    pub fn record(&self, reason: &str) -> Result<bool> {
        let mut state = self.state()?;
        if !self.account.snapshot().signed_in || self.ended(&state) {
            return Ok(false);
        }
        let changed = state.counters.record(reason);
        drop(state);
        if changed {
            self.notify();
        }
        Ok(changed)
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.events.subscribe()
    }
    fn notify(&self) {
        self.events
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
    pub fn clear(&self) -> Result<()> {
        self.state()?.counters.end_session();
        self.changed
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        self.notify();
        Ok(())
    }
    /// Unknown local settings suspend reporting. Known opt-out, self-hosting,
    /// or terms revocation destroys this process-local queue and cancels I/O.
    pub fn configure(&self, settings: Option<Settings>) -> Result<()> {
        let mut state = self.state()?;
        if state.settings == settings {
            return Ok(());
        }
        state.settings = settings;
        if self.ended(&state) {
            state.counters.end_session();
        }
        drop(state);
        self.changed
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        self.notify();
        Ok(())
    }
    fn ended(&self, state: &State) -> bool {
        let terms = self.account.snapshot().access.client_terms_version;
        let local = state.settings.is_some_and(|settings| {
            !settings.official_selected(terms)
        });
        local
            || self.account.profile().is_some_and(|profile| {
                let wire = profile.wire();
                wire["onlineConsent"]["active"] != true
                    || wire["onlineConsent"]["acceptedVersion"] != terms
                    || wire["onlineTermsVersion"] != terms
            })
    }
    fn allowed(&self) -> Result<bool> {
        let state = self.state()?;
        let snapshot = self.account.snapshot();
        Ok(state.settings.is_some()
            && !self.ended(&state)
            && snapshot.signed_in
            && snapshot.storage_available
            && !snapshot.logout_pending
            && snapshot.access.consent_accepted)
    }
    pub(super) fn detach(self: &Arc<Self>, epoch: u64) -> Result<Option<Detached>> {
        self.account.current(epoch)?;
        let lease = self.state()?.counters.detach();
        if lease.is_some() { self.notify(); }
        Ok(lease.map(|lease| Detached {
            owner: self.clone(),
            epoch,
            lease: Some(lease),
        }))
    }
    /// Only an existing, authorized batch can arm the executor's one-shot wait.
    pub fn pending_deadline(&self, online: bool) -> Option<Instant> {
        if !online || !self.allowed().ok()? { return None; }
        let state = self.state().ok()?;
        state.counters.pending_report().then_some(state.next_attempt)
    }
    /// Events and an existing batch's deadline share one single-flight owner.
    pub async fn flush(
        self: &Arc<Self>,
        epoch: u64,
        online: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<bool> {
        let Ok(_sending) = self.sending.try_lock() else {
            return Ok(false);
        };
        self.account.current(epoch)?;
        let mut changes = self.changed.subscribe();
        if !online() || !self.allowed()? {
            return Ok(false);
        }
        let detached = {
            let mut state = self.state()?;
            if Instant::now() < state.next_attempt {
                return Ok(false);
            }
            let Some(lease) = state.counters.detach() else {
                return Ok(false);
            };
            state.next_attempt = Instant::now() + Duration::from_secs(15 * 60);
            Detached {
                owner: self.clone(),
                epoch,
                lease: Some(lease),
            }
        };
        let mut detached = detached;
        let request = Request::reliability(detached.batch()?.clone())?;
        self.account.current(epoch)?;
        if !self.allowed()? {
            return Ok(false);
        }
        if !online() {
            return Ok(false);
        }
        detached.sent();
        tokio::select! {
            biased;
            _ = changes.changed() => {},
            _ = self.account.send_gateway_scoped(epoch, self.gateway.as_ref(), request) => {},
        }
        // No retries, HTTP/body logging, or diagnostics about diagnostics.
        Ok(true)
    }
    pub async fn outcome(
        &self,
        epoch: u64,
        pass: String,
        outcome: &str,
        latency: &str,
    ) -> Result<bool> {
        self.account.current(epoch)?;
        let mut changes = self.changed.subscribe();
        if !self.allowed()? || !self.account.snapshot().access.processing_ready {
            return Ok(false);
        }
        let request = Request::outcome(pass, outcome, latency)?;
        self.account.current(epoch)?;
        tokio::select! {
            biased;
            _ = changes.changed() => {},
            _ = self.account.send_gateway_scoped(epoch, self.gateway.as_ref(), request) => {},
        }
        Ok(true)
    }
}
pub(super) struct Detached {
    owner: Arc<Reporting>,
    epoch: u64,
    lease: Option<Lease>,
}
impl Detached {
    fn batch(&self) -> Result<&super::reliability::Batch> {
        self.lease
            .as_ref()
            .map(|v| &v.batch)
            .ok_or(Error::InvalidRequest)
    }
    pub(super) fn wire(&self) -> Result<Value> {
        serde_json::to_value(self.batch()?).map_err(|_| Error::InvalidRequest)
    }
    pub(super) fn sent(&mut self) {
        self.lease.take();
    }
}
impl Drop for Detached {
    fn drop(&mut self) {
        let Some(lease) = self.lease.take() else {
            return;
        };
        if self.owner.account.current(self.epoch).is_err() {
            return;
        }
        if let Ok(mut state) = self.owner.state() {
            if state.epoch == self.epoch {
                state.counters.restore_before_wire(lease);
            }
        }
        self.owner.notify();
    }
}
