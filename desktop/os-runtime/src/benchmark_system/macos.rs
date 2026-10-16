use super::{clean, Info};
use objc2_foundation::NSProcessInfo;
use std::ffi::{c_void, CString};
extern "C" {
    fn sysctlbyname(
        name: *const std::ffi::c_char,
        old: *mut c_void,
        size: *mut usize,
        new: *mut c_void,
        new_size: usize,
    ) -> i32;
}
fn query(key: &str) -> String {
    let key = CString::new(key).unwrap();
    let mut size = 0;
    if unsafe {
        sysctlbyname(
            key.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || size == 0
        || size >= 1024
    {
        return "не определён".into();
    }
    let mut bytes = vec![0u8; size];
    if unsafe {
        sysctlbyname(
            key.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return "не определён".into();
    }
    bytes.truncate(size);
    if let Some(end) = bytes.iter().position(|b| *b == 0) {
        bytes.truncate(end);
    }
    clean(&String::from_utf8_lossy(&bytes))
}
pub(super) fn read() -> Info {
    let process = NSProcessInfo::processInfo();
    Info {
        os_version: clean(&process.operatingSystemVersionString().to_string()),
        architecture: std::env::consts::ARCH.into(),
        machine: query("hw.model"),
        cpu: query("machdep.cpu.brand_string"),
        cpu_threads: process.activeProcessorCount().min(u32::MAX as usize) as u32,
        memory_mib: Some(process.physicalMemory() / 1_048_576),
        adapters: vec![],
    }
}
