#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "macos")]
    if let Some(status) = govorun_os_runtime::insertion::run_helper(&std::env::args().collect::<Vec<_>>()) {
        std::process::exit(status);
    }
    // ggml's Metal residency keep-alive wakes every 5 ms for the whole session even
    // while nothing is recognized; recognition speed is the same without it, as on
    // macOS 14 where residency sets do not exist. Set before the engine loads.
    #[cfg(target_os = "macos")]
    std::env::set_var("GGML_METAL_NO_RESIDENCY", "1");
    govorun_desktop_host::run();
}
