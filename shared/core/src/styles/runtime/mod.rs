//! Composition of the common anonymous processing and protected-credit owners.
//! Native code supplies a physical-store port only; no account credentials can
//! be represented by the separate gateway port.
use govorun_core::{
    style_pass_vault::{
        issuance::Issuance,
        presentation::{self, Observation},
        usage_sync::UsageSync,
        Repository,
    },
    stylization::{
        foreground::{Foreground, Output, Stage},
        Gateway,
    },
    AccountSession, Error, Result,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use tokio::sync::Mutex;
pub mod availability;
pub mod commands;
pub mod ipc;
mod lifecycle;
mod cadence;
mod maintenance;
mod preparation;
mod demand;
pub mod reconnect;
pub mod reporting;
pub mod scheduler;
pub mod usage;
pub mod workers;

pub type StatisticsSink = Arc<dyn Fn(u64, bool, bool) -> govorun_core::Future<'static, ()> + Send + Sync>;
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;
pub struct Stylization {
    account: AccountSession,
    vault: Arc<Repository>,
    foreground: Foreground,
    issuance: Issuance,
    usage: UsageSync,
    applied: Mutex<Value>,
    maintenance: Mutex<Option<maintenance::PendingRefresh>>,
    clock: Clock,
    statistics: OnceLock<StatisticsSink>,
    stopped: AtomicBool,
    workers: OnceLock<workers::Workers>,
    reporter: OnceLock<reporting::Reporter>,
    usage_worker: OnceLock<usage::Worker>,
    availability_worker: OnceLock<availability::Worker>,
    reconnect_worker: OnceLock<reconnect::Worker>,
    reserve_cadence: OnceLock<cadence::Worker>,
}
pub struct Preparation {
    pub ready: bool,
    pub retry_after_millis: Option<u64>,
    pub blocked_reason: Option<&'static str>,
    pub requires_recovery: bool,
    pub maintenance: Option<govorun_core::stylization::scheduling::Completion>,
}
impl Stylization {
    /// Construct after restore: the cached profile seeds the macOS scope binding
    /// before the first fresh observation; it never authorizes processing itself.
    pub fn new(
        account: AccountSession,
        vault: Arc<Repository>,
        gateway: Arc<dyn Gateway>,
        clock: Clock,
    ) -> Arc<Self> {
        Arc::new(Self {
            applied: Mutex::new(account.native_projection()),
            foreground: crate::stylization::foreground::Foreground::new(account.clone(), vault.clone(), gateway, clock.clone()),
            issuance: Issuance::new(vault.clone(), account.clone(), clock.clone()),
            usage: UsageSync::new(account.clone(), vault.clone(), clock.clone()),
            account,
            vault,
            clock,
            maintenance: Mutex::new(None),
            statistics: OnceLock::new(),
            stopped: AtomicBool::new(false),
            workers: OnceLock::new(),
            reporter: OnceLock::new(),
            usage_worker: OnceLock::new(),
            availability_worker: OnceLock::new(),
            reconnect_worker: OnceLock::new(),
            reserve_cadence: OnceLock::new(),
        })
    }
    pub fn attach_statistics(&self, sink: StatisticsSink) -> Result<()> {
        self.statistics.set(sink).map_err(|_| Error::InvalidRequest)
    }
    pub fn stop(&self) {
        self.foreground.close_connection(None);
        self.stopped.store(true, Ordering::Release);
    }
    pub fn start_workers(
        self: &Arc<Self>,
        enabled: workers::Enabled,
        online: reporting::Online,
    ) -> Result<()> {
        self.workers
            .set(workers::Workers::new(self, enabled, online))
            .map_err(|_| Error::InvalidRequest)
    }
    pub async fn shutdown(&self) {
        self.stop();
        if let Some(worker) = self.reserve_cadence.get() {
            worker.stop().await;
        }
        if let Some(worker) = self.usage_worker.get() {
            worker.stop().await;
        }
        if let Some(worker) = self.availability_worker.get() {
            worker.stop().await;
        }
        if let Some(worker) = self.reconnect_worker.get() {
            worker.stop().await;
        }
        if let Some(reporter) = self.reporter.get() {
            reporter.stop().await;
        }
        if let Some(workers) = self.workers.get() {
            workers.stop().await;
        }
    }
    pub fn start_reporting(&self, online: reporting::Online) -> Result<()> {
        self.reporter
            .set(reporting::Reporter::start(
                self.account.clone(),
                self.foreground.reporting().clone(),
                online,
            ))
            .map_err(|_| Error::InvalidRequest)
    }
    pub fn paused(&self) -> Result<bool> {
        self.foreground.availability().paused()
    }
    pub fn set_timing_sink(
        &self,
        sink: govorun_core::stylization::timing::Sink,
    ) -> Result<()> {
        self.foreground.set_timing_sink(sink)
    }
    pub fn configure_reporting(
        &self,
        settings: Option<govorun_core::stylization::reporting::Settings>,
    ) -> Result<()> {
        self.foreground.reporting().configure(settings)?;
        self.foreground.configure_selection(settings.is_some_and(|value|
            value.official_selected(self.account.snapshot().access.client_terms_version)));
        if settings.is_some_and(|value| {
            !value.official_selected(self.account.snapshot().access.client_terms_version)
        }) {
            if let Some(worker) = self.availability_worker.get() {
                worker.suspend();
            }
        }
        Ok(())
    }
    pub fn maintain(
        &self,
        epoch: u64,
        trigger: govorun_core::stylization::scheduling::Trigger,
        join: bool,
    ) -> Result<()> {
        self.current(epoch)?;
        self.workers
            .get()
            .ok_or(Error::InvalidRequest)?
            .reserve(epoch, trigger, join)
    }
    fn current(&self, epoch: u64) -> Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(Error::SessionChanged);
        }
        self.account.current(epoch)
    }
    pub async fn bind_profile(&self) -> Result<()> {
        let mut previous = self.applied.lock().await;
        loop {
            if self.stopped.load(Ordering::Acquire) { return Err(Error::SessionChanged); }
            let projection = self.account.native_projection();
            let epoch = projection["snapshot"]["sessionGeneration"].as_u64().ok_or(Error::InvalidResponse)?;
            let result = self.vault.bind(Some(previous.clone()), projection.clone(), (self.clock)(), &|| {
                !self.stopped.load(Ordering::Acquire)
                    && crate::style_reserve::same_binding(&projection, &self.account.native_projection())
            }).await;
            if matches!(result, Err(Error::SessionChanged)) && !self.stopped.load(Ordering::Acquire)
                && self.account.current(epoch).is_ok() {
                let snapshot = self.account.snapshot();
                if !snapshot.storage_available { return Err(Error::StorageUnavailable); }
                if !snapshot.logout_pending {
                    // Bind the new business scope before any new checkout. A
                    // same-account publication is not a cancellation of speech.
                    tokio::task::yield_now().await;
                    continue;
                }
            }
            result?;
            *previous = projection;
            return Ok(());
        }
    }
    pub async fn observation(&self, explicit: bool, background: bool) -> Result<Observation> {
        loop {
            if self.stopped.load(Ordering::Acquire) { return Err(Error::SessionChanged); }
            let projection = self.account.native_projection();
            let epoch = projection["snapshot"]["sessionGeneration"].as_u64().ok_or(Error::InvalidResponse)?;
            let stored = if projection["snapshot"]["storageAvailable"] == true && projection["snapshot"]["signedIn"] == true {
                match async { self.bind_profile().await?; self.vault.inspect(false).await }.await {
                    Ok(value) => Some(value),
                    Err(Error::StorageUnavailable | Error::StorageInvalid) => None,
                    Err(error) => return Err(error),
                }
            } else { None };
            let observation = presentation::observe(&self.account, stored.as_ref(), (self.clock)(), explicit, background, false)?;
            if crate::style_reserve::same_binding(&projection, &self.account.native_projection()) {
                return Ok(observation);
            }
            self.current(epoch)?;
            // Re-read/bind changed entitlement or consent as a coherent gate;
            // do not turn an ordinary access refusal into cancelled dictation.
            tokio::task::yield_now().await;
        }
    }
    /// Execute one maintenance opportunity. Common schedulers choose any retry;
    /// no stored text is retained or replayed during reserve maintenance.
    pub async fn prepare(
        &self,
        epoch: u64,
        explicit: bool,
        background: bool,
        required: usize,
        issuance_id: String,
    ) -> Result<Preparation> {
        self.current(epoch)?;
        if !(1..=crate::style_inventory::BATCH).contains(&required) {
            return Err(Error::InvalidRequest);
        }
        let mut pending = self.maintenance.lock().await;
        self.current(epoch)?;
        if let Some(prepared) = self.resume_refresh(epoch, &mut pending).await? {
            return Ok(prepared);
        }
        self.bind_profile().await?;
        self.current(epoch)?;
        // A recovery opportunity owns only its ambiguous lease. Other paid
        // passes and issuer capacity remain available to reserve maintenance.
        let requires_recovery = if self.workers.get().is_some() {
            // Reserve work only wakes the individual lease continuations; it
            // must not spend extra recovery attempts outside their budgets.
            self.foreground.has_blocking_credit(epoch).await?
        } else {
            self.foreground.recover(epoch).await?.requires_recovery
        };
        let view = self.observation(explicit, background).await?;
        if view.refill["blockedReason"] == "issuanceCapacityUnavailable" {
            return Ok(Preparation {
                ready: false,
                retry_after_millis: None,
                blocked_reason: Some("issuanceCapacityUnavailable"),
                requires_recovery,
                maintenance: None,
            });
        }
        let view = self.observation(explicit, background).await?;
        let count = view.refill["batchCredits"]
            .as_u64()
            .ok_or(Error::InvalidResponse)? as usize;
        if count == 0 {
            return Ok(Preparation {
                ready: view.ready >= required,
                retry_after_millis: None,
                blocked_reason: None,
                requires_recovery,
                maintenance: None,
            });
        }
        let profile = self.account.profile().ok_or(Error::SignedOut)?;
        let end = profile.wire()["styleAccess"]["endsAt"]
            .as_str()
            .map(|v| {
                chrono::DateTime::parse_from_rfc3339(v)
                    .ok()
                    .and_then(|v| u64::try_from(v.timestamp_millis()).ok())
                    .ok_or(Error::InvalidResponse)
            })
            .transpose()?;
        let outcome = self.issuance.warm(epoch, count, end, issuance_id).await?;
        if outcome.refresh_account {
            *pending = Some(maintenance::PendingRefresh { epoch, outcome });
            if let Some(prepared) = self.resume_refresh(epoch, &mut pending).await? {
                return Ok(prepared);
            }
        }
        self.bind_profile().await?;
        if outcome.rejected_status.is_some() {
            return self
                .rejected_preparation(outcome, explicit, background, required)
                .await;
        }
        let current = self.observation(explicit, background).await?;
        Ok(Preparation {
            ready: current.ready >= required,
            retry_after_millis: None,
            blocked_reason: None,
            requires_recovery,
            maintenance: None,
        })
    }
    /// Finish a locally admitted insertion outcome during host draining. The
    /// authoritative account still fences logout/generation and bounds the I/O;
    /// shutting down maintenance must not erase this completed delivery fact.
    pub async fn report_admitted_outcome(
        &self,
        epoch: u64,
        pass: String,
        outcome: &str,
        latency: &str,
    ) -> Result<bool> {
        self.foreground
            .reporting()
            .outcome(epoch, pass, outcome, latency)
            .await
    }
    pub async fn process(
        &self,
        epoch: u64,
        payload: Value,
        lease_id: String,
        request_id: String,
        preparation_millis: u64,
        progress: &(dyn Fn(Stage) + Send + Sync),
    ) -> Result<Output> {
        self.process_with_timing(epoch,payload,lease_id,request_id,preparation_millis,progress,None).await
    }
    pub async fn process_with_timing(
        &self, epoch: u64, payload: Value, lease_id: String, request_id: String,
        preparation_millis: u64, progress: &(dyn Fn(Stage) + Send + Sync),
        timing: Option<govorun_core::stylization::timing::Sink>,
    ) -> Result<Output> {
        self.current(epoch)?;
        let required = crate::style_text::admit(payload["text"].as_str()
            .ok_or(Error::InvalidRequest)?.encode_utf16().collect())?["requiredCredits"]
            .as_u64().ok_or(Error::InvalidResponse)? as usize;
        let result = self
            .foreground
            .run_ready_with_timing(
                epoch,
                payload,
                lease_id,
                request_id,
                preparation_millis,
                progress,
                Box::pin(async {
                    self.demand_ready(epoch, required).await
                }),
                timing,
            )
            .await;
        if self.account.current(epoch).is_ok() {
            if let (Some(failed), Some(sink)) = (
                govorun_core::stylization::failure::processing_result(result.as_ref().err().copied()),
                self.statistics.get(),
            ) {
                let slow = result.as_ref().is_ok_and(|v| v.is_slow());
                sink(epoch, failed, slow).await;
            }
        }
        if let Some(workers) = self.workers.get() {
            if result.is_ok() || matches!(
                result,
                Err(Error::Http(_)) | Err(Error::StyleAccess(
                    govorun_core::stylization::AccessFailure::ReserveEmpty
                ))
            ) {
                // Only the post-use opportunity is jittered; failures still
                // wake recovery immediately. Neither path retains text.
                if let Some(cadence) = self.reserve_cadence.get().filter(|_| result.is_ok()) {
                    cadence.used(epoch);
                } else {
                    let trigger = if result.is_ok() {
                        govorun_core::stylization::scheduling::Trigger::Maintenance
                    } else {
                        govorun_core::stylization::scheduling::Trigger::Demand
                    };
                    let _ = workers.reserve(epoch, trigger, true);
                }
            }
            // The shared recovery owner inspects durable leases before any I/O;
            // it never retries text, and duplicate requests share one budget.
            let _ = workers.credit(
                epoch,
                govorun_core::stylization::scheduling::Trigger::Maintenance,
                true,
                0,
            );
        }
        result
    }
    pub async fn sync_usage(&self, epoch: u64) -> Result<usize> {
        self.current(epoch)?;
        self.bind_profile().await?;
        self.usage.flush(epoch).await
    }
    pub async fn recovery_step(&self, epoch: u64) -> Result<Value> {
        self.current(epoch)?;
        let step = self.foreground.recover(epoch).await?;
        Ok(
            json!({"wireAttempted":step.wire_attempted,"requiresRecovery":step.requires_recovery,"retryAfterMillis":step.retry_after_millis}),
        )
    }
}
