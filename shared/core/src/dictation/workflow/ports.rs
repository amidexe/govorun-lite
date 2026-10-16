use serde_json::Value;
use std::{future::Future, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    Effect(&'static str),
    SelfHostedHttp(u16),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Recognizing,
    Styling,
}

// Deliberately no Debug/Serialize: neither raw text nor anonymous passes may
// escape through generic UI events or logs.
pub struct Recognition {
    pub text: String,
    pub speech_samples: usize,
    pub failed_chunks: usize,
}
pub struct Styled {
    pub text: String,
    pub outcome_pass: Option<String>,
    pub latency_bucket: Option<String>,
    pub epoch: u64,
}

pub trait Ports {
    fn timing_origin(&self) -> Option<super::measurement::Origin> {None}
    fn record_measurement(&mut self, _value: super::measurement::Measurement, _inserted: bool) -> impl Future<Output = bool> + Send {async {false}}
    fn cancelled(&self) -> bool;
    fn session_generation(&self) -> u64;
    fn facts(
        &mut self,
        effect: &str,
        text: &str,
        epoch: u64,
    ) -> impl Future<Output = Result<Value, Failure>> + Send;
    fn marker(&self) -> &'static str;
    fn recognize(&mut self) -> impl Future<Output = Result<Recognition, Failure>> + Send;
    fn dictionary(&mut self, text: String) -> impl Future<Output = String> + Send;
    fn record_recognition(
        &mut self,
        samples: usize,
        elapsed: Duration,
        success: bool,
    ) -> impl Future<Output = ()> + Send;
    fn record_session(
        &mut self,
        text: &str,
        speech_samples: usize,
    ) -> impl Future<Output = ()> + Send;
    fn record_local_completion(&mut self) -> impl Future<Output = ()> + Send { async {} }
    fn record_self_host_failure(&mut self, _status: Option<u16>) -> impl Future<Output = ()> + Send { async {} }
    fn prepare(&mut self, credits: u64) -> impl Future<Output = Result<u64, Failure>> + Send;
    fn stylize(
        &mut self,
        text: &str,
        preparation_millis: u64,
    ) -> impl Future<Output = Result<Styled, Failure>> + Send;
    fn self_hosted(&mut self, text: &str) -> impl Future<Output = Result<String, Failure>> + Send;
    fn paste(&mut self, text: String) -> impl Future<Output = bool> + Send;
    fn did_style(&mut self);
    fn progress(&mut self, stage: Stage);
    fn hide_progress(&mut self);
    fn fallback(&mut self);
    fn failure(&mut self, reason: &str);
    fn report_outcome(&mut self, pass: String, bucket: String, inserted: bool, epoch: u64);
}
