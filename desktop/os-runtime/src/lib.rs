//! Local device and inference adapters. No account, product workflow, HTTP,
//! renderer, or durable user storage belongs to this crate.
// OS-only marker: our paste must not activate a user binding on Command/Ctrl.
#[cfg(any(feature = "keyboard", feature = "insertion"))]
pub(crate) const PASTE_EVENT_TAG: usize = 0x4756_5041;
pub mod capture;
pub mod insertion;
#[cfg(feature = "keyboard")]
pub mod keyboard;
#[cfg(feature = "inference")]
pub mod inference;
#[cfg(feature = "inference")]
pub mod inference_model;
#[cfg(feature = "inference")]
mod pinned_asr;
pub mod recognition;
pub mod model_cache;
pub mod backend_retry;
#[cfg(all(feature = "capture", feature = "keyboard"))]
pub mod setup;
#[cfg(all(feature = "capture", feature = "keyboard"))]
pub mod media;
pub mod segmentation;
#[cfg(feature = "inference")]
pub mod vad;

#[cfg(feature = "network")]
pub mod network;

#[cfg(feature = "applications")]
pub mod applications;

#[cfg(feature="clipboard")]
pub mod clipboard;

#[cfg(feature = "benchmark")]
pub mod benchmark;

#[cfg(feature = "benchmark")]
pub mod benchmark_system;
