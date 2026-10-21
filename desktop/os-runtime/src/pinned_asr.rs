//! Call the already verified transcribe.cpp binary through its pinned Rust
//! bindings. This preserves the deployed compute engine and model behavior.
use crate::recognition::{Cancellation, Failure};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void, CStr, CString},
    path::{Path, PathBuf},
    ptr,
    sync::{Arc, Mutex},
};
use transcribe_cpp::Backend;
use transcribe_cpp_sys as sys;

type Open = unsafe extern "C" fn(
    *const c_char,
    *const sys::transcribe_model_load_params,
    *const sys::transcribe_session_params,
    *mut *mut sys::transcribe_session,
) -> sys::transcribe_status;
type Close = unsafe extern "C" fn(*mut sys::transcribe_session);
type Run = unsafe extern "C" fn(
    *mut sys::transcribe_session,
    *const f32,
    i32,
    *const sys::transcribe_run_params,
) -> sys::transcribe_status;
type Abort =
    unsafe extern "C" fn(*mut sys::transcribe_session, sys::transcribe_abort_callback, *mut c_void);
type Text = unsafe extern "C" fn(*const sys::transcribe_session) -> *const c_char;
type Model = unsafe extern "C" fn(*const sys::transcribe_session) -> *const sys::transcribe_model;
type BackendName = unsafe extern "C" fn(*const sys::transcribe_model) -> *const c_char;

