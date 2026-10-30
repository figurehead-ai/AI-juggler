#!/usr/bin/env bash
# CI's three clippy variants on Linux, from a Mac, in a container (CI's
# build-linux and verify jobs are Linux; macOS-only code paths hide
# Linux warnings). The source is mounted read-only and copied in (the
# Tauri build script writes into it); the target and the registry live
# in named volumes, so a second run is incremental. `--rm` only: no
# container is left behind.
#
# Usage: scripts/linux_clippy.sh
set -euo pipefail
cd "$(dirname "$0")/.."
IMAGE="rust:1.98.1-trixie" # rust-toolchain.toml's version is installed inside
docker run --rm \
  -v "$PWD":/src:ro \
  -v aij-linux-target:/target \
  -v aij-cargo-registry:/usr/local/cargo/registry \
  "$IMAGE" bash -c '
set -u
apt-get update -qq >/dev/null
apt-get install -y -qq --no-install-recommends libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev protobuf-compiler >/dev/null 2>&1
export CARGO_TARGET_DIR=/target CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2
mkdir -p /work && tar -C /src --exclude=./ui/node_modules --exclude=./target -cf - . | tar -C /work -xf - && cd /work
rustup component add clippy >/dev/null 2>&1
rc=0
echo "== clippy";                cargo clippy -q --workspace --all-targets -- -D warnings || rc=1
echo "== clippy (no default)";   cargo clippy -q --workspace --all-targets --no-default-features -- -D warnings || rc=1
echo "== clippy (dev-bridge)";   cargo clippy -q -p juggler-app --all-targets --features dev-bridge -- -D warnings || rc=1
exit $rc
'
