//! One clipboard transaction, serialized through restoration. Raw clipboard
//! representations and text never leave these local ports or enter diagnostics.
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::Mutex,
    time::Duration,
};
pub mod helper;
#[cfg(all(feature = "insertion", target_os = "macos"))]
mod macos;
#[cfg(all(feature = "insertion", target_os = "macos"))]
mod macos_clipboard;
pub mod snapshot;
#[cfg(all(feature = "insertion", target_os = "macos"))]
pub use macos::{run_helper, Mac, Target};
#[cfg(all(feature = "insertion", feature = "private-fixtures", target_os = "macos"))]
pub use macos::owned_key_event_facts;

#[cfg(all(feature = "insertion", target_os = "windows"))]
mod windows;
#[cfg(all(feature = "insertion", target_os = "windows"))]
mod windows_clipboard;
#[cfg(all(feature = "insertion", target_os = "windows"))]
mod windows_keyboard;
#[cfg(all(feature = "insertion", target_os = "windows"))]
pub use windows::{Target, Windows};

pub trait Port {
    type Snapshot;
    type Target;
    type Keys;
    type Revision: Copy + Eq;
    fn capture(&mut self) -> Result<Self::Snapshot, &'static str>;
    fn keys(&mut self) -> Result<Self::Keys, &'static str>;
    /// Eagerly allocate replacement and recovery before mutation. A failed
    /// write must restore while holding the SDK lock, or return a closed error.
    fn write(&mut self, saved: &Self::Snapshot, text: &str)
        -> Result<Self::Revision, &'static str>;
    fn revision(&self) -> Self::Revision;
    fn admissible(&self, _target: &Self::Target) -> bool {
        true
    }
    /// Windows may restore the captured foreground target; macOS validates its
    /// current target. Never discover or read the target's document contents.
    fn target(&mut self, target: &Self::Target, cancelled: &dyn Fn() -> bool) -> bool;
    fn press(&mut self, keys: &Self::Keys) -> bool;
    /// Always called after press, including a partial send or a panic. Releases
    /// both physical keys when necessary; cancellation cannot skip this.
    fn release(&mut self, keys: &Self::Keys, pressed: bool) -> bool;
    fn restore(
        &mut self,
        saved: &Self::Snapshot,
        expected: Self::Revision,
    ) -> Result<bool, &'static str>;
    fn wait(&mut self, duration: Duration);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub sent: bool,
    pub restored: bool,
    pub failure: Option<&'static str>,
}
impl Outcome {
    fn failed(code: &'static str) -> Self {
        Self {
            sent: false,
            restored: false,
            failure: Some(code),
        }
    }
}
pub struct Inserter<P>(Mutex<P>);
impl<P: Port> Inserter<P> {
    pub fn new(port: P) -> Self {
        Self(Mutex::new(port))
    }
    /// Run on the host's blocking executor, never the event/hook or UI thread.
    /// The same owner must serve all attempts, including a cancelled old queue.
    pub fn paste(
        &self,
        text: &str,
        target: &P::Target,
        keep: bool,
        cancelled: &dyn Fn() -> bool,
    ) -> Outcome {
        if text.trim().is_empty() {
            return Outcome::failed("insertion_empty");
        }
        let Ok(mut port) = self.0.lock() else {
            return Outcome::failed("insertion_unavailable");
        };
        if cancelled() {
            return Outcome::failed("insertion_cancelled");
        }
        if !port.admissible(target) {
            return Outcome::failed("insertion_target_changed");
        }
        let saved = match port.capture() {
            Ok(value) => value,
            Err(code) => return Outcome::failed(code),
        };
        let keys = match port.keys() {
            Ok(value) => value,
            Err(code) => return Outcome::failed(code),
        };
        if cancelled() {
            return Outcome::failed("insertion_cancelled");
        }
        if !port.admissible(target) {
            return Outcome::failed("insertion_target_changed");
        }
        let revision = match port.write(&saved, text) {
            Ok(value) => value,
            Err(code) => return Outcome::failed(code),
        };
        let mut pressed = false;
        let mut touched_keys = false;
        let sent = catch_unwind(AssertUnwindSafe(|| {
            if cancelled() || port.revision() != revision || !port.target(target, cancelled) {
                return false;
            }
            // Foreground activation may await the SDK; check CAS again before
            // sending so a new user copy can never paste the wrong text.
            if cancelled() || port.revision() != revision {
                return false;
            }
            touched_keys = true;
            pressed = port.press(&keys);
            if pressed {
                port.wait(Duration::from_millis(100));
            }
            pressed
        }))
        .unwrap_or(false);
        let released = !touched_keys
            || catch_unwind(AssertUnwindSafe(|| port.release(&keys, pressed))).unwrap_or(false);
        let sent = sent && released;
        // Restoration survives cancellation and a failing physical send. An SDK
        // failure is visible even when the recipient might have consumed text.
        let cleanup = catch_unwind(AssertUnwindSafe(|| {
            port.wait(Duration::from_millis(60));
            if keep && sent {
                Ok(false)
            } else {
                port.restore(&saved, revision)
            }
        }))
        .unwrap_or(Err("clipboard_restore_failed"));
        match cleanup {
            Ok(restored) => Outcome {
                sent,
                restored,
                failure: if sent {
                    None
                } else {
                    Some(if cancelled() {
                        "insertion_cancelled"
                    } else {
                        "insertion_failed"
                    })
                },
            },
            Err(code) => Outcome {
                sent,
                restored: false,
                failure: Some(code),
            },
        }
    }
}
