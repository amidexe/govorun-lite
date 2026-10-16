use crate::{setup_store::{Change, Snapshot, Store}, windows};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

pub fn store(app: &AppHandle) -> Result<Store, &'static str> {
    Ok(Store::new(app.path().app_config_dir().map_err(|_| "setup_unavailable")?.join("setup.json")))
}

#[tauri::command]
pub async fn setup(app: AppHandle, window: WebviewWindow, state: State<'_, Store>,
    change: Option<Change>, reset: Option<bool>) -> Result<Snapshot, &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    let state = state.inner().clone();
    let modified = change.is_some() || reset == Some(true);
    let result = tauri::async_runtime::spawn_blocking(move || state.execute(change, reset == Some(true)))
        .await.map_err(|_| "setup_unavailable")??;
    if modified { let _ = app.emit("govorun:event", serde_json::json!({ "event": "setup.changed", "data": null })); }
    Ok(result)
}
