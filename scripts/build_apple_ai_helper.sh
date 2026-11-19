#!/usr/bin/env bash
# ADR-0033 §4.1, AJ-E3802: AI Juggler's Apple helper, `juggler-apple-ai`,
# built with the system's Swift compiler, outside cargo, so the app stays
# Swift-free and `cargo run` never needs Swift. Without the helper the app
# says "helper missing" and every AI job keeps its plain path.
#
#   scripts/build_apple_ai_helper.sh [debug|release] [out-dir]
#
# Writes the helper beside the app's executable in the shared target
# ($CARGO_TARGET_DIR/<profile>/juggler-apple-ai), where the app looks for
# it. arm64 only: the system model needs Apple silicon. Deployment target
# macOS 14, the system model used only from macOS 26 (weak-linked).
# No-op off macOS. Nothing is downloaded.
set -euo pipefail
cd "$(dirname "$0")/.."

[ "$(uname)" = Darwin ] || { echo "build_apple_ai_helper: macOS only; skipped"; exit 0; }

profile="${1:-debug}"
target="${CARGO_TARGET_DIR:-target}"
out="${2:-$target/$profile}"
mkdir -p "$out"

swiftc -O -parse-as-library \
  -target arm64-apple-macos14.0 \
  -Xlinker -weak_framework -Xlinker FoundationModels \
  crates/juggler-app/helpers/juggler-apple-ai.swift \
  -o "$out/juggler-apple-ai"
chmod 0755 "$out/juggler-apple-ai"
echo "build_apple_ai_helper: $out/juggler-apple-ai"
