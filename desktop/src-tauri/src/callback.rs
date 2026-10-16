use crate::service::Services;
use serde_json::json;
use std::{collections::VecDeque, sync::{Mutex, atomic::{AtomicBool, Ordering}}};
use tauri::{AppHandle, Manager, Url};

pub struct Callback { url: Url, billing: bool }

/// The shared Rust auth flow verifies profile, pending state and PKCE.
/// Billing returns only trigger reconciliation with the existing server state.
pub fn parse(value: &str) -> Option<Callback> {
    if value.len() > 4096 || value.bytes().any(|byte| byte.is_ascii_control()) { return None; }
    let url = Url::parse(value).ok()?;
    if !url.username().is_empty() || url.password().is_some() || url.port().is_some() ||
        url.fragment().is_some() || matches!(url.scheme(), "http" | "https" | "file") {
        return None;
    }
    // A bank return is only a request to reconcile. Query values never grant access.
    if url.host_str() == Some("billing") && url.path() == "/tbank-return" {
        return Some(Callback { url, billing: true });
    }
    if url.host_str() != Some("auth") || !matches!(url.path(), "/yandex" | "/vk") { return None; }
    let query: Vec<_> = url.query_pairs().collect();
    if query.len() != 2 || !["state", "handoff"].iter().all(|key| {
        query.iter().filter(|(name, value)| name.as_ref() == *key && !value.is_empty() && value.len() <= 2048).count() == 1
    }) { return None; }
    Some(Callback { url, billing: false })
}

#[derive(Default)]
pub struct Callbacks {
    waiting: Mutex<VecDeque<Callback>>,
    draining: AtomicBool,
}

pub fn submit(app: &AppHandle, value: &str) -> bool {
    let Some(callback) = parse(value) else { return false; };
    let state = app.state::<Callbacks>();
    {
        let mut waiting = state.waiting.lock().unwrap();
        if waiting.iter().any(|item| item.url == callback.url) { return true; }
        if waiting.len() >= 8 { return false; }
        waiting.push_back(callback);
    }
    drain(app);
    true
}

pub fn drain(app: &AppHandle) {
    let Some(services) = app.try_state::<Services>() else { return; };
    if services.is_stopping() { return; }
    if app.state::<Callbacks>().draining.swap(true, Ordering::SeqCst) { return; }
    let services = services.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let callback = {
                let state = app.state::<Callbacks>();
                let mut waiting = state.waiting.lock().unwrap();
                match waiting.pop_front() {
                    Some(callback) => callback,
                    None => { state.draining.store(false, Ordering::SeqCst); return; }
                }
            };
            let Ok(snapshot) = services.connect(&app).await else {
                restore_window(&app);
                continue;
            };
            if snapshot.get("callbackScheme").and_then(|value| value.as_str()) != Some(callback.url.scheme()) {
                continue;
            }
            restore_window(&app);
            // URL values never enter renderer events, logs or command results.
            if callback.billing {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || { let _ = crate::windows::navigate(&handle, "subscription"); });
            }
            let method = if callback.billing { "billing.callback" } else { "account.callback" };
            let _ = services.invoke(method, json!({"url": callback.url.as_str()})).await;
        }
    });
}

fn restore_window(app: &AppHandle) {
    if app.state::<Services>().is_stopping() { return; }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if !handle.state::<Services>().is_stopping() { let _ = crate::windows::restore(&handle); }
    });
}
