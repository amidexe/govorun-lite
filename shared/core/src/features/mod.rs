//! Portable feature scenarios and views; callers supply effects through ports.
/// Shared dictionary scenario API; effects enter through ports.
#[path="../dictionary/feature.rs"]
pub mod dictionary;
/// Shared statistics scenario API; effects enter through ports.
#[path="../statistics/feature.rs"]
pub mod statistics;
/// Shared style library scenario API; effects enter through ports.
#[path="../styles/library_feature.rs"]
pub mod style_library;
/// Shared styles scenario API; effects enter through ports.
#[path="../styles/feature/mod.rs"]
pub mod styles;
/// Shared clipboard scenario API; effects enter through ports.
#[path="../dictation/clipboard_feature.rs"]
pub mod clipboard;
/// Shared snapshot gate scenario API; effects enter through ports.
#[path="../infrastructure/snapshot_gate.rs"]
pub mod snapshot_gate;
/// Shared dto scenario API; effects enter through ports.
#[path="../infrastructure/feature_dto.rs"]
pub mod dto;
/// Shared cancellation scenario API; effects enter through ports.
#[path="../infrastructure/cancellation.rs"]
pub mod cancellation;
/// Shared sync scenario API; effects enter through ports.
#[path="../sync/driver.rs"]
pub mod sync;
/// Shared sync documents scenario API; effects enter through ports.
#[path="../sync/documents_workflow.rs"]
pub mod sync_documents;
/// Shared stylization scenario API; effects enter through ports.
#[path="../styles/runtime/mod.rs"]
pub mod stylization;
/// Shared documents scenario API; effects enter through ports.
#[path="../sync/documents/mod.rs"]
pub mod documents;
/// Shared dictation scenario API; effects enter through ports.
#[path="../dictation/workflow/mod.rs"]
pub mod dictation;
/// Shared permission settings scenario API; effects enter through ports.
#[path="../dictation/settings_owner.rs"]
pub mod permission_settings;
/// Shared capabilities scenario API; effects enter through ports.
#[path="../infrastructure/capabilities.rs"]
pub mod capabilities;
/// Shared ports scenario API; effects enter through ports.
#[path="../infrastructure/effect_ports.rs"]
pub mod ports;
/// Shared blocking cancellation scenario API; effects enter through ports.
#[path="../infrastructure/blocking_cancellation.rs"]
pub mod blocking_cancellation;
/// Shared benchmark runner scenario API; effects enter through ports.
#[path="../diagnostics/benchmark_runner.rs"]
pub mod benchmark_runner;
/// Shared self host scenario API; effects enter through ports.
#[path="../dictation/self_host_feature/mod.rs"]
pub mod self_host;
/// Shared runtime session scenario API; effects enter through ports.
#[path="../infrastructure/runtime_session.rs"]
pub mod runtime_session;
/// Shared runtime scenario API; effects enter through ports.
#[path="../infrastructure/runtime.rs"]
pub mod runtime;
/// Shared benchmark scenario API; effects enter through ports.
#[path="../diagnostics/benchmark_feature.rs"]
pub mod benchmark;
/// Shared diagnostics scenario API; effects enter through ports.
#[path="../diagnostics/journal_feature.rs"]
pub mod diagnostics;
/// Shared recording support scenario API; effects enter through ports.
#[path="../diagnostics/recordings_feature.rs"]
pub mod recording_support;
/// Shared permission setup scenario API; effects enter through ports.
#[path="../dictation/setup_feature.rs"]
pub mod permission_setup;
/// Shared hotkeys scenario API; effects enter through ports.
#[path="../dictation/hotkey_feature/mod.rs"]
pub mod hotkeys;
/// Shared account scenario API; effects enter through ports.
#[path="../account/feature/mod.rs"]
pub mod account;
/// Shared billing scenario API; effects enter through ports.
#[path="../billing/feature/mod.rs"]
pub mod billing;
/// Shared about scenario API; effects enter through ports.
#[path="../diagnostics/application_info/mod.rs"]
pub mod about;
pub use crate::features::routing::{route, routes, Route};
/// Shared routing scenario API; effects enter through ports.
#[path="../infrastructure/routes.rs"]
pub mod routing;
/// Observable attention state shared by hosts.
#[path="../diagnostics/attention.rs"]
pub mod attention_state;
/// Command-line launch admission shared by hosts.
#[path="../updates/launch_policy.rs"]
pub mod launch_policy;
