#!/usr/bin/env bash
# Downloads the sherpa-onnx AAR into app/libs/.
# The AAR (47 MB, gitignored) ships the native ASR runtime used for GigaAM v3.
set -euo pipefail

VERSION="1.13.0"
URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${VERSION}/sherpa-onnx-${VERSION}.aar"
EXPECTED_SHA256="28e9b348736d84b610361daf161f5904440d6d516531e896bf9ee8b0ad02764e"
DEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/app/libs"
DEST="${DEST_DIR}/sherpa-onnx.aar"

mkdir -p "${DEST_DIR}"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

if [[ -f "${DEST}" ]] && [[ "$(sha256_file "${DEST}")" == "${EXPECTED_SHA256}" ]]; then
  echo "Verified: ${DEST}"
  exit 0
fi

echo "Downloading sherpa-onnx ${VERSION}…"
temporary="$(mktemp "${DEST_DIR}/.sherpa-onnx.aar.download.XXXXXX")"
trap 'rm -f -- "${temporary}"' EXIT
curl --fail --location --proto '=https' --tlsv1.2 --progress-bar \
  --output "${temporary}" "${URL}"
actual="$(sha256_file "${temporary}")"
if [[ "${actual}" != "${EXPECTED_SHA256}" ]]; then
  echo "SHA-256 mismatch for sherpa-onnx ${VERSION}" >&2
  exit 1
fi
mv -f -- "${temporary}" "${DEST}"
trap - EXIT
echo "Saved to ${DEST}"
