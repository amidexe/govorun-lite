mod build_update_feed;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    // Tauri embeds the web assets into the host binary. UI-only edits must
    // invalidate the Rust cache on both platforms, including renamed Vite files.
    println!("cargo:rerun-if-changed=../dist");
    println!("cargo:rerun-if-env-changed=GOVORUN_UPDATE_FEED_URL");
    println!("cargo:rerun-if-env-changed=GOVORUN_BUILD_ID");
    println!("cargo:rerun-if-changed=build_update_feed.rs");
    let packaged = std::env::var("PROFILE").as_deref() == Ok("release")
        || std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some();
    build_update_feed::require_update_feed(
        packaged, std::env::var("GOVORUN_UPDATE_FEED_URL").ok().as_deref(),
    ).expect("refusing to build an application that cannot receive updates");
    // The UI and native tray share one vector. Rasterization is a build dependency only.
    let bird = "../src/assets/bird.svg";
    println!("cargo:rerun-if-changed={bird}");
    let tree = resvg::usvg::Tree::from_data(&std::fs::read(bird).expect("bird source"),
        &resvg::usvg::Options::default()).expect("valid bird vector");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(40, 40).expect("tray dimensions");
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(40.0 / 24.0, 40.0 / 24.0),
        &mut pixmap.as_mut());
    std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bird.rgba"),
        pixmap.data()).expect("generated tray pixels");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["native_command", "native_reconnect", "window_action", "window_layout", "appearance", "setup", "erase_local_data", "updates", "recording_indicator"]),
    )).expect("build desktop application permissions");
}
