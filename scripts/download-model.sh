#!/usr/bin/env bash
# Downloads GigaAM v3 ONNX model files into app/src/main/assets/models/gigaam-v3/.
# Total ~327 MB, gitignored — too large for regular git but served via GitHub Release.
# Files are verified by SHA-256 so a truncated or tampered download fails loud.
set -euo pipefail

RELEASE_TAG="model-gigaam-v3"
BASE_URL="https://github.com/amidexe/govorun-lite/releases/download/${RELEASE_TAG}"
DEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/app/src/main/assets/models/gigaam-v3"

mkdir -p "${DEST_DIR}"

# Pinned SHA-256 hashes — must match the files published under the release tag above.
declare -A SHA256=(
  [gigaam_v3_e2e_rnnt_decoder.onnx]="781971998e6a355d6a714f6932a30eab295e7ba0d14fd7e0f78c83b87e811860"
  [gigaam_v3_e2e_rnnt_encoder_int8.onnx]="2cac62d0c270bd128f898f2be1a2d34780d524a6e9483888ebac7b00f97410f1"
  [gigaam_v3_e2e_rnnt_joint.onnx]="602ff7017a93311aad34df1437c8d7f49911353c13d6eae7a6ee7b041339465c"
  [gigaam_v3_e2e_rnnt_tokens.txt]="7ddf22514c42c531358182c81446a8159771e9921019f09ae743ea622d40221d"
)

check_hash() {
  local file="$1" expected="$2"
  local actual
  actual="$(sha256sum "${file}" | awk '{print $1}')"
  [[ "${actual}" == "${expected}" ]]
}

for filename in "${!SHA256[@]}"; do
  dest="${DEST_DIR}/${filename}"
  expected="${SHA256[${filename}]}"

  if [[ -f "${dest}" ]] && check_hash "${dest}" "${expected}"; then
    echo "OK     ${filename}"
    continue
  fi

  echo "Fetch  ${filename}"
  curl -fL --progress-bar -o "${dest}" "${BASE_URL}/${filename}"

  if ! check_hash "${dest}" "${expected}"; then
    actual="$(sha256sum "${dest}" | awk '{print $1}')"
    echo "SHA-256 mismatch for ${filename}" >&2
    echo "  expected ${expected}" >&2
    echo "  got      ${actual}" >&2
    rm -f "${dest}"
    exit 1
  fi
  echo "OK     ${filename}"
done

echo "Model ready in ${DEST_DIR}"
