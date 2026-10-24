//! Read current platform documents without rewriting unrelated preferences.
mod description;
pub use description::{describe, preview};
use govorun_core::hotkeys::{settings, Facts, Gesture};
use govorun_os_runtime::keyboard::{assignment::Candidate, Platform};
use serde_json::{json, Value};
fn parse_windows(raw: &str) -> Option<Candidate> {
    let parts: Vec<_> = raw
        .split('+')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();
    let last = parts.last()?.to_ascii_lowercase();
    let mut mask = 0;
    let mut side = 0;
    for value in &parts[..parts.len() - 1] {
        let lower = value.to_ascii_lowercase();
        let bit = match lower.as_str() {
            "ctrl" | "control" => 1,
            "leftctrl" => {
                side = 1;
                1
            }
            "rightctrl" => {
                side = 2;
                1
            }
            "shift" => 2,
            "alt" => 4,
            "win" | "windows" => 8,
            _ => return None,
        };
        if mask & bit != 0 {
            return None;
        }
        mask |= bit;
    }
    let key = match last.as_str() {
        "leftctrl" => 0xA2,
        "rightctrl" => 0xA3,
        "leftshift" => 0xA0,
        "rightshift" => 0xA1,
        "leftalt" => 0xA4,
        "rightalt" => 0xA5,
        "leftwin" | "win" | "windows" => 0x5B,
        "rightwin" => 0x5C,
        "escape" | "esc" => 0x1B,
        "space" => 0x20,
        _ => {
            if last.len() == 1 && last.as_bytes()[0].is_ascii_alphanumeric() {
                last.to_ascii_uppercase().as_bytes()[0] as u16
            } else if let Some(f) = last
                .strip_prefix('f')
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|value| (1..=12).contains(value))
            {
                0x70 + f - 1
            } else {
                return None;
            }
        }
    };
    if key == 0x1B && mask != 0 {
        return None;
    }
    Some(Candidate {
        key,
        modifiers: mask,
        modifier_only: mask == 0 && matches!(key, 0xA0..=0xA5 | 0x5B | 0x5C),
        control_side: side,
    })
}
fn parse_mac(value: &Value) -> Option<Candidate> {
    let key = u16::try_from(value["keyCode"].as_u64()?).ok()?;
    let flags = value["modifiers"].as_u64()?;
    let modifier_only = match value["kind"].as_str()? {
        "key" => false,
        "modifierOnly" => true,
        _ => return None,
    };
    let modifiers =
        [(1, 18), (2, 17), (4, 19), (8, 20)]
            .into_iter()
            .fold(0, |value, (bit, shift)| {
                if flags & (1 << shift) != 0 {
                    value | bit
                } else {
                    value
                }
            });
    Some(Candidate {
        key,
        modifiers,
        modifier_only,
        control_side: 0,
    })
}
pub fn bindings_raw(platform: Platform, raw: &Value) -> Result<Gesture, &'static str> {
    let candidate = if platform == Platform::MacOS {
        parse_mac(raw)
    } else {
        raw.as_str().and_then(parse_windows)
    }
    .ok_or("hotkeys_invalid_binding")?;
    Ok(describe(platform, candidate))
}
pub fn facts(document: &Value, busy: bool, permission_ready: bool) -> Result<Facts, &'static str> {
    // Unreadable preferences mean the default keys; an assignment still refuses them.
    let values = settings::values(document).unwrap_or_default();
    let platform = if document["source"] == "macos" {
        Platform::MacOS
    } else {
        Platform::Windows
    };
    let default = describe(
        platform,
        if platform == Platform::MacOS {
            Candidate {
                key: 61,
                modifiers: 4,
                modifier_only: true,
                control_side: 0,
            }
        } else {
            Candidate {
                key: 0x20,
                modifiers: 1,
                modifier_only: false,
                control_side: 1,
            }
        },
    );
    let cancel = describe(
        platform,
        Candidate {
            key: if platform == Platform::MacOS {
                53
            } else {
                0x1B
            },
            modifiers: 0,
            modifier_only: false,
            control_side: 0,
        },
    );
    let read = |prefix: &str| -> Option<Gesture> {
        if platform == Platform::MacOS {
            let raw = json!({
                "keyCode":values.get(&format!("{prefix}KeyCode"))?,
                "modifiers":values.get(&format!("{prefix}Modifiers")).cloned().unwrap_or(json!(0)),
                "kind":values.get(&format!("{prefix}Kind")).cloned().unwrap_or(json!("key")),
            });
            bindings_raw(platform, &raw)
                .ok()
                .filter(|gesture| prefix == "cancelHotkey" || gesture.key != 53)
        } else {
            values
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(prefix))
                .and_then(|(_, value)| {
                    bindings_raw(platform, value).ok().filter(|gesture| {
                        gesture.valid && (prefix == "cancelHotkey" || gesture.key != 0x1B)
                    })
                })
        }
    };
    let input_mode = if platform == Platform::MacOS {
        None
    } else {
        Some(
            if values
                .iter()
                .any(|(key, value)| key.eq_ignore_ascii_case("pushToTalk") && value == &json!(true))
            {
                "hold"
            } else {
                "adaptive"
            }
            .into(),
        )
    };
    Ok(Facts {
        dictation: read("hotkey").unwrap_or_else(|| default.clone()),
        plain: read("plainHotkey"),
        cancel: read("cancelHotkey").unwrap_or_else(|| cancel.clone()),
        default_dictation: default,
        default_cancel: cancel,
        busy,
        permission_ready,
        input_mode,
    }
    .normalized())
}
