//! Windows low-level keyboard hook implementation

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    MsgWaitForMultipleObjects, PeekMessageW, RegisterClassW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, MSLLHOOKSTRUCT, PM_REMOVE,
    QS_ALLINPUT, WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_POWERBROADCAST, WM_QUIT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW,
};

use crate::error::{Error, Result};
use crate::platform::state::{release_events, stale_modifiers, BlockingHotkeys, RECONCILABLE};
use crate::types::{Key, KeyEvent, Modifiers};

use super::keycode::{is_altgr_phantom_ctrl, map_key, vk_to_modifier};

const HOOK_LOOP_TIMEOUT_MS: u32 = 10;

/// Modifiers whose lone press-and-release makes the shell activate a menu:
/// Win opens the Start menu, Alt focuses the active window's menu bar.
const MENU_MODIFIERS: Modifiers = Modifiers::CMD.union(Modifiers::OPT);

/// Unassigned virtual key injected as the "menu mask" (AutoHotkey's default
/// mask key). The shell arms its menu when a Win/Alt down→up pair passes
/// with no other key in between; when we swallow the real key of a Win/Alt
/// hotkey, this inert keystroke is substituted so the modifier release no
/// longer reads as a lone tap (cjpais/Handy#917). The VK maps to nothing in
/// this crate (`vk_to_modifier` and `map_key` both return None), so even
/// without the dwExtraInfo marker it could never be blocked on its way to
/// the shell.
const MENU_MASK_VK: u16 = 0xE8;

/// dwExtraInfo marker stamped on injected mask events so the keyboard hook
/// recognizes its own injections and passes them through untouched
/// ("HKMM" in ASCII).
const MENU_MASK_EXTRA_INFO: usize = 0x484B_4D4D;

// WTS session notification plumbing not exposed by the `windows` crate bindings.
const NOTIFY_FOR_THIS_SESSION: u32 = 0;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
// WM_WTSSESSION_CHANGE wParam values (wtsapi32.h).
const WTS_CONSOLE_CONNECT: usize = 0x1;
const WTS_REMOTE_CONNECT: usize = 0x3;
const WTS_SESSION_LOCK: usize = 0x7;
const WTS_SESSION_UNLOCK: usize = 0x8;
// WM_POWERBROADCAST wParam values (winuser.h) -- the PBT_* constants live
// behind the Win32_System_Power feature, not worth enabling for two values.
// RESUMEAUTOMATIC fires on every wake; RESUMESUSPEND follows it only when the
// wake was user-initiated.
const PBT_APMRESUMESUSPEND: usize = 0x7;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;

/// The side-specific modifier keys we track, paired with their virtual-key
/// codes. Ordered to match `state::MODIFIER_RELEASE_ORDER`.
const MODIFIER_KEYS: [(VIRTUAL_KEY, Modifiers); 8] = [
    (VK_LWIN, Modifiers::CMD_LEFT),
    (VK_RWIN, Modifiers::CMD_RIGHT),
    (VK_LSHIFT, Modifiers::SHIFT_LEFT),
    (VK_RSHIFT, Modifiers::SHIFT_RIGHT),
    (VK_LCONTROL, Modifiers::CTRL_LEFT),
    (VK_RCONTROL, Modifiers::CTRL_RIGHT),
    (VK_LMENU, Modifiers::OPT_LEFT),
    (VK_RMENU, Modifiers::OPT_RIGHT),
];

/// Thread-local state for the keyboard hook callback.
///
/// Windows low-level hooks require a callback function with a specific signature,
/// so we use thread-local storage to access our state from within the callback.
struct HookContext {
    event_sender: Sender<KeyEvent>,
    current_modifiers: Modifiers,
    // Swallowed presses do not enter GetAsyncKeyState; retain them until release.
    blocked_modifiers: Modifiers,
    blocking_hotkeys: Option<BlockingHotkeys>,
    ignored_source_tag: Option<usize>,
    recovery: Option<Arc<dyn Fn() + Send + Sync>>,
    /// AltGr's phantom Left Ctrl is currently held. The hook drops the
    /// phantom's own events, but GetAsyncKeyState still reports LCtrl as
    /// down while AltGr is held, so reconciliation must not adopt it.
    altgr_phantom_ctrl: bool,
    /// A menu mask has been injected for the current Win/Alt hold. One per
    /// hold defuses the shell's menu heuristic for the whole hold; cleared
    /// once no Win/Alt modifier remains held.
    menu_mask_sent: bool,
}

