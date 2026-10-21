use super::{helper, macos_clipboard as clipboard, Port};
use objc2_app_kit::NSWorkspace;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
};
use std::{path::PathBuf, thread, time::Duration};
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}
#[derive(Clone, Copy)]
pub struct Target(i32);
impl Target {
    pub fn capture() -> Self {
        Self(
            NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .map_or(0, |app| app.processIdentifier()),
        )
    }
    pub fn application(&self) -> Option<String> {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        if app.processIdentifier() != self.0 {
            return None;
        }
        app.bundleIdentifier().map(|id| id.to_string())
    }
}
pub struct Mac {
    name: String,
    executable: PathBuf,
}
impl Mac {
    pub fn new() -> Result<Self, &'static str> {
        Self::with_pasteboard(
            objc2_app_kit::NSPasteboard::generalPasteboard()
                .name()
                .to_string(),
        )
    }
    /// Named boards are used by private fixtures; the ordinary host uses only
    /// the general board. No board contents or name enter public projections.
    pub fn with_pasteboard(name: String) -> Result<Self, &'static str> {
        Ok(Self {
            name,
            executable: std::env::current_exe().map_err(|_| "clipboard_helper_unavailable")?,
        })
    }
}
pub struct Keys(Vec<CFRetained<CGEvent>>);
fn events() -> Result<Keys, &'static str> {
    let source =
        CGEventSource::new(CGEventSourceStateID::Private).ok_or("paste_keys_unavailable")?;
    let base = 0x2000_0100;
    let command = base | (1 << 20) | 0x8;
    let mut events = Vec::new();
    for (code, down, flags) in [
        (0x37, true, command),
        (0x09, true, command),
        (0x09, false, command),
        (0x37, false, base),
    ] {
        let event = CGEvent::new_keyboard_event(Some(&source), code, down)
            .ok_or("paste_keys_unavailable")?;
        CGEvent::set_flags(Some(&event), CGEventFlags(flags));
        CGEvent::set_integer_value_field(
            Some(&event),
            CGEventField::EventSourceUserData,
            crate::PASTE_EVENT_TAG as i64,
        );
        events.push(event);
    }
    Ok(Keys(events))
}

impl Port for Mac {
    type Snapshot = clipboard::Saved;
    type Target = Target;
    type Keys = Keys;
    type Revision = i64;
    fn capture(&mut self) -> Result<Self::Snapshot, &'static str> {
        let original = clipboard::revision(&self.name);
        let saved = helper::read(&self.executable, &self.name).or_else(|_| {
            // As in Handy, a failed old-content read must not reject dictation.
            // Keep the original revision so a newer user copy is still protected.
            super::snapshot::Snapshot::from_parts(original, true, Vec::new())
        })?;
        clipboard::prepare(saved)
    }
    fn keys(&mut self) -> Result<Keys, &'static str> {
        if unsafe { AXIsProcessTrusted() } == 0 {
            return Err("insertion_permission_required");
        }
        events()
    }
    fn write(&mut self, saved: &Self::Snapshot, text: &str) -> Result<i64, &'static str> {
        clipboard::set_text(&self.name, saved, text)
    }
    fn revision(&self) -> i64 {
        clipboard::revision(&self.name)
    }
    fn admissible(&self, target: &Target) -> bool {
        target.0 != 0 && Target::capture().0 == target.0
    }
    fn target(&mut self, target: &Target, cancelled: &dyn Fn() -> bool) -> bool {
        if target.0 == 0 || cancelled() {
            return false;
        }
        // Chrome pages (Gemini's Quill editor) often drop a paste sent right
        // after a fresh pasteboard write; Handy waits 60 ms here as well.
        thread::sleep(Duration::from_millis(60));
        !cancelled() && Target::capture().0 == target.0
    }
    fn press(&mut self, keys: &Keys) -> bool {
        for event in &keys.0[..3] {
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
        }
        true
    }
    fn release(&mut self, keys: &Keys, _: bool) -> bool {
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&keys.0[3]));
        true
    }
    fn restore(&mut self, saved: &Self::Snapshot, expected: i64) -> Result<bool, &'static str> {
        clipboard::restore(&self.name, saved, expected)
    }
    fn wait(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Invoke before Tauri, single-instance handling, accounts and device startup.
/// Only this child reads lazy SDK representations, on its own main thread.
pub fn run_helper(arguments: &[String]) -> Option<i32> {
    if arguments.get(1).map(String::as_str) != Some("--clipboard-snapshot") {
        return None;
    }
    if arguments.len() != 3
        || arguments[2].is_empty()
        || arguments[2].len() > 4096
        || objc2::MainThreadMarker::new().is_none()
    {
        return Some(2);
    }
    use std::io::Write;
    Some(objc2::rc::autoreleasepool(|_| {
        let Ok(snapshot) = clipboard::capture(&arguments[2]) else {
            return 2;
        };
        let Ok(bytes) = snapshot.encode() else {
            return 2;
        };
        if std::io::stdout().lock().write_all(&bytes).is_ok() {
            0
        } else {
            2
        }
    }))
}

#[cfg(feature = "private-fixtures")]
pub fn owned_key_event_facts() -> Result<Vec<(i64, u32, u64, i64, i64)>, &'static str> {
    Ok(events()?
        .0
        .iter()
        .map(|event| {
            (
                CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode),
                CGEvent::r#type(Some(event)).0,
                CGEvent::flags(Some(event)).0,
                CGEvent::integer_value_field(Some(event), CGEventField::EventSourceStateID),
                CGEvent::integer_value_field(Some(event), CGEventField::EventSourceUserData),
            )
        })
        .collect())
}
