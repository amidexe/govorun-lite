//! Desktop capture uses the pinned Handy recorder; workflows and VAD stay shared.
pub mod admission;
pub mod executor;
#[cfg(feature = "capture")]
pub mod cpal_owner;
#[cfg(feature = "capture")]
pub mod device;
pub mod error;
pub mod owner;
