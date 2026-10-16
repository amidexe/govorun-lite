// OS adapters normalize system codes; the shared Rust core owns diagnostics and their format.
pub mod windows;
pub mod macos;

#[cfg(windows)]
pub(crate) use windows::{worker_exit, worker_start_failed};
#[cfg(not(windows))]
pub(crate) use macos::{worker_exit, worker_start_failed};
