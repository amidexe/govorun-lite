//! Read the same three OS consent flags as the existing Windows adapter.
use super::access::{self, Microphone};
use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ, REG_EXPAND_SZ, REG_SZ,
    },
};
const PATH: &str =
    r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn read(root: HKEY, path: &str) -> Microphone {
    let wide = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let path = wide(path);
    let mut raw = std::ptr::null_mut();
    if unsafe { RegOpenKeyExW(root, path.as_ptr(), 0, KEY_READ, &mut raw) } != ERROR_SUCCESS {
        return Microphone::Unknown;
    }
    let key = Key(raw);
    let value = wide("Value");
    let mut kind = 0;
    let mut buffer = [0u16; 16];
    let mut bytes = std::mem::size_of_val(&buffer) as u32;
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            value.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if result != ERROR_SUCCESS
        || !matches!(kind, REG_SZ | REG_EXPAND_SZ)
        || bytes % 2 != 0
        || bytes as usize > std::mem::size_of_val(&buffer)
    {
        return Microphone::Unknown;
    }
    let units = &buffer[..bytes as usize / 2];
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    String::from_utf16(&units[..end])
        .map(|value| access::registry(&value))
        .unwrap_or(Microphone::Unknown)
}
pub fn microphone() -> Microphone {
    access::windows(
        read(HKEY_LOCAL_MACHINE, PATH),
        read(HKEY_CURRENT_USER, PATH),
        read(HKEY_CURRENT_USER, &format!(r"{PATH}\NonPackaged")),
    )
}
