//! Foreground composition: direct Rust data/account owners and OS effects.
//! There is no business reply pipe, raw-text UI event or credential view.
use super::AccountHost;
use crate::features::{
    dictation::{queue::Cancellation, Failure, Ports, Recognition, Stage, Styled},
    stylization::{commands::random_id, Stylization},
};
use govorun_core::{style_repository::DictationStyle, Error};
use serde_json::{json, Value};
use std::{
    future::Future,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

// These facts stay private to one recording. No Debug/Serialize.
pub struct OsContext {
    pub application: Option<String>,
    pub application_identity: crate::style_repository::ApplicationIdentity,
    pub input_mode: &'static str,
    pub marker: &'static str,
    pub streaming: bool,
    pub bypass: bool,
    pub draft: bool,
}
pub trait OsEffects: Send {
    fn timing_origin(&self) -> Option<crate::features::dictation::measurement::Origin> {None}
    fn context(&self) -> &OsContext;
    fn prepared_request_id(&self) -> Option<String> { None }
    fn recognize(&mut self) -> impl Future<Output = Result<Recognition, Failure>> + Send;
    fn paste(&mut self, text: String) -> impl Future<Output = bool> + Send;
    fn keep_original(&mut self, _original: &str) {}
    fn progress(&self, stage: Stage);
    fn style_progress(
        &self,
    ) -> Arc<dyn Fn(govorun_core::stylization::foreground::Stage) + Send + Sync>;
    fn hide_progress(&self);
    fn fallback(&self);
    fn failure(&self, code: &'static str);
}
pub struct DictationAttempt<O> {
    host: Arc<AccountHost>,
    os: O,
    cancellation: Cancellation,
    resolved: Option<DictationStyle>,
    epoch: u64,
    timing: Arc<std::sync::Mutex<Option<String>>>,
    self_host_requested: bool,
    pending_self_host_failure: Option<Option<u16>>,
    measured: bool,
    failure: bool,
}
fn failed(error: Error) -> Failure {
    let decision = govorun_core::stylization::failure::dictation(error);
    if decision.cancelled {
        Failure::Cancelled
    } else {
        Failure::Effect(decision.reason)
    }
}
impl AccountHost {
    pub fn attach_recording_readiness(
        &self,
        read: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<(), &'static str> {
        self.document_ports
            .get()
            .ok_or("documents_unavailable")?
            .attach_recording_readiness(read);
        Ok(())
    }
    pub async fn recording_setup_mode(
        &self,
    ) -> Result<govorun_core::permission_setup::Duration, &'static str> {
        let ports = self.document_ports.get().ok_or("documents_unavailable")?;
        let settings = ports
            .style_settings()
            .await
            .map_err(|_| "dictation_storage_unavailable")?;
        Ok(govorun_core::permission_setup::duration_mode(
            settings.is_some_and(|value| value.self_hosted),
            settings.is_some_and(|value| value.official_enabled),
        ))
    }
    pub async fn recording_timer(
        &self,
        local_timer_enabled: bool,
        configured_minutes: u32,
    ) -> Result<crate::features::dictation::timer::Plan, Failure> {
        let core = self
            .core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?;
        let epoch = core.snapshot().session_generation;
        let ports = self
            .document_ports
            .get()
            .ok_or(Failure::Effect("documents_unavailable"))?;
        let settings = ports.style_settings().await.map_err(failed)?;
        core.current(epoch).map_err(failed)?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        crate::features::dictation::timer::plan(
            settings.is_some_and(|v| v.self_hosted),
            settings.is_some_and(|v| v.official_enabled),
            core.snapshot().access.processing_ready,
            false,
            local_timer_enabled,
            configured_minutes,
        )
    }
    pub fn dictation_attempt<O: OsEffects>(
        self: &Arc<Self>,
        os: O,
        cancellation: Cancellation,
    ) -> Result<DictationAttempt<O>, Failure> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        self.core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?;
        self.documents_owner().map_err(Failure::Effect)?;
        Ok(DictationAttempt {
            host: self.clone(),
            os,
            cancellation,
            resolved: None,
            epoch: 0,
            timing: Arc::new(std::sync::Mutex::new(None)),
            self_host_requested: false,
            pending_self_host_failure: None,
            measured: false,
            failure: false,
        })
    }
}
impl<O: OsEffects> DictationAttempt<O> {
    fn styles(&self) -> Result<Arc<Stylization>, Failure> {
        self.host
            .stylization
            .get()
            .cloned()
            .ok_or(Failure::Effect("styles_unavailable"))
    }
    async fn access(&mut self, epoch: u64) -> Result<Value, Failure> {
        let native = self
            .host
            .document_ports
            .get()
            .ok_or(Failure::Effect("documents_unavailable"))?;
        let selected = native.stylization_enabled().await.map_err(failed)?;
        // Bind the durable reserve to the core profile before observing access.
        self.styles()?
            .observation(false, false)
            .await
            .map_err(failed)?;
        self.host
            .core
            .get()
            .ok_or(Failure::Effect("account_storage_unavailable"))?
            .dictation_access_facts(epoch, selected, true, None)
            .map_err(failed)
    }
    async fn record(&mut self, event: Value) {
        let result = self.host.record_statistics(event).await;
        if result.is_err() {
            self.os.failure("dictation_statistics_unavailable");
        }
    }
}
impl<O: OsEffects> Ports for DictationAttempt<O> {
    fn timing_origin(&self) -> Option<crate::features::dictation::measurement::Origin> {self.os.timing_origin()}
    async fn record_measurement(&mut self, value: crate::features::dictation::measurement::Measurement, inserted: bool) -> bool {
        let Some(ports)=self.host.document_ports.get() else {return false};
        let trace=self.timing.lock().unwrap().take();
        let status=self.pending_self_host_failure.flatten();
        let failed=self.pending_self_host_failure.is_some() || self.failure;
        ports.recording_timing(value.trace(trace,self.self_host_requested,failed,status,inserted));
        self.pending_self_host_failure=None;self.measured=true;
        true
    }
    fn cancelled(&self) -> bool {
        self.cancellation.is_cancelled() || self.host.stopping.load(Ordering::Acquire)
    }
    fn session_generation(&self) -> u64 {
        self.host
            .core
            .get()
            .map_or(0, |core| core.snapshot().session_generation)
    }
    fn marker(&self) -> &'static str {
        self.os.context().marker
    }
    async fn facts(&mut self, effect: &str, text: &str, epoch: u64) -> Result<Value, Failure> {
        Ok(match effect {
            "begin" => {
                json!({"kind":"begin","streaming":self.os.context().streaming,"bypass":self.os.context().bypass})
            }
            "readDraft" => json!({"kind":"draft","active":self.os.context().draft}),
            "readRoute" => {
                let (route, enabled) = if let Some(owner) = self.host.self_host() {
                    owner.route_flags().await.map_err(Failure::Effect)?
                } else {
                    (false, false)
                };
                json!({"kind":"route","selfHosted":route,"enabled":enabled})
            }
            "readOfficial" => {
                self.epoch = epoch;
                let settings = self
                    .host
                    .document_ports
                    .get()
                    .ok_or(Failure::Effect("documents_unavailable"))?
                    .style_settings()
                    .await
                    .map_err(failed)?
                    .ok_or(Failure::Effect("styles_unavailable"))?;
                json!({"kind":"official","units":text.encode_utf16().collect::<Vec<_>>(),
                    "enabled":settings.official_enabled,"acceptedTerms":settings.accepted_terms,
                    "currentTerms":self.host.core.get().ok_or(Failure::Effect("account_storage_unavailable"))?
                        .snapshot().access.client_terms_version,"access":self.access(epoch).await?})
            }
            "readConfiguration" => {
                json!({"kind":"configuration","configured":self.host.stylization.get().is_some()})
            }
            "readSelection" => {
                let style = self
                    .host
                    .documents_owner()
                    .map_err(Failure::Effect)?
                    .dictation_style(self.os.context().application.as_deref(), self.os.context().application_identity)
                    .await
                    .map_err(Failure::Effect)?;
                let enabled = style.enabled;
                self.resolved = Some(style);
                json!({"kind":"selection","enabled":enabled})
            }
            "prepared" => json!({"kind":"prepared","access":self.access(epoch).await?}),
            "readNetwork" => {
                let available = self
                    .host
                    .document_ports
                    .get()
                    .ok_or(Failure::Effect("documents_unavailable"))?
                    .network_observation()
                    .await
                    .map_err(failed)?;
                json!({"kind":"network","available":available})
            }
            _ => return Err(Failure::Effect("dictation_invalid_effect")),
        })
    }
    async fn recognize(&mut self) -> Result<Recognition, Failure> {
        self.os.recognize().await
    }
    async fn dictionary(&mut self, text: String) -> String {
        let result = match self.host.documents_owner() {
            Ok(owner) => owner.apply_dictionary(&text).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(text) => text,
            Err(_) => {
                self.os.failure("dictation_dictionary_unavailable");
                text
            }
        }
    }
    async fn record_recognition(&mut self, samples: usize, elapsed: Duration, success: bool) {
        self.record(json!({"kind":"performance","speechMilliseconds":samples.saturating_mul(1000)/16000,
            "processingMilliseconds":elapsed.as_millis().min(i64::MAX as u128) as i64,"success":success})).await;
    }
    async fn record_session(&mut self, text: &str, speech_samples: usize) {
        let milliseconds =
            (speech_samples.saturating_mul(1000) / 16000).min(i64::MAX as usize) as i64;
        match govorun_core::statistics_repository::recording_session_facts(
            text,
            milliseconds,
        ) {
            Ok(event) => self.record(event).await,
            Err(_) => self.os.failure("dictation_statistics_unavailable"),
        }
    }
    async fn record_local_completion(&mut self) {
        if let Some(ports) = self.host.document_ports.get() {ports.recording_completed();}
    }
    async fn record_self_host_failure(&mut self, status: Option<u16>) {
        if self.os.timing_origin().is_some() {self.pending_self_host_failure=Some(status);}
        else if let Some(ports) = self.host.document_ports.get() {ports.self_host_failed(status);}
    }
    async fn prepare(&mut self, credits: u64) -> Result<u64, Failure> {
        let required =
            usize::try_from(credits).map_err(|_| Failure::Effect("dictation_invalid_credits"))?;
        tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => Err(Failure::Cancelled),
            elapsed = self.host.join_dictation_preparation(self.epoch, required) => Ok(elapsed),
        }
    }
    async fn stylize(&mut self, text: &str, preparation_millis: u64) -> Result<Styled, Failure> {
        let mut payload = self
            .resolved
            .as_ref()
            .ok_or(Failure::Effect("dictation_missing_style"))?
            .payload
            .clone();
        payload["text"] = json!(text);
        payload["inputMode"] = json!(self.os.context().input_mode);
        let owner = self.styles()?;
        let _activity = self.host.foreground.enter().map_err(failed)?;
        let progress = self.os.style_progress();
        let request_id = match self.os.prepared_request_id() {
            Some(id) => id,
            None => random_id().map_err(failed)?,
        };
        let timing=self.os.timing_origin().map(|_| {
            let record=self.timing.clone();Arc::new(move |value| *record.lock().unwrap()=Some(value)) as govorun_core::stylization::timing::Sink
        });
        let output = tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => return Err(Failure::Cancelled),
            output = owner.process_with_timing(self.epoch, payload, random_id().map_err(failed)?, request_id,
                preparation_millis, progress.as_ref(),timing) => output,
        };
        let result = match output {
            Ok(output) => output,
            Err(error) => {
                let decision = govorun_core::stylization::failure::decide(error);
                if !decision.cancelled {
                    // One terminal dictation failure, independent of poll retries
                    // and of a later successful request carrying piggyback counts.
                    let _ = owner.dispatch(crate::features::stylization::commands::Command::Failure {
                        session_generation: self.epoch, reason: decision.reason.into(),
                    }, &|_| {}).await;
                }
                return Err(failed(error));
            }
        };
        Ok(Styled {
            text: result.text,
            outcome_pass: Some(result.outcome_pass),
            latency_bucket: Some(result.latency_bucket.into()),
            epoch: self.epoch,
        })
    }
    async fn self_hosted(&mut self, text: &str) -> Result<String, Failure> {
        self.self_host_requested=true;
        let owner = self
            .host
            .self_host()
            .ok_or(Failure::Effect("self_host_unavailable"))?;
        tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => Err(Failure::Cancelled),
            result = owner.process(text) => result.map_err(|error| match error {
                crate::features::self_host::network::Failure::Http(status) => Failure::SelfHostedHttp(status),
                _ => Failure::Effect("requestFailed"),
            }),
        }
    }
    async fn paste(&mut self, text: String) -> bool {
        self.os.paste(text).await
    }
    fn keep_original(&mut self, original: &str) {
        self.os.keep_original(original);
    }
    fn did_style(&mut self) {
        let host = self.host.clone();
        let epoch = self.epoch;
        tokio::spawn(async move {
            let _ = host.dismiss_starter_guidance(epoch).await;
        });
    }
    fn progress(&mut self, stage: Stage) {
        self.os.progress(stage);
    }
    fn hide_progress(&mut self) {
        self.os.hide_progress();
    }
    fn fallback(&mut self) {
        self.os.fallback();
    }
    fn failure(&mut self, reason: &str) {
        self.failure=true;
        // Core already chose a closed reason; avoid forwarding arbitrary text.
        let reason = match reason {
            "offline" => "offline",
            "serverBusy" => "serverBusy",
            "serverTimeout" => "serverTimeout",
            "serverUnavailable" => "serverUnavailable",
            "transportFailed" => "transportFailed",
            "accountStateChanged" => "accountStateChanged",
            "stylePassReserveEmpty" => "stylePassReserveEmpty",
            "stylePassRecoveryPending" => "stylePassRecoveryPending",
            "termsUpdateRequired" => "termsUpdateRequired",
            "serviceUnavailable" => "serviceUnavailable",
            "textTooLong" => "textTooLong",
            "notConfigured" => "notConfigured",
            "noChange" => "noChange",
            "invalidResponse" => "invalidResponse",
            _ => "requestFailed",
        };
        self.os.failure(reason);
    }
    fn report_outcome(&mut self, pass: String, bucket: String, inserted: bool, epoch: u64) {
        let host = self.host.clone();
        if host.stopping.load(Ordering::Acquire) {
            return;
        }
        // Admit before spawning so shutdown cannot pass the drain while this
        // already completed insertion still has an outcome waiting to run.
        let Ok(activity) = host.foreground.enter() else {
            self.os.failure("dictation_outcome_unavailable");
            return;
        };
        tokio::spawn(async move {
            let _activity = activity;
            if let Some(owner) = host.stylization.get() {
                let _ = owner
                    .report_admitted_outcome(
                        epoch,
                        pass,
                        if inserted {
                            "delivered"
                        } else {
                            "insertion_failed"
                        },
                        &bucket,
                    )
                    .await;
            }
        });
    }
}
impl<O> Drop for DictationAttempt<O> {
    fn drop(&mut self) {
        if self.measured {return;}
        if let Some(ports)=self.host.document_ports.get() {
            if let Some(status)=self.pending_self_host_failure.take() {ports.self_host_failed(status);}
            if let Some(trace)=self.timing.lock().unwrap().take() {
                if crate::journal_report::timing::parse(&trace).is_some_and(|f| !["success","cancelled"].contains(&f["result"])) {ports.recording_timing(trace);}
            }
        }
    }
}
