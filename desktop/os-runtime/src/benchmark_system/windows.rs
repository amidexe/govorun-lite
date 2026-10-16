use super::{clean, Adapter, Info};
use std::ptr;
use windows_sys::Win32::{Foundation::ERROR_SUCCESS, System::Registry::*};
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn open(path: &str) -> Option<Key> {
    let mut key = ptr::null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            wide(path).as_ptr(),
            0,
            KEY_READ,
            &mut key,
        )
    };
    if status == ERROR_SUCCESS {
        Some(Key(key))
    } else {
        None
    }
}
fn string(key: &Key, name: &str) -> Option<String> {
    let mut kind = 0;
    let mut size = 0;
    let name = wide(name);
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            ptr::null_mut(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || kind != REG_SZ || size == 0 || size > 8192 || size % 2 != 0 {
        return None;
    }
    let mut bytes = vec![0u16; size as usize / 2];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            bytes.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || kind != REG_SZ || size % 2 != 0 || size as usize > bytes.len() * 2
    {
        return None;
    }
    bytes.truncate(size as usize / 2);
    if let Some(end) = bytes.iter().position(|c| *c == 0) {
        bytes.truncate(end);
    }
    let value = String::from_utf16(&bytes).ok()?;
    let mut units = 0;
    Some(
        clean(&value)
            .chars()
            .take_while(|c| {
                units += c.len_utf16();
                units <= 160
            })
            .collect::<String>()
            .trim()
            .into(),
    )
}
#[repr(C)]
struct Memory {
    length: u32,
    load: u32,
    total_physical: u64,
    available_physical: u64,
    total_page: u64,
    available_page: u64,
    total_virtual: u64,
    available_virtual: u64,
    extended: u64,
}
#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(memory: *mut Memory) -> i32;
}
#[repr(C)]
struct Version {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service: [u16; 128],
}
#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(version: *mut Version) -> i32;
}
pub(super) fn read() -> Info {
    let mut memory = Memory {
        length: std::mem::size_of::<Memory>() as u32,
        load: 0,
        total_physical: 0,
        available_physical: 0,
        total_page: 0,
        available_page: 0,
        total_virtual: 0,
        available_virtual: 0,
        extended: 0,
    };
    let memory_mib = if unsafe { GlobalMemoryStatusEx(&mut memory) } != 0 {
        Some(memory.total_physical / 1_048_576)
    } else {
        None
    };
    let mut version = Version {
        size: std::mem::size_of::<Version>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        service: [0; 128],
    };
    let os_version = if unsafe { RtlGetVersion(&mut version) } >= 0 {
        format!(
            "Microsoft Windows NT {}.{}.{}.0",
            version.major, version.minor, version.build
        )
    } else {
        "не определена".into()
    };
    let cpu = open(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .and_then(|k| string(&k, "ProcessorNameString"))
        .unwrap_or_else(|| "не определён".into());
    let mut adapters = Vec::<Adapter>::new();
    if let Some(video) = open(r"SYSTEM\CurrentControlSet\Control\Video") {
        for index in 0..16 {
            let mut name = [0u16; 512];
            let mut size = name.len() as u32;
            let status = unsafe {
                RegEnumKeyExW(
                    video.0,
                    index,
                    name.as_mut_ptr(),
                    &mut size,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if status != ERROR_SUCCESS {
                continue;
            }
            let Ok(name) = String::from_utf16(&name[..size as usize]) else {
                continue;
            };
            let Some(adapter) = open(&format!(
                r"SYSTEM\CurrentControlSet\Control\Video\{name}\0000"
            )) else {
                continue;
            };
            let Some(name) = string(&adapter, "DriverDesc") else {
                continue;
            };
            let driver = string(&adapter, "DriverVersion").unwrap_or_else(|| "не определён".into());
            if !adapters
                .iter()
                .any(|a| a.name == name && a.driver == driver)
            {
                adapters.push(Adapter { name, driver });
            }
            if adapters.len() == 4 {
                break;
            }
        }
    }
    let architecture = match std::env::consts::ARCH {
        "x86_64" => "X64",
        "aarch64" => "Arm64",
        "x86" => "X86",
        other => other,
    }
    .into();
    Info {
        os_version,
        architecture,
        machine: String::new(),
        cpu,
        cpu_threads: std::thread::available_parallelism()
            .map(|n| n.get().min(u32::MAX as usize) as u32)
            .unwrap_or(1),
        memory_mib,
        adapters,
    }
}
