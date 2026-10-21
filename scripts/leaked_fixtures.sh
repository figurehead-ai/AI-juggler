#!/usr/bin/env bash
# Test fixtures must not outlive the suite: fails if a fake-desktop started
# from this checkout's build, or a dev-server fixture started from this
# checkout's tests (crates/juggler-app/tests/fixtures/http.server.py), is
# still running after `cargo test`. (Tests stop what they launch through a
# guard, and each fixture exits once its test is gone; this catches
# anything that slips past both.)
#
# `leaked_fixtures.sh --start` (run just before `cargo test`) marks the
# run's start in this worktree's git folder; the check then also fails on
# `aij-e401-<pid>-*` temporary folders made since then by a test process
# that has exited (their tests drop them; older leftovers are ignored).
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
marker="$(git -C "$root" rev-parse --absolute-git-dir)/aij-test-run-started"
if [ "${1:-}" = --start ]; then
  touch "$marker"
  exit 0
fi
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
# Dev-server fixtures: left behind = the test process they follow
# (`--follow PID`) is gone. Tests orphan them on purpose while running, so
# the parent says nothing here.
server="^[^ ]*[Pp]ython[^ ]* $(escape "$root")/crates/juggler-app/tests/fixtures/http\\.server\\.py "
for pid in $(pgrep -f "$server"); do
  follow="$(ps -o command= -p "$pid" | sed -n 's/.*--follow \([0-9][0-9]*\).*/\1/p')"
  if [ -z "$follow" ] || ! kill -0 "$follow" 2>/dev/null; then
    left+="$(ps -o pid=,command= -p "$pid")"$'\n'
  fi
done
# Temporary folders: this run's, whose test process is gone.
if [ -f "$marker" ]; then
  tmp="${TMPDIR:-/tmp}"
  while IFS= read -r dir; do
    pid="$(basename "$dir" | sed -n 's/^aij-e401-\([0-9][0-9]*\)-.*/\1/p')"
    if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
      left+="$dir"$'\n'
    fi
  done < <(find "${tmp%/}" -maxdepth 1 -type d -name 'aij-e401-*' -newer "$marker" 2>/dev/null)
fi
if [ -n "$left" ]; then
  echo "leaked-fixtures: these test fixtures outlived the suite:" >&2
  printf '%s' "$left" >&2
  exit 1
fi
echo "leaked-fixtures: none"
