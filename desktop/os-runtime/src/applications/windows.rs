use super::Choice;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::Path,
};
use windows_sys::Win32::{
    Foundation::*,
    Globalization::*,
    Storage::FileSystem::*,
    System::{Diagnostics::ToolHelp::*, Environment::*, Registry::*, Threading::*},
    UI::WindowsAndMessaging::*,
};
fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn open(hive: HKEY, path: &str, view: u32) -> Option<Key> {
    let mut key = std::ptr::null_mut();
    (unsafe {
        RegOpenKeyExW(
            hive,
            wide(OsStr::new(path)).as_ptr(),
            0,
            KEY_READ | view,
            &mut key,
        )
    } == 0)
        .then_some(Key(key))
}
pub fn identity(value: &str) -> String {
    let name = value.rsplit(['\\', '/']).next().unwrap_or(value);
    let name = name.rsplit_once('.').map_or(name, |(base, _)| base).trim();
    let input: Vec<u16> = name.encode_utf16().collect();
    if input.is_empty() {
        return String::new();
    }
    let length = unsafe {
        LCMapStringEx(
            LOCALE_NAME_INVARIANT,
            LCMAP_LOWERCASE,
            input.as_ptr(),
            input.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    if length <= 0 {
        return name.to_lowercase();
    }
    let mut result = vec![0u16; length as usize];
    if unsafe {
        LCMapStringEx(
            LOCALE_NAME_INVARIANT,
            LCMAP_LOWERCASE,
            input.as_ptr(),
            input.len() as i32,
            result.as_mut_ptr(),
            length,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    } != length
    {
        return name.to_lowercase();
    }
    String::from_utf16_lossy(&result)
}
fn description(path: &OsStr, fallback: &str) -> String {
    let filename = wide(path);
    let size = unsafe { GetFileVersionInfoSizeW(filename.as_ptr(), std::ptr::null_mut()) };
    if size == 0 || size > 1024 * 1024 {
        return fallback.into();
    }
    let mut bytes = vec![0u8; size as usize];
    if unsafe { GetFileVersionInfoW(filename.as_ptr(), 0, size, bytes.as_mut_ptr().cast()) } == 0 {
        return fallback.into();
    }
    let mut translations = std::ptr::null_mut();
    let mut count = 0;
    let translation = wide(OsStr::new("\\VarFileInfo\\Translation"));
    let mut variants = vec![(0x0409u16, 0x04b0u16), (0x0409, 0x04e4)];
    if unsafe {
        VerQueryValueW(
            bytes.as_ptr().cast(),
            translation.as_ptr(),
            &mut translations,
            &mut count,
        )
    } != 0
        && count >= 4
    {
        let values =
            unsafe { std::slice::from_raw_parts(translations.cast::<u16>(), (count / 2) as usize) };
        for pair in values.chunks_exact(2).rev() {
            variants.insert(0, (pair[0], pair[1]));
        }
    }
    for (language, encoding) in variants {
        let query = wide(OsStr::new(&format!(
            "\\StringFileInfo\\{language:04x}{encoding:04x}\\FileDescription"
        )));
        let mut text = std::ptr::null_mut();
        let mut length = 0;
        if unsafe {
            VerQueryValueW(
                bytes.as_ptr().cast(),
                query.as_ptr(),
                &mut text,
                &mut length,
            )
        } != 0
            && length > 0
            && length <= 121
        {
            let raw =
                unsafe { std::slice::from_raw_parts(text.cast::<u16>(), length as usize - 1) };
            let value = String::from_utf16_lossy(raw).trim().to_owned();
            if !value.is_empty() {
                return value;
            }
        }
    }
    fallback.into()
}
unsafe extern "system" fn visible(window: HWND, parameter: LPARAM) -> windows_sys::core::BOOL {
    if IsWindowVisible(window) != 0 && GetWindow(window, GW_OWNER).is_null() {
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        (*(parameter as *mut HashSet<u32>)).insert(pid);
    }
    1
}
fn running() -> Vec<Choice> {
    let mut pids: HashSet<u32> = HashSet::new();
    unsafe {
        EnumWindows(Some(visible), &mut pids as *mut _ as LPARAM);
    }
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return vec![];
    }
    let _snapshot = Handle(snapshot);
    let mut row: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    row.dwSize = std::mem::size_of_val(&row) as u32;
    let mut present = unsafe { Process32FirstW(snapshot, &mut row) } != 0;
    let mut result = vec![];
    while present {
        if pids.contains(&row.th32ProcessID) {
            let size = row
                .szExeFile
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(row.szExeFile.len());
            let id = identity(&String::from_utf16_lossy(&row.szExeFile[..size]));
            let process =
                unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, row.th32ProcessID) };
            let name = if process.is_null() {
                id.clone()
            } else {
                let _process = Handle(process);
                let mut path = vec![0u16; 32768];
                let mut length = path.len() as u32;
                if unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) }
                    != 0
                {
                    description(&OsString::from_wide(&path[..length as usize]), &id)
                } else {
                    id.clone()
                }
            };
            result.push(Choice { id, name });
        }
        present = unsafe { Process32NextW(snapshot, &mut row) } != 0;
    }
    result
}
fn registered() -> Vec<Choice> {
    let mut result = vec![];
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let Some(root) = open(
                hive,
                "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths",
                view,
            ) else {
                continue;
            };
            for index in 0..65536 {
                let mut name = vec![0u16; 1024];
                let mut length = name.len() as u32;
                let status = unsafe {
                    RegEnumKeyExW(
                        root.0,
                        index,
                        name.as_mut_ptr(),
                        &mut length,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                if status == 259 {
                    break;
                }
                if status != 0 {
                    continue;
                }
                let name = String::from_utf16_lossy(&name[..length as usize]);
                if !name.to_ascii_lowercase().ends_with(".exe") {
                    continue;
                }
                let Some(key) = open(root.0, &name, view) else {
                    continue;
                };
                let mut value = vec![0u16; 32768];
                let mut bytes = (value.len() * 2) as u32;
                let mut kind = 0;
                if unsafe {
                    RegQueryValueExW(
                        key.0,
                        std::ptr::null(),
                        std::ptr::null(),
                        &mut kind,
                        value.as_mut_ptr().cast(),
                        &mut bytes,
                    )
                } != 0
                    || !matches!(kind, REG_SZ | REG_EXPAND_SZ)
                    || bytes < 2
                    || bytes % 2 != 0
                {
                    continue;
                }
                let raw = String::from_utf16_lossy(&value[..bytes as usize / 2 - 1]);
                let raw = wide(OsStr::new(raw.trim_matches('"')));
                let mut expanded = vec![0u16; 32768];
                let size = unsafe {
                    ExpandEnvironmentStringsW(
                        raw.as_ptr(),
                        expanded.as_mut_ptr(),
                        expanded.len() as u32,
                    )
                };
                if size == 0 || size > expanded.len() as u32 {
                    continue;
                }
                let path = OsString::from_wide(&expanded[..size as usize - 1]);
                if !Path::new(&path).is_absolute() || !Path::new(&path).is_file() {
                    continue;
                }
                let id = identity(&path.to_string_lossy());
                let name = description(&path, &id);
                result.push(Choice { id, name });
            }
        }
    }
    result
}
pub fn normalize(choices: impl IntoIterator<Item = Choice>) -> Vec<Choice> {
    let mut names = BTreeMap::new();
    for choice in choices {
        if choice.id.trim().is_empty()
            || choice.id.encode_utf16().count() > 120
            || choice.name.trim().is_empty()
            || choice.name.encode_utf16().count() > 120
            || choice.name.chars().any(char::is_control)
        {
            continue;
        }
        let id = identity(&choice.id);
        if !id.is_empty() {
            names.entry(id).or_insert(choice.name);
        }
    }
    let mut result: Vec<_> = names
        .into_iter()
        .map(|(id, name)| Choice { id, name })
        .collect();
    result.sort_by(|a, b| {
        let left: Vec<_> = a.name.encode_utf16().collect();
        let right: Vec<_> = b.name.encode_utf16().collect();
        let order = unsafe {
            CompareStringEx(
                std::ptr::null(),
                NORM_IGNORECASE,
                left.as_ptr(),
                left.len() as i32,
                right.as_ptr(),
                right.len() as i32,
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        };
        order.cmp(&2)
    });
    result.truncate(4096);
    result
}
pub fn discover(_: &[Choice]) -> Vec<Choice> {
    normalize(running().into_iter().chain(registered()))
}
