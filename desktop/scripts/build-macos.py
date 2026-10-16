#!/usr/bin/env python3
"""Build standard Tauri app/DMG locally; never install or publish."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def run(command, cwd, env):
    subprocess.run(command, cwd=cwd, env=env, check=True)


def main():
    if sys.platform != "darwin":
        raise SystemExit("This build requires macOS on Apple Silicon.")
    root = Path(__file__).resolve().parents[2]
    desktop = root / "desktop"
    env = dict(os.environ)
    sdk = Path(env.get("GOVORUN_SHERPA_RUNTIME", root / "macos/Frameworks")).resolve()
    if not sdk.is_dir():
        run(["make", "setup"], root / "macos", env)
    run(["bash", "scripts/verify-bundled-models.sh"], root / "macos", env)
    target = Path(env.get("CARGO_TARGET_DIR", desktop / "target")).resolve()
    target.mkdir(parents=True, exist_ok=True)
    env.update(CARGO_TARGET_DIR=str(target), GOVORUN_SHERPA_RUNTIME=str(sdk),
               GOVORUN_UPDATE_FEED_URL="https://govorun.club/updates/tauri/test.json")
    for key in ("GOVORUN_ACCOUNT_ORIGIN", "GOVORUN_AUTH_SCHEME"):
        env.pop(key, None)
    framework = sdk / "TranscribeCpp.xcframework/macos-arm64_x86_64/CTranscribe.framework"
    onnx = sdk / "libonnxruntime.1.24.4.dylib"
    if not framework.is_dir() or not onnx.is_file():
        raise SystemExit("Prepared transcribe.cpp / ONNX Runtime SDK is incomplete.")
    key = json.loads((desktop / "src-tauri/update-public-keys.json").read_text())["ordinary"]
    config = {"identifier": "com.govorun.app", "productName": "Говорун",
        "plugins": {"updater": {"pubkey": key}},
        "bundle": {"targets": ["app", "dmg"], "createUpdaterArtifacts": False,
            "resources": {
                str(root / "macos/Govorun/Resources/Model"): "Model/",
                str(root / "macos/LICENSES"): "LICENSES/",
                str(root / "macos/THIRD_PARTY_LICENSES.txt"): "THIRD_PARTY_LICENSES.txt",
                str(desktop / "THIRD_PARTY_LICENSES.txt"): "DESKTOP_THIRD_PARTY_LICENSES.txt"},
            "macOS": {"signingIdentity": "-", "hardenedRuntime": False,
                "infoPlist": str(desktop / "src-tauri/Info.plist"),
                "entitlements": str(root / "macos/Govorun/Govorun.entitlements"),
                "files": {"Frameworks/CTranscribe.framework": str(framework),
                    "Frameworks/libonnxruntime.1.24.4.dylib": str(onnx)}}}}
    config_path = target / "local-tauri-bundle.json"
    config_path.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n")
    run(["npm", "ci"], desktop, env)
    run(["npm", "run", "check"], desktop, env)
    run(["npm", "exec", "--", "tauri", "build", "--ci", "--features", "custom-protocol",
         "--config", str(config_path)], desktop, env)
    print(f"PASS standard Tauri packages: {target / 'release/bundle'}")


if __name__ == "__main__":
    main()
