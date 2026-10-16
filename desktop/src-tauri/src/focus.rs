use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Manager, Window, WindowEvent};

/// Capture uses the same main-window focus as command admission. The hook
/// reads this observation without making AppKit calls on its worker thread.
#[derive(Default)]
pub struct AssignmentFocus(Arc<AtomicBool>);

pub fn observed(window: &Window, event: &WindowEvent) {
    if window.label() != "main" { return; }
    let focused = match event {
        WindowEvent::Focused(value) => *value,
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => false,
        _ => return,
    };
    window.state::<AssignmentFocus>().0.store(focused, Ordering::Release);
}

pub fn assignment_guard(app: &AppHandle) -> Arc<dyn Fn() -> bool + Send + Sync> {
    #[cfg(target_os = "windows")]
    { let _ = app; Arc::new(govorun_os_runtime::keyboard::host_focused) }
    #[cfg(not(target_os = "windows"))]
    {
        let focused = app.state::<AssignmentFocus>().0.clone();
        Arc::new(move || focused.load(Ordering::Acquire))
    }
}

pub fn begin_assignment(window: &Window) -> bool {
    let focused = window.label() == "main" && is_focused(window);
    window.state::<AssignmentFocus>().0.store(focused, Ordering::Release);
    focused
}

/// WebView2 can own keyboard focus while the containing Tao window reports a
/// focus loss. Shortcut capture belongs to the foreground top-level window.
pub fn is_focused(window: &Window) -> bool {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetForegroundWindow() -> *mut core::ffi::c_void;
        }
        window.hwnd().is_ok_and(|handle| {
            let foreground = unsafe { GetForegroundWindow() };
            !foreground.is_null() && foreground == handle.0 as *mut core::ffi::c_void
        })
    }
    #[cfg(not(target_os = "windows"))]
    window.is_focused().unwrap_or(false)
}
