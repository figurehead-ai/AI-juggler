#!/usr/bin/env bash
# Test fixtures must not outlive the suite: fails if a fake-desktop started
# from this checkout's build is still running after `cargo test`. (Tests
# stop what they launch through a guard, and the fixture exits once its
# parent is gone; this catches anything that slips past both.)
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
targets=("$root/")
[ -n "${CARGO_TARGET_DIR:-}" ] && targets+=("$(cd "$CARGO_TARGET_DIR" 2>/dev/null && pwd)/")
sleep 1 # an orphaned fixture follows its parent within moments
left=""
for target in "${targets[@]}"; do
  found="$(pgrep -fl "^${target}.*fake-desktop" || true)"
  [ -n "$found" ] && left+="$found"$'\n'
done
if [ -n "${left//[$'\n']/}" ]; then
  echo "leaked-fixtures: these test fixtures outlived the suite:" >&2
  printf '%s' "$left" >&2
  exit 1
fi
echo "leaked-fixtures: none"
