//! Process-local availability. This drives the existing shared outage policy;
//! it retains no account identity, submitted text, token or persisted history.
use super::{Failure, Gateway, Request, ServiceFailure};
use crate::{Error, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::time::Instant;

pub(super) struct AttemptNotes(Mutex<u64>, pub(super) super::timing::Timing);
impl AttemptNotes {
    pub(super) fn new(deadline: u64) -> Self {
        Self(Mutex::new(0), super::timing::Timing::new(deadline))
    }
    pub(super) fn observe(&self, response: &super::Response, now: u64) {
        if matches!(response.status, 408 | 429 | 500 | 502 | 503 | 504) {
            let delay = crate::style_start::retry_after_delay(
                response.retry_after.as_deref(),
                now.min(i64::MAX as u64) as i64,
            )
            .unwrap_or(0);
            if let Ok(mut old) = self.0.lock() {
                *old = (*old).max(delay);
            }
        }
    }
    pub(super) fn delay(&self) -> u64 {
        self.0.lock().map_or(0, |v| *v)
    }
}

pub struct Outage {
    state: Mutex<Option<crate::online_outage::State>>,
    clock: Instant,
    jitter: Arc<dyn Fn() -> f64 + Send + Sync>,
    events: watch::Sender<u64>,
}
impl Outage {
    pub fn new(jitter: Arc<dyn Fn() -> f64 + Send + Sync>) -> Self {
        Self {
            state: Mutex::new(None),
            clock: Instant::now(),
            jitter,
            events: watch::channel(0).0,
        }
    }
    fn now(&self) -> i64 {
        self.clock.elapsed().as_millis().min(i64::MAX as u128) as i64
    }
    fn event(&self, event: crate::online_outage::Event) -> Result<Value> {
        let demand = matches!(event, crate::online_outage::Event::Demand | crate::online_outage::Event::Recording);
        let mut state = self.state.lock().map_err(|_| Error::InvalidResponse)?;
        let before = serde_json::to_value(state.as_ref()).map_err(|_| Error::InvalidResponse)?;
        let generation = before["generation"].as_u64().unwrap_or(0);
        let mut result = crate::online_outage::transition(state.take(), event, self.now())?;
        result["opened"] = json!(
            result["paused"] == true && result["state"]["generation"].as_u64() != Some(generation)
        );
        *state = Some(
            serde_json::from_value(result["state"].clone()).map_err(|_| Error::InvalidResponse)?,
        );
        if before["paused"].as_bool().unwrap_or(false) != (result["paused"] == true)
            || demand && result["paused"] == true
        {
            self.events
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
        Ok(result)
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.events.subscribe()
    }
    pub fn begin(&self, normal_deadline_millis: u64) -> Result<(u64, u64)> {
        let request = self.event(crate::online_outage::Event::BeginRequest)?;
        if request["action"] != "request" {
            return Err(Error::StyleService(ServiceFailure::Unavailable));
        }
        let deadline = self.event(crate::online_outage::Event::Deadline {
            normal: i64::try_from(normal_deadline_millis).map_err(|_| Error::InvalidRequest)?,
        })?;
        Ok((
            request["ticket"].as_u64().ok_or(Error::InvalidResponse)?,
            deadline["deadlineMillis"]
                .as_u64()
                .ok_or(Error::InvalidResponse)?,
        ))
    }
    pub fn finish(
        &self,
        ticket: u64,
        error: Option<Error>,
        retry_after_millis: u64,
    ) -> Result<bool> {
        let failure = classify(error);
        let failure = serde_json::from_value(failure).map_err(|_| Error::InvalidResponse)?;
        let result = self.event(crate::online_outage::Event::FinishRequest {
            ticket,
            failure,
            cancelled: matches!(
                error,
                Some(Error::SessionChanged | Error::StyleTransport(Failure::Cancelled))
            ),
            retry_after: retry_after_millis.min(i64::MAX as u64) as i64,
            jitter: (self.jitter)(),
            retry_hints: vec![],
        })?;
        // The native executor used to request recovery when a request opened
        // the circuit. Preserve that effect once; late results add no new budget.
        if result["opened"] == true {
            self.demand()?;
        }
        Ok(result["paused"] == true)
    }
    pub fn recording_started(&self) -> Result<()> {
        self.event(crate::online_outage::Event::Recording).map(|_| ())
    }
    pub fn demand(&self) -> Result<()> {
        self.event(crate::online_outage::Event::Demand).map(|_| ())
    }
    pub fn paused(&self) -> Result<bool> {
        Ok(self.event(crate::online_outage::Event::Deadline { normal: 1 })?["paused"] == true)
    }
    /// One scheduled opportunity. The policy bounds retries and requires a fresh
    /// availability sample; a stale CDN response cannot reopen processing.
    pub async fn probe(&self, gateway: &dyn Gateway) -> Result<Value> {
        let next = self.event(crate::online_outage::Event::NextProbe)?;
        if next["action"] != "probe" {
            return Ok(next);
        }
        let ticket = next["ticket"].as_u64().ok_or(Error::InvalidResponse)?;
        let guard = ProbeGuard {
            owner: self,
            ticket,
        };
        let started = Instant::now();
        let request = Request::availability()?;
        let response = tokio::time::timeout(request.timeout(), gateway.send(request))
            .await
            .unwrap_or(Err(Failure::Deadline));
        let elapsed = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
        let (healthy, age, retry) = match response {
            Ok(response) => {
                let proof =
                    crate::online_protocol::evaluate(crate::online_protocol::Command::Proof {
                        status: response.status,
                        body: response.body,
                        date: response.date,
                        modified: response.last_modified,
                    });
                let retry = if matches!(response.status, 408 | 429 | 500 | 502 | 503 | 504) {
                    crate::style_start::retry_after_delay(
                        response.retry_after.as_deref(),
                        chrono::Utc::now().timestamp_millis(),
                    )
                    .unwrap_or(0)
                } else {
                    0
                };
                (
                    proof.as_ref().is_ok_and(|v| v["healthy"] == true),
                    proof
                        .as_ref()
                        .ok()
                        .and_then(|v| v["sampleAge"].as_i64())
                        .unwrap_or(-1),
                    retry,
                )
            }
            Err(_) => (false, -1, 0),
        };
        let result = self.event(crate::online_outage::Event::FinishProbe {
            ticket,
            healthy,
            retry_after: retry.min(i64::MAX as u64) as i64,
            jitter: (self.jitter)(),
            sample_age: age,
            elapsed,
            retry_hints: vec![],
        });
        drop(guard);
        result
    }
}
struct ProbeGuard<'a> {
    owner: &'a Outage,
    ticket: u64,
}
impl Drop for ProbeGuard<'_> {
    fn drop(&mut self) {
        let _ = self.owner.event(crate::online_outage::Event::AbandonProbe {
            ticket: self.ticket,
        });
    }
}
fn classify(error: Option<Error>) -> Value {
    let (kind, status, code) = match error {
        Some(Error::NetworkUnavailable | Error::StyleTransport(Failure::Network)) => ("transport", None, None),
        Some(Error::StyleTransport(Failure::Deadline)) => ("deadline", None, None),
        Some(Error::Http(status)) => ("http", Some(status), None),
        Some(Error::StyleService(failure)) => (
            "service",
            None,
            Some(match failure {
                ServiceFailure::Busy => "busy",
                ServiceFailure::Timeout => "timeout",
                ServiceFailure::Unavailable => "unavailable",
            }),
        ),
        Some(Error::StyleTransport(Failure::Security)) => ("security", None, None),
        Some(Error::StyleTransport(Failure::Offline)) => ("offline", None, None),
        Some(Error::StyleTransport(Failure::Cancelled)) => ("cancelled", None, None),
        _ => ("ignored", None, None),
    };
    json!({"kind":kind,"status":status,"code":code})
}
