//! Device catalogs and permission effects belong to this host process.
//! Enumeration never opens a microphone, requests access or persists a choice.
pub mod access;
pub mod login;
#[cfg(target_os = "macos")]
mod macos_login;
#[cfg(target_os = "macos")]
pub use macos_login::{login_status, open_login_settings, set_login};
#[cfg(target_os = "windows")]
mod windows_login;
#[cfg(all(target_os = "windows", feature = "private-fixtures"))]
pub use windows_login::owned_fixture as owned_login_fixture;
#[cfg(target_os = "windows")]
pub use windows_login::{login_status, set_login};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
pub use macos::{microphone, request_microphone};
#[cfg(target_os = "windows")]
pub use windows::microphone;
pub struct Input {
    pub id: String,
    pub name: String,
    pub channels: u32,
}
pub struct Catalog {
    pub inputs: Vec<Input>,
    pub channels: u32,
}
pub fn catalog(preferred: &str) -> Result<Catalog, &'static str> {
    let devices = crate::capture::device::inputs()?;
    let default = devices.iter().find(|device| device.is_default).map(|device| device.id.clone());
    let mut inputs = Vec::new();
    for device in devices {
        let id = device.id;
        if inputs.len() == 256 || inputs.iter().any(|input: &Input| input.id == id) {
            return Err("microphone_catalog_invalid");
        }
        inputs.push(Input { id, name: device.name, channels: u32::from(device.channels).clamp(1, 32) });
    }
    // Like the current Windows catalog, use the default input when the saved
    // endpoint is missing. The saved endpoint itself is never overwritten.
    inputs.sort_by_key(|input| default.as_deref() != Some(input.id.as_str()));
    let channels = inputs
        .iter()
        .find(|input| input.id == preferred)
        .or(inputs.first())
        .map_or(1, |input| input.channels);
    Ok(Catalog { inputs, channels })
}
