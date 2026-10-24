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
pub fn overlay(document: &Value, facts: Snapshot) -> Result<Snapshot, &'static str> {
    overlay_noted(document, facts).map(|(snapshot, _)| snapshot)
}
/// The same overlay, also saying whether a stored value fell back to its default
/// so the host can journal it once.
pub fn overlay_noted(document: &Value, mut facts: Snapshot) -> Result<(Snapshot, bool), &'static str> {
    // An unreadable preferences file records with the defaults; edits still refuse it.
    let stored = values(document);
    let defaulted = std::cell::Cell::new(stored.is_err());
    let values = stored.unwrap_or_default();
    let mac = document["source"] == "macos";
    // Each preference is read on its own: a value this version does not know
    // (another version, another device) falls back to its default and never
    // stops recording. Edits stay strict in `assign`.
    let read = |key: &str, usable: fn(&Value) -> bool| {
        values
            .iter()
            .find(|(name, value)| name.eq_ignore_ascii_case(key) && !value.is_null())
            .map(|(_, value)| value)
            .filter(|value| usable(value) || { defaulted.set(true); false })
    };
    let flag = |key: &str, default| read(key, Value::is_boolean).and_then(Value::as_bool).unwrap_or(default);
    facts.play_sounds = flag(if mac { "playRecordingSounds" } else { "playSounds" }, true);
    facts.mute_audio = flag("muteAudioDuringRecording", false);
    facts.max_minutes = read("maxRecordingMinutes", |v| v.as_u64().is_some_and(|minutes| minutes <= 180))
        .and_then(Value::as_u64)
        .map_or(5, |minutes| minutes as u32);
    facts.microphone_id = read(if mac { "audioInputDeviceUID" } else { "audioInputDevice" },
        |v| v.as_str().is_some_and(|id| id.encode_utf16().count() <= 1024))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    facts.acceleration = if mac {
        match read("asrAccelerationMode", |v| matches!(v.as_str(), Some("automatic" | "cpu"))).and_then(Value::as_str) {
            Some("cpu") => Acceleration::Cpu,
            _ => Acceleration::Automatic,
        }
    } else {
        match read("asrAcceleration", |v| matches!(v.as_u64(), Some(0..=2))).and_then(Value::as_u64) {
            Some(1) => Acceleration::Cpu,
            Some(2) => Acceleration::Gpu,
            _ => Acceleration::Automatic,
        }
    };
    facts.voice_marker = if mac {
        match read("voiceInputMarkerMode", |v| matches!(v.as_str(), Some("off" | "microphone" | "text"))).and_then(Value::as_str) {
            Some("microphone") => Marker::Microphone,
            Some("text") => Marker::Text,
            _ => Marker::Off,
        }
    } else {
        match read("voiceMarker", |v| matches!(v.as_u64(), Some(0..=2))).and_then(Value::as_u64) {
            Some(1) => Marker::Microphone,
            Some(2) => Marker::Text,
            _ => Marker::Off,
        }
    };
    if !mac {
        facts.push_to_talk = Some(flag("pushToTalk", false));
        facts.keep_model_loaded = Some(flag("keepRecognitionModelLoaded", true));
        facts.always_on_microphone = Some(flag("alwaysOnMicrophone", false));
        facts.input_channel = Some(
            read("audioInputChannel", |v| v.as_u64().is_some_and(|channel| channel < 32))
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    if !facts.checked() {
        return Err("dictation_storage_invalid");
    }
    Ok((facts, defaulted.get()))
}
