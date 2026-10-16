#!/usr/bin/env bash
set -euo pipefail

url="${1:?usage: fetch-verified.sh URL SHA256 DESTINATION}"
expected="${2:?usage: fetch-verified.sh URL SHA256 DESTINATION}"
destination="${3:?usage: fetch-verified.sh URL SHA256 DESTINATION}"

if [[ ! "$expected" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Expected SHA-256 is not configured." >&2
    exit 2
fi

sha256_file() {
    shasum -a 256 "$1" | awk '{print $1}'
}

if [[ -f "$destination" && "$(sha256_file "$destination")" == "$expected" ]]; then
    printf 'Verified: %s\n' "$destination"
    exit 0
fi

mkdir -p "$(dirname "$destination")"
temporary="$(mktemp "${TMPDIR:-/tmp}/govorun-public-fetch.XXXXXX")"
cleanup() { rm -f -- "$temporary"; }
trap cleanup EXIT
curl --fail --location --proto '=https' --tlsv1.2 --output "$temporary" "$url"
actual="$(sha256_file "$temporary")"
if [[ "$actual" != "$expected" ]]; then
    echo "SHA-256 mismatch." >&2
    exit 1
fi
mv -f -- "$temporary" "$destination"
trap - EXIT
printf 'Verified: %s\n' "$destination"
