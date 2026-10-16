//! The same per-user Run value and --background command as WindowsStartup.
use super::login::{self, Login};
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    Globalization::CompareStringOrdinal,
    System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
        REG_SZ,
    },
};
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn names(
    identifier: &str,
    executable: &Path,
) -> Result<(Vec<u16>, Vec<u16>, Vec<u16>), &'static str> {
    let name = wide(login::value_name(identifier)?);
    if !executable.is_absolute() {
        return Err("login_invalid_executable");
    }
    let path: Vec<u16> = executable.as_os_str().encode_wide().collect();
    let (legacy, command) = login::commands(&path)?;
    Ok((name, legacy, command))
}
pub fn login_status(identifier: &str, executable: &Path) -> Login {
    status_at(RUN, identifier, executable)
}
fn status_at(run: &str, identifier: &str, executable: &Path) -> Login {
    let Ok((name, legacy, command)) = names(identifier, executable) else {
        return Login::Unavailable;
    };
    let mut raw = std::ptr::null_mut();
    let run = wide(run);
    let opened = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, run.as_ptr(), 0, KEY_READ, &mut raw) };
    if opened == ERROR_FILE_NOT_FOUND {
        return Login::Disabled;
    }
    if opened != ERROR_SUCCESS {
        return Login::Unavailable;
    }
    let key = Key(raw);
    let mut bytes = 0;
    let mut kind = 0;
    let query = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if query == ERROR_FILE_NOT_FOUND {
        return Login::Disabled;
    }
    if query != ERROR_SUCCESS || kind != REG_SZ || bytes > 65536 || bytes % 2 != 0 {
        return Login::Unavailable;
    }
    let mut value = vec![0u16; bytes as usize / 2];
    if unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    } != ERROR_SUCCESS
        || kind != REG_SZ
        || bytes as usize > value.len() * 2
        || bytes % 2 != 0
    {
        return Login::Unavailable;
    }
    value.truncate(bytes as usize / 2);
    if value.last() == Some(&0) {
        value.pop();
    }
    let equal = |expected: &[u16]| unsafe {
        CompareStringOrdinal(
            value.as_ptr(),
            value.len() as i32,
            expected.as_ptr(),
            expected.len() as i32,
            1,
        ) == 2
    };
    if equal(&command) || equal(&legacy) {
        Login::Enabled
    } else {
        Login::Disabled
    }
}
/// Only the existing explicit launch-at-login action calls this setter.
/// No automatic registration rewrite or cleanup occurs while reading status.
pub fn set_login(identifier: &str, executable: &Path, enabled: bool) -> Result<(), &'static str> {
    set_at(RUN, identifier, executable, enabled)
}
fn set_at(
    run: &str,
    identifier: &str,
    executable: &Path,
    enabled: bool,
) -> Result<(), &'static str> {
    let (name, _, mut command) = names(identifier, executable)?;
    let mut raw = std::ptr::null_mut();
    let run = wide(run);
    let created = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            run.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut raw,
            std::ptr::null_mut(),
        )
    };
    if created != ERROR_SUCCESS {
        return Err("login_registration_unavailable");
    }
    let key = Key(raw);
    let result = if enabled {
        command.push(0);
        unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key.0, name.as_ptr()) }
    };
    if result == ERROR_SUCCESS || !enabled && result == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err("login_registration_failed")
    }
}
#[cfg(feature = "private-fixtures")]
pub fn owned_fixture() -> Result<(), &'static str> {
    const OWNED: &str = r"Software\GovorunOwnedSdkWp27\Run";
    let exe = Path::new(r"C:\GovorunNative-wp-13-20261003\synthetic\Говорун Тест.exe");
    let main = "com.govorun.app";
    let test = "com.govorun.app.testbackend";
    let result = (|| {
        set_at(OWNED, main, exe, true)?;
        set_at(OWNED, test, exe, true)?;
        if status_at(OWNED, main, exe) != Login::Enabled
            || status_at(OWNED, test, exe) != Login::Enabled
        {
            return Err("login_fixture_roundtrip_failed");
        }
        let folded = Path::new(r"c:\govorunnative-wp-13-20261003\SYNTHETIC\ГОВОРУН ТЕСТ.EXE");
        if status_at(OWNED, main, folded) != Login::Enabled {
            return Err("login_fixture_unicode_case_failed");
        }
        if status_at(OWNED, main, Path::new(r"C:\Other\Govorun.exe")) != Login::Disabled {
            return Err("login_fixture_unrelated_path_accepted");
        }
        set_at(OWNED, main, exe, false)?;
        if status_at(OWNED, main, exe) != Login::Disabled
            || status_at(OWNED, test, exe) != Login::Enabled
        {
            return Err("login_fixture_profile_isolation_failed");
        }
        Ok(())
    })();
    // Always clear both values, including after a failed assertion. This key
    // is synthetic; the actual Windows Run key is never accessed by this test.
    let first = set_at(OWNED, main, exe, false);
    let second = set_at(OWNED, test, exe, false);
    let run = wide(OWNED);
    use windows_sys::Win32::System::Registry::RegDeleteKeyW;
    let removed = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, run.as_ptr()) };
    if first.is_err() || second.is_err() || !matches!(removed, ERROR_SUCCESS | ERROR_FILE_NOT_FOUND)
    {
        return Err("login_fixture_cleanup_failed");
    }
    result
}
