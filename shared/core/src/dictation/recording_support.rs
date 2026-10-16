//! Temporary, explicitly confirmed recording diagnostics; never enabled at startup.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Facts {
    pub enabled: bool,
    pub recording: bool,
    pub has_files: bool,
    pub can_start: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", schemars(rename = "RecordingDiagnosticState"))]
pub struct Snapshot {
    pub enabled: bool,
    pub recording: bool,
    pub has_files: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
pub enum Command {
    Start { confirmed: bool },
    Stop {},
    Open {},
}

#[derive(Debug, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Effect {
    Start { confirmed: bool },
    Stop {},
    Open {},
}

impl Facts {
    pub fn snapshot(self) -> Snapshot {
        Snapshot {
            enabled: self.enabled,
            recording: self.recording,
            has_files: self.has_files,
        }
    }

    pub fn effect(self, command: Command) -> Result<Effect, &'static str> {
        match command {
            Command::Start { confirmed: false } => {
                Err("recording_diagnostics_confirmation_required")
            }
            Command::Start { confirmed: true } if !self.can_start => {
                Err("recording_diagnostics_busy")
            }
            Command::Start { confirmed: true } => Ok(Effect::Start { confirmed: true }),
            Command::Stop {} => Ok(Effect::Stop {}),
            Command::Open {} => Ok(Effect::Open {}),
        }
    }
}
