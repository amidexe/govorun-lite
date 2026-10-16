//! Shared wire types for OS-owned application surfaces; no effects or rules.
use serde::{Deserialize, Serialize};
use crate::update_policy::Channel;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode { #[default] System, Light, Dark }

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AppearanceSnapshot { pub mode: ThemeMode, pub revision: u64 }


#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SetupStep { Welcome, Dictation, Trial, Done }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupProgress { pub step: SetupStep }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupSnapshot { pub version: u8, pub revision: u64, pub progress: Option<SetupProgress> }

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupChange { pub expected_revision: u64, pub progress: SetupProgress }


/// Presentation metadata only; Tauri owns manifests, version selection and signatures.
#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Release { pub version: String, pub notes: String }

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub status: &'static str, pub release: Option<Release>, pub dismissed: bool,
    pub transfer: Option<UpdateTransfer>, pub error: Option<&'static str>,
    pub channel: Channel, pub channel_selectable: bool,
}
#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpdateTransfer { pub downloaded: u64, pub total: u64 }
