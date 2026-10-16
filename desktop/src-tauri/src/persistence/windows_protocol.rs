//! Existing per-user OAuth URI registration, skipped for packaged Windows apps.
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::System::Registry::*;

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentPackageFullName(length: *mut u32, name: *mut u16) -> i32;
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
fn create(path: &str) -> Result<Key, &'static str> {
    let name = wide(OsStr::new(path));
    let mut key = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            name.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err("storage_unavailable");
    }
    Ok(Key(key))
}
fn set(key: &Key, name: Option<&str>, value: &OsStr) -> Result<(), &'static str> {
    let name = name.map(|value| wide(OsStr::new(value)));
    let value = wide(value);
    let status = unsafe {
        RegSetValueExW(
            key.0,
            name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            0,
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * 2) as u32,
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err("storage_unavailable")
    }
}
pub fn register(scheme: &str, executable: &Path) -> Result<(), &'static str> {
    let mut length = 0;
    match unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) } {
        122 => return Ok(()), // Package manifest owns this registration.
        15700 => {}
        _ => return Err("storage_unavailable"),
    }
    let root = format!("Software\\Classes\\{scheme}");
    let key = create(&root)?;
    set(&key, None, OsStr::new("URL:Govorun Protocol"))?;
    set(&key, Some("URL Protocol"), OsStr::new(""))?;
    let command = create(&format!("{root}\\shell\\open\\command"))?;
    // Preserve the executable's Unicode path without lossy conversion.
    let mut value = std::ffi::OsString::from("\"");
    value.push(executable.as_os_str());
    value.push("\" \"%1\"");
    set(&command, None, &value)
}
