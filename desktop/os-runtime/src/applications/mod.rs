//! Executable/bundle identifiers and display names. Window/document titles are never read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub name: String,
}
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
pub use macos::discover;
#[cfg(target_os = "windows")]
pub use windows::{discover, normalize};
