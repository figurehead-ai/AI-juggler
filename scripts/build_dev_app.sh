#!/usr/bin/env bash
# The Dev edition's release-mode .app (unsigned, never updates): the
# release candidate. One recipe for this Mac (`just app`) and for CI's
# `rc-dev-app` job, so the app a walk runs is built the same way wherever
# it was built.
#
#   scripts/build_dev_app.sh          the .app, in $CARGO_TARGET_DIR/release/bundle/macos
#   scripts/build_dev_app.sh icon     only the Dev edition's marked icon (`just dev-icon`)
#
# Like the release workflow, it ships `juggler-agent` beside the app
# (AJ-E1704, Tauri's externalBin): without it a launch's hooks report
# nothing and a pool's tools aren't there (the rc4/rc5 walks' "Working",
# "Probably stuck" and scouts that never reached the pool's tools).
set -euo pipefail
cd "$(dirname "$0")/.."

# The marked icon (generated; target/ is not committed), and its .icns, as
# the release workflow makes one: the bundle needs it (a PNG alone fails
# with "No matching IconType"). tauri.dev.conf.json names this folder.
swift scripts/edition_icon.swift crates/juggler-app/icons/icon.png target/edition-icons/dev/icon.png Dev C2410C
cargo tauri icon target/edition-icons/dev/icon.png --output target/edition-icons/dev > /dev/null
[ "${1:-}" = icon ] && exit 0

target="${CARGO_TARGET_DIR:-target}"
triple="$(rustc -vV | sed -n 's/^host: //p')"
cargo build --release -q -p juggler-agentbridge --bin juggler-agent
mkdir -p crates/juggler-app/binaries
cp "$target/release/juggler-agent" "crates/juggler-app/binaries/juggler-agent-$triple"
cargo tauri build --bundles app --config crates/juggler-app/tauri.dev.conf.json \
    --config crates/juggler-app/tauri.bundle.json
python3 scripts/bundle_check.py "$target/release/bundle/macos/AI Juggler Dev.app"
