use tauri::AppHandle;

pub use crate::launch_policy::Launch;

fn apply(launch: Launch, app: &AppHandle) -> Result<(), &'static str> {
    match launch {
        Launch::Home => crate::windows::navigate(app, "home"),
        Launch::Settings => crate::windows::show(app, "settings", ""),
        Launch::Background => Ok(()),
    }
}

pub fn second_instance(app: &AppHandle, arguments: Vec<String>) {
    let arguments: Vec<_> = arguments.into_iter().skip(1).collect();
    let handle = app.clone();
    // Creating a WebView synchronously in the Windows message callback can deadlock.
    let _ = app.run_on_main_thread(move || { let _ = open(&handle, arguments); });
}

pub fn open(app: &AppHandle, arguments: Vec<String>) -> Result<(), &'static str> {
    let mut callback = false;
    for argument in &arguments { callback |= crate::callback::submit(app, argument); }
    if callback { Ok(()) } else { apply(Launch::parse(arguments), app) }
}
