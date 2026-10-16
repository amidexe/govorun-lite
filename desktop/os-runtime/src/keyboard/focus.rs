//! Assignment capture is permitted only while this host owns foreground focus.
#[cfg(target_os = "windows")]
pub fn host_focused() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    unsafe {
        let window = GetForegroundWindow();
        let mut pid = 0;
        if window.is_null() {
            return false;
        }
        GetWindowThreadProcessId(window, &mut pid);
        pid == std::process::id()
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn host_focused() -> bool {
    false
}
#[cfg(target_os = "macos")]
pub fn permission_ready() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}
#[cfg(not(target_os = "macos"))]
pub fn permission_ready() -> bool {
    true
}
