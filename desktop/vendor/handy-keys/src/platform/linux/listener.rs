//! Linux keyboard listener reading evdev devices directly
//!
//! A single thread polls every keyboard-capable `/dev/input/event*` device
//! plus an inotify watch on `/dev/input` for hotplug. Reading evdev
//! directly needs no X11 or Wayland connection, so the listener works
//! identically on both and on a bare console; it requires read access to
//! the device nodes (typically membership in the `input` group).
//!
//! # Blocking
//!
//! Without blocking, devices are opened non-exclusively — nothing is
//! grabbed, nothing is injected. With blocking enabled, every *keyboard*
//! is grabbed exclusively (EVIOCGRAB) and its events re-injected through a
//! per-device uinput clone, minus the key events that match a blocking
//! hotkey. Pointer devices are never grabbed (their buttons are observed
//! read-only, so mouse-button hotkeys are detected but not blocked —
//! matching the Windows backend). Blocking therefore additionally needs
//! write access to `/dev/uinput`, verified at spawn.
//!
//! Each listener tracks the device nodes of its own uinput clones and
//! filters exactly those from its enumeration (reading one's own output
//! would loop). Other listeners — including in other processes — read the
//! clones like ordinary keyboards, so a listener stacked behind a blocking
//! one still sees every non-blocked event. Blocking listeners, however,
//! never *grab* each other's clones (recognized by name prefix): two
//! blockers cloning each other's clones would mint new uinput devices
//! without bound. First blocker wins; later ones detect but cannot block
//! what it already grabbed.
//!
//! # Modifier-state recovery
//!
//! Tracked modifiers are seeded from the kernel's key state (EVIOCGKEY) at
//! startup and re-reconciled against it whenever the kernel reports a
//! dropped-buffer overflow (`SYN_DROPPED`) or a device disappears while
//! keys are held. Stale modifiers are cleared with synthetic release
//! events, sharing `release_events` with the Windows listener.
//!
//! # Shutdown
//!
//! Dropping the listener flips the shared `running` flag; the poll loop
//! wakes at least every `POLL_TIMEOUT_MS`, observes it, and exits. Closing
//! the device fds releases every grab, and dropping the uinput clones
//! destroys them.

use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use evdev::raw_stream::RawDevice;
use evdev::uinput::VirtualDevice;
use evdev::{
    AbsoluteAxisCode, EventSummary, EventType, InputEvent, KeyCode, RelativeAxisCode,
    SynchronizationCode,
};

use crate::error::{Error, Result};
use crate::platform::state::{release_events, stale_modifiers, RECONCILABLE};
use crate::platform::state::{BlockingHotkeys, ListenerState};
use crate::types::{Key, KeyEvent, Modifiers};

use super::keycode::{key_code_to_key, key_code_to_modifier};

const DEV_INPUT: &str = "/dev/input";

/// Upper bound on how long shutdown and hotplug pickup can lag: the poll
/// loop re-checks the `running` flag at least this often.
const POLL_TIMEOUT_MS: i32 = 100;

/// The side-specific modifier keys we track, paired with their evdev
/// codes. Ordered to match `state::MODIFIER_RELEASE_ORDER`.
const MODIFIER_KEYS: [(KeyCode, Modifiers); 8] = [
    (KeyCode::KEY_LEFTMETA, Modifiers::CMD_LEFT),
    (KeyCode::KEY_RIGHTMETA, Modifiers::CMD_RIGHT),
    (KeyCode::KEY_LEFTSHIFT, Modifiers::SHIFT_LEFT),
    (KeyCode::KEY_RIGHTSHIFT, Modifiers::SHIFT_RIGHT),
    (KeyCode::KEY_LEFTCTRL, Modifiers::CTRL_LEFT),
    (KeyCode::KEY_RIGHTCTRL, Modifiers::CTRL_RIGHT),
    (KeyCode::KEY_LEFTALT, Modifiers::OPT_LEFT),
    (KeyCode::KEY_RIGHTALT, Modifiers::OPT_RIGHT),
];

