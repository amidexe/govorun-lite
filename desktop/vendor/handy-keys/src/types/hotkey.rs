//! Hotkey definitions and related types

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

use super::key::Key;
use super::modifiers::Modifiers;

/// A unique identifier for a registered hotkey
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HotkeyId(pub(crate) u32);

impl HotkeyId {
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// A hotkey definition - either a key with modifiers, or modifiers only
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: Option<Key>,
}

impl Hotkey {
    /// Create a hotkey with modifiers and/or a key
    ///
    /// At least one of modifiers or key must be provided.
    /// Returns an error if both are empty/None.
    ///
    /// # Examples
    /// ```
    /// use handy_keys::{Hotkey, Modifiers, Key};
    ///
    /// // With modifiers and key
    /// let hotkey = Hotkey::new(Modifiers::CMD | Modifiers::SHIFT, Key::K).unwrap();
    ///
    /// // Modifier-only
    /// let hotkey = Hotkey::new(Modifiers::CMD | Modifiers::SHIFT, None).unwrap();
    ///
    /// // Key-only
    /// let hotkey = Hotkey::new(Modifiers::empty(), Key::F1).unwrap();
    /// ```
    pub fn new(modifiers: Modifiers, key: impl Into<Option<Key>>) -> Result<Self> {
        let key = key.into();
        if modifiers.is_empty() && key.is_none() {
            return Err(Error::EmptyHotkey);
        }
        Ok(Self { modifiers, key })
    }

    /// Format hotkey as lowercase string (e.g., "cmd+shift+k")
    ///
    /// This is useful for compatibility with systems that expect lowercase
    /// key names.
    pub fn to_lowercase_string(&self) -> String {
        self.to_string().to_lowercase()
    }

    /// Format hotkey using Handy-compatible key names (lowercase with full modifier names)
    ///
    /// Uses platform-appropriate naming:
    /// - macOS: "command", "option", "ctrl", "shift"
    /// - Windows/Linux: "ctrl", "alt", "super", "shift"
    ///
    /// Side-specific modifiers use `_left`/`_right` suffixes.
    pub fn to_handy_string(&self) -> String {
        #[cfg(target_os = "macos")]
        fn mod_names(
            mods: Modifiers,
            left: Modifiers,
            right: Modifiers,
            compound: Modifiers,
            name: &str,
        ) -> Option<String> {
            if mods.contains(compound) {
                Some(name.to_string())
            } else if mods.contains(left) {
                Some(format!("{}_left", name))
            } else if mods.contains(right) {
                Some(format!("{}_right", name))
            } else {
                None
            }
        }

        #[cfg(not(target_os = "macos"))]
        fn mod_names(
            mods: Modifiers,
            left: Modifiers,
            right: Modifiers,
            compound: Modifiers,
            name: &str,
        ) -> Option<String> {
            if mods.contains(compound) {
                Some(name.to_string())
            } else if mods.contains(left) {
                Some(format!("{}_left", name))
            } else if mods.contains(right) {
                Some(format!("{}_right", name))
            } else {
                None
            }
        }

        let mut parts = Vec::new();

        // Ctrl
        if let Some(s) = mod_names(
            self.modifiers,
            Modifiers::CTRL_LEFT,
            Modifiers::CTRL_RIGHT,
            Modifiers::CTRL,
            "ctrl",
        ) {
            parts.push(s);
        }

        // Opt/Alt
        #[cfg(target_os = "macos")]
        let opt_name = "option";
        #[cfg(not(target_os = "macos"))]
        let opt_name = "alt";
        if let Some(s) = mod_names(
            self.modifiers,
            Modifiers::OPT_LEFT,
            Modifiers::OPT_RIGHT,
            Modifiers::OPT,
            opt_name,
        ) {
            parts.push(s);
        }

        // Shift
        if let Some(s) = mod_names(
            self.modifiers,
            Modifiers::SHIFT_LEFT,
            Modifiers::SHIFT_RIGHT,
            Modifiers::SHIFT,
            "shift",
        ) {
            parts.push(s);
        }

        // Cmd/Super
        #[cfg(target_os = "macos")]
        let cmd_name = "command";
        #[cfg(not(target_os = "macos"))]
        let cmd_name = "super";
        if let Some(s) = mod_names(
            self.modifiers,
            Modifiers::CMD_LEFT,
            Modifiers::CMD_RIGHT,
            Modifiers::CMD,
            cmd_name,
        ) {
            parts.push(s);
        }

        // Fn (macOS only)
        #[cfg(target_os = "macos")]
        if self.modifiers.contains(Modifiers::FN) {
            parts.push("fn".to_string());
        }

        if let Some(key) = &self.key {
            let key_str = key.to_string().to_lowercase();
            let mut result = parts.join("+");
            if !result.is_empty() {
                result.push('+');
            }
            result.push_str(&key_str);
            result
        } else {
            parts.join("+")
        }
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            if let Some(key) = &self.key {
                write!(f, "{}", key)
            } else {
                write!(f, "(none)")
            }
        } else if let Some(key) = &self.key {
            write!(f, "{}+{}", self.modifiers, key)
        } else {
            write!(f, "{}", self.modifiers)
        }
    }
}

impl FromStr for Hotkey {
    type Err = Error;

    /// Parse a hotkey from a string like "Cmd+Shift+K" or "Ctrl+Space"
    ///
    /// # Examples
    /// ```
    /// use handy_keys::Hotkey;
    ///
    /// let hotkey: Hotkey = "Cmd+Shift+K".parse().unwrap();
    /// let hotkey: Hotkey = "Ctrl+Alt+Delete".parse().unwrap();
    /// let hotkey: Hotkey = "KeypadPlus".parse().unwrap();
    /// let hotkey: Hotkey = "F1".parse().unwrap();  // Key only
    /// let hotkey: Hotkey = "Cmd+Shift".parse().unwrap();  // Modifiers only
    /// ```
    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(Error::EmptyHotkey);
        }

        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        let mut modifiers = Modifiers::empty();
        let mut key: Option<Key> = None;

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Try to parse as modifier first
            if let Some(m) = Modifiers::parse_single(part) {
                modifiers |= m;
            } else {
                // Not a modifier, must be a key
                if key.is_some() {
                    return Err(Error::InvalidHotkeyFormat(format!(
                        "Multiple keys specified: already have a key, found '{}'",
                        part
                    )));
                }
                key = Some(Key::from_str(part)?);
            }
        }

        Hotkey::new(modifiers, key)
    }
}

/// The state of a hotkey (pressed or released)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyState {
    /// The hotkey was just pressed
    Pressed,
    /// The hotkey was just released
    Released,
}

/// Event emitted when a hotkey is pressed or released
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HotkeyEvent {
    pub id: HotkeyId,
    pub state: HotkeyState,
}

/// Event emitted during key recording
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeyEvent {
    pub modifiers: Modifiers,
    pub key: Option<Key>,
    pub is_key_down: bool,
    /// For modifier-only events (FlagsChanged), indicates which modifier changed.
    /// `None` for regular key events.
    pub changed_modifier: Option<Modifiers>,
}

impl KeyEvent {
    /// Convert this key event to a hotkey definition
    pub fn as_hotkey(&self) -> Result<Hotkey> {
        Hotkey::new(self.modifiers, self.key)
    }
}

