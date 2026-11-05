#!/usr/bin/env bash
# AJ-E3301: a 0.4 scenario's lab — a throwaway HOME shaped like a heavy
# real Mac (the replica) with the scenario's own state seeded into AI
# Juggler Dev's data folder there, and its walk sheet ready. Every walk
# starts here and ends with teardown: the lab's processes stopped (found by
# their open files under the lab, never by reading environments), the lab
# folder removed, nothing left running.
#
#   scripts/qa/lab_scenario.sh setup P1 [--home /tmp/aij-lab-p1] [--import]
#   scripts/qa/lab_scenario.sh open  P1 [--home …]   # AI Juggler Dev on the lab
#   scripts/qa/lab_scenario.sh bridge P1 [--home …]  # its backend + the read-only bridge
#   scripts/qa/lab_scenario.sh teardown P1 [--home …]
#
# --import brings the replica's setups in where they are first (as
# `just dev-bridge-data` does on a real Mac), for walks on its Apps.
#
# The HOME must be under /tmp and contain "aij-lab"; a real HOME is refused.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cmd="${1:-}"; id="${2:-}"
[ -n "$cmd" ] && [ -n "$id" ] || { sed -n '2,13p' "$0"; exit 2; }
shift 2
lab="/tmp/aij-lab-$(echo "$id" | tr '[:upper:]' '[:lower:]')"
import=0
while [ $# -gt 0 ]; do
  case "$1" in
    --home) lab="$2"; shift 2 ;;
    --import) import=1; shift ;;
    *) echo "lab-scenario: unknown option $1" >&2; exit 2 ;;
  esac
done
# Cargo and rustup keep their own homes when HOME is the lab's.
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
case "$lab" in
  /tmp/*aij-lab*|/private/tmp/*aij-lab*) ;;
  *) echo "lab-scenario: $lab isn't a lab (under /tmp, named aij-lab…) — refusing" >&2; exit 1 ;;
esac
data="$lab/Library/Application Support/AI Juggler Dev"

case "$cmd" in
  setup)
    [ -e "$lab" ] && { echo "lab-scenario: $lab exists — tear it down first" >&2; exit 1; }
    python3 -I "$root/scripts/qa/scenario.py" seed "$id" > /dev/null # the scenario exists
    python3 -I "$root/scripts/replica.py" build --home "$lab"
    if [ "$import" = 1 ]; then
      (cd "$root" && HOME="$lab" cargo run -q --offline -p juggler-app --features dev-bridge --example dev_walk_data)
    fi
    mkdir -p "$data"
    python3 -I "$root/scripts/qa/scenario.py" seed "$id" > "$lab/scenario-seed.json"
    (cd "$root" && AIJ_QA_SEED="$lab/scenario-seed.json" AIJ_QA_DATA="$data" AIJ_QA_HOME="$lab" \
      cargo test -q --offline -p juggler-app --test qa_scenario_seed -- --ignored --nocapture 2>&1 | grep '^seed:' || true)
    sheet="$root/docs/qa/0.4-walks/$id.md"
    if [ ! -e "$sheet" ]; then python3 -I "$root/scripts/qa/scenario.py" sheet "$id" --out "$sheet"; echo "lab-scenario: walk sheet $sheet"; fi
    echo "lab-scenario: $id is set up in $lab — open it with: scripts/qa/lab_scenario.sh open $id"
    ;;
  open)
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    app="${AIJ_DEV_APP:-$root/target/release/bundle/macos/AI Juggler Dev.app}"
    [ -d "$app" ] || app="${CARGO_TARGET_DIR:-$root/target}/release/bundle/macos/AI Juggler Dev.app"
    [ -d "$app" ] || { echo "lab-scenario: no AI Juggler Dev build (just build-dev, or set AIJ_DEV_APP)" >&2; exit 1; }
    # Its own HOME: the data folder, the vendors' folders and the sign-ins are the lab's.
    HOME="$lab" "$root/scripts/qa/lab_env.sh" "$app/Contents/MacOS/$(defaults read "$app/Contents/Info" CFBundleExecutable)" &
    echo "lab-scenario: AI Juggler Dev on $lab (pid $!)"
    ;;
  bridge)
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    # The Dev backend (a debug build is always Dev) on the lab's HOME, with
    # the read-only bridge; its page needs the UI on localhost:5173.
    (cd "$root" && HOME="$lab" AIJ_DEV_BRIDGE=1 cargo run -q --offline -p juggler-app --features dev-bridge) &
    echo "lab-scenario: the lab's backend is starting (pid $!); its page: HOME=$lab python3 scripts/dev_bridge_url.py"
    ;;
  teardown)
    [ -d "$lab" ] || { echo "lab-scenario: nothing at $lab"; exit 0; }
    pids="$(lsof -t +D "$lab" 2>/dev/null | sort -u || true)"
    for pid in $pids; do
      [ "$pid" = "$$" ] && continue
      echo "lab-scenario: stopping $(ps -o comm= -p "$pid" 2>/dev/null || echo "pid $pid")"
      kill "$pid" 2>/dev/null || true
    done
    sleep 1
    for pid in $pids; do kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true; done
    rm -rf "$lab"
    echo "lab-scenario: $lab removed; nothing of it runs"
    ;;
  *) sed -n '2,13p' "$0"; exit 2 ;;
esac
