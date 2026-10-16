//! Strict benchmark selection extends the pinned runtime without changing the
//! ordinary recording fallback. Only bounded descriptive facts leave this port.
use super::*;
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub backend: String,
    index: i32,
}
struct Devices {
    count: unsafe extern "C" fn() -> i32,
    get: unsafe extern "C" fn(i32) -> sys::transcribe_device_t,
    info: unsafe extern "C" fn(
        sys::transcribe_device_t,
        *mut sys::transcribe_device_info,
    ) -> sys::transcribe_status,
    selected: unsafe extern "C" fn(*const sys::transcribe_model) -> sys::transcribe_device_t,
}
impl Devices {
    fn load(api: &Api) -> Result<Self, &'static str> {
        unsafe {
            let size = *api
                ._library
                .get::<unsafe extern "C" fn(sys::transcribe_abi_struct) -> usize>(
                    b"transcribe_abi_struct_size\0",
                )
                .map_err(|_| "recognition_runtime_invalid")?;
            if size(sys::transcribe_abi_struct::TRANSCRIBE_ABI_DEVICE_INFO)
                != std::mem::size_of::<sys::transcribe_device_info>()
            {
                return Err("recognition_runtime_abi_mismatch");
            }
            Ok(Self {
                count: *api
                    ._library
                    .get(b"transcribe_device_count\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                get: *api
                    ._library
                    .get(b"transcribe_device_get\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                info: *api
                    ._library
                    .get(b"transcribe_device_get_info\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
                selected: *api
                    ._library
                    .get(b"transcribe_model_device\0")
                    .map_err(|_| "recognition_runtime_invalid")?,
            })
        }
    }
    fn info(
        &self,
        index: i32,
    ) -> Result<(sys::transcribe_device_t, sys::transcribe_device_info), &'static str> {
        let handle = unsafe { (self.get)(index) };
        if handle.is_null() {
            return Err("recognition_devices_unavailable");
        }
        let mut info: sys::transcribe_device_info = unsafe { std::mem::zeroed() };
        info.struct_size = std::mem::size_of::<sys::transcribe_device_info>() as u64;
        if unsafe { (self.info)(handle, &mut info) } != sys::transcribe_status::TRANSCRIBE_OK {
            return Err("recognition_devices_unavailable");
        }
        Ok((handle, info))
    }
}
fn text(value: *const c_char) -> String {
    if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    }
}
impl Session {
    pub(crate) fn devices(runtime: &Path) -> Result<Vec<Device>, &'static str> {
        let api = Api::load(runtime)?;
        let ports = Devices::load(&api)?;
        let count = unsafe { (ports.count)() };
        if !(1..=64).contains(&count) {
            return Err("recognition_devices_unavailable");
        }
        let mut devices = Vec::<Device>::new();
        let mut gpu = 0;
        for index in 0..count {
            let (_, info) = ports.info(index)?;
            let backend = text(info.kind);
            #[cfg(target_os = "macos")]
            let supported = backend == "cpu" || backend == "metal";
            #[cfg(target_os = "windows")]
            let supported = backend == "cpu" || backend == "vulkan";
            if !supported
                || backend == "cpu" && devices.iter().any(|device| device.backend == "cpu")
            {
                continue;
            }
            let description = text(info.description);
            let raw = if description.is_empty() {
                text(info.name)
            } else {
                description
            };
            let mut name: String = raw.chars().filter(|c| !c.is_control()).take(160).collect();
            #[cfg(target_os = "windows")]
            {
                let mut units = 0;
                name = name
                    .chars()
                    .take_while(|c| {
                        units += c.len_utf16();
                        units <= 160
                    })
                    .collect::<String>()
                    .trim()
                    .into();
            }
            if name.is_empty() {
                name = if backend == "cpu" {
                    "Процессор"
                } else {
                    "Видеокарта"
                }
                .into();
            }
            let id = if backend == "cpu" {
                "cpu".into()
            } else {
                gpu += 1;
                format!("gpu-{gpu}")
            };
            devices.push(Device {
                id,
                name,
                backend,
                index,
            });
        }
        if !devices.iter().any(|device| device.backend == "cpu") {
            return Err("recognition_devices_unavailable");
        }
        devices.sort_by_key(|device| device.backend != "cpu");
        Ok(devices)
    }
    pub(crate) fn load_selected(
        model: &Path,
        runtime: &Path,
        device: &Device,
    ) -> Result<Self, &'static str> {
        let api = Api::load(runtime)?;
        let ports = Devices::load(&api)?;
        let (selected, info) = ports.info(device.index)?;
        if text(info.kind) != device.backend {
            return Err("recognition_compute_backend_unavailable");
        }
        let backend = match device.backend.as_str() {
            "cpu" => Backend::Cpu,
            "metal" => Backend::Metal,
            "vulkan" => Backend::Vulkan,
            _ => return Err("recognition_backend_invalid"),
        };
        let path = CString::new(model.to_str().ok_or("recognition_model_path_invalid")?)
            .map_err(|_| "recognition_model_path_invalid")?;
        let mut session = Self::open_on(api, path, backend, selected)?;
        // The deployed adapters classify runtime labels such as MTL0/Metal
        // device names into the same closed CPU/Metal/Vulkan contract.
        let lower = session.backend.to_ascii_lowercase();
        session.backend = if lower.contains("metal") || lower.starts_with("mtl") {
            "metal".into()
        } else if lower.contains("vulkan") {
            "vulkan".into()
        } else if lower.contains("cpu") {
            "cpu".into()
        } else if lower.is_empty() {
            "unknown".into()
        } else {
            lower
        };
        let model = unsafe { (session.api.model)(session.raw) };
        if model.is_null()
            || unsafe { (ports.selected)(model) } != selected
            || session.backend.to_ascii_lowercase() != device.backend
        {
            return Err("recognition_compute_backend_unavailable");
        }
        Ok(session)
    }
    pub(crate) fn decode_selected(
        &mut self,
        pcm: &[f32],
        cancel: &Cancellation,
    ) -> Result<String, Failure> {
        // A GPU fallback must be a failed measurement, never a successful GPU row.
        self.run(pcm, cancel)
    }
}
