//! Physical key decoding and pairing. Product assignment conflicts, recording
//! modes and the tap/hold threshold stay in Core, outside the OS adapter.
pub mod assignment;
mod focus;
mod hook;
pub use focus::permission_ready;
#[cfg(not(target_os = "macos"))]
pub use focus::host_focused;
mod keycodes;
mod labels;
pub use labels::key_label;
mod pairing;
mod tracker;
use handy_keys::{Key, Modifiers};
pub use hook::Hook;
pub use pairing::{Packet, Pairing};
pub use tracker::{Action, Event, Target, Tracker};

/// A Windows key's name as Handy writes it («ContextMenu», «F13», «PageUp»), for settings documents.
pub fn windows_key_name(code: u16) -> Option<String> {
    keycodes::key(Platform::Windows, code).map(|key| key.to_string())
}

/// The Windows key a name stands for, read the way Handy reads it («menu», «apps», «f24»).
pub fn windows_key_code(name: &str) -> Option<u16> {
    let key = name.parse::<Key>().ok()?;
    (0..=0xFEu16).find(|&code| keycodes::key(Platform::Windows, code) == Some(key))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Windows,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub modifiers: Modifiers,
    pub key: Option<Key>,
    pub side: Option<Modifiers>,
}
fn groups(mask: u8) -> Result<Modifiers, &'static str> {
    if mask > 15 {
        return Err("hotkeys_invalid_binding");
    }
    let mut value = Modifiers::empty();
    for (bit, group) in [
        (1, Modifiers::CTRL),
        (2, Modifiers::SHIFT),
        (4, Modifiers::OPT),
        (8, Modifiers::CMD),
    ] {
        if mask & bit != 0 {
            value |= group;
        }
    }
    Ok(value)
}
impl Binding {
    /// Decodes an already selected Core gesture. This neither writes settings
    /// nor replaces Core validation/overlap rules.
    pub fn decode(
        platform: Platform,
        key: u16,
        mask: u8,
        modifier_only: bool,
        control_side: u8,
    ) -> Result<Self, &'static str> {
        let mut modifiers = groups(mask)?;
        let side = if modifier_only {
            Some(keycodes::modifier(platform, key).ok_or("hotkeys_invalid_binding")?)
        } else {
            None
        };
        if let Some(side) = side {
            let group = [
                Modifiers::CTRL,
                Modifiers::SHIFT,
                Modifiers::OPT,
                Modifiers::CMD,
            ]
            .into_iter()
            .find(|group| group.intersects(side))
            .unwrap_or(Modifiers::FN);
            modifiers.remove(group);
            modifiers |= side;
        }
        if platform == Platform::Windows && control_side != 0 {
            if mask & 1 == 0 || control_side > 2 {
                return Err("hotkeys_invalid_binding");
            }
            modifiers.remove(Modifiers::CTRL);
            modifiers |= if control_side == 1 {
                Modifiers::CTRL_LEFT
            } else {
                Modifiers::CTRL_RIGHT
            };
        } else if control_side != 0 {
            return Err("hotkeys_invalid_binding");
        }
        let key = if modifier_only {
            None
        } else {
            Some(keycodes::key(platform, key).ok_or("hotkeys_invalid_binding")?)
        };
        Ok(Self {
            modifiers,
            key,
            side,
        })
    }
    fn matches(self, mut modifiers: Modifiers) -> bool {
        // Existing macOS chords ignore SecondaryFn, including hardware F keys.
        if !self.modifiers.contains(Modifiers::FN) {
            modifiers.remove(Modifiers::FN);
        }
        self.modifiers.matches(modifiers)
    }
}