thread_local! {
    static HOOK_CONTEXT: std::cell::RefCell<Option<HookContext>> = const { std::cell::RefCell::new(None) };

    /// Set by `watcher_wndproc` when a message delivered via SendMessage
    /// (bypassing the queue) asks for a hook re-install. The message loop
    /// owns the hook handles, so the wndproc can only request the re-install,
    /// not perform it. Thread-local: the wndproc runs on the hook thread, and
    /// a second listener in the same process must not see this one's requests.
    static REINSTALL_REQUESTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Take (and clear) a pending hook re-install request from the wndproc.
fn take_reinstall_request() -> bool {
    REINSTALL_REQUESTED.with(|flag| flag.replace(false))
}

/// What draining the thread message queue observed.
#[derive(Default)]
struct DrainOutcome {
    /// WM_QUIT received -- exit the message loop.
    quit: bool,
    /// A session change (lock/unlock/connect) or a resume from sleep
    /// occurred -- reconcile modifier state, since key-ups on the secure
    /// desktop (or from before a suspend) never reach the hook.
    reconcile: bool,
    /// The interactive desktop came back (unlock, console/remote connect, or
    /// resume from sleep) -- re-install hooks in case Windows silently
    /// removed them.
    reinstall_hooks: bool,
}

/// Drain all pending thread messages.
fn drain_thread_messages(msg: &mut MSG) -> DrainOutcome {
    let mut outcome = DrainOutcome::default();
    unsafe {
        while PeekMessageW(msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                outcome.quit = true;
                return outcome;
            }
            if msg.message == WM_WTSSESSION_CHANGE {
                match msg.wParam.0 {
                    WTS_SESSION_LOCK => outcome.reconcile = true,
                    WTS_SESSION_UNLOCK | WTS_CONSOLE_CONNECT | WTS_REMOTE_CONNECT => {
                        outcome.reconcile = true;
                        outcome.reinstall_hooks = true;
                    }
                    _ => {}
                }
            }
            // WM_POWERBROADCAST is documented as sent, not posted, so the
            // wndproc is the expected delivery path; this arm is a cheap
            // safety net in case some Windows build posts it instead.
            if msg.message == WM_POWERBROADCAST
                && matches!(msg.wParam.0, PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC)
            {
                outcome.reconcile = true;
                outcome.reinstall_hooks = true;
            }
            let _ = TranslateMessage(msg);
            DispatchMessageW(msg);
        }
    }
    outcome
}

/// Wait for new input/messages or until timeout expires.
fn wait_for_message_or_timeout(timeout_ms: u32) {
    unsafe {
        let _ = MsgWaitForMultipleObjects(None, false, timeout_ms, QS_ALLINPUT);
    }
}

/// Snapshot which tracked modifier keys are physically held right now.
///
/// Note: if the calling thread's desktop is not active (e.g. the lock screen's
/// secure desktop is up), GetAsyncKeyState reports every key as up -- which is
/// the correct answer for our purposes: treat everything as released.
fn physical_modifiers() -> Modifiers {
    let mut held = Modifiers::empty();
    for (vk, modifier) in MODIFIER_KEYS {
        // High bit set = key currently down.
        if unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0 {
            held |= modifier;
        }
    }
    held
}

/// Physically-held modifiers we missed the press of and should adopt.
/// (`stale_modifiers`/`release_events`, the other half of reconciliation,
/// live in crate::platform::state, shared with the Linux listener.)
///
/// While AltGr's phantom Left Ctrl is held, CTRL_LEFT is excluded: it is
/// physically down per GetAsyncKeyState, but the user only pressed Right
/// Alt. A real Left Ctrl press during that window still sets CTRL_LEFT
/// through its own hook event (which carries a non-phantom scancode).
fn adoptable_modifiers(
    tracked: Modifiers,
    physical: Modifiers,
    altgr_phantom_ctrl: bool,
) -> Modifiers {
    let mut adopt = physical & RECONCILABLE & !tracked;
    if altgr_phantom_ctrl {
        adopt &= !Modifiers::CTRL_LEFT;
    }
    adopt
}

