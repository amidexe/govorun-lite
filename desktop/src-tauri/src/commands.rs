use crate::{service::Services, windows};
use serde_json::{json, Value};
use tauri::{AppHandle, State, WebviewWindow};

#[tauri::command]
pub async fn native_command(app: AppHandle, window: WebviewWindow, state: State<'_, Services>,
    method: String, params: Option<Value>) -> Result<Value, &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    if !govorun_core::features::capabilities::renderer_command_allowed(&method) { return Err("unsupported_command"); }
    let mut params = params.unwrap_or_else(|| json!({}));
    if !params.is_object() || (method == "runtime.snapshot" && params.as_object().is_some_and(|value| !value.is_empty())) {
        return Err("invalid_command");
    }
    if method.starts_with("diagnostics.") {
        return crate::extended_diagnostics::command(&app, &state, &method, params).await;
    }
    if method == "runtime.snapshot" {
        return state.connect(&app).await;
    }
    if method == "hotkeys.action" && params.get("action").and_then(Value::as_str) == Some("start") {
        if !crate::focus::begin_assignment(&window.as_ref().window()) { return Err("window_not_focused"); }
        params["owner"] = json!(window.label());
        params["focused"] = json!(true);
    }
    let started = std::time::Instant::now();
    let result = state.invoke(&method, params).await;
    crate::extended_diagnostics::finished(&app, &method, started, result.as_ref().err().copied());
    result
}

#[tauri::command]
pub async fn native_reconnect(app: AppHandle, window: WebviewWindow, state: State<'_, Services>) -> Result<Value, &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    state.connect(&app).await
}

#[tauri::command]
pub async fn window_layout(app: AppHandle, window: WebviewWindow, home: bool) -> Result<(), &'static str> {
    window_operation(app, window, move |app, window| windows::layout(app, window, home)).await
}

#[tauri::command]
pub async fn window_action(app: AppHandle, window: WebviewWindow, action: String, route: Option<String>) -> Result<(), &'static str> {
    window_operation(app, window, move |app, window| match action.as_str() {
        "home" => windows::navigate(app, "home"),
        "settings" => windows::show(app, "settings", route.as_deref().unwrap_or("")),
        "close" => window.close().map_err(|_| "window_unavailable"),
        "minimize" => window.minimize().map_err(|_| "window_unavailable"),
        _ => Err("unsupported_window_action"),
    }).await
}

async fn window_operation(app: AppHandle, window: WebviewWindow,
    operation: impl FnOnce(&AppHandle, &WebviewWindow) -> Result<(), &'static str> + Send + 'static,
) -> Result<(), &'static str> {
    // An async command must not hold the layout lock while waiting for the UI
    // thread: a simultaneous callback/tray navigation needs that same lock.
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = if windows::trusted(&window) { operation(&handle, &window) } else { Err("invalid_origin") };
        let _ = sender.send(result);
    }).map_err(|_| "window_unavailable")?;
    receiver.await.map_err(|_| "window_unavailable")?
}
