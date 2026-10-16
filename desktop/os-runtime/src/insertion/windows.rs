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
