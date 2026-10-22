#!/usr/bin/env bash
# AJ-E2001: installs the pinned, prebuilt tauri-cli for the release jobs —
# Tauri's own release binary, checked against its SHA-256, so nothing is
# compiled (no dependency's build script runs) and the version can't
# drift. Prints the directory to put on PATH.
#
#   install_tauri_cli.sh DEST_DIR
#
# To move to a new version: the release `tauri-cli-vX.Y.Z` on
# github.com/tauri-apps/tauri lists each asset's digest
# (`gh release view tauri-cli-vX.Y.Z --repo tauri-apps/tauri --json assets`).
set -euo pipefail

VERSION=2.12.2
case "$(uname -m)" in
  arm64) asset=cargo-tauri-aarch64-apple-darwin.zip
         sha256=4a27a61b06e688308a5c17333008b6df52fd5780fa7a313a9ce8fb4a80dfb9ac ;;
  x86_64) asset=cargo-tauri-x86_64-apple-darwin.zip
          sha256=0f81160909873e2c37c1856d08be50603dc908acd67e08e4ee9c505b90ff5e16 ;;
  *) echo "::error::no pinned tauri-cli for $(uname -m)" >&2; exit 1 ;;
esac

dest="$1"
mkdir -p "$dest"
zip="$dest/$asset"
curl --fail --silent --show-error --location --retry 3 --proto '=https' --tlsv1.2 \
  -o "$zip" "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v${VERSION}/${asset}"
echo "${sha256}  ${zip}" | shasum -a 256 -c - >&2
unzip -q -o "$zip" -d "$dest"
rm -f -- "$zip"
chmod 755 "$dest/cargo-tauri"
"$dest/cargo-tauri" --version | grep -qx "tauri-cli ${VERSION}" || {
  echo "::error::cargo-tauri is not ${VERSION}" >&2
  exit 1
}
echo "$dest"
