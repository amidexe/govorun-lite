use serde_json::Value;
use std::sync::Mutex;
use tauri::Manager;
use crate::attention_state::Reason;
use tauri::{AppHandle, image::Image, menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}};

const ID: &str = "govorun";
struct State(Mutex<Option<Option<Reason>>>);

fn display_name(app: &AppHandle) -> &str {
    app.config().product_name.as_deref().unwrap_or("Говорун")
}

fn menu(app: &AppHandle, reason: Option<Reason>) -> tauri::Result<Menu<tauri::Wry>> {
    let home = MenuItem::with_id(app, "home", format!("Открыть {}", display_name(app)), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Настройки…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", format!("Завершить {}", display_name(app)), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&home])?;
    if let Some(reason) = reason {
        menu.append(&MenuItem::with_id(app, "attention", reason.caption(), true, None::<&str>)?)?;
    }
    if crate::last_dictation::enabled() {
        menu.append(&MenuItem::with_id(app, "copyLastDictation", "Скопировать последнюю диктовку", true, None::<&str>)?)?;
    }
    menu.append_items(&[&settings, &separator, &quit])?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu(app, None)?;
    app.manage(State(Mutex::new(Some(None))));
    let pixels = crate::tray_state::pixels(include_bytes!(concat!(env!("OUT_DIR"), "/bird.rgba")).to_vec(),
        app.config().identifier == "com.govorun.app.testbackend", cfg!(target_os = "macos"));
    let builder = TrayIconBuilder::with_id(ID);
    #[cfg(target_os = "macos")]
    let builder = builder.title(if app.config().identifier == "com.govorun.app.testbackend" { "Т" } else { "" });
    builder
        .icon(Image::new_owned(pixels, 40, 40))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(crate::tray_state::tooltip(display_name(app), &app.package_info().version.to_string(), None))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "home" => { let _ = crate::windows::navigate(app, "home"); }
                "settings" => { let _ = crate::windows::show(app, "settings", ""); }
                "copyLastDictation" => { let _ = crate::last_dictation::copy(false); }
                "attention" => { if let Some(reason) = crate::attention::reason(app) {
                    let _ = crate::windows::navigate(app, reason.route());
                } }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click {
                button: MouseButton::Left, button_state: MouseButtonState::Up, ..
            }) { let _ = crate::windows::navigate(tray.app_handle(), "home"); }
        })
        .build(app)?;
    Ok(())
}

pub fn update(app: &AppHandle, event: &str, data: &Value) {
    crate::attention::event(app, event, data);
}

pub fn refresh(app: &AppHandle) {
    let reason = crate::attention::reason(app);
    let state = app.state::<State>();
    let Ok(mut displayed) = state.0.lock() else { return; };
    if *displayed == Some(reason) { return; }
    *displayed = Some(reason);
    drop(displayed);
    let tooltip = crate::tray_state::tooltip(display_name(app), &app.package_info().version.to_string(), reason.map(Reason::caption));
    let tooltip: String = tooltip.chars().take(127).collect();
    if let Some(tray) = app.tray_by_id(ID) {
        let test = app.config().identifier == "com.govorun.app.testbackend";
        let pixels = crate::tray_state::pixels(include_bytes!(concat!(env!("OUT_DIR"), "/bird.rgba")).to_vec(),
            test, cfg!(target_os = "macos"));
        #[cfg(not(target_os = "macos"))]
        let pixels = if reason.is_some() { crate::attention_state::mark(pixels) } else { pixels };
        // A replacement NSImage loses its template flag with set_icon. Apply both
        // atomically so status changes keep the system contrast without flicker.
        let _ = tray.set_icon_with_as_template(Some(Image::new_owned(pixels, 40, 40)), cfg!(target_os = "macos"));
        #[cfg(target_os = "macos")]
        let _ = macos_attention_title(&tray, reason.is_some(), app.config().identifier == "com.govorun.app.testbackend");
        let _ = tray.set_tooltip(Some(tooltip));
        if let Ok(menu) = menu(app, reason) { let _ = tray.set_menu(Some(menu)); }
    }
    let _ = refresh_window(app);
}

#[cfg(target_os = "macos")]
fn macos_attention_title(tray: &tauri::tray::TrayIcon, active: bool, test: bool) -> tauri::Result<()> {
    let title = match (active, test) { (true, true) => "! Т", (true, false) => "!", (false, true) => "Т", _ => "" };
    tray.set_title(Some(title))
}

/// Shows or hides «Скопировать последнюю диктовку» after the setting changed.
pub fn rebuild_menu(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(ID), menu(app, crate::attention::reason(app))) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn refresh_window(app: &AppHandle) -> tauri::Result<()> {
    let active = crate::attention::reason(app).is_some();
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        {
            let overlay = active.then(crate::attention_state::overlay);
            window.set_overlay_icon(overlay.map(|pixels| Image::new_owned(pixels, 16, 16)))?;
        }
        #[cfg(target_os = "macos")]
        window.set_badge_label(active.then(|| "!".into()))?;
    }
    Ok(())
}
