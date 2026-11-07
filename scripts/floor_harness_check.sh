#!/usr/bin/env bash
# The Floor's harness and its test input (`floor-harness`: AJ-E3513's
# budgets, AJ-E3514's persona walk) never ship:
#  1. no default feature turns it on (the feature graph);
#  2. no release workflow builds with it or with --all-features;
#  3. the shipped binary carries none of its variables
#     (scripts/release_seams_check.sh, run by build-macos.yml on the
#     binary it ships).
# It has to build in release (it measures the release build), so unlike
# the dev bridge it can't refuse to compile there; (3) checks the result.
set -euo pipefail
cd "$(dirname "$0")/.."
on='juggler-app feature "floor-harness"'
features() { cargo tree -p juggler-app -e features -i juggler-app --prefix none "$@" 2>/dev/null; }
if features | grep -qF "$on"; then
  echo "floor-harness: the default build includes the feature" >&2
  exit 1
fi
if ! features --features floor-harness | grep -qF "$on"; then
  echo "floor-harness: cargo tree doesn't show the feature even when asked for" >&2
  exit 1
fi
release=(.github/workflows/build-macos.yml .github/workflows/nightly.yml .github/workflows/promote.yml)
if grep -nE -- '--all-features|floor-harness' "${release[@]}"; then
  echo "floor-harness: a release workflow turns the feature on" >&2
  exit 1
fi
if ! grep -q 'AIJ_FLOOR_INPUT' scripts/release_seams_check.sh; then
  echo "floor-harness: release_seams_check.sh doesn't look for its variables" >&2
  exit 1
fi
echo "floor-harness: off by default, in no release workflow, looked for in the shipped binary"
