#!/usr/bin/env bash
# AJ-E812/E814: one idle reading of a lab AI Juggler build, by pid only.
# It starts the build on the lab's HOME behind the front app (`open -g`,
# never activated), waits SETTLE seconds, then reads 60 s with the window
# shown, 60 s with it covered by a lab-owned panel just above it (no app
# activated; the window server's uncovered share is checked), and 60 s
# hidden (the lab app hidden by pid). Each reading is the app, its web
# content and its GPU process, % of one core (scripts/qa/idle_cpu.py), with
# the outside events from the app's own log that drop it. The build is
# quit by TERM on its pid at the end; nothing it started is left running.
#
#   scripts/qa/idle_run.sh APP LAB OUT_DIR TAG [SETTLE=75]
#
# It prints one line per reading:
#   TAG shown|covered|hidden ALL app WEB gpu LOAD uncovered=N KEEP|DROP (why)
# Live sessions (`scripts/replica.py live`) are the caller's to start and stop.
set -euo pipefail
app="$1"; lab="$2"; out="$3"; tag="$4"; settle="${5:-75}"
here="$(cd "$(dirname "$0")" && pwd)"
case "$lab" in /tmp/aij-lab-*|/private/tmp/aij-lab-*) ;; *) echo "idle-run: $lab isn't a lab (/tmp/aij-lab-*)" >&2; exit 1 ;; esac
exe="$app/Contents/MacOS/juggler-app"
[ -x "$exe" ] || { echo "idle-run: no $exe" >&2; exit 1; }
cover_bin="${TMPDIR:-/tmp}/aij-window-cover"
if [ ! -x "$cover_bin" ] || [ "$here/window_cover.swift" -nt "$cover_bin" ]; then
  swiftc -O -o "$cover_bin" "$here/window_cover.swift"
fi
mkdir -p "$out"
csv="$out/$tag.csv"; marks="$out/$tag.marks"; : > "$marks"
now() { python3 -c 'import time; print(f"{time.time():.3f}")'; }
mark() { echo "$(now) $*" >> "$marks"; }
pgrep -f "^$exe" > /dev/null && { echo "idle-run: a run of $app is still up" >&2; exit 1; }

open -g -n --env HOME="$lab" "$app"
pid=""
for _ in $(seq 1 50); do
  pid="$(pgrep -n -f "^$exe\$" || true)"
  [ -n "$pid" ] && break
  sleep 0.2
done
[ -n "$pid" ] || { echo "idle-run: the build didn't start" >&2; exit 1; }
mark launch "$pid"
watcher=""; cover=""
cleanup() {
  [ -n "$cover" ] && kill "$cover" 2>/dev/null || true
  [ -n "$watcher" ] && kill "$watcher" 2>/dev/null || true
  kill -TERM "$pid" 2>/dev/null || true
  for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
  kill -9 "$pid" 2>/dev/null || true
  # its crash watcher and anything else started from this copy of the app
  for left in $(pgrep -f "^$exe" || true); do kill -TERM "$left" 2>/dev/null || true; done
}
trap cleanup EXIT
trap 'exit 130' INT TERM
python3 -I "$here/idle_cpu.py" watch "$pid" "$csv" --every 1 --seconds $((settle + 300)) &
watcher=$!
wait_for() { sleep "$1" & wait $!; }
uncovered() { "$cover_bin" share "$pid" | sed -n 's/^share=\([^ ]*\).*/\1/p'; }

wait_for "$settle"
s0="$(uncovered)"; t_shown0="$(now)"; wait_for 60; t_shown1="$(now)"; s1="$(uncovered)"
"$cover_bin" cover "$pid" 80 > "$out/$tag.cover" 2>&1 &
cover=$!
wait_for 10
c0="$(uncovered)"; t_cov0="$(now)"; wait_for 60; t_cov1="$(now)"; c1="$(uncovered)"
kill "$cover" 2>/dev/null || true; wait "$cover" 2>/dev/null || true; cover=""
"$cover_bin" hide "$pid" > /dev/null
wait_for 15
t_hid0="$(now)"; wait_for 60; t_hid1="$(now)"; h1="$(uncovered)"
wait_for 2 # the watcher's next sample, after the last window's end
mark shown "$t_shown0" "$t_shown1" "$s0/$s1"
mark covered "$t_cov0" "$t_cov1" "$c0/$c1"
mark hidden "$t_hid0" "$t_hid1" "-/$h1"
kill "$watcher" 2>/dev/null || true; watcher=""

reading() { # phase t0 t1 shares
  local line verdict why
  line="$(python3 -I "$here/idle_cpu.py" windows "$csv" "$2" "$3" --window "$(python3 -c "print($3 - $2 - 0.01)")" | head -1)"
  verdict="$(python3 -I "$here/idle_cpu.py" events "$pid" "$2" "$3")"
  why="$(echo "$verdict" | grep -vE '^(KEEP|DROP)$' | tr '\n' ';' | sed 's/  */ /g' || true)"
  case "$line" in
    *" - "*|*" -") verdict="DROP"; why="$why no reading: the app quit (an outside close is quit, AJ-E1503);" ;;
  esac
  echo "$tag $1 ${line#*s } uncovered=$4 $(echo "$verdict" | tail -1) ($why)"
}
{
  reading shown "$t_shown0" "$t_shown1" "$s0/$s1"
  reading covered "$t_cov0" "$t_cov1" "$c0/$c1"
  reading hidden "$t_hid0" "$t_hid1" "-/$h1"
} | tee "$out/$tag.readings"
