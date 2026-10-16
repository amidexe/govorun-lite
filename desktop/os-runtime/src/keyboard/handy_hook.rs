//! Handy owns OS hooks, modifier reconciliation and the Windows Start-menu mask.
//! This adapter only routes its events to the existing Core/capture owners.
use super::{
    super::{keycodes, Platform, Target},
    Context,
};
use handy_keys::{Hotkey, KeyEvent, KeyboardListener, Modifiers};
use std::{
    collections::HashSet,
    sync::{atomic::Ordering, mpsc::SyncSender, Arc},
    time::Duration,
};

fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Windows
    }
}
fn code(raw: KeyEvent) -> Option<u16> {
    if let Some(side) = raw.changed_modifier {
        return (0..=0xFE).find(|&code| keycodes::modifier(platform(), code) == Some(side));
    }
    (0..=0xFE).find(|&code| raw.key.is_some() && keycodes::key(platform(), code) == raw.key)
}

pub(super) fn synchronize(context: &Context) {
    let Ok(state) = context.state.lock() else {
        return;
    };
    let mut keys = HashSet::new();
    let assigning = state.capture.active()
        && state
            .capture_ports
            .as_ref()
            .is_some_and(|ports| (ports.focus)());
    if assigning {
        // Only while the existing assignment window has focus. Handy performs
        // the actual suppression and Win/Alt menu masking; no second OS hook.
        for mask in 0..=if cfg!(target_os = "macos") { 31 } else { 15 } {
            let mut modifiers = Modifiers::empty();
            for (bit, group) in [
                (1, Modifiers::CTRL),
                (2, Modifiers::SHIFT),
                (4, Modifiers::OPT),
                (8, Modifiers::CMD),
                (16, Modifiers::FN),
            ] {
                if mask & bit != 0 {
                    modifiers |= group;
                }
            }
            if let Ok(key) = Hotkey::new(modifiers, None) {
                keys.insert(key);
            }
            for code in 0..=0xFE {
                if let Some(key) = keycodes::key(platform(), code) {
                    keys.insert(Hotkey {
                        modifiers,
                        key: Some(key),
                    });
                }
            }
        }
    } else if !state.capture.active() {
        for (target, binding) in &state.bindings {
            if *target != Target::Cancel || context.cancel_enabled.load(Ordering::Acquire) {
                keys.insert(Hotkey {
                    modifiers: binding.modifiers,
                    key: binding.key,
                });
                if platform() == Platform::MacOS && !binding.modifiers.contains(Modifiers::FN) {
                    keys.insert(Hotkey {
                        modifiers: binding.modifiers | Modifiers::FN,
                        key: binding.key,
                    });
                }
            }
        }
    }
    if let Ok(mut current) = context.blocking.lock() {
        if *current != keys {
            *current = keys;
        }
    }
}
pub(super) fn run(context: Arc<Context>, ready: SyncSender<Result<(), &'static str>>) {
    synchronize(&context);
    let recovery = context.clone();
    let listener = match KeyboardListener::new_with_blocking_hooks(
        context.blocking.clone(),
        crate::PASTE_EVENT_TAG,
        Arc::new(move || recovery.recover()),
    ) {
        Ok(listener) => listener,
        Err(_) => {
            let _ = ready.send(Err("hotkeys_unavailable"));
            return;
        }
    };
    if ready.send(Ok(())).is_err() {
        return;
    }
    let mut was_focused = false;
    while context.running.load(Ordering::Acquire) {
        // Focus loss ends assignment suppression even when no new key arrives.
        let focused = context.state.lock().is_ok_and(|state| {
            state.capture.active()
                && state
                    .capture_ports
                    .as_ref()
                    .is_some_and(|ports| (ports.focus)())
        });
        if focused != was_focused {
            synchronize(&context);
            was_focused = focused;
        }
        match listener.recv_timeout(Duration::from_millis(5)) {
            Ok(raw) => {
                if let Some(code) = code(raw) {
                    context.route(code, raw);
                }
            }
            Err(handy_keys::Error::Timeout) => {}
            Err(_) => break,
        }
    }
    context.recover();
}
