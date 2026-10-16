fn main() {
    println!("cargo:rerun-if-env-changed=GOVORUN_SHERPA_RUNTIME");
    if std::env::var_os("CARGO_FEATURE_INFERENCE").is_none()
        || std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos")
    {
        return;
    }
    // Supplied by the private build/packaging gate; no model or native binary
    // is committed or downloaded by this adapter.
    let runtime = std::path::PathBuf::from(
        std::env::var_os("GOVORUN_SHERPA_RUNTIME")
            .expect("pinned Sherpa runtime directory required"),
    )
    .canonicalize()
    .expect("verified native runtime directory");
    let sherpa = runtime.join("sherpa-onnx.xcframework/macos-arm64_x86_64");
    assert!(
        sherpa.join("libsherpa-onnx.a").is_file(),
        "pinned Sherpa archive required"
    );
    println!("cargo:rustc-link-search=native={}", sherpa.display());
    println!("cargo:rustc-link-search=native={}", runtime.display());
    println!("cargo:rustc-link-lib=static=sherpa-onnx");
    println!("cargo:rustc-link-lib=dylib=onnxruntime.1.24.4");
    println!("cargo:rustc-link-lib=dylib=c++");
}
