#!/usr/bin/env bash
# The first open after an update brings the main window (0.4.1's rc8 walk:
# after the data update the main window never came; the updating window,
# drawn in the app's own process, had set AppKit up before Tauri). A
# release candidate check, run by hand or by the RC build:
#
#   scripts/qa/first_open_window_check.sh <app bundle> <old database>
#
# <old database>: a juggler.sqlite from before the update, big enough that
# the update takes over 300 ms so its window shows (the synthetic one
# `cargo test --release -p juggler-app --test first_open_measure -- --ignored`
# keeps with AIJ_KEEP_V49=<file>). It is cloned, never opened itself.
#
# The app runs in a throwaway home (no real data), with no sessions
# restored, and is driven by its pid only (an installed AI Juggler shares
# the process name). Passes when the updating window showed and then the
# main window ("AI Juggler · …") came within the time limit, and a TERM
# quit it cleanly (no reopen). Everything it started is ended, and the
# home removed, however it ends.
set -euo pipefail

app=${1:?usage: first_open_window_check.sh <app bundle> <old database>}
old=${2:?usage: first_open_window_check.sh <app bundle> <old database>}
limit=${AIJ_FIRST_OPEN_LIMIT_S:-90}
[ -d "$app" ] || { echo "first-open: no app at $app" >&2; exit 2; }
[ -f "$old" ] || { echo "first-open: no database at $old" >&2; exit 2; }

# The data folder is the edition's ("AI Juggler Dev" for a Dev build).
name=$(basename "$app" .app)
home=$(mktemp -d "${TMPDIR:-/tmp}/aij-first-open.XXXXXX")
data="$home/Library/Application Support/$name"
mkdir -p "$data"
main=""

cleanup() {
  if [ -n "$main" ]; then
    for child in $(pgrep -P "$main" 2>/dev/null || true); do kill -TERM "$child" 2>/dev/null || true; done
    kill -TERM "$main" 2>/dev/null || true
    sleep 1
    if is_ours "$main"; then kill -KILL "$main" 2>/dev/null || true; fi
  fi
  # A watchdog or window launch of this run outlives nothing: they're
  # this home's, found by their parent being gone or the main pid.
  rm -rf "$home"
}
trap cleanup EXIT

# A clone (APFS: instant, and cold to the file cache); a copy elsewhere.
cp -c "$old" "$data/juggler.sqlite" 2>/dev/null || cp "$old" "$data/juggler.sqlite"

# This check's own app binary: every pid it signals is one it found
# running exactly this (an installed AI Juggler, or another session's
# build, shares the process name).
exe="$(cd "$app" && pwd -P)/Contents/MacOS/juggler-app"
is_ours() { [ "$( (ps -o comm= -p "$1" || true) )" = "$exe" ]; }

windows_of() {
  osascript -e "tell application \"System Events\" to get name of windows of (first process whose unix id is $1)" 2>/dev/null || true
}

now_ms() { python3 -c 'import time; print(int(time.time() * 1000))'; }

# One instance per edition: a launch while another of this edition runs
# hands over to it and ends (the rc11 check failed on a leftover rc10).
# Never touch that one (it may be the maintainer's): say so and stop.
for pid in $(pgrep -x juggler-app || true); do
  case "$( (ps -o comm= -p "$pid" || true) )" in
    *"/$name.app/Contents/MacOS/juggler-app")
      echo "first-open: can't check: another $name runs (pid $pid); quit it first" >&2
      exit 2 ;;
  esac
done
before=" $( (pgrep -x juggler-app || true) | tr '\n' ' ') "
t0=$(now_ms)
load_before=$(sysctl -n vm.loadavg 2>/dev/null | tr -d "{}" | awk "{print \$1}")
open -n --env AIJ_NO_RESTORE=1 --env HOME="$home" "$app"

# The launch's own pid: a juggler-app that wasn't there before, started
# by the system (parent 1), not a window or watchdog launch of it.
for _ in $(seq 1 100); do
  for pid in $(pgrep -x juggler-app || true); do
    case "$before" in *" $pid "*) continue ;; esac
    [ "$( (ps -o ppid= -p "$pid" || true) | tr -d ' ')" = "1" ] && is_ours "$pid" && main=$pid && break
  done
  [ -n "$main" ] && break
  sleep 0.1
done
[ -n "$main" ] || { echo "first-open: FAIL: the app didn't start" >&2; exit 1; }

updating_ms="" main_ms=""
while [ $(( $(now_ms) - t0 )) -lt $(( limit * 1000 )) ]; do
  # The updating window is titled "AI Juggler" (the main one "AI
  # Juggler · …"), in its own launch or, as it was before, in the app's.
  if [ -z "$updating_ms" ]; then
    for pid in "$main" $(pgrep -P "$main" -x juggler-app 2>/dev/null || true); do
      case ", $(windows_of "$pid"), " in
        *", AI Juggler, "*) updating_ms=$(( $(now_ms) - t0 )); break ;;
      esac
    done
  fi
  case ", $(windows_of "$main")" in
    *", AI Juggler ·"*) main_ms=$(( $(now_ms) - t0 )); break ;;
  esac
  kill -0 "$main" 2>/dev/null || { echo "first-open: FAIL: the app ended before its main window came" >&2; exit 1; }
  sleep 0.1
done

if [ -z "$updating_ms" ]; then
  echo "first-open: FAIL: the updating window never showed, so the update path wasn't tested (is the database from before the update, and big enough?)" >&2
  exit 1
fi
if [ -z "$main_ms" ]; then
  echo "first-open: FAIL: the updating window showed at ${updating_ms} ms but no main window came within ${limit} s" >&2
  grep -h "first open:" "$data"/logs/*.log 2>/dev/null | sed 's/^/first-open: the app said: /' >&2 || true
  exit 1
fi

# A TERM quits as asked: the run marked clean, nothing reopened.
kill -TERM "$main"
for _ in $(seq 1 100); do kill -0 "$main" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$main" 2>/dev/null; then
  echo "first-open: FAIL: a TERM didn't quit the app within 10 s" >&2
  exit 1
fi
gone=$main
main=""
[ "$(cat "$data/last-run" 2>/dev/null)" = "clean" ] || { echo "first-open: FAIL: the quit wasn't marked clean (the watchdog would reopen it)" >&2; exit 1; }
sleep 3
for pid in $(pgrep -x juggler-app || true); do
  case "$before" in *" $pid "*) continue ;; esac
  is_ours "$pid" || continue
  echo "first-open: FAIL: a juggler-app ($pid) started after the quit of $gone (a reopen)" >&2
  kill -TERM "$pid" 2>/dev/null || true
  exit 1
done
echo "first-open: OK (updating window at ${updating_ms} ms, main window at ${main_ms} ms, quit clean)"
# The Mac's load around the open: a slow open on a busy Mac isn't a
# regression until it's slow on a quiet one (rc17: 25 s on a loaded Mac).
echo "first-open: load before ${load_before:-?}, after $(sysctl -n vm.loadavg 2>/dev/null | tr -d '{}' | awk '{print $1}')"
# The app's own account of the open (when the window showed, when the
# data was ready), so a slow run says where its time went.
grep -h "first open:" "$data"/logs/*.log 2>/dev/null | sed 's/^/first-open: the app said: /' || true
