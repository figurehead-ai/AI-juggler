#!/usr/bin/env bash
# AJ-E805 R1: cold start to an interactive shell on a release build, at
# the scale the product promises (10 Accounts, 50 Instances, 20 Projects).
#
#   scripts/qa/release_cold_start.sh <app bundle> <fixture folder> [runs]
#
# <app bundle>: a release build of AI Juggler Dev (CI's rc-dev-app,
#   `scripts/qa/fetch_rc.sh <commit>`); never an installed AI Juggler.
# <fixture folder>: the seeded data folder the backend load test keeps:
#   AIJ_KEEP_LOAD_FIXTURE=<folder> cargo test -p juggler-app --test load_budgets backend_cold_start_at_scale
#   Synthetic, no real data.
#
# Each run gets its own throwaway home holding a fresh copy of the fixture
# as the edition's data folder, so every run is a cold start of the app on
# the same data. One warm-up run first (a new build's first launch waits
# for macOS's scan of it), not counted; then [runs] (default 5), median.
# The probe (cold_start_probe.swift, compiled into the throwaway folder)
# opens the app as a double-click does, times it to the main window's
# "AI Juggler · …" title, and quits it. Everything it started is ended and
# every home removed, however it ends; it never signals a process that
# isn't running this bundle's binary.
set -euo pipefail
app=${1:?usage: release_cold_start.sh <app bundle> <fixture folder> [runs]}
fixture=${2:?usage: release_cold_start.sh <app bundle> <fixture folder> [runs]}
runs=${3:-5}
here="$(cd "$(dirname "$0")" && pwd)"
[ -x "$app/Contents/MacOS/juggler-app" ] || { echo "cold-start: no app at $app" >&2; exit 2; }
[ -f "$fixture/juggler.sqlite" ] || { echo "cold-start: no juggler.sqlite in $fixture" >&2; exit 2; }
name=$(basename "$app" .app)
case "$name" in
  "AI Juggler Dev") ;;
  *) echo "cold-start: $name isn't a Dev build; measure a release candidate (fetch_rc.sh), never an installed app" >&2; exit 2 ;;
esac
exe="$(cd "$app" && pwd -P)/Contents/MacOS/juggler-app"

# One instance per edition: a launch while another Dev runs hands over to
# it. Never touch that one: say so and stop.
for pid in $(pgrep -x juggler-app || true); do
  case "$( (ps -o comm= -p "$pid" || true) )" in
    *"/$name.app/Contents/MacOS/juggler-app")
      echo "cold-start: another $name runs (pid $pid); quit it first" >&2
      exit 2 ;;
  esac
done

work=$(mktemp -d "${TMPDIR:-/tmp}/aij-cold-start.XXXXXX")
cleanup() {
  for pid in $(pgrep -x juggler-app || true); do
    [ "$( (ps -o comm= -p "$pid" || true) )" = "$exe" ] && kill -TERM "$pid" 2>/dev/null || true
  done
  rm -rf "$work"
}
trap cleanup EXIT

swiftc -O "$here/cold_start_probe.swift" -o "$work/probe" 2>&1 | grep -v "^$" >&2 || true
[ -x "$work/probe" ] || { echo "cold-start: the probe didn't build" >&2; exit 2; }

one() {
  local home="$work/home-$1"
  local data="$home/Library/Application Support/$name"
  mkdir -p "$data"
  ditto "$fixture" "$data"
  local out
  out=$("$work/probe" "$app" "$home" 60) || { echo "cold-start: run $1 failed" >&2; exit 1; }
  # The app is gone before its home goes.
  sleep 1
  rm -rf "$home"
  echo "$out"
}

echo "cold-start: $(sw_vers -productName) $(sw_vers -productVersion), $(sysctl -n hw.model), $(sysctl -n hw.ncpu) cores, load $(sysctl -n vm.loadavg | tr -d '{}' | awk '{print $1}')"
echo "cold-start: app $(defaults read "$app/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo '?'), fixture $(shasum -a 256 "$fixture/juggler.sqlite" | cut -c1-12)"
line=$(one warmup) || exit 1
echo "cold-start: warm-up $line"
results=()
for i in $(seq 1 "$runs"); do
  line=$(one "$i") || exit 1
  echo "cold-start: run $i $line"
  results+=("$line")
done
printf '%s\n' "${results[@]}" | python3 -I -c '
import json, statistics, sys
rows = [json.loads(l) for l in sys.stdin if l.strip()]
for key in ("open_to_process_ms", "process_to_shell_ms", "open_to_shell_ms"):
    values = sorted(r[key] for r in rows)
    print(f"cold-start: {key}: median {statistics.median(values):.0f} (runs {values})")
mid = statistics.median(r["open_to_shell_ms"] for r in rows)
verdict = "PASS" if mid < 2500 else "FAIL"
print(f"cold-start: {verdict} open to an interactive shell, median {mid:.0f} ms (budget 2500 ms)")
'
