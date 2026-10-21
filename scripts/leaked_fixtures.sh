#!/usr/bin/env bash
# Test fixtures must not outlive the suite: fails if a fake-desktop started
# from this checkout's build is still running after `cargo test`. (Tests
# stop what they launch through a guard, and the fixture exits once its
# parent is gone; this catches anything that slips past both.)
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
# This run's own build only: its target folder (CARGO_TARGET_DIR, or
# target/ here, which tests reach as crates/<crate>/../../target). Other
# worktrees under .claude/worktrees share the checkout's path prefix, so
# matching on the prefix alone would blame their runs on this one.
escape() { printf '%s' "$1" | sed 's/[][\.*^$+?(){}|]/\\&/g'; }
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  pattern="^$(escape "$(cd "$CARGO_TARGET_DIR" 2>/dev/null && pwd)")/[^ ]*fake-desktop"
else
  pattern="^$(escape "$root")/(crates/[^/ ]+/\\.\\./\\.\\./)?target/[^ ]*fake-desktop"
fi
sleep 1 # an orphaned fixture follows its parent within moments
# Left behind = orphaned (parent pid 1). A fixture whose test is still
# running (another run sharing the target folder) has a live parent.
left=""
for pid in $(pgrep -f "$pattern"); do # pgrep patterns are extended regexes
  [ "$(ps -o ppid= -p "$pid" | tr -d ' ')" = 1 ] && left+="$(ps -o pid=,command= -p "$pid")"$'\n'
done
if [ -n "$left" ]; then
  echo "leaked-fixtures: these test fixtures outlived the suite:" >&2
  printf '%s' "$left" >&2
  exit 1
fi
echo "leaked-fixtures: none"
