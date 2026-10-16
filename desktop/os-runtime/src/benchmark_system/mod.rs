//! Local hardware facts already shown in the recognition report. No identifiers,
//! paths, account fields, network requests or report formatting live here.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
pub struct Adapter {
    pub name: String,
    pub driver: String,
}
pub struct Info {
    pub os_version: String,
    pub architecture: String,
    pub machine: String,
    pub cpu: String,
    pub cpu_threads: u32,
    pub memory_mib: Option<u64>,
    pub adapters: Vec<Adapter>,
}
pub fn clean(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(160)
        .collect()
}
pub fn read() -> Info {
    #[cfg(target_os = "macos")]
    {
        macos::read()
    }
    #[cfg(target_os = "windows")]
    {
        windows::read()
    }
}
