//! First-run permissions and preparation commands.
pub mod settings;
mod snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use snapshot::{
    Acceleration, Device, Duration, Login, Marker, Microphone, Permissions, Snapshot,
};

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Facts {
    pub can_prepare: bool,
    pub settings: Snapshot,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Microphone,
    Accessibility,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Field {
    MicrophoneId,
    PlaySounds,
    MuteAudio,
    MaxMinutes,
    Acceleration,
    VoiceMarker,
    LaunchAtLogin,
    PushToTalk,
    KeepModelLoaded,
    AlwaysOnMicrophone,
    InputChannel,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    Prepare {},
    Permission { permission: Permission },
    LoginItems {},
    // Route typed fields to the settings store.
    Set { field: Field, value: Value },
}

impl Facts {
    pub fn effect(self, command: Command) -> Result<Command, &'static str> {
        if !self.settings.checked() {
            return Err("dictation_invalid_facts");
        }
        if matches!(command, Command::Prepare {}) && !self.can_prepare {
            return Err("dictation_busy");
        }
        Ok(command)
    }
}

/// The setup screen describes the same durable route used by recording.
pub fn duration_mode(self_hosted: bool, official_enabled: bool) -> Duration {
    if self_hosted {
        Duration::Unlimited
    } else if official_enabled {
        Duration::Styles
    } else {
        Duration::Local
    }
}
