//! Desktop composition: the shared runner owns transitions; these local ports
//! reserve recording and measure the already packaged runtime on fixed files.
use crate::{
    features::benchmark::runner::{Lease, Ports, Runner},
    recording_runtime::Runtime,
};
use govorun_core::benchmark_report::{Device, Measurement, SystemFacts, Timing};
use govorun_os_runtime::{
    benchmark, benchmark_system, model_cache::Readiness, recognition::Cancellation,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
};
use tauri::{AppHandle, Manager};
struct Physical {
    runtime: Weak<Runtime>,
    model: PathBuf,
    library: PathBuf,
    fixtures: PathBuf,
    devices: Mutex<Vec<benchmark::Device>>,
}
impl Ports for Physical {
    fn acquire(&self) -> Lease<'_> {
        Box::pin(async {
            let runtime = self.runtime.upgrade().ok_or("benchmark_unavailable")?;
            runtime.quiesce().await.map_err(|code| {
                if code == "maintenance_busy" {
                    "benchmark_running"
                } else {
                    "benchmark_unavailable"
                }
            })?;
            if runtime.model_readiness() == Readiness::Loading {
                runtime.resume();
                return Err("benchmark_running");
            }
            Ok(())
        })
    }
    fn release(&self) -> Lease<'_> {
        Box::pin(async {
            if let Some(runtime) = self.runtime.upgrade() {
                runtime.resume();
            }
            Ok(())
        })
    }
    fn discover(&self) -> Result<Vec<Device>, &'static str> {
        let devices = benchmark::discover(&self.library)?;
        let rows = devices
            .iter()
            .map(|device| Device {
                id: device.id.clone(),
                name: device.name.clone(),
                backend: device.backend.clone(),
                state: "pending".into(),
                stage: "fixtures".into(),
                result: None,
                failure_code: None,
            })
            .collect();
        *self.devices.lock().map_err(|_| "initialization_failed")? = devices;
        Ok(rows)
    }
    fn measure(
        &self,
        index: usize,
        cancel: &Cancellation,
        progress: &mut dyn FnMut(&str),
    ) -> Result<Measurement, &'static str> {
        let device = self
            .devices
            .lock()
            .map_err(|_| "initialization_failed")?
            .get(index)
            .cloned()
            .ok_or("initialization_failed")?;
        let t = benchmark::run(
            &self.model,
            &self.library,
            &self.fixtures,
            &device,
            cancel,
            |stage| progress(stage),
        )?;
        let timing = |milliseconds| Timing {
            milliseconds,
            backend: t.backend.clone(),
        };
        Ok(Measurement {
            suite: "gigaam-samples-v1".into(),
            platform: if cfg!(target_os = "macos") {
                "macos"
            } else {
                "windows"
            }
            .into(),
            model: "GigaAM v3 E2E RNNT · GGUF Q8_0 · transcribe.cpp 0.2.1".into(),
            requested_backend: if device.backend == "cpu" {
                "cpu"
            } else {
                "automatic"
            }
            .into(),
            short_audio_ms: t.short_audio_ms,
            long_audio_ms: t.long_audio_ms,
            load_ms: t.load_ms,
            cold: timing(t.cold_ms),
            warm: timing(t.warm_ms),
            longer: timing(t.long_ms),
        })
    }
}
pub fn compose(
    app: &AppHandle,
    runtime: &Arc<Runtime>,
    models: &Path,
) -> Result<Arc<Runner>, &'static str> {
    let resources = app
        .path()
        .resource_dir()
        .map_err(|_| "benchmark_unavailable")?;
    #[cfg(target_os = "macos")]
    let (library, fixtures) = (
        resources
            .parent()
            .ok_or("benchmark_unavailable")?
            .join("Frameworks/CTranscribe.framework/CTranscribe"),
        resources.join("Benchmark"),
    );
    #[cfg(target_os = "windows")]
    let (library, fixtures) = (
        resources.join("native/transcribe.dll"),
        resources.join("native/Benchmark"),
    );
    let info = benchmark_system::read();
    let version = env!("CARGO_PKG_VERSION").into();
    let build_id = option_env!("GOVORUN_BUILD_ID")
        .filter(|id| !id.is_empty())
        .unwrap_or("локальная сборка")
        .into();
    #[cfg(target_os = "macos")]
    let system = {
        let gpu = benchmark::discover(&library)
            .ok()
            .and_then(|devices| devices.into_iter().find(|d| d.backend == "metal"))
            .map(|device| device.name)
            .unwrap_or_else(|| "Metal-устройство не найдено".into());
        SystemFacts::Macos {
            version,
            build_id,
            os_version: info.os_version,
            machine: info.machine,
            cpu: info.cpu,
            cpu_threads: info.cpu_threads,
            memory_mi_b: info.memory_mib.unwrap_or(0),
            gpu,
        }
    };
    #[cfg(target_os = "windows")]
    let system = SystemFacts::Windows {
        version,
        build_id,
        os_version: info.os_version,
        architecture: info.architecture,
        cpu: info.cpu,
        cpu_threads: info.cpu_threads,
        memory_mi_b: info.memory_mib,
        adapters: info
            .adapters
            .into_iter()
            .map(|adapter| govorun_core::benchmark_report::Adapter {
                name: adapter.name,
                driver: adapter.driver,
            })
            .collect(),
    };
    let ports = Arc::new(Physical {
        runtime: Arc::downgrade(runtime),
        model: models.join("gigaam-v3-e2e-rnnt-Q8_0.gguf"),
        library,
        fixtures,
        devices: Mutex::new(vec![]),
    });
    let event_app = app.clone();
    Runner::new(
        system,
        ports,
        Arc::new(move || {
            crate::service::deliver(
                &event_app,
                govorun_core::features::ports::NativeEvent {
                    event: "benchmark.changed".into(),
                    data: serde_json::Value::Null,
                },
            )
        }),
    )
}