// Key event values from the kernel input protocol.
/// Name prefix of the uinput re-injection clones. Doubles as the marker by
/// which blocking listeners recognize each other's outputs and refuse to
/// grab them (see `is_passthrough_clone`).
const CLONE_NAME_PREFIX: &str = "handy-keys passthrough: ";

/// Kernel cap on a uinput device name (`UINPUT_MAX_NAME_SIZE`). The evdev
/// crate enforces it with a hard `assert!(name.len() + 1 < 80)` inside
/// `VirtualDevice::builder().name(..)`, so an over-long name is a *panic*,
/// not an `Err` — which kills the whole listener thread and silently
/// disables every hotkey. We truncate before building so the name always
/// fits: the usable budget is 78 bytes (80 - 1 for the NUL - the `<`, not
/// `<=`). Source device names longer than 78 - prefix bytes are common in
/// practice (e.g. input-remapper's `input-remapper <make> <model> forwarded`
/// virtual keyboards), so this is a real crash, not a theoretical one.
const MAX_CLONE_NAME_BYTES: usize = 78;

const KEY_RELEASED: i32 = 0;
const KEY_PRESSED: i32 = 1;
const KEY_AUTOREPEAT: i32 = 2;

/// Blocking state for one listener instance.
struct GrabConfig {
    /// Device nodes of this listener's own uinput clones, excluded from
    /// its enumeration: reading one's own output would loop. Entries are
    /// removed when the backing device (and thus its clone) goes away, so
    /// a later reuse of the node path by real hardware is not filtered.
    own_outputs: HashSet<PathBuf>,
}

/// Listener-wide event-processing state.
struct LinuxState {
    listener: ListenerState,
    /// Non-modifier keys whose press was blocked. Their autorepeats and
    /// release are blocked too, so the re-injected stream never carries an
    /// orphan repeat or key-up.
    blocked_keys: HashSet<KeyCode>,
    /// Modifiers whose press was blocked; the matching release is blocked
    /// for the same reason. Symmetry also protects the reverse case: a
    /// modifier whose press was forwarded is never blocked on release,
    /// even if a matching hotkey was registered mid-hold — otherwise the
    /// compositor would see a press with no release and stick the key.
    blocked_modifiers: Modifiers,
}

/// An input device the listener is reading.
struct OpenDevice {
    path: PathBuf,
    dev: RawDevice,
    /// The kernel reported SYN_DROPPED: events were lost. Per the evdev
    /// protocol, everything up to and including the next SYN_REPORT is
    /// unreliable for state tracking; modifier state is then reconciled
    /// against EVIOCGKEY. (The unreliable events are still forwarded to a
    /// grabbed device's clone — the user really typed them.)
    resyncing: bool,
    /// Present when this device is grabbed for blocking: the uinput clone
    /// through which non-blocked events are re-injected.
    output: Option<VirtualDevice>,
    /// The clone's /dev/input nodes, held to unregister them from
    /// `GrabConfig::own_outputs` when this device is removed.
    output_nodes: Vec<PathBuf>,
    /// This keyboard should be grabbed, but keys were physically held when
    /// it was opened. Grabbing mid-press would split the press/release pair
    /// across two devices and wedge the key at the compositor (libinput
    /// discards unbalanced key-ups), so the device stays read-only until
    /// EVIOCGKEY reports it quiet — the pending release itself wakes the
    /// poll loop, and the grab completes then.
    pending_grab: bool,
    /// Events of the in-flight batch, forwarded to `output` as one unit on
    /// SYN_REPORT so batching is preserved. Unused when not grabbed.
    pending: Vec<InputEvent>,
}

/// Internal listener state returned to KeyboardListener
pub(crate) struct LinuxListenerState {
    pub event_receiver: Receiver<KeyEvent>,
    pub thread_handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
    pub blocking_hotkeys: Option<BlockingHotkeys>,
}

