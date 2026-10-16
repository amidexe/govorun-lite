//! Existing OS document formats and physical labels, without a native process.
//! Assignment conflicts and fallback decisions remain in Core Facts/Workflow.
use govorun_core::hotkeys::Gesture;
use govorun_os_runtime::keyboard::{assignment::Candidate, Platform};
use serde_json::{json, Value};

fn windows_key(code: u16) -> Option<String> {
    Some(match code {
        0xA2 => "LeftCtrl".into(),
        0xA3 => "RightCtrl".into(),
        0xA0 => "LeftShift".into(),
        0xA1 => "RightShift".into(),
        0xA4 => "LeftAlt".into(),
        0xA5 => "RightAlt".into(),
        0x5B => "LeftWin".into(),
        0x5C => "RightWin".into(),
        0x1B => "Escape".into(),
        0x20 => "Space".into(),
        0x41..=0x5A | 0x30..=0x39 => char::from_u32(code as u32)?.to_string(),
        0x70..=0x7B => format!("F{}", code - 0x70 + 1),
        _ => return None,
    })
}
fn mac_key(code: u16) -> String {
    let name = match code {
        0 => "A",
        1 => "S",
        2 => "D",
        3 => "F",
        4 => "H",
        5 => "G",
        6 => "Z",
        7 => "X",
        8 => "C",
        9 => "V",
        11 => "B",
        12 => "Q",
        13 => "W",
        14 => "E",
        15 => "R",
        16 => "Y",
        17 => "T",
        18 => "1",
        19 => "2",
        20 => "3",
        21 => "4",
        22 => "6",
        23 => "5",
        24 => "=",
        25 => "9",
        26 => "7",
        27 => "-",
        28 => "8",
        29 => "0",
        30 => "]",
        31 => "O",
        32 => "U",
        33 => "[",
        34 => "I",
        35 => "P",
        36 => "Return",
        37 => "L",
        38 => "J",
        39 => "'",
        40 => "K",
        41 => ";",
        42 => "\\",
        43 => ",",
        44 => "/",
        45 => "N",
        46 => "M",
        47 => ".",
        48 => "Tab",
        49 => "Space",
        50 => "`",
        51 => "Delete",
        53 => "Esc",
        54 => "Right ⌘",
        55 => "Left ⌘",
        56 => "Left ⇧",
        60 => "Right ⇧",
        58 => "Left ⌥",
        61 => "Right ⌥",
        59 => "Left ⌃",
        62 => "Right ⌃",
        63 => "Fn",
        122 => "F1",
        120 => "F2",
        99 => "F3",
        118 => "F4",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        100 => "F8",
        101 => "F9",
        109 => "F10",
        103 => "F11",
        111 => "F12",
        105 => "F13",
        107 => "F14",
        113 => "F15",
        106 => "F16",
        64 => "F17",
        79 => "F18",
        80 => "F19",
        90 => "F20",
        123 => "←",
        124 => "→",
        125 => "↓",
        126 => "↑",
        _ => return format!("Key{code}"),
    };
    name.into()
}
pub(super) fn modifier(code: u16, platform: Platform) -> Option<u8> {
    Some(match (platform, code) {
        (Platform::MacOS, 59 | 62) | (Platform::Windows, 0xA2 | 0xA3) => 1,
        (Platform::MacOS, 56 | 60) | (Platform::Windows, 0xA0 | 0xA1) => 2,
        (Platform::MacOS, 58 | 61) | (Platform::Windows, 0xA4 | 0xA5) => 4,
        (Platform::MacOS, 54 | 55) | (Platform::Windows, 0x5B | 0x5C) => 8,
        (Platform::MacOS, 63) => 0,
        _ => return None,
    })
}
fn win_display(name: &str) -> String {
    match name {
        "Escape" => "Esc",
        "LeftCtrl" => "Левый Ctrl",
        "RightCtrl" => "Правый Ctrl",
        "LeftShift" => "Левый Shift",
        "RightShift" => "Правый Shift",
        "LeftAlt" => "Левый Alt",
        "RightAlt" => "Правый Alt",
        "LeftWin" => "Левый Win",
        "RightWin" => "Правый Win",
        _ => name,
    }
    .into()
}
pub fn preview(platform: Platform, candidate: Candidate) -> Vec<String> {
    if !candidate.modifier_only {
        return describe(platform, candidate).tokens;
    }
    if platform == Platform::MacOS {
        [(1, "⌃"), (4, "⌥"), (2, "⇧"), (8, "⌘")]
            .into_iter()
            .filter(|(bit, _)| candidate.modifiers & bit != 0)
            .map(|(_, label)| label.into())
            .collect()
    } else {
        [
            (
                1,
                if candidate.control_side == 2 {
                    "Правый Ctrl"
                } else {
                    "Левый Ctrl"
                },
            ),
            (2, "Shift"),
            (4, "Alt"),
            (8, "Win"),
        ]
        .into_iter()
        .filter(|(bit, _)| candidate.modifiers & bit != 0)
        .map(|(_, label)| label.into())
        .collect()
    }
}
pub fn describe(platform: Platform, mut candidate: Candidate) -> Gesture {
    let is_modifier = modifier(candidate.key, platform);
    let (valid, reserved, tokens, raw) = if platform == Platform::MacOS {
        let special = matches!(
            candidate.key,
            122 | 120
                | 99
                | 118
                | 96
                | 97
                | 98
                | 100
                | 101
                | 109
                | 103
                | 111
                | 105
                | 107
                | 113
                | 106
                | 64
                | 79
                | 80
                | 90
        );
        let valid = if candidate.modifier_only {
            is_modifier.is_some()
        } else {
            is_modifier.is_none() && (candidate.modifiers != 0 || special || candidate.key == 53)
        };
        let reserved =
            !candidate.modifier_only && candidate.key == 9 && candidate.modifiers & 8 != 0;
        let tokens = if candidate.modifier_only {
            vec![mac_key(candidate.key)]
        } else {
            let mut value = preview(
                platform,
                Candidate {
                    modifier_only: true,
                    ..candidate
                },
            );
            let special_label =
                matches!(candidate.key, 36 | 48 | 49 | 51 | 53 | 123..=126) || special;
            value.push(if special_label {
                mac_key(candidate.key)
            } else {
                govorun_os_runtime::keyboard::key_label(candidate.key)
                    .unwrap_or_else(|| mac_key(candidate.key))
            });
            value
        };
        let flags =
            [(1, 18), (2, 17), (4, 19), (8, 20)]
                .into_iter()
                .fold(0u64, |value, (bit, shift)| {
                    if candidate.modifiers & bit != 0 {
                        value | (1 << shift)
                    } else {
                        value
                    }
                });
        (
            valid,
            reserved,
            tokens,
            json!({"keyCode":candidate.key,"modifiers":flags,"kind":if candidate.modifier_only{"modifierOnly"}else{"key"}}),
        )
    } else {
        let name = windows_key(candidate.key);
        let valid = name.is_some()
            && if candidate.modifier_only {
                matches!(candidate.key, 0xA0..=0xA5 | 0x5B | 0x5C)
            } else {
                is_modifier.is_none()
                    && (candidate.modifiers != 0
                        || matches!(candidate.key, 0x70..=0x7B)
                        || candidate.key == 0x1B && candidate.modifiers == 0)
            };
        if candidate.modifier_only {
            candidate.modifiers = is_modifier.unwrap_or(0);
            candidate.control_side = if candidate.key == 0xA2 {
                1
            } else if candidate.key == 0xA3 {
                2
            } else {
                0
            };
        }
        let name = name.unwrap_or_else(|| format!("Key {}", candidate.key));
        let mut parts = Vec::<String>::new();
        if !candidate.modifier_only {
            for (bit, label) in [
                (
                    1,
                    match candidate.control_side {
                        1 => "LeftCtrl",
                        2 => "RightCtrl",
                        _ => "Ctrl",
                    },
                ),
                (2, "Shift"),
                (4, "Alt"),
                (8, "Win"),
            ] {
                if candidate.modifiers & bit != 0 {
                    parts.push(label.into());
                }
            }
        }
        parts.push(name);
        let raw = json!(parts.join("+"));
        let tokens = parts.iter().map(|value| win_display(value)).collect();
        (
            valid,
            candidate.key == 0x56 && candidate.modifiers == 1,
            tokens,
            raw,
        )
    };
    Gesture {
        key: candidate.key,
        modifiers: candidate.modifiers,
        modifier_only: candidate.modifier_only,
        control_side: candidate.control_side,
        reserved_for_paste: reserved,
        valid,
        tokens,
        raw,
    }
}
