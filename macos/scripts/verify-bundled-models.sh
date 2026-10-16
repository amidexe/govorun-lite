#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT/.build/Build/Products/Release/Говорун.app}"
MODEL_DIR="$APP/Contents/Resources/Model"

fail() {
    echo "macOS model bundle check failed: $*" >&2
    exit 1
}

[[ -d "$APP/Contents" ]] || fail "app bundle is missing: $APP"
[[ -d "$MODEL_DIR" ]] || fail "Resources/Model is missing"

verify_model() {
    name="$1"
    expected="$2"
    path="$MODEL_DIR/$name"
    [[ -f "$path" ]] || fail "required model is missing: Model/$name"
    actual="$(shasum -a 256 "$path" | awk '{print $1}')"
    [[ "$actual" == "$expected" ]] || fail "model hash mismatch: Model/$name"
    occurrence_count="$(find "$APP/Contents/Resources" -type f -name "$name" | wc -l | tr -d ' ')"
    [[ "$occurrence_count" == "1" ]] || fail "model must occur exactly once in Resources: $name"
}

verify_model \
    "gigaam-v3-e2e-rnnt-Q8_0.gguf" \
    "78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a"
verify_model \
    "silero_vad.onnx" \
    "9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6"

bundled_count="$(find "$MODEL_DIR" -type f | wc -l | tr -d ' ')"
[[ "$bundled_count" == "2" ]] || fail "Model must contain exactly two reviewed files"
echo "macOS bundled recognition models passed"