struct Api {
    open: Open,
    close: Close,
    run: Run,
    abort: Abort,
    text: Text,
    model: Model,
    backend: BackendName,
    _library: libloading::Library,
}
impl Api {
    fn load(runtime: &Path) -> Result<Arc<Self>, &'static str> {
        // ggml retains backend registrations and a log bridge into this image.
        // Keep each admitted library alive for the process, like a linked SDK;
        // unloading a model must close only its session, not that shared image.
        static LIBRARIES: Mutex<BTreeMap<PathBuf, Arc<Api>>> = Mutex::new(BTreeMap::new());
        let path = runtime
            .canonicalize()
            .map_err(|_| "recognition_runtime_unavailable")?;
        let mut libraries = LIBRARIES
            .lock()
            .map_err(|_| "recognition_runtime_unavailable")?;
        if let Some(api) = libraries.get(&path) {
            return Ok(api.clone());
        }
        let api = Self::open(&path)?;
        libraries.insert(path, api.clone());
        Ok(api)
    }
    fn open(runtime: &Path) -> Result<Arc<Self>, &'static str> {
        // The host supplies its exact verified framework/DLL path. No developer
        // checkout layout or loader search-path fallback belongs in production.
        #[cfg(target_os = "macos")]
        let library = unsafe { libloading::Library::new(runtime) }
            .map_err(|_| "recognition_runtime_unavailable")?;
        #[cfg(windows)]
        let library: libloading::Library = {
            use libloading::os::windows::{
                Library, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
            };
            unsafe {
                Library::load_with_flags(
                    runtime,
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
                )
            }
            .map_err(|_| "recognition_runtime_unavailable")?
            .into()
        };
        unsafe {
            let version = *library
                .get::<unsafe extern "C" fn() -> *const c_char>(b"transcribe_version\0")
                .map_err(|_| "recognition_runtime_invalid")?;
            let value = version();
            if value.is_null() || CStr::from_ptr(value).to_bytes() != b"0.2.1" {
                return Err("recognition_runtime_version_mismatch");
            }
            let size = *library
                .get::<unsafe extern "C" fn(sys::transcribe_abi_struct) -> usize>(
                    b"transcribe_abi_struct_size\0",
                )
                .map_err(|_| "recognition_runtime_invalid")?;
            if size(sys::transcribe_abi_struct::TRANSCRIBE_ABI_MODEL_LOAD_PARAMS)
                != std::mem::size_of::<sys::transcribe_model_load_params>()
            {
                return Err("recognition_runtime_abi_mismatch");
            }
            let initialize = *library
                .get::<unsafe extern "C" fn() -> sys::transcribe_status>(
                    b"transcribe_init_backends_default\0",
                )
                .map_err(|_| "recognition_runtime_invalid")?;
            if initialize() != sys::transcribe_status::TRANSCRIBE_OK {
                return Err("recognition_devices_unavailable");
            }
            let logs = *library
                .get::<unsafe extern "C" fn(sys::transcribe_log_callback, *mut c_void)>(
                    b"transcribe_log_set\0",
                )
                .map_err(|_| "recognition_runtime_invalid")?;
            logs(None, ptr::null_mut());
            Ok(Arc::new(Self {
                open: *library
                    .get(b"transcribe_open\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                close: *library
                    .get(b"transcribe_close\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                run: *library
                    .get(b"transcribe_run\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                abort: *library
                    .get(b"transcribe_set_abort_callback\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                text: *library
                    .get(b"transcribe_full_text\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                model: *library
                    .get(b"transcribe_get_model\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                backend: *library
                    .get(b"transcribe_model_backend\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                _library: library,
            }))
        }
    }
}

pub(super) struct Session {
    api: Arc<Api>,
    raw: *mut sys::transcribe_session,
    path: CString,
    pub backend: String,
}
// Same transcribe.cpp session contract as the safe crate: it can move across
// threads, but calls must never overlap. Recognizer owns it behind one Mutex.
unsafe impl Send for Session {}

impl Session {
    pub fn load(path: &Path, runtime: &Path, backend: Backend) -> Result<Self, &'static str> {
        let api = Api::load(runtime)?;
        let path = CString::new(path.to_str().ok_or("recognition_model_path_invalid")?)
            .map_err(|_| "recognition_model_path_invalid")?;
        crate::backend_retry::once(
            backend == Backend::Cpu,
            |cpu| {
                Self::open(
                    api.clone(),
                    path.clone(),
                    if cpu { Backend::Cpu } else { backend },
                )
            },
            |error| *error == "recognition_compute_backend_unavailable",
        )
    }
    fn open(api: Arc<Api>, path: CString, backend: Backend) -> Result<Self, &'static str> {
        Self::open_status(api, path, backend, ptr::null_mut()).map_err(|(code, _)| code)
    }
    /// The engine status of a failed open stays beside the public code so the
    /// benchmark can name the closed reason; dictation keeps the public code.
    fn open_status(
        api: Arc<Api>,
        path: CString,
        backend: Backend,
        device: sys::transcribe_device_t,
    ) -> Result<Self, (&'static str, Option<sys::transcribe_status>)> {
        let mut parameters = sys::transcribe_model_load_params {
            struct_size: std::mem::size_of::<sys::transcribe_model_load_params>() as u64,
            backend: match backend {
                Backend::Auto => sys::transcribe_backend_request::TRANSCRIBE_BACKEND_AUTO,
                Backend::Cpu => sys::transcribe_backend_request::TRANSCRIBE_BACKEND_CPU,
                Backend::Metal => sys::transcribe_backend_request::TRANSCRIBE_BACKEND_METAL,
                Backend::Vulkan => sys::transcribe_backend_request::TRANSCRIBE_BACKEND_VULKAN,
                _ => return Err(("recognition_backend_invalid", None)),
            },
            device,
        };
        let mut raw = ptr::null_mut();
        let status = unsafe { (api.open)(path.as_ptr(), &mut parameters, ptr::null(), &mut raw) };
        if status != sys::transcribe_status::TRANSCRIBE_OK || raw.is_null() {
            if !raw.is_null() {
                unsafe {
                    (api.close)(raw);
                }
            }
            return Err((
                if status == sys::transcribe_status::TRANSCRIBE_ERR_BACKEND {
                    "recognition_compute_backend_unavailable"
                } else {
                    "recognition_model_unavailable"
                },
                Some(status),
            ));
        }
        let mut session = Self {
            api,
            raw,
            path,
            backend: String::new(),
        };
        let model = unsafe { (session.api.model)(session.raw) };
        if model.is_null() {
            return Err(("recognition_backend_unavailable", None));
        }
        let name = unsafe { (session.api.backend)(model) };
        if name.is_null() {
            return Err(("recognition_backend_unavailable", None));
        }
        session.backend = unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned();
        Ok(session)
    }

    pub fn decode(&mut self, pcm: &[f32], cancellation: &Cancellation) -> Result<String, Failure> {
        if cancellation.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        // A failed CPU reload leaves a closed session, never a dangling pointer.
        if self.raw.is_null() {
            *self = Self::open(self.api.clone(), self.path.clone(), Backend::Cpu)
                .map_err(|_| Failure::Decode)?;
        }
        let already_cpu = self.backend.to_ascii_lowercase().starts_with("cpu");
        crate::backend_retry::once(
            already_cpu,
            |cpu| {
                if cancellation.is_cancelled() {
                    return Err(Failure::Cancelled);
                }
                if cpu {
                    unsafe {
                        (self.api.close)(self.raw);
                    }
                    self.raw = ptr::null_mut();
                    *self = Self::open(self.api.clone(), self.path.clone(), Backend::Cpu)
                        .map_err(|_| Failure::Decode)?;
                }
                self.run(pcm, cancellation)
            },
            |error| *error == Failure::Backend,
        )
    }
    fn run(&mut self, pcm: &[f32], cancellation: &Cancellation) -> Result<String, Failure> {
        if cancellation.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        let count = i32::try_from(pcm.len()).map_err(|_| Failure::Decode)?;
        unsafe {
            (self.api.abort)(
                self.raw,
                Some(aborted),
                cancellation as *const _ as *mut c_void,
            );
        }
        let status = unsafe { (self.api.run)(self.raw, pcm.as_ptr(), count, ptr::null()) };
        // Clear borrowed stack data before returning, also on failure/abort.
        unsafe {
            (self.api.abort)(self.raw, None, ptr::null_mut());
        }
        if cancellation.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if status == sys::transcribe_status::TRANSCRIBE_ERR_BACKEND {
            return Err(Failure::Backend);
        }
        if status != sys::transcribe_status::TRANSCRIBE_OK {
            return Err(Failure::Decode);
        }
        let text = unsafe { (self.api.text)(self.raw) };
        if text.is_null() {
            return Err(Failure::Decode);
        }
        Ok(unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .trim()
            .to_owned())
    }
}
unsafe extern "C" fn aborted(data: *mut c_void) -> bool {
    // Valid only during the synchronous run; Session clears it before return.
    !data.is_null() && unsafe { &*(data as *const Cancellation) }.is_cancelled()
}
impl Drop for Session {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                (self.api.close)(self.raw);
            }
        }
    }
}

#[cfg(feature = "benchmark")]
#[path = "benchmark_device.rs"]
mod benchmark_device;
#[cfg(feature = "benchmark")]
pub use benchmark_device::Device;
