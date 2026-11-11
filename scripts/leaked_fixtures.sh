#!/usr/bin/env bash
# Test fixtures must not outlive the suite: fails if a fake-desktop, or an
# AI Juggler a test launched, started from this checkout's build is still
# running after `cargo test`. (Tests
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
# A throwaway desktop profile a live test started (places_real's "Lab
# Claude profile", under the system's temporary folder): the app runs
# under launchd, so any one still up after the suite is left behind. On
# 0.4.0 one ran long enough for the installed AI Juggler to record it.
for pid in $(pgrep -f -- '--user-data-dir=(/private)?/var/folders/[^ ]*/Lab [A-Za-z]+ profile'); do
  left+="$(ps -o pid=,command= -p "$pid")"$'\n'
done
# AI Juggler itself, started by a test (rc27's preflight: blank, frozen-
# looking windows on the maintainer's screen, and an AI Juggler Dev they
# had to kill): this run's built binary, or a second_launch copy of it
# ($TMPDIR/aij-second-launch-bin/juggler-app-<test pid>, a folder every
# checkout shares). Left behind = its test is gone (handed to launchd, or
# the copy's test pid no longer runs), or it started after
# AIJ_TESTS_STARTED (epoch seconds; preflight sets it as its tests begin).
# Said by pid, start time and path only.
real() { (cd "$1" 2>/dev/null && pwd -P) || printf '%s' "$1"; }
built="$(real "${CARGO_TARGET_DIR:-$root/target}")"
copies="$(real "${TMPDIR:-/tmp}")/aij-second-launch-bin"
epoch_of() { # a `ps -o lstart=` time, in epoch seconds
  date -j -f '%a %b %d %T %Y' "$1" +%s 2>/dev/null || date -d "$1" +%s 2>/dev/null || echo 0
}
while read -r pid ppid exe; do
  case "$exe" in
    "$built"/debug/juggler-app | "$built"/release/juggler-app) test_pid="" ;;
    "$copies"/juggler-app-*) test_pid="${exe##*/juggler-app-}" ;;
    *) continue ;;
  esac
  started="$(LC_ALL=C ps -o lstart= -p "$pid" 2>/dev/null | sed 's/^ *//; s/ *$//')"
  [ -n "$started" ] || continue # ended meanwhile
  gone=false
  [ "$ppid" = 1 ] && gone=true
  if [ -n "$test_pid" ] && ! kill -0 "$test_pid" 2>/dev/null; then gone=true; fi
  if [ -n "${AIJ_TESTS_STARTED:-}" ] && [ "$(epoch_of "$started")" -ge "$AIJ_TESTS_STARTED" ]; then gone=true; fi
  $gone && left+="pid $pid, started $started: $exe"$'\n'
done < <(ps -axo pid=,ppid=,comm=)
if [ -n "$left" ]; then
  echo "leaked-fixtures: these test fixtures outlived the suite:" >&2
  printf '%s' "$left" >&2
  exit 1
fi
echo "leaked-fixtures: none"
