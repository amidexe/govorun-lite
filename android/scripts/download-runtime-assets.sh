#!/usr/bin/env bash
# Restore runtime resources already published with version 1.x, verified by hash.
set -euo pipefail
SCRIPT_PATH="$(realpath "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "$SCRIPT_PATH")/.." && pwd)"
DEST="$ROOT/app/src/main/assets"
BASE="https://raw.githubusercontent.com/amidexe/govorun-lite/91444be6316a17a05054cd3f815ae15a02e2dda8/app/src/main/assets"
mkdir -p "$DEST"
hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
for entry in \
  'benchmark.wav|d8aaaa18a5098d7c6de0595ae7ac1e64cacd0d4022af3595213bdaf23be77e69' \
  'benchmark_long.wav|1868ece0195dfa9fc2394be24865d1133c8452a8292a397db45ba8c3ed9e01e3' \
  'silero_vad.onnx|9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6'; do
  name="${entry%%|*}"
  expected="${entry#*|}"
  if [[ -f "$DEST/$name" && "$(hash_file "$DEST/$name")" == "$expected" ]]; then
    printf 'OK %s\n' "$name"
    continue
  fi
  temporary="$(mktemp "$DEST/.runtime-download.XXXXXX")"
  trap 'rm -f -- "${temporary:-}"' EXIT
  curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error \
    --output "$temporary" "$BASE/$name"
  [[ "$(hash_file "$temporary")" == "$expected" ]] || {
    printf 'Runtime asset hash mismatch: %s\n' "$name" >&2
    exit 1
  }
  mv -f "$temporary" "$DEST/$name"
  trap - EXIT
  printf 'OK %s\n' "$name"
done
