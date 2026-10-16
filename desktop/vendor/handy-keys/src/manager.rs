//! Platform-agnostic hotkey manager built on top of KeyboardListener

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::error::{Error, Result};
use crate::listener::{BlockingHotkeys, KeyboardListener};
use crate::types::{Hotkey, HotkeyEvent, HotkeyId, HotkeyState, KeyEvent};

/// Internal state shared between the manager and the processing thread
struct ManagerState {
    hotkeys: HashMap<HotkeyId, Hotkey>,
    next_id: u32,
    /// Track which hotkeys are currently pressed
    pressed_hotkeys: HashSet<HotkeyId>,
}

impl ManagerState {
    fn new() -> Self {
        Self {
            hotkeys: HashMap::new(),
            next_id: 0,
            pressed_hotkeys: HashSet::new(),
        }
    }

    /// Process a key event and return any matching hotkey events
    fn process_event(&mut self, event: &KeyEvent) -> Vec<HotkeyEvent> {
        let mut results = Vec::new();

        if event.is_key_down {
            // Check for hotkeys that should be pressed
            let to_press: Vec<HotkeyId> = self
                .hotkeys
                .iter()
                .filter(|(&id, hotkey)| {
                    hotkey.modifiers.matches(event.modifiers)
                        && hotkey.key == event.key
                        && !self.pressed_hotkeys.contains(&id)
                })
                .map(|(&id, _)| id)
                .collect();

            for id in to_press {
                self.pressed_hotkeys.insert(id);
                results.push(HotkeyEvent {
                    id,
                    state: HotkeyState::Pressed,
                });
            }
        } else {
            // Check for hotkeys that should be released
            // A hotkey is released when its key is released, or — for modifier
            // events — when the modifiers no longer match. A modifier event
            // (key == None) whose modifiers still match must not release a
            // modifier-only hotkey (e.g. tapping Shift while a Cmd-only hotkey
            // is held).
            let to_release: Vec<HotkeyId> = self
                .hotkeys
                .iter()
                .filter(|(&id, hotkey)| {
                    self.pressed_hotkeys.contains(&id)
                        && ((event.key.is_some() && hotkey.key == event.key)
                            || (event.key.is_none() && !hotkey.modifiers.matches(event.modifiers)))
                })
                .map(|(&id, _)| id)
                .collect();

            for id in to_release {
                self.pressed_hotkeys.remove(&id);
                results.push(HotkeyEvent {
                    id,
                    state: HotkeyState::Released,
                });
            }
        }

        results
    }
}

/// Platform-agnostic Hotkey Manager
///
/// This manager wraps a `KeyboardListener` and filters events against
/// registered hotkeys, emitting `HotkeyEvent`s when matches occur.
///
/// Registered hotkeys are blocked from reaching other applications.
/// Note: On Linux, blocking requires write access to `/dev/uinput`
/// (non-blocked keystrokes are re-injected through it).
pub struct HotkeyManager {
    state: Arc<Mutex<ManagerState>>,
    event_receiver: Receiver<HotkeyEvent>,
    _thread_handle: Option<JoinHandle<()>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Shared set of hotkeys to block
    blocking_hotkeys: Option<BlockingHotkeys>,
}

