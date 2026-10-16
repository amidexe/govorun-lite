//! Keyboard listener for streaming raw key events
//!
//! This module provides a `KeyboardListener` that streams all keyboard events,
//! useful for implementing "record hotkey" UI flows.
//!
//! # Platform Notes
//!
//! - **macOS**: Uses CGEventTap. Requires accessibility permissions.
//! - **Windows**: Uses low-level keyboard hooks. Clean thread shutdown.
//! - **Linux**: Reads evdev devices directly (Wayland, X11, and console
//!   alike). Requires read access to `/dev/input` (`input` group).
//!   Blocking grabs keyboards exclusively and re-injects non-blocked
//!   events through uinput, so it additionally requires write access to
//!   `/dev/uinput`. Clean thread shutdown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::types::KeyEvent;

pub use crate::platform::state::BlockingHotkeys;

/// Platform-agnostic Keyboard Listener
///
/// Streams all keyboard events. Can optionally block events that match
/// registered hotkeys.
pub struct KeyboardListener {
    event_receiver: Receiver<KeyEvent>,
    _thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    blocking_hotkeys: Option<BlockingHotkeys>,
    /// Backend-specific wakeup invoked on Drop after `running` is cleared.
    /// macOS parks its listener thread in a CFRunLoop with no periodic
    /// wakeups (see run_event_tap), so Drop must stop that loop explicitly.
    stop_wakeup: Option<Box<dyn Fn() + Send>>,
}

impl KeyboardListener {
    /// Create a new KeyboardListener (non-blocking mode)
    ///
    /// Events are observed but not blocked. Use this for "record hotkey" UI flows.
    ///
    /// On macOS, this will check for accessibility permissions and fail if not granted.
    pub fn new() -> Result<Self> {
        Self::new_internal(None, None, None)
    }

    /// Create a new KeyboardListener with blocking support
    ///
    /// Events matching hotkeys in the provided set will be blocked from reaching
    /// other applications. The set can be modified after creation to add/remove
    /// hotkeys dynamically.
    ///
    /// Note: On Linux this grabs keyboards exclusively and re-injects
    /// non-blocked events through uinput, so it requires write access to
    /// `/dev/uinput` (and fails with an actionable error without it).
    /// Mouse-button hotkeys are detected but not blocked — pointer devices
    /// are never grabbed (same behavior as Windows).
    pub fn new_with_blocking(blocking_hotkeys: BlockingHotkeys) -> Result<Self> {
        Self::new_internal(Some(blocking_hotkeys), None, None)
    }

    /// Ignore only events injected with the caller's exact source tag.
    /// Hardware input and unrelated injected input remain unchanged.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn new_with_blocking_ignoring_source_tag(blocking_hotkeys: BlockingHotkeys, tag: usize) -> Result<Self> {
        Self::new_internal(Some(blocking_hotkeys), Some(tag), None)
    }

    /// Notify the host when the OS listener recovers after sleep/session/tap loss.
    /// The host retains ownership of cancellation; this library owns OS recovery.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn new_with_blocking_hooks(blocking_hotkeys: BlockingHotkeys, tag: usize, recovery: Arc<dyn Fn() + Send + Sync>) -> Result<Self> {
        Self::new_internal(Some(blocking_hotkeys), Some(tag), Some(recovery))
    }

    fn new_internal(blocking_hotkeys: Option<BlockingHotkeys>, ignored_source_tag: Option<usize>, recovery: Option<Arc<dyn Fn() + Send + Sync>>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use crate::platform::macos::listener;
            let state = listener::spawn(blocking_hotkeys, ignored_source_tag, recovery)?;
            let run_loop = state.run_loop;
            Ok(KeyboardListener {
                event_receiver: state.event_receiver,
                _thread_handle: state.thread_handle,
                running: state.running,
                blocking_hotkeys: state.blocking_hotkeys,
                stop_wakeup: Some(Box::new(move || run_loop.stop())),
            })
        }

        #[cfg(target_os = "windows")]
        {
            use crate::platform::windows::listener;
            let state = listener::spawn(blocking_hotkeys, ignored_source_tag, recovery)?;
            Ok(KeyboardListener {
                event_receiver: state.event_receiver,
                _thread_handle: state.thread_handle,
                running: state.running,
                blocking_hotkeys: state.blocking_hotkeys,
                stop_wakeup: None,
            })
        }

        #[cfg(target_os = "linux")]
        {
            use crate::platform::linux::listener;
            let state = listener::spawn(blocking_hotkeys)?;
            Ok(KeyboardListener {
                event_receiver: state.event_receiver,
                _thread_handle: state.thread_handle,
                running: state.running,
                blocking_hotkeys: state.blocking_hotkeys,
                stop_wakeup: None,
            })
        }
    }

    /// Get a reference to the blocking hotkeys set (if blocking is enabled)
    pub fn blocking_hotkeys(&self) -> Option<&BlockingHotkeys> {
        self.blocking_hotkeys.as_ref()
    }

    /// Blocking receive for key events
    ///
    /// Blocks until a key event is received or the listener stops.
    pub fn recv(&self) -> Result<KeyEvent> {
        self.event_receiver
            .recv()
            .map_err(|_| Error::EventLoopNotRunning)
    }

    /// Blocking receive with timeout
    ///
    /// Blocks until a key event is received, the timeout expires, or the listener stops.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<KeyEvent> {
        self.event_receiver
            .recv_timeout(timeout)
            .map_err(|e| match e {
                std::sync::mpsc::RecvTimeoutError::Timeout => Error::Timeout,
                std::sync::mpsc::RecvTimeoutError::Disconnected => Error::EventLoopNotRunning,
            })
    }

    /// Non-blocking receive for key events
    ///
    /// Returns `Some(event)` if an event is available, `None` otherwise.
    pub fn try_recv(&self) -> Option<KeyEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

impl Drop for KeyboardListener {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        // Wake the listener thread if the backend parks it (macOS stops the
        // tap thread's CFRunLoop). Windows/Linux event loops re-check
        // `running` at least every ~100ms on their own, so the join below is
        // short and shutdown is clean on all platforms.
        if let Some(wake) = &self.stop_wakeup {
            wake();
        }
        if let Some(handle) = self._thread_handle.take() {
            let _ = handle.join();
        }
    }
}
