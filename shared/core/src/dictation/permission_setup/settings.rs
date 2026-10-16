//! Existing local recording preference documents; the host owns CAS and OS
//! application. No migration, account/sync mutation or new persistent store.
use super::{
    snapshot::{Acceleration, Duration, Marker},
    Field, Snapshot,
};
use serde_json::{json, Map, Value};
pub const KEY: &str = "recording.document";
pub const MAC_KEYS: [&str; 6] = [
    "audioInputDeviceUID",
    "playRecordingSounds",
    "muteAudioDuringRecording",
    "maxRecordingMinutes",
    "asrAccelerationMode",
    "voiceInputMarkerMode",
];
pub const WINDOWS_KEYS: [&str; 10] = [
    "audioInputDevice",
    "audioInputChannel",
    "playSounds",
    "muteAudioDuringRecording",
    "maxRecordingMinutes",
    "asrAcceleration",
    "voiceMarker",
    "pushToTalk",
    "keepRecognitionModelLoaded",
    "alwaysOnMicrophone",
];
fn invalid<T>() -> Result<T, &'static str> {
    Err("dictation_invalid_setting")
}
pub fn values(document: &Value) -> Result<Map<String, Value>, &'static str> {
    if document.as_object().is_none_or(|value| value.len() != 2) {
        return invalid();
    }
    match document["source"].as_str() {
        Some("macos") => document["values"]
            .as_object()
            .filter(|values| values.keys().all(|key| MAC_KEYS.contains(&key.as_str())))
            .cloned()
            .ok_or("dictation_storage_invalid"),
        Some("windows") if document["values"].is_null() => Ok(Map::new()),
        Some("windows") => document["values"]
            .as_str()
            .and_then(|raw| serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}')).ok())
            .and_then(|value| value.as_object().cloned())
            .ok_or("dictation_storage_invalid"),
        _ => invalid(),
    }
}
/// Evaluate once against current observed devices/route. Reread the document
/// after a slow driver open and reapply this patch before committing its CAS.
pub fn assign(
    document: &Value,
    field: &Field,
    value: &Value,
    facts: &Snapshot,
    busy: bool,
) -> Result<Value, &'static str> {
    if !facts.checked() {
        return Err("dictation_invalid_facts");
    }
    let mac = document["source"] == "macos";
    let mut values = values(document)?;
    let bool_value = || {
        value
            .as_bool()
            .map(Value::Bool)
            .ok_or("dictation_invalid_setting")
    };
    let (key, stored) = match field {
        Field::PlaySounds => (
            if mac {
                "playRecordingSounds"
            } else {
                "playSounds"
            },
            Some(bool_value()?),
        ),
        Field::MuteAudio => ("muteAudioDuringRecording", Some(bool_value()?)),
        Field::PushToTalk | Field::AlwaysOnMicrophone if !mac => {
            if busy {
                return Err("dictation_busy");
            }
            (
                if matches!(field, Field::PushToTalk) {
                    "pushToTalk"
                } else {
                    "alwaysOnMicrophone"
                },
                Some(bool_value()?),
            )
        }
        Field::KeepModelLoaded if !mac => ("keepRecognitionModelLoaded", Some(bool_value()?)),
        Field::MaxMinutes => {
            if matches!(facts.duration_mode, Duration::Unlimited) {
                return invalid();
            }
            let minutes = value
                .as_u64()
                .filter(|minutes| *minutes <= 180)
                .ok_or("dictation_invalid_setting")?;
            ("maxRecordingMinutes", Some(json!(minutes)))
        }
        Field::MicrophoneId => {
            let stored = if value.is_null() {
                if mac {
                    None
                } else {
                    Some(json!(""))
                }
            } else {
                let id = value
                    .as_str()
                    .filter(|id| facts.microphones.iter().any(|device| device.id == *id))
                    .ok_or("dictation_invalid_setting")?;
                Some(json!(id))
            };
            if !mac {
                put(&mut values, "audioInputChannel", None);
            }
            (
                if mac {
                    "audioInputDeviceUID"
                } else {
                    "audioInputDevice"
                },
                stored,
            )
        }
        Field::InputChannel if !mac => {
            let channel = if value.is_null() {
                None
            } else {
                Some(json!(value
                    .as_u64()
                    .filter(|channel| facts
                        .input_channels
                        .is_some_and(|count| *channel < u64::from(count)))
                    .ok_or("dictation_invalid_setting")?))
            };
            ("audioInputChannel", channel)
        }
        Field::Acceleration => {
            let text = value.as_str().ok_or("dictation_invalid_setting")?;
            let stored = if mac {
                match text {
                    "automatic" | "cpu" => json!(text),
                    _ => return invalid(),
                }
            } else {
                json!(match text {
                    "automatic" => 0,
                    "cpu" => 1,
                    "gpu" => 2,
                    _ => return invalid(),
                })
            };
            (
                if mac {
                    "asrAccelerationMode"
                } else {
                    "asrAcceleration"
                },
                Some(stored),
            )
        }
        Field::VoiceMarker => {
            let text = value.as_str().ok_or("dictation_invalid_setting")?;
            let stored = if mac {
                match text {
                    "off" | "microphone" | "text" => json!(text),
                    _ => return invalid(),
                }
            } else {
                json!(match text {
                    "off" => 0,
                    "microphone" => 1,
                    "text" => 2,
                    _ => return invalid(),
                })
            };
            (
                if mac {
                    "voiceInputMarkerMode"
                } else {
                    "voiceMarker"
                },
                Some(stored),
            )
        }
        _ => return invalid(), // OS login/permission/preparation are not document writes.
    };
    put(&mut values, key, stored);
    if mac {
        Ok(json!({"source":"macos","values":values}))
    } else {
        Ok(
            json!({"source":"windows","values":serde_json::to_string(&values).map_err(|_|"dictation_storage_invalid")?}),
        )
    }
}
fn put(values: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    let old: Vec<_> = values
        .keys()
        .filter(|old| old.eq_ignore_ascii_case(key))
        .cloned()
        .collect();
    for old in old {
        values.remove(&old);
    }
    if let Some(value) = value {
        values.insert(key.into(), value);
    }
}
/// Overlay durable preferences on freshly observed OS-only facts. A selected
/// endpoint that disappeared stays selected and is never overwritten.
pub fn overlay(document: &Value, mut facts: Snapshot) -> Result<Snapshot, &'static str> {
    let values = values(document)?;
    let mac = document["source"] == "macos";
    let read = |key: &str| {
        values
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value)
    };
    let bool_value = |key: &str, default| {
        read(key)
            .map(|value| value.as_bool().ok_or("dictation_storage_invalid"))
            .unwrap_or(Ok(default))
    };
    facts.play_sounds = bool_value(
        if mac {
            "playRecordingSounds"
        } else {
            "playSounds"
        },
        true,
    )?;
    facts.mute_audio = bool_value("muteAudioDuringRecording", false)?;
    facts.max_minutes = read("maxRecordingMinutes")
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or("dictation_storage_invalid")
        })
        .unwrap_or(Ok(5))?;
    let device = read(if mac {
        "audioInputDeviceUID"
    } else {
        "audioInputDevice"
    })
    .map(|value| value.as_str().ok_or("dictation_storage_invalid"))
    .transpose()?;
    facts.microphone_id = device.filter(|value| !value.is_empty()).map(str::to_owned);
    facts.acceleration = if mac {
        match read("asrAccelerationMode").and_then(Value::as_str) {
            None | Some("automatic") => Acceleration::Automatic,
            Some("cpu") => Acceleration::Cpu,
            _ => return invalid(),
        }
    } else {
        match read("asrAcceleration").and_then(Value::as_u64) {
            None | Some(0) => Acceleration::Automatic,
            Some(1) => Acceleration::Cpu,
            Some(2) => Acceleration::Gpu,
            _ => return invalid(),
        }
    };
    facts.voice_marker = if mac {
        match read("voiceInputMarkerMode").and_then(Value::as_str) {
            None | Some("off") => Marker::Off,
            Some("microphone") => Marker::Microphone,
            Some("text") => Marker::Text,
            _ => return invalid(),
        }
    } else {
        match read("voiceMarker").and_then(Value::as_u64) {
            None | Some(0) => Marker::Off,
            Some(1) => Marker::Microphone,
            Some(2) => Marker::Text,
            _ => return invalid(),
        }
    };
    if !mac {
        facts.push_to_talk = Some(bool_value("pushToTalk", false)?);
        facts.keep_model_loaded = Some(bool_value("keepRecognitionModelLoaded", true)?);
        facts.always_on_microphone = Some(bool_value("alwaysOnMicrophone", false)?);
        facts.input_channel = Some(read("audioInputChannel").cloned().unwrap_or(Value::Null));
    }
    if !facts.checked() {
        return Err("dictation_storage_invalid");
    }
    Ok(facts)
}
