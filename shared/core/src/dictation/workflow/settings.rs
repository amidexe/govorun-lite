//! Typed view from the Core settings snapshot to local execution inputs.
//! No store, network, device or UI effect occurs while projecting it.
use govorun_core::{
    permission_setup::{Acceleration, Marker, Microphone, Snapshot},
    recording_controls::Mode,
};
use super::inputs::{Preferences, Compute};
#[derive(Clone)]
pub struct Settings {
    pub input: Preferences,
    pub mode: Mode,
    pub marker: &'static str,
    pub keep_clipboard: bool,
    pub local_timer_enabled: bool,
    pub configured_minutes: u32,
    pub compute: Compute,
    pub keep_model: bool,
    pub play_sounds: bool,
    pub mute_audio: bool,
}
impl Settings {
    pub fn from_snapshot(snapshot: &Snapshot, keep_clipboard: bool) -> Result<Self, &'static str> {
        if !snapshot.checked() {
            return Err("dictation_invalid_facts");
        }
        Ok(Self {
            input: Preferences {
                preferred: snapshot.microphone_id.clone().unwrap_or_default(),
                channel: snapshot
                    .input_channel
                    .as_ref()
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize),
                warm: snapshot.always_on_microphone.unwrap_or(false)
                    && matches!(snapshot.permissions.microphone, Microphone::Authorized),
            },
            mode: if snapshot.push_to_talk == Some(true) {
                Mode::Hold
            } else {
                Mode::Adaptive
            },
            marker: match snapshot.voice_marker {
                Marker::Off => "off",
                Marker::Microphone => "microphone",
                Marker::Text => "text",
            },
            keep_clipboard,
            local_timer_enabled: snapshot.max_minutes != 0,
            configured_minutes: snapshot.max_minutes,
            compute: match snapshot.acceleration {
                Acceleration::Automatic => Compute::Automatic,
                Acceleration::Cpu => Compute::Cpu,
                Acceleration::Gpu => Compute::Gpu,
            },
            // macOS preserves its existing 15-minute idle release. Windows
            // preserves the existing keep-model choice and the same idle budget.
            keep_model: snapshot.keep_model_loaded.unwrap_or(false),
            play_sounds: snapshot.play_sounds,
            mute_audio: snapshot.mute_audio,
        })
    }
}

use super::resources;
impl Settings {
    pub fn resources(&self) -> resources::Config {
        resources::Config {
            input: self.input.clone(),
            compute: self.compute,
            keep_model: self.keep_model,
        }
    }
}