/// Spawn an evdev keyboard listener for Linux
///
/// Fails if no input device can be opened, or if blocking is requested but
/// `/dev/uinput` is not writable — reported with the fix in the error
/// message — so a caller can tell hotkeys will actually work before this
/// returns Ok.
pub(crate) fn spawn(blocking_hotkeys: Option<BlockingHotkeys>) -> Result<LinuxListenerState> {
    let mut grab = blocking_hotkeys.as_ref().map(|_| GrabConfig {
        own_outputs: HashSet::new(),
    });
    if grab.is_some() {
        check_uinput_access()?;
    }

    // Watch for hotplug *before* scanning: a node that appears (or becomes
    // readable via udev's chmod) during the scan then shows up as a pending
    // inotify event for the first poll instead of falling into the gap.
    let inotify = Inotify::watch(DEV_INPUT).map_err(|e| {
        Error::Platform(format!(
            "failed to watch {DEV_INPUT} for device hotplug: {e}"
        ))
    })?;
    let devices = open_capturable_devices(&mut grab)?;

    let (tx, rx) = mpsc::channel();
    let mut state = LinuxState {
        listener: ListenerState::new(tx, blocking_hotkeys.clone(), None, None),
        blocked_keys: HashSet::new(),
        blocked_modifiers: Modifiers::empty(),
    };
    // Adopt modifiers physically held right now (e.g. the listener was
    // started from a hotkey press) so they ride along on the next event.
    state.listener.current_modifiers = physical_modifiers(&devices);

    let running = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);

    let handle = thread::spawn(move || {
        run_loop(devices, inotify, grab, state, thread_running);
    });

    Ok(LinuxListenerState {
        event_receiver: rx,
        thread_handle: Some(handle),
        running,
        blocking_hotkeys,
    })
}

/// Verify /dev/uinput is usable before promising blocking.
fn check_uinput_access() -> Result<()> {
    // VirtualDevice::builder() opens /dev/uinput; nothing is created yet.
    VirtualDevice::builder().map(|_| ()).map_err(|e| {
        Error::Platform(format!(
            "hotkey blocking requires write access to /dev/uinput (non-blocked keystrokes are \
             re-injected through it): {e}. Grant it with a udev rule (e.g. KERNEL==\"uinput\", \
             GROUP=\"input\", MODE=\"0660\" plus input-group membership), or use the non-blocking \
             listener"
        ))
    })
}

/// Open every capturable device under /dev/input, or explain why none could
/// be opened.
fn open_capturable_devices(grab: &mut Option<GrabConfig>) -> Result<Vec<OpenDevice>> {
    let entries = fs::read_dir(DEV_INPUT)
        .map_err(|e| Error::Platform(format!("cannot read {DEV_INPUT}: {e}")))?;

    let mut devices = Vec::new();
    let mut permission_denied = 0usize;

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_node(&path) {
            continue;
        }
        match open_device(&path, grab) {
            Ok(Some(device)) => devices.push(device),
            Ok(None) => {} // no keys or buttons we can map (e.g. accelerometers)
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => permission_denied += 1,
            Err(_) => {} // transient (device unplugged mid-scan, etc.)
        }
    }

    if devices.is_empty() {
        return Err(Error::Platform(if permission_denied > 0 {
            format!(
                "permission denied opening {permission_denied} device node(s) under {DEV_INPUT}. \
                 Reading keyboard events requires read access to these nodes: add your user to \
                 the 'input' group (sudo usermod -aG input $USER) and log out and back in"
            )
        } else {
            format!("no keyboard-capable input devices found under {DEV_INPUT}")
        }));
    }

    Ok(devices)
}

fn is_event_node(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("event"))
}

