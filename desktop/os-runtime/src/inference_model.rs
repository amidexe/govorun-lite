//! Exact packaged compute paths and serialized, independent session handles.
pub use crate::model_cache::Compute;
use crate::{
    inference::{Recognizer, Recording},
    model_cache::{Cache, Loader, Readiness},
};
use std::{
    path::PathBuf,
    sync::{Mutex, TryLockError},
    time::{Duration, Instant},
};
use transcribe_cpp::Backend;
pub struct Files {
    pub model: PathBuf,
    pub runtime: PathBuf,
    pub vad: PathBuf,
    pub vad_runtime: PathBuf,
}
struct ComputeLoader {
    model: PathBuf,
    runtime: PathBuf,
}
impl Loader for ComputeLoader {
    type Key = Compute;
    type Session = Recognizer;
    fn load(&mut self, key: &Compute) -> Result<Recognizer, &'static str> {
        let backend = match key {
            Compute::Automatic => Backend::Auto,
            Compute::Cpu => Backend::Cpu,
            #[cfg(target_os = "windows")]
            Compute::Gpu => Backend::Vulkan,
            #[cfg(not(target_os = "windows"))]
            Compute::Gpu => Backend::Metal,
        };
        Recognizer::load(&self.model, &self.runtime, backend)
    }
}
pub struct Model {
    cache: Mutex<Cache<ComputeLoader>>,
    vad: PathBuf,
    vad_runtime: PathBuf,
}
impl Model {
    /// Construction neither loads a model nor opens a device.
    pub fn new(files: Files, compute: Compute, keep: bool) -> Self {
        Self {
            cache: Mutex::new(Cache::new(
                ComputeLoader {
                    model: files.model,
                    runtime: files.runtime,
                },
                compute,
                keep,
            )),
            vad: files.vad,
            vad_runtime: files.vad_runtime,
        }
    }
    pub fn prepare(&self) -> Result<(), &'static str> {
        self.cache
            .lock()
            .map_err(|_| "recognition_model_unavailable")?
            .session()
            .map(|_| ())
    }
    pub fn begin(&self) -> Result<Recording, &'static str> {
        let session = self
            .cache
            .lock()
            .map_err(|_| "recognition_model_unavailable")?
            .session()?;
        Recording::begin(&session, &self.vad, &self.vad_runtime)
    }
    pub fn configure(&self, compute: Compute, keep: bool, busy: bool) -> Result<(), &'static str> {
        self.cache
            .lock()
            .map_err(|_| "recognition_model_unavailable")?
            .configure(compute, keep, busy)
    }
    pub fn readiness(&self) -> Readiness {
        match self.cache.try_lock() {
            Ok(cache) => cache.readiness(),
            Err(TryLockError::WouldBlock) => Readiness::Loading,
            Err(TryLockError::Poisoned(_)) => Readiness::Failed,
        }
    }
    pub fn idle(&self, now: Instant, busy: bool) -> Option<Instant> {
        match self.cache.try_lock() {
            Ok(mut cache) => {
                cache.idle(now, busy);
                cache.idle_deadline()
            }
            Err(TryLockError::WouldBlock) => now.checked_add(Duration::from_secs(1)),
            Err(TryLockError::Poisoned(_)) => None,
        }
    }
}
