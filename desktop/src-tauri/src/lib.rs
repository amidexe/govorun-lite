pub mod commands;
pub mod extended_diagnostics;
pub mod system_diagnostics;
mod installation_diagnostics;
pub mod focus;
pub mod appearance;
pub mod callback;
pub mod launch;
mod launch_policy;
pub mod setup;
pub mod erase;
pub mod setup_store;
pub mod model_store;
mod updates;
mod update_install;
mod update_asset;
mod update_cancel;
mod update_apply;
pub mod profile;
mod service;
pub mod features;
#[path="features/hotkeys/gesture.rs"]
pub(crate) mod hotkey_gestures;
#[path="features/hotkeys/physical.rs"]
pub(crate) mod physical_hotkeys;
pub mod agent;
mod persistence;
mod account_host;
#[cfg(any(target_os="macos",target_os="windows"))]
#[path="features/dictation/local.rs"]
mod local_dictation;
#[path="features/dictation/connection.rs"]
mod prepared_connection;
#[cfg(any(target_os="macos",target_os="windows"))]
#[path="features/dictation/runtime.rs"]
mod recording_runtime;
#[cfg(not(feature = "private-fixtures"))]
mod recording_indicator;
#[cfg(feature = "private-fixtures")]
pub mod recording_indicator;
mod recording_host;
mod benchmark_host;
mod recording_feedback;
#[path="features/permission_ports.rs"]
mod recording_setup;
mod recording_origin;
mod recording_position;
mod account_storage;
mod tray;
pub mod last_dictation;
pub mod attention;
mod attention_state;
mod tray_state;
mod windows;

use service::Services;
#[cfg(feature = "private-fixtures")]
pub use service::Services as PrivateServices;
#[cfg(feature = "private-fixtures")]
pub fn with_private_runtime_adapter(builder: tauri::Builder<tauri::Wry>,
    adapter: fn(serde_json::Value) -> serde_json::Value) -> tauri::Builder<tauri::Wry> {
    builder.manage(service::PrivateRuntimeAdapter(adapter))
}
#[cfg(feature = "private-fixtures")]
pub fn set_private_callback_adapter(app: &tauri::AppHandle,
    adapter: fn(&str, serde_json::Value) -> Option<serde_json::Value>) {
    use tauri::Manager;
    app.state::<Services>().set_private_callback_adapter(adapter);
}
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Manager, RunEvent, WindowEvent};

struct Lifecycle { quitting: AtomicBool, stopped: AtomicBool }

pub fn builder() -> tauri::Builder<tauri::Wry> {
    let builder = tauri::Builder::default();
    // Handy owns keyboard input. Tao Raw Input can hide presses from its hook
    // while our WebView has focus (tauri-apps/wry#1761).
    #[cfg(target_os = "windows")]
    let builder = builder.device_event_filter(tauri::DeviceEventFilter::Always);
    builder
        .manage(recording_indicator::State::default())
        .manage(callback::Callbacks::default())
        .manage(windows::Layout::default())
        .manage(focus::AssignmentFocus::default())
        .manage(attention::State::default())
        .plugin(tauri_plugin_single_instance::init(|app, arguments, _| {
            launch::second_instance(app, arguments);
        }))
        .invoke_handler(tauri::generate_handler![commands::native_command, commands::native_reconnect, commands::window_action, commands::window_layout, appearance::appearance, setup::setup, erase::erase_local_data, updates::updates, recording_indicator::recording_indicator])
        .on_window_event(|window, event| {
            focus::observed(window, event);
            if matches!(event, WindowEvent::Focused(true)) {
                if let Some(services) = window.app_handle().try_state::<Services>() {
                    let services = services.inner().clone();
                    tauri::async_runtime::spawn(async move { services.refresh_account_on_focus().await; });
                }
            }
            if matches!(event, WindowEvent::CloseRequested { .. })
                || (matches!(event, WindowEvent::Focused(false))
                    && (!cfg!(target_os = "windows") || !focus::is_focused(window))) {
                if let Some(services) = window.app_handle().try_state::<Services>() {
                    let services = services.inner().clone();
                    let owner = window.label().to_string();
                    tauri::async_runtime::spawn(async move {
                        let _ = services.invoke("hotkeys.cancel", serde_json::json!({ "owner": owner })).await;
                    });
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            app.handle().plugin(tauri_plugin_updater::Builder::new()
                .pubkey(update_install::public_key(&app.config().identifier)?).build())?;
            #[cfg(target_os = "macos")]
            {
                app.handle().plugin(tauri_nspanel::init())?;
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            app.manage(appearance::Appearance::load(app.handle())?);
            app.manage(setup::store(app.handle())?);
            app.manage(updates::Updates::load(app.handle())?);
            app.manage(extended_diagnostics::ExtendedDiagnostics::load(app.handle()));
            let services = Services::new(app.handle());
            app.manage(services.clone());
            callback::drain(app.handle());
            app.manage(Lifecycle { quitting: AtomicBool::new(false), stopped: AtomicBool::new(false) });
            tray::create(app.handle())?;
            launch::open(app.handle(), std::env::args().skip(1).collect())?;
            // Dictation must remain available when launched without a WebView.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if services.connect(&handle).await.is_err() {
                    tray::update(&handle, "runtime.unavailable", &serde_json::Value::Null);
                }
            });
            Ok(())
        })
}

pub fn lifecycle(app: &tauri::AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, code, .. } => {
            if app.state::<Lifecycle>().stopped.load(Ordering::SeqCst) { return; }
            api.prevent_exit();
            // Closing the last window leaves the native service and tray running.
            if code.is_none() { return; }
            if !app.state::<Lifecycle>().quitting.swap(true, Ordering::SeqCst) {
                let services = app.state::<Services>().inner().clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    services.stop().await;
                    app.state::<Lifecycle>().stopped.store(true, Ordering::SeqCst);
                    app.exit(code.unwrap_or(0));
                });
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            for url in urls { callback::submit(app, url.as_str()); }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => { let _ = windows::navigate(app, "home"); }
        _ => {}
    }
}

pub fn run() {
    let context = tauri::generate_context!();
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let bundled_mode = !cfg!(debug_assertions) || cfg!(feature = "packaged-native");
    if arguments == ["--desktop-build-info"] {
        println!("{}", serde_json::json!({
            "schemaVersion": 1, "identifier": context.config().identifier,
            "version": context.config().version,
            "nativeMode": if bundled_mode { "bundled" } else { "inspection" },
        }));
        return;
    }
    if arguments == ["--desktop-runtime-info"] {
        let runtime = tokio::runtime::Builder::new_current_thread().build()
            .expect("stateless package probe runtime");
        let result = runtime.block_on(features::runtime::inspection::report(
            if cfg!(target_os = "macos") { "macos" } else { "windows" },
            &context.config().identifier,
            context.config().product_name.as_deref().unwrap_or("Говорун"),
            env!("CARGO_PKG_VERSION"), option_env!("GOVORUN_BUILD_ID").unwrap_or("")));
        match result {
            Ok(report) => println!("{report}"),
            Err(code) => { eprintln!("{code}"); std::process::exit(19); }
        }
        return;
    }
    builder().build(context)
        .expect("start Govorun desktop interface").run(lifecycle);
}