/// Reconcile tracked modifiers against the physical keyboard state.
///
/// Corrects drift from missed events: secure desktop transitions swallow
/// key-ups (Win+L delivers the Win key DOWN but its UP happens on the secure
/// desktop), leaving modifiers stuck on until the user happens to press them
/// again. Mirrors `reconcile_modifiers` in the macOS listener.
///
/// Stale modifiers are cleared with synthetic release events so consumers
/// tracking press/release state recover; missed presses are adopted silently
/// and ride along on the next real event.
///
/// Known micro-race: GetAsyncKeyState reads the state *now*, not at the time
/// the event being processed was generated, so a modifier pressed in the
/// sub-millisecond window between a keystroke entering the queue and its hook
/// callback running gets adopted onto that earlier keystroke's event. It is
/// self-correcting (the modifier's own event arrives right after) and
/// unavoidable with this API — macOS avoids it because CGEvent flags are
/// stamped per event.
fn reconcile_modifiers(ctx: &mut HookContext) {
    let physical = physical_modifiers() | ctx.blocked_modifiers;
    // Phantom Ctrl only exists while AltGr is held; if LCtrl reads as up,
    // the phantom is gone (covers a phantom key-up swallowed by a secure
    // desktop transition, which would otherwise suppress CTRL_LEFT adoption
    // forever).
    if ctx.altgr_phantom_ctrl && !physical.contains(Modifiers::CTRL_LEFT) {
        ctx.altgr_phantom_ctrl = false;
    }
    for event in release_events(
        ctx.current_modifiers,
        stale_modifiers(ctx.current_modifiers, physical),
    ) {
        ctx.current_modifiers = event.modifiers;
        let _ = ctx.event_sender.send(event);
    }
    ctx.current_modifiers |=
        adoptable_modifiers(ctx.current_modifiers, physical, ctx.altgr_phantom_ctrl);
    // A Win/Alt-up swallowed by a secure desktop transition ends the hold
    // without the modifier branch observing it; clear the mask flag here so
    // the next hold gets its own mask.
    if !ctx.current_modifiers.intersects(MENU_MODIFIERS) {
        ctx.menu_mask_sent = false;
    }
}

/// Reconcile modifier state from the hook thread's message loop (used on
/// session change and resume from sleep, where no input event accompanies
/// the state change).
fn reconcile_modifiers_in_context() {
    HOOK_CONTEXT.with(|ctx_cell| {
        if let Some(ctx) = ctx_cell.borrow_mut().as_mut() {
            ctx.blocked_modifiers = Modifiers::empty();
            reconcile_modifiers(ctx);
        }
    });
}

