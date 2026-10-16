//! Shared foreground execution. The gateway port performs I/O; all admission,
//! replay, polling and usage confirmation decisions use the existing common policies.
use super::{Gateway, Request, Response};
use crate::{
    style_pass_vault::{Action, Repository},
    AccountSession, Error, Result,
};
use serde_json::{json, Value};
use std::sync::{atomic::{AtomicU64, Ordering}, Arc, OnceLock};
use tokio::{
    sync::{Mutex, Semaphore},
    time::{Duration, Instant},
};

pub struct Foreground {
    pub(super) account: AccountSession,
    pub(super) vault: Arc<Repository>,
    pub(super) gateway: Arc<dyn Gateway>,
    pub(super) clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub(super) admission: Mutex<()>,
    pub(super) active: Semaphore,
    outage: Arc<super::outage::Outage>,
    reporting: Arc<super::reporting::Reporting>,
    timing: OnceLock<super::timing::Sink>,
    selection: tokio::sync::watch::Sender<bool>,
    on_demand_issues: AtomicU64,
}
/// Private result for the caller that requested processing, never diagnostics.
pub struct Output {
    pub text: String,
    pub outcome_pass: String,
    pub latency_bucket: &'static str,
    pub(super) slow: bool,
}
impl Output {
    pub fn is_slow(&self) -> bool {
        self.slow
    }
}
#[derive(Clone, Copy)]
pub enum Stage {
    Checking,
    Accepted,
    Processing,
}
impl Foreground {
    pub fn new(
        account: AccountSession,
        vault: Arc<Repository>,
        gateway: Arc<dyn Gateway>,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Self {
        let entropy=account.inner.entropy.clone();
        Self {
            reporting: super::reporting::Reporting::new(account.clone(), gateway.clone()),
            account,
            vault,
            gateway,
            clock,
            admission: Mutex::new(()),
            active: Semaphore::new(2),
            outage: Arc::new(super::outage::Outage::new(Arc::new(move || entropy() as f64/u64::MAX as f64))),
            timing: OnceLock::new(),
            selection: tokio::sync::watch::channel(true).0,
            on_demand_issues: AtomicU64::new(0),
        }
    }
    pub(super) fn current(&self, epoch: u64) -> bool {
        self.account.current(epoch).is_ok()
    }
    pub(crate) fn prepare_connection(&self, request_id: String) {
        self.gateway.prepare_connection(request_id);
    }
    pub(crate) fn close_connection(&self, request_id: Option<&str>) {
        self.gateway.close_connection(request_id);
    }
    pub(crate) fn configure_selection(&self, selected: bool) {
        if !selected { self.close_connection(None); }
        self.selection.send_if_modified(|current| {
            if *current == selected { false } else { *current = selected; true }
        });
    }
    /// Validate current processing authority before a host starts any effects.
    pub fn allowed(&self, epoch: u64) -> Result<()> {
        self.attempt_current(epoch)?;
        if !self.account.style_authority().processing_ready || !self.account.style_access_current((self.clock)()) {
            return Err(Error::StyleAccess(super::AccessFailure::NoAccess));
        }
        Ok(())
    }
    /// An admitted attempt survives profile observations. Only local stored pass state,
    /// account boundaries and the user's cancellation revoke its lifetime.
    pub(super) fn attempt_current(&self, epoch: u64) -> Result<()> {
        self.account.current(epoch)?;
        let snapshot = self.account.snapshot();
        if !snapshot.storage_available { return Err(Error::StorageUnavailable); }
        if !snapshot.signed_in || snapshot.logout_pending { return Err(Error::SessionChanged); }
        if !*self.selection.borrow() { return Err(Error::ConsentUnconfirmed); }
        Ok(())
    }
    pub(super) async fn mutate(
        &self,
        epoch: u64,
        action: Action,
    ) -> Result<crate::style_pass_vault::InventoryChange> {
        self.attempt_current(epoch)?;
        self.vault
            .change(action, (self.clock)(), false, false, &|| self.current(epoch))
            .await
    }
    pub async fn run(
        &self,
        epoch: u64,
        payload: Value,
        lease_id: String,
        request_id: String,
        preparation_millis: u64,
        progress: &(dyn Fn(Stage) + Send + Sync),
    ) -> Result<Output> {
        self.run_ready(
            epoch,
            payload,
            lease_id,
            request_id,
            preparation_millis,
            progress,
            Box::pin(async { Ok(()) }),
        )
        .await
    }
    /// The common admission clock includes reconnect readiness before any lease.
    pub async fn run_ready(
        &self, epoch: u64, payload: Value, lease_id: String, request_id: String,
        preparation_millis: u64, progress: &(dyn Fn(Stage) + Send + Sync),
        readiness: crate::Future<'_, Result<()>>,
    ) -> Result<Output> {
        self.run_ready_with_timing(epoch,payload,lease_id,request_id,preparation_millis,progress,readiness,None).await
    }
    pub async fn run_ready_with_timing(
        &self,
        epoch: u64,
        payload: Value,
        lease_id: String,
        request_id: String,
        preparation_millis: u64,
        progress: &(dyn Fn(Stage) + Send + Sync),
        readiness: crate::Future<'_, Result<()>>,
        timing: Option<super::timing::Sink>,
    ) -> Result<Output> {
        self.allowed(epoch)?;
        let text = payload["text"].as_str().ok_or(Error::InvalidRequest)?;
        let admission = crate::style_text::admit(text.encode_utf16().collect())?;
        let (ticket, cap) = self.outage.begin(number(&admission, "deadlineMillis")?)?;
        let notes = super::outage::AttemptNotes::new(cap);
        let mut selection = self.selection.subscribe();
        let result = tokio::select! {
            biased;
            _ = async {
                loop {
                    if !*selection.borrow_and_update() { break; }
                    if selection.changed().await.is_err() { break; }
                }
            } => Err(Error::ConsentUnconfirmed),
            result = self.run_job(epoch, payload, lease_id, request_id, preparation_millis,
                cap, &notes, progress, readiness) => result,
        };
        notes.1.on_demand(self.on_demand_issues.load(Ordering::Relaxed));
        if let (Some(sink), Some(record)) = (
            timing.as_ref().or_else(|| self.timing.get()),
            notes.1.finish(result.as_ref().err().copied()),
        ) {
            sink(record);
        }
        self.outage.finish(ticket, result.as_ref().err().copied(), notes.delay())?;
        result
    }
    pub(crate) fn record_on_demand_issue(&self) {
        self.on_demand_issues.fetch_add(1, Ordering::Relaxed);
    }
    pub fn availability(&self) -> &Arc<super::outage::Outage> {
        &self.outage
    }
    pub fn set_timing_sink(&self, sink: super::timing::Sink) -> Result<()> {
        self.timing.set(sink).map_err(|_| Error::InvalidRequest)
    }
    pub fn reporting(&self) -> &Arc<super::reporting::Reporting> {
        &self.reporting
    }
    pub async fn probe_availability(&self) -> Result<Value> {
        self.outage.probe(self.gateway.as_ref()).await
    }
    pub(super) async fn start(
        &self,
        epoch: u64,
        payload: Value,
        request_id: String,
        end: Instant,
        timeout: (&'static str, u64),
        _lease_id: &str,
        progress: &(dyn Fn(Stage) + Send + Sync),
        notes: &super::outage::AttemptNotes,
        detached: &mut Option<super::reporting::Detached>,
    ) -> Result<Value> {
        let mut state = None;
        notes.1.phase("start");
        loop {
            self.attempt_current(epoch)?;
            let begin = crate::style_start::transition(
                state,
                crate::style_start::Event::Begin {
                    remaining_millis: remaining(end),
                },
            )?;
            state = Some(
                serde_json::from_value(begin["state"].clone())
                    .map_err(|_| Error::InvalidResponse)?,
            );
            if begin["action"] != "attempt" {
                notes.1.timeout(timeout.0, timeout.1);
                return Err(Error::StyleTransport(super::Failure::Deadline));
            }
            let request = Request::start(
                payload.clone(),
                request_id.clone(),
                Duration::from_millis(number(&begin, "timeoutMillis")?),
            )?;
            self.attempt_current(epoch)?;
            if let Some(detached) = detached.as_mut() {
                detached.sent();
            }
            let exchange = notes.1.begin(super::Kind::Start);
            let response = self
                .account
                .send_gateway_observed(epoch, self.gateway.as_ref(), request, &|event| {
                    exchange.observe(event)
                })
                .await;
            notes.1.end(exchange, &response);
            self.attempt_current(epoch)?;
            if let Ok(response) = &response {
                notes.observe(response, (self.clock)());
            }
            let next = match response {
                Ok(ref response) => crate::style_start::transition(
                    state,
                    crate::style_start::Event::Response {
                        status: response.status,
                        body: response.body.clone(),
                        retry_after: response.retry_after.clone(),
                        now_millis: (self.clock)().min(i64::MAX as u64) as i64,
                        remaining_millis: remaining(end),
                    },
                )?,
                Err(failure) => crate::style_start::transition(
                    state,
                    crate::style_start::Event::Transport {
                        failure: failure.start(),
                        remaining_millis: remaining(end),
                    },
                )?,
            };
            state = Some(
                serde_json::from_value(next["state"].clone())
                    .map_err(|_| Error::InvalidResponse)?,
            );
            match next["action"].as_str() {
                Some("retry") => {
                    tokio::time::sleep(Duration::from_millis(number(&next, "delayMillis")?)).await
                }
                Some("accept") => {
                    progress(Stage::Accepted);
                    let result = job(
                        crate::style_job_response::Phase::Start,
                        &response.map_err(super::Failure::error)?,
                    )?;
                    notes.1.server(&result);
                    return Ok(result);
                }
                Some("restore" | "discard") => {
                    return Ok(
                        json!({"kind":"rejected","settlement":if next["action"]=="restore" {"restore"} else {"discard"},
                        "httpStatus":if next["action"]=="restore" {429} else {401}}),
                    );
                }
                _ => return Err(response_error(response)),
            }
        }
    }
    pub(super) async fn poll(
        &self,
        epoch: u64,
        token: String,
        end: Instant,
        notes: &super::outage::AttemptNotes,
    ) -> Result<Value> {
        let mut state = None;
        notes.1.phase("wait");
        loop {
            self.attempt_current(epoch)?;
            let begin = crate::style_poll::transition(
                state,
                crate::style_poll::Event::Begin {
                    remaining_millis: remaining(end),
                },
            )?;
            state = Some(
                serde_json::from_value(begin["state"].clone())
                    .map_err(|_| Error::InvalidResponse)?,
            );
            if begin["action"] != "attempt" {
                return Err(Error::StyleTransport(super::Failure::Deadline));
            }
            let started = Instant::now();
            let exchange = notes.1.begin(super::Kind::Status);
            let response = self
                .account
                .send_gateway_observed(
                    epoch,
                    self.gateway.as_ref(),
                    Request::status(
                        token.clone(),
                        Duration::from_millis(number(&begin, "timeoutMillis")?),
                    )?,
                    &|event| exchange.observe(event),
                )
                .await;
            notes.1.end(exchange, &response);
            self.attempt_current(epoch)?;
            if let Ok(response) = &response {
                notes.observe(response, (self.clock)());
            }
            let next = match response {
                Ok(ref response) => crate::style_poll::transition(
                    state,
                    crate::style_poll::Event::Response {
                        status: response.status,
                        retry_after: response.retry_after.clone(),
                        now_millis: (self.clock)().min(i64::MAX as u64) as i64,
                        remaining_millis: remaining(end),
                    },
                )?,
                Err(failure) => crate::style_poll::transition(
                    state,
                    crate::style_poll::Event::Transport {
                        failure: failure.start(),
                        remaining_millis: remaining(end),
                    },
                )?,
            };
            state = Some(
                serde_json::from_value(next["state"].clone())
                    .map_err(|_| Error::InvalidResponse)?,
            );
            if next["action"] == "retry" {
                tokio::time::sleep(Duration::from_millis(number(&next, "delayMillis")?)).await;
                continue;
            }
            if next["action"] != "read" {
                return Err(response_error(response));
            }
            let job = job(
                crate::style_job_response::Phase::Status,
                &response.map_err(super::Failure::error)?,
            )?;
            notes.1.server(&job);
            if job["kind"] != "processing" {
                return Ok(job);
            }
            let next = crate::style_poll::transition(
                state,
                crate::style_poll::Event::Processing {
                    elapsed_millis: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                    remaining_millis: remaining(end),
                },
            )?;
            state = Some(
                serde_json::from_value(next["state"].clone())
                    .map_err(|_| Error::InvalidResponse)?,
            );
            if next["action"] != "wait" {
                return Err(Error::StyleTransport(super::Failure::Deadline));
            }
            tokio::time::sleep(Duration::from_millis(number(&next, "delayMillis")?)).await;
        }
    }
}
pub(super) fn number(value: &Value, key: &str) -> Result<u64> {
    value[key].as_u64().ok_or(Error::InvalidResponse)
}
pub(super) fn remaining(end: Instant) -> u64 {
    end.saturating_duration_since(Instant::now())
        .as_millis()
        .min(u64::MAX as u128) as u64
}
fn job(phase: crate::style_job_response::Phase, response: &Response) -> Result<Value> {
    serde_json::to_value(crate::style_job_response::decide(phase, &response.body))
        .map_err(|_| Error::InvalidResponse)
}
fn response_error(response: std::result::Result<Response, super::Failure>) -> Error {
    match response {
        Ok(response) => Error::Http(response.status),
        Err(failure) => failure.error(),
    }
}
pub(super) fn service_error(job: &Value) -> Error {
    use super::ServiceFailure;
    match job["failure"].as_str() {
        Some("busy") => Error::StyleService(ServiceFailure::Busy),
        Some("timeout") => Error::StyleService(ServiceFailure::Timeout),
        Some("unavailable") => Error::StyleService(ServiceFailure::Unavailable),
        _ => Error::InvalidResponse,
    }
}
pub(super) fn latency(ms: u128) -> &'static str {
    match ms {
        0..1000 => "lt1s",
        1000..2000 => "1to2s",
        2000..5000 => "2to5s",
        5000..10000 => "5to10s",
        10000..20000 => "10to20s",
        _ => "gte20s",
    }
}
