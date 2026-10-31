use super::{windows_clipboard as clipboard, windows_keyboard as keyboard, Port};
use std::{thread, time::Duration};
use windows_sys::Win32::{
    Foundation::HWND,
    System::DataExchange::GetClipboardSequenceNumber,
    System::Threading::GetCurrentProcessId,
    UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    },
};

// Opaque physical identity, not projected into account/UI events.
#[derive(Clone, Copy)]
pub struct Target(usize);
impl Target {
    pub fn capture() -> Self {
        Self(unsafe { GetForegroundWindow() } as usize)
    }
    /// Application identity stays local to style selection, never UI/logs.
    pub fn application(&self) -> Option<String> {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(self.0 as HWND, &mut pid);
        }
        if pid == 0 {
            return None;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return None;
        }
        struct Process(windows_sys::Win32::Foundation::HANDLE);
        impl Drop for Process {
            fn drop(&mut self) {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
        let process = Process(process);
        let mut name = vec![0u16; 32768];
        let mut length = name.len() as u32;
        if unsafe { QueryFullProcessImageNameW(process.0, 0, name.as_mut_ptr(), &mut length) } == 0
        {
            return None;
        }
        String::from_utf16(&name[..length as usize]).ok()
    }
    /// Windows drops keystrokes sent into a window of a higher integrity level (UIPI):
    /// an app run as administrator while Govorun is not. Unknown stays false.
    pub fn elevated(&self) -> bool {
        window_elevated(self.0 as HWND)
    }
}
fn window_elevated(window: HWND) -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let Some(own) = integrity(unsafe { GetCurrentProcess() }) else {
        return false;
    };
    if pid == 0 || pid == unsafe { GetCurrentProcessId() } {
        return false;
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return false;
    }
    let level = integrity(process);
    unsafe {
        CloseHandle(process);
    }
    level.is_some_and(|level| level > own)
}
/// A window run as administrator hides its keys from Govorun's keyboard hook, so the
/// hotkey silently does nothing there. One thread hears foreground changes (no timer)
/// and reports the first such window of the run, then stops listening. An elevated
/// Govorun hears every window and starts nothing.
pub fn watch_foreground(report: Box<dyn FnOnce() + Send>) -> Option<thread::JoinHandle<()>> {
    use std::cell::RefCell;
    use windows_sys::Win32::UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, EVENT_SYSTEM_FOREGROUND,
            MSG, OBJID_WINDOW, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        },
    };
    thread_local! {
        static REPORT: RefCell<Option<Box<dyn FnOnce() + Send>>> = const { RefCell::new(None) };
    }
    unsafe extern "system" fn changed(
        _: HWINEVENTHOOK, _: u32, window: HWND, object: i32, _: i32, _: u32, _: u32,
    ) {
        if object != OBJID_WINDOW || window.is_null() || !window_elevated(window) {
            return;
        }
        if let Some(report) = REPORT.with(|slot| slot.borrow_mut().take()) {
            report();
            unsafe { PostQuitMessage(0) };
        }
    }
    if integrity(unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() })
        .is_none_or(|own| own >= 0x3000)
    {
        return None;
    }
    thread::Builder::new()
        .name("govorun-foreground".into())
        .spawn(move || {
            REPORT.with(|slot| *slot.borrow_mut() = Some(report));
            let hook = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    std::ptr::null_mut(),
                    Some(changed),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
            if hook.is_null() {
                return;
            }
            let mut message: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            unsafe { UnhookWinEvent(hook) };
        })
        .ok()
}
/// The mandatory integrity level of a process (0x2000 medium, 0x3000 high), from its token.
fn integrity(process: windows_sys::Win32::Foundation::HANDLE) -> Option<u32> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        },
        System::Threading::OpenProcessToken,
    };
    let mut token: HANDLE = std::ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    let mut buffer = [0u64; 16];
    let mut needed = 0;
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
            &mut needed,
        )
    };
    unsafe {
        CloseHandle(token);
    }
    if read == 0 {
        return None;
    }
    let label = unsafe { &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL) };
    let sid = label.Label.Sid;
    if sid.is_null() {
        return None;
    }
    let count = unsafe { *GetSidSubAuthorityCount(sid) };
    if count == 0 {
        return None;
    }
    Some(unsafe { *GetSidSubAuthority(sid, u32::from(count) - 1) })
}
pub struct Windows {
    owner: usize,
}
impl Windows {
    /// Clipboard owner must be a window belonging to this Rust host.
    pub fn new(owner: usize) -> Result<Self, &'static str> {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(owner as HWND, &mut pid);
        }
        if owner == 0 || pid != unsafe { GetCurrentProcessId() } {
            return Err("clipboard_owner_invalid");
        }
        Ok(Self { owner })
    }
}
impl Port for Windows {
    type Snapshot = clipboard::Snapshot;
    type Target = Target;
    type Keys = ();
    type Revision = u32;
    fn capture(&mut self) -> Result<Self::Snapshot, &'static str> {
        clipboard::capture(self.owner as HWND)
    }
    fn keys(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn write(&mut self, saved: &Self::Snapshot, text: &str) -> Result<u32, &'static str> {
        clipboard::set_text(self.owner as HWND, saved, text)
    }
    fn revision(&self) -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }
    fn refuses_keys(&self, target: &Target) -> bool {
        target.elevated()
    }
    fn target(&mut self, target: &Target, cancelled: &dyn Fn() -> bool) -> bool {
        let target = target.0 as HWND;
        if target.is_null() || unsafe { IsWindow(target) } == 0 || cancelled() {
            return false;
        }
        if unsafe { GetForegroundWindow() } != target {
            unsafe {
                SetForegroundWindow(target);
            }
            thread::sleep(Duration::from_millis(80));
        } else {
            // Windows' clipboard service reads new data before the recipient.
            // Keep this before the caller rechecks cancellation and revision.
            thread::sleep(Duration::from_millis(60));
        }
        !cancelled() && unsafe { GetForegroundWindow() } == target
    }
    fn press(&mut self, _: &()) -> bool {
        keyboard::press()
    }
    fn release(&mut self, _: &(), pressed: bool) -> bool {
        keyboard::release(pressed)
    }
    fn restore(&mut self, saved: &Self::Snapshot, expected: u32) -> Result<bool, &'static str> {
        clipboard::restore(self.owner as HWND, saved, expected)
    }
    fn wait(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}