/// Wndproc for the watcher window. (The `windows` crate's DefWindowProcW is
/// a generic Rust wrapper, so it cannot be used as lpfnWndProc directly.)
///
/// Handles both delivery modes together with the drain loop: the drain loop
/// covers posted delivery of WM_WTSSESSION_CHANGE (what Windows 11 26200
/// does, verified live), while this path covers builds that deliver it via
/// SendMessage, which bypasses the message queue — and WM_POWERBROADCAST,
/// which is documented as always sent. Double handling on the posted path is
/// harmless: the second reconcile finds nothing stale, and re-install
/// requests coalesce through REINSTALL_REQUESTED. The re-install itself
/// stays on the message loop, which owns the hook handles; this wndproc only
/// requests it.
unsafe extern "system" fn watcher_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let recovering = msg == WM_POWERBROADCAST && matches!(wparam.0, PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC)
        || msg == WM_WTSSESSION_CHANGE && matches!(wparam.0, WTS_SESSION_LOCK | WTS_SESSION_UNLOCK | WTS_CONSOLE_CONNECT | WTS_REMOTE_CONNECT);
    if recovering {
        let recovery = HOOK_CONTEXT.with(|ctx| ctx.borrow().as_ref().and_then(|ctx| ctx.recovery.clone()));
        if let Some(recovery) = recovery { recovery(); }
    }
    match msg {
        WM_WTSSESSION_CHANGE => match wparam.0 {
            WTS_SESSION_LOCK => reconcile_modifiers_in_context(),
            WTS_SESSION_UNLOCK | WTS_CONSOLE_CONNECT | WTS_REMOTE_CONNECT => {
                reconcile_modifiers_in_context();
                REINSTALL_REQUESTED.with(|flag| flag.set(true));
            }
            _ => {}
        },
        // Resume from sleep/hibernate: Windows can silently drop LL hooks
        // across a suspend (Handy #1620), and key-ups from before the
        // suspend are long gone -- re-arm the hooks and reconcile.
        WM_POWERBROADCAST => {
            if matches!(wparam.0, PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC) {
                reconcile_modifiers_in_context();
                REINSTALL_REQUESTED.with(|flag| flag.set(true));
            }
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Create a hidden top-level window that watches WTS session changes and
/// power (suspend/resume) broadcasts. Top-level, not message-only, is load-
/// bearing: broadcast messages such as WM_POWERBROADCAST are never delivered
/// to message-only windows. Without WS_VISIBLE it has no taskbar or Alt-Tab
/// presence. WM_WTSSESSION_CHANGE is posted to it and picked out of the
/// queue by `drain_thread_messages` (verified live: lock 0x7 and unlock 0x8
/// both arrive); WM_POWERBROADCAST is sent and lands in `watcher_wndproc`.
///
/// Returns None on failure. Non-fatal: hooks still work, and stale modifiers
/// are still corrected lazily by `reconcile_modifiers` on the next key event.
unsafe fn create_watcher_window() -> Option<HWND> {
    let class_name: Vec<u16> = "HandyKeysSystemWatcher\0".encode_utf16().collect();
    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(watcher_wndproc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    // May fail with ERROR_CLASS_ALREADY_EXISTS (e.g. a second listener in the
    // same process); CreateWindowExW below still succeeds against the
    // existing class, so the result is deliberately ignored.
    RegisterClassW(&wnd_class);

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        PCWSTR(class_name.as_ptr()),
        PCWSTR::null(),
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        None,
        None,
        None,
        None,
    )
    .ok()?;

    if WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION).is_err() {
        let _ = DestroyWindow(hwnd);
        return None;
    }

    Some(hwnd)
}

/// Clean up the watcher window. Must run on the creating thread.
unsafe fn destroy_watcher_window(hwnd: HWND) {
    let _ = WTSUnRegisterSessionNotification(hwnd);
    let _ = DestroyWindow(hwnd);
}

/// Re-install the low-level hooks, defensively: Windows silently removes an LL
/// hook whose callback exceeds its timeout budget, and a session away from the
/// interactive desktop or a suspend/resume cycle is a common moment for that
/// to surface (Handy #1620). (Note lock/unlock does NOT inherently invalidate
/// hooks -- they survived it in live testing.)
///
/// The replacements are installed before the old hooks are removed, so a
/// failure never leaves us hook-less. No messages are pumped between install
/// and unhook, so no event is delivered twice.
unsafe fn reinstall_hooks(kb_hook: &mut HHOOK, mouse_hook: &mut HHOOK) -> bool {
    let new_kb = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) {
        Ok(h) => h,
        Err(_) => return false,
    };
    let new_mouse = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0) {
        Ok(h) => h,
        Err(_) => {
            let _ = UnhookWindowsHookEx(new_kb);
            return false;
        }
    };
    let _ = UnhookWindowsHookEx(*kb_hook);
    let _ = UnhookWindowsHookEx(*mouse_hook);
    *kb_hook = new_kb;
    *mouse_hook = new_mouse;
    true
}

/// Internal listener state returned to KeyboardListener
pub(crate) struct WindowsListenerState {
    pub event_receiver: mpsc::Receiver<KeyEvent>,
    pub thread_handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
    pub blocking_hotkeys: Option<BlockingHotkeys>,
}

