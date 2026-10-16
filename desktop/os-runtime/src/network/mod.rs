//! OS path availability only. Requests and retry policy remain in Core/host.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
pub use macos::PathMonitor;
#[cfg(target_os = "windows")]
pub use windows::PathMonitor;

mod state;
pub use state::{Event, Handler, State};