/// Open one device node, returning Ok(None) if it can't produce any event
/// we can map, or if it is one of this listener's own uinput clones.
///
/// In blocking mode, keyboards are additionally grabbed with a uinput
/// clone attached — immediately if the device is quiet, deferred until
/// its keys are released otherwise (see `OpenDevice::pending_grab`). A
/// failed grab (e.g. another exclusive grabber like keyd got there first)
/// degrades that device to read-only with a warning, so hotkey detection
/// keeps working.
fn open_device(path: &Path, grab: &mut Option<GrabConfig>) -> io::Result<Option<OpenDevice>> {
    if let Some(cfg) = grab {
        if cfg.own_outputs.contains(path) {
            return Ok(None); // our own clone: reading it back would loop
        }
    }
    let dev = RawDevice::open(path)?;
    if !is_capturable(&dev) {
        return Ok(None);
    }
    // Non-blocking so the poll loop can drain each readable device fully
    // without ever sleeping inside a read.
    set_nonblocking(dev.as_raw_fd())?;

    let mut device = OpenDevice {
        path: path.to_path_buf(),
        dev,
        resyncing: false,
        output: None,
        output_nodes: Vec::new(),
        pending_grab: false,
        pending: Vec::new(),
    };

    if let Some(cfg) = grab {
        if is_passthrough_clone(&device.dev) {
            // Another blocking listener's re-injection output. Reading it
            // gives us its keyboard's (non-blocked) events; grabbing it
            // would make that listener see OUR clone as a new keyboard and
            // clone it back — an unbounded device storm. First blocker
            // wins; we detect but cannot block behind it.
            eprintln!(
                "handy-keys: {} is another handy-keys blocking listener's output; its hotkeys \
                 will be detected but not blocked by this listener",
                path.display()
            );
        } else if is_keyboard(&device.dev) {
            if keys_held(&device.dev) {
                device.pending_grab = true; // grab at first quiet moment
            } else {
                complete_grab(&mut device, cfg);
            }
        } else if has_keyboard_keys(&device.dev) {
            // Combo device exposing keyboard keys on a pointer node: the
            // no-grabbing-pointers rule wins, but say so — "why isn't my
            // hotkey blocked" is undebuggable otherwise.
            eprintln!(
                "handy-keys: {} exposes keyboard keys on a pointer device; its hotkeys will be \
                 detected but not blocked (pointer devices are never grabbed)",
                path.display()
            );
        }
    }

    Ok(Some(device))
}

/// Attempt the exclusive grab + clone for a keyboard, degrading to
/// read-only with a warning on failure. Clears `pending_grab` either way —
/// a refused grab (EBUSY from another grabber) won't heal by retrying.
fn complete_grab(device: &mut OpenDevice, cfg: &mut GrabConfig) {
    device.pending_grab = false;
    match grab_with_output(&mut device.dev) {
        Ok((output, nodes)) => {
            cfg.own_outputs.extend(nodes.iter().cloned());
            device.output = Some(output);
            device.output_nodes = nodes;
        }
        Err(e) => {
            eprintln!(
                "handy-keys: cannot grab {} for hotkey blocking ({e}); its hotkeys will be \
                 detected but not blocked",
                device.path.display()
            );
        }
    }
}

/// Whether any key is physically held on the device right now (EVIOCGKEY).
fn keys_held(dev: &RawDevice) -> bool {
    dev.get_key_state()
        .map(|keys| keys.iter().next().is_some())
        .unwrap_or(false)
}

/// Whether this device is a re-injection clone created by a handy-keys
/// blocking listener (this process or another).
fn is_passthrough_clone(dev: &RawDevice) -> bool {
    dev.name()
        .is_some_and(|name| name.starts_with(CLONE_NAME_PREFIX))
}

/// A device is worth reading iff it can emit at least one key or button we
/// can translate: keyboards and keypads via `key_code_to_key`/modifiers,
/// mice via their button codes (BTN_SIDE/BTN_EXTRA are hotkey-relevant).
fn is_capturable(dev: &RawDevice) -> bool {
    if !dev.supported_events().contains(EventType::KEY) {
        return false;
    }
    match dev.supported_keys() {
        Some(keys) => keys
            .iter()
            .any(|code| key_code_to_key(code).is_some() || key_code_to_modifier(code).is_some()),
        None => false,
    }
}

