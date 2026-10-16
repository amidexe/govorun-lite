//! Per-operation, content-free measurements. Transport facts never contain text,
//! tokens, raw headers or endpoints, and cannot authorize credit usage confirmation.
use super::{Failure, Kind, Response};
use crate::Error;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::time::Instant;

#[derive(Clone, Copy)]
pub enum ExchangeEvent {
    Headers(u16),
    Complete,
    Deadline(u64),
}
pub type Sink = Arc<dyn Fn(String) + Send + Sync>;
pub(super) struct Timing(Mutex<State>);
struct State {
    started: Instant,
    phase_started: Instant,
    phase: &'static str,
    numbers: BTreeMap<&'static str, u64>,
    acceptance: Vec<Arc<Mutex<Sample>>>,
    server: Option<&'static str>,
    timeout_scope: Option<&'static str>,
    completed: Option<String>,
}
struct Sample {
    started: Instant,
    elapsed: u64,
    stage: &'static str,
    status: Option<u16>,
    result: &'static str,
    deadline: bool,
    timeout: Option<u64>,
    finished: bool,
}
pub(super) struct Exchange {
    sample: Arc<Mutex<Sample>>,
}
impl Exchange {
    pub(super) fn observe(&self, event: ExchangeEvent) {
        if let Ok(mut sample) = self.sample.lock() {
            match event {
                ExchangeEvent::Headers(status) if (100..=599).contains(&status) => {
                    sample.status = Some(status);
                    sample.stage = "body";
                }
                ExchangeEvent::Complete => sample.stage = "complete",
                ExchangeEvent::Deadline(limit) => {
                    sample.deadline = true;
                    sample.timeout = Some(limit.clamp(1, 120_000));
                }
                _ => {}
            }
        }
    }
}
impl Timing {
    pub(super) fn new(deadline: u64) -> Self {
        let numbers = [
            "local_ms",
            "start_ms",
            "wait_ms",
            "finalize_ms",
            "starts",
            "polls",
            "transport_errors",
            "transport_timeouts",
            "http_errors",
        ]
        .into_iter()
        .map(|k| (k, 0))
        .chain([("deadline_ms", deadline)])
        .collect();
        Self(Mutex::new(State {
            started: Instant::now(),
            phase_started: Instant::now(),
            phase: "local",
            numbers,
            acceptance: vec![],
            server: None,
            timeout_scope: None,
            completed: None,
        }))
    }
    pub(super) fn on_demand(&self, count: u64) {
        if let Ok(mut state) = self.0.lock() {
            state.numbers.insert("on_demand_issues", count.min(120_000));
        }
    }
    pub(super) fn timeout(&self, scope: &'static str, limit: u64) {
        if !matches!(scope, "overall" | "admission" | "round_trip")
            || !(1..=120_000).contains(&limit)
        {
            return;
        }
        if let Ok(mut s) = self.0.lock() {
            if s.completed.is_none() {
                s.timeout_scope = Some(scope);
                s.numbers.insert("timeout_limit_ms", limit);
            }
        }
    }
    pub(super) fn current_phase(&self) -> &'static str {
        self.0.lock().map(|state|state.phase).unwrap_or("local")
    }
    pub(super) fn phase(&self, phase: &'static str) {
        if let Ok(mut s) = self.0.lock() {
            if s.completed.is_some() {
                return;
            }
            advance(&mut s);
            s.phase = phase;
        }
    }
    pub(super) fn begin(&self, kind: Kind) -> Exchange {
        let sample = Arc::new(Mutex::new(Sample {
            started: Instant::now(),
            elapsed: 0,
            stage: "transport",
            status: None,
            result: "local_error",
            deadline: false,
            timeout: None,
            finished: false,
        }));
        if let Ok(mut s) = self.0.lock() {
            if s.completed.is_some() {
                return Exchange { sample };
            }
            *s.numbers
                .entry(if kind == Kind::Start {
                    "starts"
                } else {
                    "polls"
                })
                .or_default() += 1;
            s.numbers.remove("last_http");
            s.numbers.remove("last_response_ms");
            if kind == Kind::Start && s.acceptance.len() < 2 {
                s.acceptance.push(sample.clone());
            }
        }
        Exchange { sample }
    }
    pub(super) fn end(
        &self,
        exchange: Exchange,
        response: &std::result::Result<Response, Failure>,
    ) {
        if let (Ok(mut s), Ok(mut sample)) = (self.0.lock(), exchange.sample.lock()) {
            if s.completed.is_some() {
                return;
            }
            sample.finished = true;
            sample.elapsed = millis(sample.started);
            sample.result = match response {
                Ok(response) if response.status >= 400 => "http_error",
                Ok(_) => "success",
                Err(_) if sample.deadline => "round_trip_deadline",
                Err(Failure::Deadline) => "round_trip_deadline",
                Err(Failure::Cancelled) => "cancelled",
                Err(_) => "transport_error",
            };
            s.numbers.insert("last_response_ms", sample.elapsed);
            if let Ok(response) = response {
                s.numbers.insert("last_http", response.status as u64);
                if response.status >= 400 {
                    *s.numbers.entry("http_errors").or_default() += 1;
                }
            } else {
                *s.numbers.entry("transport_errors").or_default() += 1;
                if sample.result == "round_trip_deadline" {
                    *s.numbers.entry("transport_timeouts").or_default() += 1;
                    if let Some(limit) = sample.timeout {
                        s.timeout_scope = Some("round_trip");
                        s.numbers.insert("timeout_limit_ms", limit);
                    }
                }
            }
        }
    }
    pub(super) fn server(&self, job: &serde_json::Value) {
        if let Ok(mut s) = self.0.lock() {
            if s.completed.is_some() {
                return;
            }
            let kind = job["kind"].as_str();
            if !matches!(kind, Some("processing" | "completed" | "failed"))
                || !job["serverElapsedMs"].is_u64()
            {
                return;
            }
            s.server = match kind {
                Some("processing") => Some("processing"),
                Some("completed") => Some("completed"),
                _ => Some("failed"),
            };
            for (wire, local) in [
                ("serverElapsedMs", "server_elapsed_ms"),
                ("serverAdmissionMs", "server_admission_ms"),
                ("serverQueueMs", "server_queue_ms"),
                ("serverForwardMs", "server_forward_ms"),
                ("serverFinalizeMs", "server_finalize_ms"),
            ] {
                s.numbers.remove(local);
                if let Some(value) = job[wire].as_u64() {
                    s.numbers.insert(local, value);
                }
            }
        }
    }
    pub(super) fn finish(&self, error: Option<Error>) -> Option<String> {
        let mut s = self.0.lock().ok()?;
        if let Some(completed) = &s.completed {
            return Some(completed.clone());
        }
        advance(&mut s);
        let elapsed = millis(s.started);
        s.numbers.insert("elapsed_ms", elapsed);
        let deadline = s
            .acceptance
            .iter()
            .any(|a| a.lock().is_ok_and(|a| a.result == "round_trip_deadline"));
        let issuer = match error {
            Some(Error::StyleIssuance { stage, status }) => Some((stage,status)),
            Some(Error::StyleQuota(_)) => Some(("issue",429)),
            _ => None,
        };
        let access = if let Some((stage,status)) = issuer {
            s.phase = "access";
            s.numbers.insert("access_http", status as u64);
            s.numbers.insert("access_attempts", 1);
            s.numbers.insert("last_http", status as u64);
            *s.numbers.entry("http_errors").or_default() += 1;
            Some(stage)
        } else { None };
        let result = match error {
            None => "success",
            Some(Error::StyleIssuance { .. }) => "issue_failed",
            Some(Error::StyleTransport(Failure::Cancelled)) => "cancelled",
            Some(Error::StyleTransport(Failure::Deadline)) if s.timeout_scope == Some("overall") => {
                "client_deadline"
            }
            Some(Error::StyleTransport(Failure::Deadline)) if s.timeout_scope == Some("admission") => {
                "admission_deadline"
            }
            Some(Error::StyleTransport(Failure::Deadline))
                if deadline || s.timeout_scope == Some("round_trip") =>
            {
                "round_trip_deadline"
            }
            // An unannotated transport deadline has no proven timer origin.
            Some(Error::StyleTransport(Failure::Deadline)) => "transport_timeout",
            Some(Error::Http(504)) => "http_timeout",
            Some(error) => match super::failure::decide(error).reason {
                "style_pass_recovery_pending" => "recovery_pending",
                "style_pass_reserve_empty" => "reserve_empty",
                "account_state_changed" => match error {
                    Error::StyleAccess(super::AccessFailure::UsageUnavailable) | Error::StyleQuota(super::AccessFailure::UsageUnavailable) => "usage_unavailable",
                    Error::StyleAccess(super::AccessFailure::DailyLimit) | Error::StyleQuota(super::AccessFailure::DailyLimit) => "daily_limit",
                    Error::StyleAccess(super::AccessFailure::PeriodLimit) | Error::StyleQuota(super::AccessFailure::PeriodLimit) => "period_limit",
                    Error::ConsentUnconfirmed | Error::TermsMismatch
                    | Error::StyleAccess(super::AccessFailure::NoAccess) => "access_required",
                    _ => "account_changed",
                },
                "confirmed_offline" => "confirmed_offline",
                "transport_failed" => "transport_error",
                "server_busy" => "server_busy",
                "server_timeout" => "server_timeout",
                "server_unavailable" => if matches!(error, Error::Http(_)) { "http_error" } else { "server_unavailable" },
                "invalid_response" => "invalid_response",
                _ => "local_error",
            },
        };
        let mut fields = vec![
            "timing_v1".to_owned(),
            format!("phase={}", s.phase),
            format!("result={result}"),
        ];
        if let Some(stage) = access {
            fields.push(format!("access_stage={stage}"));
            fields.push("access_reason=http".into());
        }
        fields.extend(
            s.numbers
                .iter()
                .map(|(k, v)| format!("{k}={}", v.min(&120_000))),
        );
        if let Some(server) = s.server {
            fields.push(format!("server_sample={server}"));
        }
        if let Some(scope) = s.timeout_scope {
            fields.push(format!("timeout_scope={scope}"));
        }
        for (index, sample) in s.acceptance.iter().enumerate() {
            let mut sample = sample.lock().ok()?;
            if !sample.finished {
                sample.elapsed = millis(sample.started);
                sample.result = match result {
                    "admission_deadline"
                    | "client_deadline"
                    | "round_trip_deadline"
                    | "cancelled" => result,
                    _ => "local_error",
                };
                sample.finished = true;
            }
            let index = index + 1;
            fields.push(format!("a{index}_ms={}", sample.elapsed.min(120_000)));
            fields.push(format!("a{index}_stage={}", sample.stage));
            fields.push(format!("a{index}_result={}", sample.result));
            if let Some(status) = sample.status {
                fields.push(format!("a{index}_http={status}"));
            }
        }
        let record = fields.join(" ");
        if crate::journal_report::timing::parse(&record).is_none() {
            return None;
        }
        s.completed = Some(record.clone());
        Some(record)
    }
}
fn millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(120_000) as u64
}
fn advance(s: &mut State) {
    let key = match s.phase {
        "start" => "start_ms",
        "wait" => "wait_ms",
        "finalize" => "finalize_ms",
        _ => "local_ms",
    };
    *s.numbers.entry(key).or_default() += millis(s.phase_started);
    s.phase_started = Instant::now();
}