/// Spawn a Windows low-level keyboard hook listener
pub(crate) fn spawn(blocking_hotkeys: Option<BlockingHotkeys>, ignored_source_tag: Option<usize>, recovery: Option<Arc<dyn Fn() + Send + Sync>>) -> Result<WindowsListenerState> {
    let (tx, rx) = mpsc::channel();
    let (initialized, initialization) = mpsc::sync_channel(1);
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let thread_blocking = blocking_hotkeys.clone();

    let handle = thread::spawn(move || {
        // Initialize thread-local hook context
        HOOK_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(HookContext {
                event_sender: tx,
                current_modifiers: Modifiers::empty(),
                blocked_modifiers: Modifiers::empty(),
                blocking_hotkeys: thread_blocking,
                ignored_source_tag,
                recovery,
                altgr_phantom_ctrl: false,
                menu_mask_sent: false,
            });
        });

        // Install the low-level keyboard hook
        let kb_hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) };

        let mut kb_hook = match kb_hook {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to install keyboard hook: {:?}", e);
                let _ = initialized.send(Err(Error::EventLoopNotRunning));
                return;
            }
        };

        // Install the low-level mouse hook
        let mouse_hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0) };

        let mut mouse_hook = match mouse_hook {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to install mouse hook: {:?}", e);
                let _ = initialized.send(Err(Error::EventLoopNotRunning));
                // Clean up keyboard hook before returning
                unsafe {
                    let _ = UnhookWindowsHookEx(kb_hook);
                }
                return;
            }
        };

        // Watch for session changes (Win+L lock/unlock, RDP connect) and
        // suspend/resume: the secure desktop swallows key-up events, so
        // modifier state must be reconciled when the session comes back, and
        // Windows can silently drop LL hooks across a sleep, so they are
        // re-armed on resume.
        let watcher_hwnd = unsafe { create_watcher_window() };
        let _ = initialized.send(Ok(()));

        // Message loop - required for low-level hooks to function.
        // Keep the short timeout so shutdown polling behavior remains unchanged.
        let mut msg = MSG::default();
        loop {
            // Check if we should stop
            if !thread_running.load(Ordering::SeqCst) {
                break;
            }

            // Process all pending messages
            let outcome = drain_thread_messages(&mut msg);
            if outcome.quit {
                break;
            }
            if outcome.reconcile {
                reconcile_modifiers_in_context();
            }
            // The wndproc requests re-installs for messages that arrive via
            // SendMessage (dispatched inside PeekMessageW, never seen by the
            // drain loop) -- WM_POWERBROADCAST always, WM_WTSSESSION_CHANGE
            // on builds that send rather than post it.
            if outcome.reinstall_hooks || take_reinstall_request() {
                unsafe {
                    if reinstall_hooks(&mut kb_hook, &mut mouse_hook) {
                        // At most a few lines per unlock/resume, and it makes
                        // the re-arm observable in the field.
                        eprintln!("handy-keys: re-armed hooks after session/power change");
                    } else {
                        // Keep the old hooks: they usually still work (the
                        // reinstall is defensive hardening, not a repair).
                        eprintln!(
                            "handy-keys: failed to re-install hooks after session/power change"
                        );
                    }
                }
            }

            // Wait for messages or timeout — unlike thread::sleep, this returns
            // immediately when a message arrives, so hook callbacks are never delayed.
            wait_for_message_or_timeout(HOOK_LOOP_TIMEOUT_MS);
        }

        // Clean up the watcher window, then the hooks
        if let Some(hwnd) = watcher_hwnd {
            unsafe {
                destroy_watcher_window(hwnd);
            }
        }
        unsafe {
            let _ = UnhookWindowsHookEx(kb_hook);
            let _ = UnhookWindowsHookEx(mouse_hook);
        }

        // Clear thread-local state
        HOOK_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = None;
        });
    });

    initialization.recv().map_err(|_| Error::EventLoopNotRunning)??;
    Ok(WindowsListenerState {
        event_receiver: rx,
        thread_handle: Some(handle),
        running,
        blocking_hotkeys,
    })
}