/// Only keyboards are ever grabbed. A pointer (mouse, touchpad — anything
/// with relative or absolute X motion) must never be grabbed: re-injection
/// would add latency to every motion event, and a crash mid-grab is far
/// more disruptive. A device qualifies as a keyboard by emitting at least
/// one mappable non-mouse key.
fn is_keyboard(dev: &RawDevice) -> bool {
    let is_pointer = dev
        .supported_relative_axes()
        .is_some_and(|axes| axes.contains(RelativeAxisCode::REL_X))
        || dev
            .supported_absolute_axes()
            .is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_X));
    !is_pointer && has_keyboard_keys(dev)
}

/// Whether the device can emit at least one mappable non-mouse key.
fn has_keyboard_keys(dev: &RawDevice) -> bool {
    dev.supported_keys().is_some_and(|keys| {
        keys.iter().any(|code| {
            key_code_to_modifier(code).is_some()
                || key_code_to_key(code).is_some_and(|key| {
                    !matches!(
                        key,
                        Key::MouseLeft
                            | Key::MouseRight
                            | Key::MouseMiddle
                            | Key::MouseX1
                            | Key::MouseX2
                    )
                })
        })
    })
}

/// Truncate `s` to at most `max_bytes`, never splitting a UTF-8 code point.
/// Returns the whole string when it already fits.
fn truncate_on_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Grab a keyboard exclusively and create the uinput clone that carries
/// its non-blocked events onward to the rest of the system.
fn grab_with_output(dev: &mut RawDevice) -> io::Result<(VirtualDevice, Vec<PathBuf>)> {
    let full_name = format!("{CLONE_NAME_PREFIX}{}", dev.name().unwrap_or("keyboard"));
    // Must fit UINPUT_MAX_NAME_SIZE or the evdev builder panics (see
    // MAX_CLONE_NAME_BYTES). CLONE_NAME_PREFIX (24 bytes) survives the
    // truncation, so `is_passthrough_clone` self-filtering still works.
    let name = truncate_on_char_boundary(&full_name, MAX_CLONE_NAME_BYTES);

    let mut builder = VirtualDevice::builder()?
        .name(name)
        // Clone the vendor/product id so layout-specific quirk handling in
        // compositors follows the real hardware.
        .input_id(dev.input_id());
    if let Some(keys) = dev.supported_keys() {
        builder = builder.with_keys(keys)?;
    }
    if let Some(misc) = dev.misc_properties() {
        // MSC_SCAN accompanies key events on most keyboards; declare it so
        // forwarded batches keep it.
        builder = builder.with_msc(misc)?;
    }
    let mut output = builder.build()?;
    // Blocks briefly until sysfs lists the clone's device node, so the
    // node is registered for self-filtering before inotify reports it.
    let nodes: Vec<PathBuf> = output
        .enumerate_dev_nodes_blocking()?
        .filter_map(|node| node.ok())
        .collect();

    // The kernel has queued events on our fd since open(), but until the
    // grab lands the compositor received them too — forwarding them would
    // deliver every pre-grab keystroke twice. Discard them (modifier state
    // is seeded from EVIOCGKEY, not from these events); only what arrives
    // after the grab is exclusively ours to re-inject.
    discard_pending_events(dev)?;

    // Grab only after the clone exists, so there is never a moment where
    // keystrokes are swallowed with nowhere to go.
    dev.grab()?;
    Ok((output, nodes))
}

