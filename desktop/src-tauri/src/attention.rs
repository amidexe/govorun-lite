use crate::attention_state::{Attention, Reason};
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct State(Mutex<Attention>);

pub fn reason(app: &AppHandle) -> Option<Reason> {
    app.state::<State>().0.lock().ok().and_then(|value| value.reason())
}

pub fn event(app: &AppHandle, event: &str, data: &Value) {
    if !matches!(event, "runtime.snapshot" | "runtime.unavailable" | "updates.changed") { return; }
    {
        let state = app.state::<State>();
        let Ok(mut value) = state.0.lock() else { return; };
        value.runtime(event, data);
        if event == "updates.changed" { value.updates(data); }
    }
    crate::tray::refresh(app);
}

pub fn apply_window_badge(app: &AppHandle) -> tauri::Result<()> { crate::tray::refresh_window(app) }
