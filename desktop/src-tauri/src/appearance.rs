use crate::windows;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, Theme, WebviewWindow};

pub use govorun_core::boundary_snapshots::{ThemeMode as Mode, AppearanceSnapshot as Snapshot};
fn theme(mode: Mode) -> Option<Theme> {
    match mode { Mode::System => None, Mode::Light => Some(Theme::Light), Mode::Dark => Some(Theme::Dark) }
}

/// Windows 11 tints the system title bar with the user's accent colour, so it does not match
/// the window. Paint it like the sidebar. Windows 10 and contrast themes ignore this attribute
/// and keep the system colour.
#[cfg(target_os = "windows")]
pub fn paint_title_bar(hwnd: *mut core::ffi::c_void, theme: Theme) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};
    // COLORREF is 0x00BBGGRR: --sidebar from theme.css.
    let color: u32 = if theme == Theme::Light { 0x00eb_f0f3 } else { 0x001e_1e1e };
    unsafe { DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR as u32, (&color as *const u32).cast(), 4) };
}

pub struct Appearance { path: PathBuf, value: Mutex<Snapshot> }

impl Appearance {
    pub fn load(app: &AppHandle) -> Result<Self, &'static str> {
        let path = app.path().app_config_dir().map_err(|_| "appearance_unavailable")?.join("appearance.json");
        let mode = fs::read(&path).ok().filter(|bytes| bytes.len() < 64)
            .and_then(|bytes| serde_json::from_slice::<Mode>(&bytes).ok()).unwrap_or_default();
        app.set_theme(theme(mode));
        Ok(Self { path, value: Mutex::new(Snapshot { mode, revision: 0 }) })
    }

    pub fn theme(&self) -> Option<Theme> {
        self.value.lock().ok().and_then(|value| theme(value.mode))
    }
}

#[tauri::command]
pub fn appearance(app: AppHandle, window: WebviewWindow, mode: Option<Mode>) -> Result<Snapshot, &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    let state = app.state::<Appearance>();
    let mut value = state.value.lock().map_err(|_| "appearance_unavailable")?;
    if let Some(mode) = mode {
        // Persist before applying, so a failed save does not pretend the preference was stored.
        let directory = state.path.parent().ok_or("appearance_unavailable")?;
        fs::create_dir_all(directory).map_err(|_| "appearance_save_failed")?;
        let temporary = directory.join("appearance.pending");
        fs::write(&temporary, serde_json::to_vec(&mode).map_err(|_| "appearance_save_failed")?)
            .and_then(|_| fs::rename(&temporary, &state.path)).map_err(|_| "appearance_save_failed")?;
        value.mode = mode; value.revision += 1;
        app.set_theme(theme(mode));
        for window in app.webview_windows().values() {
            let _ = window.set_theme(theme(mode));
            let _ = window.emit("govorun:event", serde_json::json!({ "event": "appearance.changed", "data": *value }));
        }
    }
    Ok(*value)
}
