//! Shared state for platform-specific keyboard listeners

use std::collections::HashSet;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::types::Hotkey;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::types::Key;
use crate::types::{KeyEvent, Modifiers};

/// Hotkeys that should be blocked when triggered
pub type BlockingHotkeys = Arc<Mutex<HashSet<Hotkey>>>;

/// Internal state shared with platform-specific event callbacks
///
/// Used by the macOS and Linux backends; the Windows backend keeps its
/// state in a thread-local `HookContext` instead.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub struct ListenerState {
    pub event_sender: Sender<KeyEvent>,
    /// Track which modifiers are currently held
    pub current_modifiers: Modifiers,
    /// Hotkeys to block (if any)
    pub blocking_hotkeys: Option<BlockingHotkeys>,
    pub ignored_source_tag: Option<usize>,
    pub recovery: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl ListenerState {
    pub fn new(event_sender: Sender<KeyEvent>, blocking_hotkeys: Option<BlockingHotkeys>, ignored_source_tag: Option<usize>, recovery: Option<Arc<dyn Fn() + Send + Sync>>) -> Self {
        Self {
            event_sender,
            current_modifiers: Modifiers::empty(),
            blocking_hotkeys,
            ignored_source_tag,
            recovery,
        }
    }

    /// Check if an event matches a blocking hotkey
    pub fn should_block(&self, modifiers: Modifiers, key: Option<Key>) -> bool {
        if let Some(ref hotkeys) = self.blocking_hotkeys {
            if let Ok(set) = hotkeys.lock() {
                return set
                    .iter()
                    .any(|h| h.modifiers.matches(modifiers) && h.key == key);
            }
        }
        false
    }
}

/// Canonical order in which synthetic modifier release events are emitted
/// by reconciliation, shared by the Windows and Linux listeners so both
/// backends recover from a stuck modifier with the same event sequence.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub const MODIFIER_RELEASE_ORDER: [Modifiers; 8] = [
    Modifiers::CMD_LEFT,
    Modifiers::CMD_RIGHT,
    Modifiers::SHIFT_LEFT,
    Modifiers::SHIFT_RIGHT,
    Modifiers::CTRL_LEFT,
    Modifiers::CTRL_RIGHT,
    Modifiers::OPT_LEFT,
    Modifiers::OPT_RIGHT,
];

/// Every modifier bit reconciliation may touch. FN is excluded: neither
/// Windows nor Linux evdev ever reports the laptop Fn key (it is handled
/// in firmware), so reconciliation must not clear it.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub const RECONCILABLE: Modifiers = Modifiers::CMD
    .union(Modifiers::SHIFT)
    .union(Modifiers::CTRL)
    .union(Modifiers::OPT);

/// Modifiers we track as held that are no longer physically held.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn stale_modifiers(tracked: Modifiers, physical: Modifiers) -> Modifiers {
    (tracked & RECONCILABLE) & !physical
}

/// Build the synthetic release events that clear `stale` from `tracked`, in
/// MODIFIER_RELEASE_ORDER. Each event carries the modifier set as it
/// shrinks, exactly as if the keys had been released one by one.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn release_events(tracked: Modifiers, stale: Modifiers) -> Vec<KeyEvent> {
    let mut modifiers = tracked;
    let mut events = Vec::new();
    for modifier in MODIFIER_RELEASE_ORDER {
        if stale.contains(modifier) {
            modifiers &= !modifier;
            events.push(KeyEvent {
                modifiers,
                key: None,
                is_key_down: false,
                changed_modifier: Some(modifier),
            });
        }
    }
    events
}