impl HotkeyManager {
    /// Create a new HotkeyManager (non-blocking mode)
    ///
    /// On macOS, this will check for accessibility permissions and fail if not granted.
    pub fn new() -> Result<Self> {
        let listener = KeyboardListener::new()?;

        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(ManagerState::new()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let thread_state = Arc::clone(&state);
        let thread_running = Arc::clone(&running);

        let handle = thread::spawn(move || {
            Self::event_loop(listener, thread_state, tx, thread_running);
        });

        Ok(Self {
            state,
            event_receiver: rx,
            _thread_handle: Some(handle),
            running,
            blocking_hotkeys: None,
        })
    }

    /// Create a new HotkeyManager with blocking support
    ///
    /// On macOS, this will check for accessibility permissions and fail if not granted.
    /// Registered hotkeys will be blocked from reaching other applications.
    ///
    /// Note: On Linux, blocking requires write access to `/dev/uinput`
    /// and this fails with an actionable error without it.
    pub fn new_with_blocking() -> Result<Self> {
        let blocking_hotkeys: BlockingHotkeys = Arc::new(Mutex::new(HashSet::new()));
        let listener = KeyboardListener::new_with_blocking(blocking_hotkeys.clone())?;

        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(ManagerState::new()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let thread_state = Arc::clone(&state);
        let thread_running = Arc::clone(&running);

        let handle = thread::spawn(move || {
            Self::event_loop(listener, thread_state, tx, thread_running);
        });

        Ok(Self {
            state,
            event_receiver: rx,
            _thread_handle: Some(handle),
            running,
            blocking_hotkeys: Some(blocking_hotkeys),
        })
    }

    /// Event processing loop
    fn event_loop(
        listener: KeyboardListener,
        state: Arc<Mutex<ManagerState>>,
        sender: Sender<HotkeyEvent>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        const RECV_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Block until we receive an event or timeout (to check running flag)
            match listener.recv_timeout(RECV_TIMEOUT) {
                Ok(key_event) => {
                    if let Ok(mut state) = state.lock() {
                        let hotkey_events = state.process_event(&key_event);
                        for event in hotkey_events {
                            if sender.send(event).is_err() {
                                // Receiver dropped, exit
                                return;
                            }
                        }
                    }
                }
                Err(crate::error::Error::Timeout) => {
                    // No event received, loop continues to check running flag
                }
                Err(_) => {
                    // Listener disconnected, exit
                    return;
                }
            }
        }
    }

    /// Register a hotkey and return its unique ID
    ///
    /// Returns an error if the hotkey is already registered.
    pub fn register(&self, hotkey: Hotkey) -> Result<HotkeyId> {
        let mut state = self.state.lock().map_err(|_| Error::MutexPoisoned)?;

        // Check if already registered
        for (id, existing) in &state.hotkeys {
            if existing == &hotkey {
                return Err(Error::HotkeyAlreadyRegistered(format!(
                    "{} (id: {:?})",
                    hotkey, id
                )));
            }
        }

        let id = HotkeyId(state.next_id);
        state.next_id += 1;
        state.hotkeys.insert(id, hotkey);

        // Add to blocking set
        if let Some(blocking_hotkeys) = &self.blocking_hotkeys {
            if let Ok(mut blocking) = blocking_hotkeys.lock() {
                blocking.insert(hotkey);
            }
        }

        Ok(id)
    }

    /// Unregister a hotkey by its ID
    ///
    /// Returns an error if the hotkey ID is not found.
    pub fn unregister(&self, id: HotkeyId) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| Error::MutexPoisoned)?;

        let hotkey = state.hotkeys.remove(&id);
        if hotkey.is_none() {
            return Err(Error::HotkeyNotFound(id));
        }

        // Remove from blocking set
        if let Some(blocking_hotkeys) = &self.blocking_hotkeys {
            if let Some(hotkey) = hotkey {
                if let Ok(mut blocking) = blocking_hotkeys.lock() {
                    blocking.remove(&hotkey);
                }
            }
        }

        Ok(())
    }

    /// Get the hotkey definition associated with an ID
    ///
    /// Returns `None` if the ID is not found.
    pub fn get_hotkey(&self, id: HotkeyId) -> Option<Hotkey> {
        let state = self.state.lock().ok()?;
        state.hotkeys.get(&id).copied()
    }

    /// Blocking receive for hotkey events
    ///
    /// Blocks until a hotkey event is received or the event loop stops.
    pub fn recv(&self) -> Result<HotkeyEvent> {
        self.event_receiver
            .recv()
            .map_err(|_| Error::EventLoopNotRunning)
    }

    /// Non-blocking receive for hotkey events
    ///
    /// Returns `Some(event)` if an event is available, `None` otherwise.
    pub fn try_recv(&self) -> Option<HotkeyEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Get the number of currently registered hotkeys
    pub fn hotkey_count(&self) -> usize {
        let state = if let Ok(s) = self.state.lock() {
            s
        } else {
            return 0;
        };
        state.hotkeys.len()
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        // Join the thread to ensure clean shutdown
        if let Some(handle) = self._thread_handle.take() {
            let _ = handle.join();
        }
    }
}

