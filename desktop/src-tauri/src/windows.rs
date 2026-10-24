use serde_json::json;
use std::{sync::{atomic::{AtomicU64, Ordering}, Mutex}, time::Duration};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, webview::NewWindowResponse};

#[derive(Default)]
pub struct Layout(Mutex<Option<bool>>);

pub fn layout(app: &AppHandle, window: &WebviewWindow, home: bool) -> Result<(), &'static str> {
    let state = app.state::<Layout>();
    let mut current = state.0.lock().map_err(|_| "window_unavailable")?;
    if *current == Some(home) { return Ok(()); }
    let (width, height) = if home { (430.0, 590.0) } else { (860.0, 680.0) };
    let minimum = if home { (430.0, 400.0) } else { (760.0, 600.0) };
    window.set_min_size(Some(tauri::LogicalSize::new(minimum.0, minimum.1)))
        .and_then(|_| window.set_size(tauri::LogicalSize::new(width, height)))
        .map_err(|_| "window_unavailable")?;
    *current = Some(home);
    Ok(())
}

/// WebView2 contacts Microsoft by itself about a minute after start (its component
/// updates). Signed out Govorun sends nothing at all (owner, 24 Oct 2026), so its windows
/// never do either; the runtime is updated by Microsoft's own service. All webviews of the
/// app share one browser process, so every window passes exactly these arguments; the
/// first one is Tauri's default.
#[cfg(target_os = "windows")]
pub const BROWSER_ARGUMENTS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
    --disable-background-networking --disable-component-update";

/// A closed main window stays hidden for an instant reopen. If it is still hidden after
/// this long, its webview is released and the next opening builds the window again.
const RELEASE_HIDDEN_AFTER: Duration = Duration::from_secs(5 * 60);
static CLOSES: AtomicU64 = AtomicU64::new(0);

pub fn close_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    let _ = window.hide();
    let close = CLOSES.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(RELEASE_HIDDEN_AFTER).await;
        if CLOSES.load(Ordering::SeqCst) != close { return; }
        let Some(window) = app.get_webview_window("main") else { return };
        // destroy() emits no window events, so the rebuilt window's layout is reset here.
        if matches!(window.is_visible(), Ok(false)) && window.destroy().is_ok() {
            if let Ok(mut current) = app.state::<Layout>().0.lock() { *current = None; }
        }
    });
}

const ROUTES: &[&str] = &["general", "account", "subscription", "styles", "llm", "stats", "dictionary", "diagnostics", "about"];

fn local(url: &Url) -> bool {
    let origin = (url.scheme(), url.host_str(), url.port());
    let bundled = matches!(origin, ("tauri", Some("localhost"), None) |
        ("http" | "https", Some("tauri.localhost"), None));
    let development = cfg!(debug_assertions) && origin == ("http", Some("127.0.0.1"), Some(1420));
    (bundled || development) && matches!(url.path(), "/" | "/index.html")
}

pub fn trusted(window: &WebviewWindow) -> bool {
    window.label() == "main" && window.url().is_ok_and(|url| local(&url))
}

pub fn navigate(app: &AppHandle, route: &str) -> Result<(), &'static str> {
    if route == "home" { show(app, "home", "") }
    else if ROUTES.contains(&route) { show(app, "settings", route) }
    else { Err("invalid_route") }
}

pub fn restore(app: &AppHandle) -> Result<(), &'static str> {
    if let Some(window) = app.get_webview_window("main") { reveal(&window) }
    else { navigate(app, "home") }
}

fn reveal(window: &WebviewWindow) -> Result<(), &'static str> {
    window.unminimize().and_then(|_| window.show()).and_then(|_| window.set_focus())
        .map_err(|_| "window_unavailable")
}

pub fn show(app: &AppHandle, kind: &str, route: &str) -> Result<(), &'static str> {
    if !matches!(kind, "home" | "settings") || (!route.is_empty() && !ROUTES.contains(&route)) {
        return Err("invalid_window");
    }
    if let Some(window) = app.get_webview_window("main") {
        let destination = if kind == "home" { "home" } else if route.is_empty() { "settings" } else { route };
        layout(app, &window, matches!(destination, "home" | "stats"))?;
        window.emit("govorun:event", json!({"event":"navigation","data":{"route":destination}}))
            .map_err(|_| "window_unavailable")?;
        return reveal(&window);
    }
    let destination = if kind == "home" { "home" } else if route.is_empty() { "general" } else { route };
    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(
        format!("index.html?window=main&route={destination}").into(),
    ))
    .title(app.config().product_name.as_deref().unwrap_or("Говорун"))
    .theme(app.state::<crate::appearance::Appearance>().theme())
    .inner_size(if matches!(destination, "home" | "stats") { 430.0 } else { 860.0 }, if matches!(destination, "home" | "stats") { 590.0 } else { 680.0 })
    .min_inner_size(430.0, 400.0)
    .disable_drag_drop_handler()
    .on_navigation(local)
    .on_new_window(|_, _| NewWindowResponse::Deny);
    #[cfg(target_os = "macos")]
    let builder = builder.hidden_title(true);
    #[cfg(target_os = "windows")]
    let builder = builder.additional_browser_args(BROWSER_ARGUMENTS);
    let window = builder.build().map_err(|_| "window_unavailable")?;
    #[cfg(target_os = "windows")]
    if let (Ok(hwnd), Ok(theme)) = (window.hwnd(), window.theme()) { crate::appearance::paint_title_bar(hwnd.0, theme) }
    let _ = crate::tray::refresh_window(app);
    layout(app, &window, matches!(destination, "home" | "stats"))?;
    window.set_focus().map_err(|_| "window_unavailable")
}