/// Drain and discard everything queued on a (non-blocking) device fd.
fn discard_pending_events(dev: &mut RawDevice) -> io::Result<()> {
    loop {
        match dev.fetch_events() {
            Ok(events) => {
                if events.count() == 0 {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Snapshot which tracked modifier keys are physically held right now,
/// per the kernel's own key state (EVIOCGKEY), across all open devices.
fn physical_modifiers(devices: &[OpenDevice]) -> Modifiers {
    let mut held = Modifiers::empty();
    for device in devices {
        if let Ok(keys) = device.dev.get_key_state() {
            for (code, modifier) in MODIFIER_KEYS {
                if keys.contains(code) {
                    held |= modifier;
                }
            }
        }
    }
    held
}

/// Reconcile tracked modifiers against the kernel's key state.
///
/// Stale modifiers are cleared with synthetic release events so consumers
/// tracking press/release state recover; missed presses are adopted
/// silently and ride along on the next real event.
fn reconcile_modifiers(state: &mut LinuxState, physical: Modifiers) {
    for event in release_events(
        state.listener.current_modifiers,
        stale_modifiers(state.listener.current_modifiers, physical),
    ) {
        state.listener.current_modifiers = event.modifiers;
        let _ = state.listener.event_sender.send(event);
    }
    state.listener.current_modifiers |= physical & RECONCILABLE & !state.listener.current_modifiers;
    // A modifier no longer physically held anywhere can't produce a
    // release event that would need blocking; drop its stale blocked bit
    // (e.g. left by a keyboard unplugged mid-hold) so a later forwarded
    // press on another keyboard is never paired with a blocked release.
    state.blocked_modifiers &= physical;
}

/// The poll loop: one pollfd per device plus the inotify watch, re-checking
/// the shutdown flag on every wakeup or timeout.
fn run_loop(
    mut devices: Vec<OpenDevice>,
    inotify: Inotify,
    mut grab: Option<GrabConfig>,
    mut state: LinuxState,
    running: Arc<AtomicBool>,
) {
    while running.load(Ordering::SeqCst) {
        let mut pollfds = Vec::with_capacity(devices.len() + 1);
        pollfds.push(libc::pollfd {
            fd: inotify.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
        for device in &devices {
            pollfds.push(libc::pollfd {
                fd: device.dev.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }

        let rc = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as _, POLL_TIMEOUT_MS) };
        if rc < 0 {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                // Transient (signal delivery, kernel allocation pressure):
                // retry, with a nap so a persistent EAGAIN/ENOMEM can't
                // become a busy loop.
                Some(libc::EINTR) => continue,
                Some(libc::EAGAIN) | Some(libc::ENOMEM) => {
                    thread::sleep(std::time::Duration::from_millis(POLL_TIMEOUT_MS as u64));
                    continue;
                }
                // Anything else (EFAULT/EINVAL) is a programming error that
                // won't heal; exiting drops the sender, so receivers see
                // EventLoopNotRunning instead of silently waiting forever.
                _ => {
                    eprintln!("handy-keys: poll on input devices failed: {err}");
                    break;
                }
            }
        }
        if rc == 0 {
            continue;
        }

        let mut needs_reconcile = false;
        let mut dead = Vec::new();
        // pollfds[1 + i] corresponds to devices[i].
        for (i, pollfd) in pollfds[1..].iter().enumerate() {
            if pollfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                dead.push(i);
                continue;
            }
            if pollfd.revents & libc::POLLIN == 0 {
                continue;
            }
            match drain_device(&mut devices[i], &mut grab, &mut state) {
                Ok(reconcile) => needs_reconcile |= reconcile,
                Err(_) => dead.push(i), // ENODEV: device unplugged
            }
        }

        if !dead.is_empty() {
            for i in dead.into_iter().rev() {
                let device = devices.remove(i);
                // Dropping the device destroys its clone; unregister the
                // clone's node paths so real hardware reusing them later
                // is not mistaken for our own output.
                if let Some(cfg) = &mut grab {
                    for node in &device.output_nodes {
                        cfg.own_outputs.remove(node);
                    }
                }
            }
            // A keyboard that vanished mid-chord takes its key-ups with it.
            needs_reconcile = true;
        }

        // Add new devices only after dead entries are gone: a replugged
        // keyboard typically reuses the same /dev/input/eventN path, and
        // when its removal and reappearance land in the same wakeup the
        // dedup check must not match the stale entry.
        if pollfds[0].revents & libc::POLLIN != 0 {
            for path in inotify.drain_event_nodes() {
                try_add_device(&mut devices, &path, &mut grab, &mut state);
            }
        }

        if needs_reconcile {
            reconcile_modifiers(&mut state, physical_modifiers(&devices));
        }
    }
}

/// Drain everything currently readable from one device, forwarding
/// non-blocked events to its uinput clone if grabbed. Completes a deferred
/// grab once the device goes quiet. Returns whether modifier
/// reconciliation is needed (a SYN_DROPPED resync completed).
fn drain_device(
    device: &mut OpenDevice,
    grab: &mut Option<GrabConfig>,
    state: &mut LinuxState,
) -> io::Result<bool> {
    let mut needs_reconcile = false;
    let mut saw_release = false;
    loop {
        match device.dev.fetch_events() {
            Ok(events) => {
                for event in events {
                    match event.destructure() {
                        EventSummary::Synchronization(_, SynchronizationCode::SYN_DROPPED, _) => {
                            device.resyncing = true;
                        }
                        EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                            if device.resyncing {
                                device.resyncing = false;
                                needs_reconcile = true;
                            }
                            // Forward the completed batch as one unit; emit()
                            // appends the SYN_REPORT itself.
                            if let Some(output) = &mut device.output {
                                if !device.pending.is_empty() {
                                    if let Err(e) = output.emit(&device.pending) {
                                        eprintln!(
                                            "handy-keys: failed to re-inject events for {}: {e}",
                                            device.path.display()
                                        );
                                    }
                                    device.pending.clear();
                                }
                            }
                        }
                        EventSummary::Key(_, code, value) => {
                            saw_release |= value == KEY_RELEASED;
                            // During a resync the event is unreliable for
                            // state tracking (and for a block decision), but
                            // the user really typed it: forward it.
                            let block = if device.resyncing {
                                false
                            } else {
                                process_key(state, code, value)
                            };
                            if !block && device.output.is_some() {
                                device.pending.push(event);
                            }
                        }
                        // MSC_SCAN and friends: forward within the batch.
                        _ => {
                            if device.output.is_some() {
                                device.pending.push(event);
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            // A signal during the read syscall is not device death; retry.
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }

    // Deferred grab: the device was opened while keys were held (see
    // `pending_grab`); if a release just came in and the device is quiet
    // now, finish the grab.
    if device.pending_grab && saw_release {
        if let Some(cfg) = grab {
            if !keys_held(&device.dev) {
                complete_grab(device, cfg);
            }
        }
    }
    Ok(needs_reconcile)
}

/// Translate one kernel key event into the cross-platform KeyEvent stream.
/// Returns whether a grabbed device must withhold this event from its
/// clone (it matched a blocking hotkey, or completes one that did).
fn process_key(state: &mut LinuxState, code: KeyCode, value: i32) -> bool {
    let is_key_down = match value {
        KEY_RELEASED => false,
        // Autorepeat maps to a repeated key-down, matching the other
        // platforms' behavior.
        KEY_PRESSED | KEY_AUTOREPEAT => true,
        _ => return false,
    };

    if let Some(changed_modifier) = key_code_to_modifier(code) {
        if value == KEY_AUTOREPEAT {
            // Repeat of a held modifier changes no state; stays blocked
            // iff its press was.
            return state.blocked_modifiers.contains(changed_modifier);
        }
        let prev_modifiers = state.listener.current_modifiers;
        if is_key_down {
            state.listener.current_modifiers |= changed_modifier;
        } else {
            state.listener.current_modifiers &= !changed_modifier;
        }

        // Only emit when the state actually changed (two keyboards holding
        // the same modifier produce duplicate presses).
        let changed = state.listener.current_modifiers != prev_modifiers;
        if changed {
            let _ = state.listener.event_sender.send(KeyEvent {
                modifiers: state.listener.current_modifiers,
                key: None,
                is_key_down,
                changed_modifier: Some(changed_modifier),
            });
        }

        return if is_key_down {
            let block = changed
                && state
                    .listener
                    .should_block(state.listener.current_modifiers, None);
            if block {
                state.blocked_modifiers |= changed_modifier;
            } else {
                // A forwarded press must never be paired with a blocked
                // release: clear any stale blocked bit (e.g. left behind by
                // a keyboard that disappeared mid-hold), mirroring the
                // non-modifier path below.
                state.blocked_modifiers &= !changed_modifier;
            }
            block
        } else {
            // Block the release iff the press was blocked, regardless of
            // what the hotkey set says now (see `blocked_modifiers`).
            let block = state.blocked_modifiers.contains(changed_modifier);
            state.blocked_modifiers &= !changed_modifier;
            block
        };
    }

    let Some(key) = key_code_to_key(code) else {
        return false;
    };

    // The press decides; repeats and the release follow it, so the
    // re-injected stream never carries an orphan repeat or key-up.
    let block = match value {
        KEY_PRESSED => {
            let block = state
                .listener
                .should_block(state.listener.current_modifiers, Some(key));
            if block {
                state.blocked_keys.insert(code);
            } else {
                state.blocked_keys.remove(&code);
            }
            block
        }
        KEY_AUTOREPEAT => state.blocked_keys.contains(&code),
        _ => state.blocked_keys.remove(&code),
    };

    // Only report left/right clicks when modifiers are held, to avoid
    // noise (same rule as the macOS and Windows listeners).
    if matches!(key, Key::MouseLeft | Key::MouseRight)
        && state.listener.current_modifiers.is_empty()
    {
        return block;
    }

    let _ = state.listener.event_sender.send(KeyEvent {
        modifiers: state.listener.current_modifiers,
        key: Some(key),
        is_key_down,
        changed_modifier: None,
    });

    block
}

/// Try to open a device node that just appeared (or became accessible) in
/// /dev/input. Failures are expected and silent: inotify fires IN_CREATE
/// before udev has granted the input group read access, and the follow-up
/// IN_ATTRIB after the chmod retries the open. Hotplugged keyboards are
/// grabbed like scanned ones when blocking is enabled.
fn try_add_device(
    devices: &mut Vec<OpenDevice>,
    path: &Path,
    grab: &mut Option<GrabConfig>,
    state: &mut LinuxState,
) {
    if devices.iter().any(|d| d.path == path) {
        return;
    }
    if let Ok(Some(device)) = open_device(path, grab) {
        // Adopt modifiers already held on the new device silently, same as
        // reconciliation does for missed presses.
        state.listener.current_modifiers |= physical_modifiers(std::slice::from_ref(&device));
        devices.push(device);
    }
}

/// Minimal inotify wrapper watching one directory for device hotplug.
struct Inotify {
    fd: OwnedFd,
}

impl Inotify {
    fn watch(dir: &str) -> io::Result<Self> {
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };

        let c_dir = CString::new(dir).expect("watch dir contains no NUL");
        // IN_CREATE: a node appeared. IN_ATTRIB: udev adjusted its
        // ownership/mode, which is the moment opening typically first
        // succeeds for non-root users.
        let wd = unsafe {
            libc::inotify_add_watch(
                fd.as_raw_fd(),
                c_dir.as_ptr(),
                libc::IN_CREATE | libc::IN_ATTRIB,
            )
        };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    /// Drain pending inotify events, returning the paths of event* nodes
    /// that appeared or changed attributes.
    fn drain_event_nodes(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            if n <= 0 {
                break; // drained (EAGAIN), or watch gone — nothing to do
            }

            let mut offset = 0usize;
            let header_len = std::mem::size_of::<libc::inotify_event>();
            while offset + header_len <= n as usize {
                // read_unaligned: the byte buffer offers no alignment
                // guarantee for the packed event stream.
                let event = unsafe {
                    std::ptr::read_unaligned(buf.as_ptr().add(offset) as *const libc::inotify_event)
                };
                let name_start = offset + header_len;
                let name_end = name_start + event.len as usize;
                if name_end > n as usize {
                    break; // malformed/truncated entry; drop the rest
                }
                // The name field is NUL-padded to its declared length.
                let name_bytes = &buf[name_start..name_end];
                let name_len = name_bytes
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(name_bytes.len());
                if let Ok(name) = std::str::from_utf8(&name_bytes[..name_len]) {
                    if name.starts_with("event") {
                        paths.push(Path::new(DEV_INPUT).join(name));
                    }
                }
                offset = name_end;
            }
        }
        paths.sort();
        paths.dedup();
        paths
    }
}

