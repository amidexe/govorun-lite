//! Async executor for the common benchmark state. Physical ports measure only
//! recorded fixtures, and a host lease excludes ordinary recording until release.
use govorun_core::benchmark_report::{run::Run, Device, Facts, Measurement, SystemFacts};
use crate::features::blocking_cancellation::Cancellation;
use serde_json::Value;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
pub type Lease<'a> = Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send + 'a>>;
pub trait Ports: Send + Sync + 'static {
    fn acquire(&self) -> Lease<'_>;
    fn release(&self) -> Lease<'_>;
    fn discover(&self) -> Result<Vec<Device>, &'static str>;
    fn measure(
        &self,
        index: usize,
        cancel: &Cancellation,
        progress: &mut dyn FnMut(&str),
    ) -> Result<Measurement, &'static str>;
}
pub struct Runner {
    state: Mutex<Run>,
    ports: Arc<dyn Ports>,
    admission: tokio::sync::Mutex<()>,
    token: Mutex<Option<Cancellation>>,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    changed: Arc<dyn Fn() + Send + Sync>,
    published: Mutex<Option<Value>>,
    closed: AtomicBool,
}
impl Runner {
    pub fn new(
        system: SystemFacts,
        ports: Arc<dyn Ports>,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Arc<Self>, &'static str> {
        Ok(Arc::new(Self {
            state: Mutex::new(Run::new(system)?),
            ports,
            admission: Default::default(),
            token: Mutex::new(None),
            task: Mutex::new(None),
            changed,
            published: Mutex::new(None),
            closed: AtomicBool::new(false),
        }))
    }
    pub fn running(&self) -> Result<bool, &'static str> {
        self.state
            .lock()
            .map(|state| state.running())
            .map_err(|_| "benchmark_unavailable")
    }
    pub fn facts(&self) -> Result<Facts, &'static str> {
        self.state
            .lock()
            .map(|s| s.facts())
            .map_err(|_| "benchmark_unavailable")
    }
    fn publish(&self) {
        let Ok(facts) = self
            .facts()
            .and_then(|f| serde_json::to_value(f).map_err(|_| "benchmark_invalid_facts"))
        else {
            return;
        };
        let changed = {
            let Ok(mut previous) = self.published.lock() else {
                return;
            };
            if previous.as_ref() == Some(&facts) {
                false
            } else {
                *previous = Some(facts);
                true
            }
        };
        if changed && !self.closed.load(Ordering::Acquire) {
            (self.changed)();
        }
    }
    pub async fn invoke(
        self: &Arc<Self>,
        method: &str,
        params: &Value,
    ) -> Result<Value, &'static str> {
        let fields = params.as_object().ok_or("invalid_parameters")?;
        if method == "benchmark.localFacts" {
            if !fields.is_empty() {
                return Err("invalid_parameters");
            }
            return serde_json::to_value(self.facts()?).map_err(|_| "benchmark_invalid_facts");
        }
        if method != "benchmark.localEffect" || fields.len() != 1 {
            return Err("invalid_parameters");
        }
        let action = params["action"].as_str().ok_or("invalid_parameters")?;
        let _admission = self.admission.lock().await;
        if self.closed.load(Ordering::Acquire) {
            return Err("benchmark_stopped");
        }
        match action {
            "start" => {
                if self
                    .state
                    .lock()
                    .map_err(|_| "benchmark_unavailable")?
                    .running()
                {
                    return Err("benchmark_running");
                }
                self.ports.acquire().await?;
                let generation = self
                    .state
                    .lock()
                    .map_err(|_| "benchmark_unavailable")
                    .and_then(|mut state| state.begin(true));
                let generation = match generation {
                    Ok(value) => value,
                    Err(code) => {
                        self.ports.release().await?;
                        return Err(code);
                    }
                };
                let token = Cancellation::default();
                *self.token.lock().map_err(|_| "benchmark_unavailable")? = Some(token.clone());
                self.publish();
                let owner = self.clone();
                let task = tokio::spawn(async move {
                    owner.execute(generation, token).await;
                });
                *self.task.lock().map_err(|_| "benchmark_unavailable")? = Some(task);
            }
            "cancel" => {
                if let Some(token) = self
                    .token
                    .lock()
                    .map_err(|_| "benchmark_unavailable")?
                    .as_ref()
                {
                    token.cancel();
                }
            }
            _ => return Err("invalid_parameters"),
        }
        serde_json::to_value(self.facts()?).map_err(|_| "benchmark_invalid_facts")
    }
    fn fail(&self, generation: u64, code: &str) {
        if let Ok(mut state) = self.state.lock() {
            let facts = state.facts();
            let detail = govorun_core::benchmark_report::FAILURE_DETAILS
                .contains(&code)
                .then_some(code);
            let code =
                govorun_core::benchmark_report::run::public_failure(&facts.stage, code);
            let _ = state.failed(generation, code, detail);
        }
        self.publish();
    }
    async fn execute(self: Arc<Self>, generation: u64, token: Cancellation) {
        let ports = self.ports.clone();
        let discovered = tokio::task::spawn_blocking(move || ports.discover())
            .await
            .unwrap_or(Err("runtime_incompatible"));
        if token.is_cancelled() {
            self.cancelled(generation);
        } else {
            match discovered {
                Err(code) => self.fail(generation, code),
                Ok(devices) => {
                    let admitted = self
                        .state
                        .lock()
                        .ok()
                        .and_then(|mut s| s.discovered(generation, devices.clone()).ok())
                        .unwrap_or(false);
                    if !admitted {
                        self.fail(generation, "initialization_failed");
                    } else {
                        self.publish();
                        for (index, device) in devices.iter().enumerate() {
                            if token.is_cancelled() {
                                self.cancelled(generation);
                                break;
                            }
                            let selected = self
                                .state
                                .lock()
                                .ok()
                                .and_then(|mut s| s.select(generation, index).ok())
                                .unwrap_or(false);
                            if !selected {
                                self.fail(generation, "initialization_failed");
                                break;
                            }
                            self.publish();
                            let owner = self.clone();
                            let id = device.id.clone();
                            let ports = self.ports.clone();
                            let cancel = token.clone();
                            let measured = tokio::task::spawn_blocking(move || {
                                ports.measure(index, &cancel, &mut |stage| {
                                    let changed = owner
                                        .state
                                        .lock()
                                        .ok()
                                        .and_then(|mut s| s.progress(generation, &id, stage).ok())
                                        .unwrap_or(false);
                                    if changed {
                                        owner.publish();
                                    }
                                })
                            })
                            .await
                            .unwrap_or(Err("recognition_failed"));
                            if token.is_cancelled() {
                                self.cancelled(generation);
                                break;
                            }
                            match measured {
                                Err(code) => self.fail(generation, code),
                                Ok(result) => {
                                    let accepted = self
                                        .state
                                        .lock()
                                        .ok()
                                        .and_then(|mut s| {
                                            s.measured(generation, &device.id, result).ok()
                                        })
                                        .unwrap_or(false);
                                    if !accepted {
                                        self.fail(generation, "recognition_failed");
                                    } else {
                                        self.publish();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Resume is queued before the common owner permits another start. A new
        // recording/benchmark cannot race an old delayed lease release.
        if self.ports.release().await.is_err() {
            self.fail(generation, "initialization_failed");
        }
        let _admission = self.admission.lock().await;
        if let Ok(mut state) = self.state.lock() {
            let _ = state.finish(generation);
        }
        if let Ok(mut current) = self.token.lock() {
            *current = None;
        }
        self.publish();
    }
    fn cancelled(&self, generation: u64) {
        if let Ok(mut state) = self.state.lock() {
            state.cancelled(generation);
        }
        self.publish();
    }
    pub async fn stop(self: &Arc<Self>) {
        let _admission = self.admission.lock().await;
        self.closed.store(true, Ordering::Release);
        if let Some(token) = self.token.lock().ok().and_then(|s| s.clone()) {
            token.cancel();
        }
        let task = self.task.lock().ok().and_then(|mut task| task.take());
        drop(_admission);
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
