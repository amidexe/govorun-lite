//! Neutral hardware labels from the same old OS facts; never the user's computer name.
pub fn mac_family(model: &str) -> &'static str {
    for (prefix, name) in [
        ("MacBookPro", "MacBook Pro"),
        ("MacBookAir", "MacBook Air"),
        ("MacBook", "MacBook"),
        ("Macmini", "Mac mini"),
        ("iMac", "iMac"),
        ("MacPro", "Mac Pro"),
    ] {
        if model.starts_with(prefix) {
            return name;
        }
    }
    for (name, models) in [
        (
            "MacBook Pro",
            &[
                "Mac14,5", "Mac14,6", "Mac14,9", "Mac14,10", "Mac15,3", "Mac15,6", "Mac15,7",
                "Mac15,8", "Mac15,9", "Mac15,10", "Mac15,11", "Mac16,1", "Mac16,5", "Mac16,6",
                "Mac16,7", "Mac16,8",
            ][..],
        ),
        (
            "MacBook Air",
            &[
                "Mac14,2", "Mac14,15", "Mac15,12", "Mac15,13", "Mac16,12", "Mac16,13",
            ][..],
        ),
        (
            "Mac mini",
            &["Mac14,3", "Mac14,12", "Mac16,10", "Mac16,11"][..],
        ),
        ("iMac", &["Mac15,4", "Mac15,5", "Mac16,2", "Mac16,3"][..]),
        (
            "Mac Studio",
            &[
                "Mac13,1", "Mac13,2", "Mac14,13", "Mac14,14", "Mac15,14", "Mac16,9",
            ][..],
        ),
        ("Mac Pro", &["Mac14,8"][..]),
    ] {
        if models.contains(&model) {
            return name;
        }
    }
    "Mac"
}
pub fn windows_label(
    manufacturer: Option<&str>,
    family: Option<&str>,
    product: Option<&str>,
) -> String {
    fn clean(value: Option<&str>) -> Option<String> {
        let value = value?.split_whitespace().collect::<Vec<_>>().join(" ");
        if value.is_empty()
            || value.chars().any(char::is_control)
            || [
                "Unknown",
                "None",
                "Not Applicable",
                "Default string",
                "System Product Name",
                "To be filled by O.E.M.",
            ]
            .iter()
            .any(|generic| value.eq_ignore_ascii_case(generic))
        {
            return None;
        }
        Some(value)
    }
    let maker = clean(manufacturer);
    let model = clean(family).or_else(|| clean(product));
    let value = match (maker, model) {
        (Some(maker), Some(model)) if !model.to_lowercase().contains(&maker.to_lowercase()) => {
            format!("{maker} {model}")
        }
        (_, Some(model)) => model,
        (Some(maker), None) => maker,
        (None, None) => "Windows PC".into(),
    };
    let units: Vec<_> = value.encode_utf16().take(64).collect();
    String::from_utf16_lossy(&units).trim_end().into()
}
#[cfg(target_os = "macos")]
pub fn current() -> String {
    use std::ffi::{c_char, c_void, CStr};
    extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            old: *mut c_void,
            len: *mut usize,
            new: *mut c_void,
            newlen: usize,
        ) -> i32;
    }
    let key = c"hw.model";
    let mut length = 0;
    if unsafe {
        sysctlbyname(
            key.as_ptr(),
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || !(2..=4096).contains(&length)
    {
        return "Mac".into();
    }
    let mut bytes = vec![0u8; length];
    if unsafe {
        sysctlbyname(
            key.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return "Mac".into();
    }
    let model = CStr::from_bytes_until_nul(&bytes)
        .ok()
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    mac_family(model.trim()).into()
}
#[cfg(target_os = "windows")]
pub fn current() -> String {
    use windows_sys::Win32::System::Registry::*;
    fn read(name: &str) -> Option<String> {
        let path: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\BIOS\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut length = 0;
        let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
        if unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                name.as_ptr(),
                flags,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut length,
            )
        } != 0
            || length > 16_384
            || length % 2 != 0
        {
            return None;
        }
        let mut bytes = vec![0u16; length as usize / 2];
        if unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                name.as_ptr(),
                flags,
                std::ptr::null_mut(),
                bytes.as_mut_ptr().cast(),
                &mut length,
            )
        } != 0
        {
            return None;
        }
        bytes.truncate(length as usize / 2);
        if let Some(index) = bytes.iter().position(|value| *value == 0) {
            bytes.truncate(index);
        }
        String::from_utf16(&bytes).ok()
    }
    windows_label(
        read("SystemManufacturer").as_deref(),
        read("SystemFamily").as_deref(),
        read("SystemProductName").as_deref(),
    )
}
