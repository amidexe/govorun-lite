#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "macos")]
    if let Some(status) = govorun_os_runtime::insertion::run_helper(&std::env::args().collect::<Vec<_>>()) {
        std::process::exit(status);
    }
    govorun_desktop_host::run();
}