/// Low-level keyboard hook callback
///
/// This function is called by Windows for every keyboard event system-wide.
/// It must return quickly to avoid input lag.
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // If code < 0, we must pass to next hook without processing
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let mut should_block = false;
    let mut needs_mask = false;

    // Process the keyboard event
    HOOK_CONTEXT.with(|ctx_cell| {
        let mut ctx_ref = ctx_cell.borrow_mut();
        if let Some(ctx) = ctx_ref.as_mut() {
            // Extract key information from KBDLLHOOKSTRUCT
            let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if ctx.ignored_source_tag.is_some_and(|tag| kb_struct.dwExtraInfo == tag) {
                return;
            }
            let vk_code = kb_struct.vkCode as u16;
            let scan_code = kb_struct.scanCode;
            let is_extended = (kb_struct.flags.0 & LLKHF_EXTENDED.0) != 0;

            let is_key_down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);

            // Our own injected menu mask: pass it through untouched. It
            // exists solely for the shell's benefit and must not update
            // modifier state, trigger reconciliation, or be emitted.
            if vk_code == MENU_MASK_VK && kb_struct.dwExtraInfo == MENU_MASK_EXTRA_INFO {
                return;
            }

            // On AltGr layouts Windows synthesizes a Left Ctrl press/release
            // around every Right Alt event (captured live on UK layout:
            // identical timestamps, scancode 0x21D). Drop it entirely — no
            // modifier update, no emit — so AltGr+key reads as OptRight
            // only. The event is still passed down the hook chain.
            if is_altgr_phantom_ctrl(vk_code, scan_code) {
                ctx.altgr_phantom_ctrl = is_key_down;
                return;
            }

            // Check if this is a modifier key
            if let Some(modifier) = vk_to_modifier(vk_code) {
                let prev_modifiers = ctx.current_modifiers;

                // Update modifier state
                if is_key_down {
                    ctx.current_modifiers |= modifier;
                } else {
                    ctx.current_modifiers &= !modifier;
                    ctx.blocked_modifiers &= !modifier;
                }

                // Only emit event if modifiers actually changed
                if ctx.current_modifiers != prev_modifiers {
                    // Check if modifier-only combo should be blocked
                    should_block =
                        should_block_hotkey(&ctx.blocking_hotkeys, ctx.current_modifiers, None);
                    if is_key_down && should_block {
                        ctx.blocked_modifiers |= modifier;
                    }

                    // A blocked modifier press (the Shift of a Win+Shift
                    // modifier-only hotkey) is as invisible to the shell as
                    // a blocked regular key, so it needs the same mask.
                    if needs_menu_mask(should_block, ctx.current_modifiers, ctx.menu_mask_sent) {
                        ctx.menu_mask_sent = true;
                        needs_mask = true;
                    }

                    let _ = ctx.event_sender.send(KeyEvent {
                        modifiers: ctx.current_modifiers,
                        key: None,
                        is_key_down,
                        changed_modifier: Some(modifier),
                    });
                }

                // The hold ends once no Win/Alt modifier remains held; the
                // next hold gets a fresh mask.
                if !ctx.current_modifiers.intersects(MENU_MODIFIERS) {
                    ctx.menu_mask_sent = false;
                }
            } else {
                // Non-modifier key: reconcile tracked modifiers against the
                // physical keyboard first, so a key-up missed during a secure
                // desktop transition can't stick a modifier onto this event.
                // (Not done for modifier events: inside a low-level hook the
                // async key state does not yet include the in-flight change,
                // so reconciling there would fight the toggle logic above.)
                reconcile_modifiers(ctx);

                if let Some(key) = map_key(vk_code, scan_code, is_extended) {
                    // Regular key event
                    should_block = should_block_hotkey(
                        &ctx.blocking_hotkeys,
                        ctx.current_modifiers,
                        Some(key),
                    );

                    // The shell can no longer see the key we are about to
                    // swallow; without a substitute, releasing Win/Alt would
                    // read as a lone tap and open the Start menu (#917).
                    if needs_menu_mask(should_block, ctx.current_modifiers, ctx.menu_mask_sent) {
                        ctx.menu_mask_sent = true;
                        needs_mask = true;
                    }

                    let _ = ctx.event_sender.send(KeyEvent {
                        modifiers: ctx.current_modifiers,
                        key: Some(key),
                        is_key_down,
                        changed_modifier: None,
                    });
                }
            }
        }
    });

    // Outside the HOOK_CONTEXT borrow: the injected events re-enter this
    // hook (where the dwExtraInfo marker passes them straight through).
    if needs_mask {
        send_menu_mask();
    }

    if should_block {
        // Return non-zero to block the event from propagating
        LRESULT(1)
    } else {
        // Pass to next hook in chain
        CallNextHookEx(None, code, wparam, lparam)
    }
}

