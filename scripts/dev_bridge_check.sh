#!/usr/bin/env bash
# The read-only test bridge (docs/qa/DEV-BRIDGE.md) never ships:
#  1. no default feature turns `dev-bridge` on (the feature graph);
#  2. no release workflow builds with it or with --all-features;
#  3. a release build with it doesn't compile (lib.rs compile_error!).
set -euo pipefail
cd "$(dirname "$0")/.."
on='juggler-app feature "dev-bridge"'
# The features juggler-app is built with (its own, inverted).
features() { cargo tree -p juggler-app -e features -i juggler-app --prefix none "$@" 2>/dev/null; }
if features | grep -qF "$on"; then
  echo "dev-bridge: the default build includes the feature" >&2
  exit 1
fi
# The check can see the feature when it is on.
if ! features --features dev-bridge | grep -qF "$on"; then
  echo "dev-bridge: cargo tree doesn't show the feature even when asked for" >&2
  exit 1
fi
release=(.github/workflows/build-macos.yml .github/workflows/nightly.yml .github/workflows/promote.yml)
if grep -nE -- '--all-features|dev-bridge' "${release[@]}"; then
  echo "dev-bridge: a release workflow turns the feature on" >&2
  exit 1
fi
if [[ "${1:-}" != "--quick" ]]; then
  log=$(mktemp)
  trap 'rm -f "$log"' EXIT
  if cargo check --release -q -p juggler-app --features dev-bridge >"$log" 2>&1; then
    echo "dev-bridge: a release build with the feature compiled" >&2
    exit 1
  fi
  if ! grep -q 'the `dev-bridge` feature is for debug builds only' "$log"; then
    echo "dev-bridge: the release check failed for another reason:" >&2
    tail -20 "$log" >&2
    exit 1
  fi
fi
echo "dev-bridge: off by default, in no release workflow, refused by release builds"