/// Low-level mouse hook callback
///
/// This function is called by Windows for every mouse event system-wide.
unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // If code < 0, we must pass to next hook without processing
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    // Only button transitions matter; bail out early for moves and wheel
    // events so the hot path stays free of state access.
    if !matches!(
        wparam.0 as u32,
        WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_RBUTTONDOWN
            | WM_RBUTTONUP
            | WM_MBUTTONDOWN
            | WM_MBUTTONUP
            | WM_XBUTTONDOWN
            | WM_XBUTTONUP
    ) {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let mut should_block = false;
    let mut needs_mask = false;

    // Process the mouse event
    HOOK_CONTEXT.with(|ctx_cell| {
        let mut ctx_ref = ctx_cell.borrow_mut();
        if let Some(ctx) = ctx_ref.as_mut() {
            let mouse_struct = &*(lparam.0 as *const MSLLHOOKSTRUCT);

            // Stale modifiers must not gate button reporting (or decorate the
            // event), so reconcile before reading them.
            reconcile_modifiers(ctx);

            // Only report left/right clicks when modifiers are held (to avoid noise)
            let has_modifiers = !ctx.current_modifiers.is_empty();

            let (key, is_down) = match wparam.0 as u32 {
                WM_LBUTTONDOWN if has_modifiers => (Some(Key::MouseLeft), true),
                WM_LBUTTONUP if has_modifiers => (Some(Key::MouseLeft), false),
                WM_RBUTTONDOWN if has_modifiers => (Some(Key::MouseRight), true),
                WM_RBUTTONUP if has_modifiers => (Some(Key::MouseRight), false),
                // Middle and X buttons always reported
                WM_MBUTTONDOWN => (Some(Key::MouseMiddle), true),
                WM_MBUTTONUP => (Some(Key::MouseMiddle), false),
                WM_XBUTTONDOWN => {
                    // High word of mouseData contains which X button (1 or 2)
                    let xbutton = (mouse_struct.mouseData >> 16) & 0xFFFF;
                    let key = if xbutton == 1 {
                        Some(Key::MouseX1)
                    } else if xbutton == 2 {
                        Some(Key::MouseX2)
                    } else {
                        None
                    };
                    (key, true)
                }
                WM_XBUTTONUP => {
                    let xbutton = (mouse_struct.mouseData >> 16) & 0xFFFF;
                    let key = if xbutton == 1 {
                        Some(Key::MouseX1)
                    } else if xbutton == 2 {
                        Some(Key::MouseX2)
                    } else {
                        None
                    };
                    (key, false)
                }
                _ => (None, false),
            };

            if let Some(key) = key {
                should_block =
                    should_block_hotkey(&ctx.blocking_hotkeys, ctx.current_modifiers, Some(key));

                // A blocked Win/Alt+click hides the click from the shell
                // just like a blocked key, so the hold needs the same mask.
                if needs_menu_mask(should_block, ctx.current_modifiers, ctx.menu_mask_sent) {
                    ctx.menu_mask_sent = true;
                    needs_mask = true;
                }

                let _ = ctx.event_sender.send(KeyEvent {
                    modifiers: ctx.current_modifiers,
                    key: Some(key),
                    is_key_down: is_down,
                    changed_modifier: None,
                });
            }
        }
    });

    if needs_mask {
        send_menu_mask();
    }

    if should_block {
        // Return non-zero to block the event from propagating
        LRESULT(1)
    } else {
        // Pass to next hook in chain
        CallNextHookEx(None, code, wparam, lparam)
    }
}

/// Whether blocking this event must be accompanied by a menu-mask injection:
/// a real event is being swallowed while a Win/Alt modifier is held, and the
/// current hold has not been masked yet. Key-ups need masking too: a key
/// pressed *before* the modifier and released during the hold has its up
/// blocked, but its down passed to the shell before the hold began, so it
/// never disarmed the menu heuristic (verified live: without the up-mask,
/// F20↓ Win↓ F20↑ Win↑ with Win+F20 blocked opens Start). `already_sent`
/// dedupes auto-repeat, release edges, and later events within one hold.
fn needs_menu_mask(should_block: bool, modifiers: Modifiers, already_sent: bool) -> bool {
    should_block && !already_sent && modifiers.intersects(MENU_MODIFIERS)
}

/// Inject a press+release of the menu mask key via SendInput. Must be called
/// outside the HOOK_CONTEXT borrow, since the injected events re-enter the
/// keyboard hook on this thread's next message pump.
fn send_menu_mask() {
    let key = |flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(MENU_MASK_VK),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MENU_MASK_EXTRA_INFO,
            },
        },
    };
    let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        // Rare (e.g. UIPI filtering); the cost is the pre-mask behavior:
        // the shell may open its menu when the modifier is released.
        eprintln!("handy-keys: menu mask injection failed");
    }
}

/// Check if a hotkey combination should be blocked
fn should_block_hotkey(
    blocking_hotkeys: &Option<BlockingHotkeys>,
    modifiers: Modifiers,
    key: Option<Key>,
) -> bool {
    if let Some(ref hotkeys) = blocking_hotkeys {
        if let Ok(set) = hotkeys.lock() {
            return set
                .iter()
                .any(|h| h.modifiers.matches(modifiers) && h.key == key);
        }
    }
    false
}

