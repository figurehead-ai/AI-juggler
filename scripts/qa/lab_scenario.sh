#!/usr/bin/env bash
# AJ-E3301: a 0.4 scenario's lab — a throwaway HOME shaped like a heavy
# real Mac (the replica) with the scenario's own state seeded into AI
# Juggler Dev's data folder there; its walk sheet is the tracked
# docs/qa/0.4-walks/<ID>.md (setup never writes into the repo). Every walk
# starts here and ends with teardown: the lab's processes stopped (found by
# their open files under the lab, never by reading environments), the lab
# folder removed, nothing left running.
#
#   scripts/qa/lab_scenario.sh setup P1 [--home /tmp/aij-lab-p1] [--import]
#   scripts/qa/lab_scenario.sh open  P1 [--home …]   # AI Juggler Dev on the lab
#   scripts/qa/lab_scenario.sh bridge P1 [--home …]  # its backend + the read-only bridge
#   scripts/qa/lab_scenario.sh lab-bridge P1 [--home …]  # the same, with the lab write mode (ADR-0027)
#   scripts/qa/lab_scenario.sh start-apps PL1 [--home …] # its running Claude app profiles, as Open starts them
#   scripts/qa/lab_scenario.sh stop-apps PL1 [--home …]  # quits them (teardown does too)
#   scripts/qa/lab_scenario.sh teardown P1 [--home …] [--force-abandon]
#   scripts/qa/lab_scenario.sh setup T16 --standins [--with-hermes]   # a dry run on stand-ins
#
# --standins (T16's dry run, AJ-E1612): the stand-in tools
# (scripts/qa/standins/) are put in <lab>/standins and Dev is pointed at
# them by the adapters' test overrides (<lab>/standins/env.sh, sourced by
# every command on that lab); nothing real is run or signed in.
# --with-hermes / --no-hermes: T16's Hermes Apps (setup.optional_apps,
# D-123) are seeded only with it; without either, only when Hermes is
# installed (never on stand-ins unless asked). The choice is kept in
# <lab>/with-hermes, which lab_sign_ins.py reads too.
#
# A sign-ins lab (T16) is torn down only once every tool's every run-sheet
# step has a recorded result (`lab_sign_ins.py complete`); --force-abandon
# throws the lab and its sign-ins away regardless, and says so.
#
# --import brings the replica's setups in where they are first (as
# `just dev-bridge-data` does on a real Mac), for walks on its Apps.
#
# A scenario with `lab_sign_ins = true` (T16, AJ-E1612) seeds and opens
# with the lab's Muse switch: the seed and the app are debug `lab-bridge`
# builds with AIJ_LAB_MUSE_SIGN_INS=1 (never a release build: it doesn't
# compile there), its fresh Apps' Sign in recorded in <lab>/sign-ins.json
# and checked by scripts/qa/lab_sign_ins.py; `open` runs the lab's Dev
# build (`scripts/build_dev_app.sh lab`).
#
# The HOME must be under /tmp and contain "aij-lab"; a real HOME is refused.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cmd="${1:-}"; id="${2:-}"
[ -n "$cmd" ] && [ -n "$id" ] || { sed -n '2,31p' "$0"; exit 2; }
shift 2
lab="/tmp/aij-lab-$(echo "$id" | tr '[:upper:]' '[:lower:]')"
import=0
force_abandon=0
standins=0
hermes=auto
while [ $# -gt 0 ]; do
  case "$1" in
    --home) lab="$2"; shift 2 ;;
    --import) import=1; shift ;;
    --force-abandon) force_abandon=1; shift ;;
    --standins) standins=1; shift ;;
    --with-hermes) hermes=1; shift ;;
    --no-hermes) hermes=0; shift ;;
    *) echo "lab-scenario: unknown option $1" >&2; exit 2 ;;
  esac
done
# Cargo and rustup keep their own homes when HOME is the lab's.
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
# The real home, before any command's HOME is set to the lab (ADR-0027
# condition 7's seeding needs it).
real_home="$HOME"
# cargo and rustup run under the lab's HOME too: they keep their own
# homes, or rustup fetches a whole toolchain into the lab and the seed
# fails offline (walk A2).
export CARGO_HOME="${CARGO_HOME:-$real_home/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$real_home/.rustup}" RUSTUP_AUTO_INSTALL=0
case "$lab" in
  /tmp/*aij-lab*|/private/tmp/*aij-lab*) ;;
  *) echo "lab-scenario: $lab isn't a lab (under /tmp, named aij-lab…) — refusing" >&2; exit 1 ;;
esac
data="$lab/Library/Application Support/AI Juggler Dev"
# The scenario's own switches (`lab_sign_ins`), read from its seed.
lab_sign_ins=0
python3 -I "$root/scripts/qa/scenario.py" seed "$id" 2>/dev/null | grep -q '"lab_sign_ins": true' && lab_sign_ins=1
if [ "$lab_sign_ins" = 1 ]; then
  # The lab's Muse switch (AJ-E1612), in a debug lab-bridge build only.
  features=(--features lab-bridge)
  export AIJ_LAB_MUSE_SIGN_INS=1
else
  features=()
fi
# A dry run's stand-ins (AJ-E1612): every command on that lab points Dev
# and its seed at them (lab_env.sh passes the overrides on only into it).
if [ -f "$lab/standins/env.sh" ]; then
  # shellcheck source=/dev/null
  . "$lab/standins/env.sh"
fi
# The lab's stand-ins for `claude` and `codex`, when its setup put them
# there (`.aij-lab-bin`): what AI Juggler runs for them (its own test
# overrides). Not `standins`, the --standins switch above.
standin_bins=(env)
[ -x "$lab/.aij-lab-bin/claude" ] && standin_bins+=("AI_JUGGLER_CLAUDE_BIN=$lab/.aij-lab-bin/claude")
[ -x "$lab/.aij-lab-bin/codex" ] && standin_bins+=("AI_JUGGLER_CODEX_BIN=$lab/.aij-lab-bin/codex")

case "$cmd" in
  setup)
    [ -e "$lab" ] && { echo "lab-scenario: $lab exists — tear it down first" >&2; exit 1; }
    python3 -I "$root/scripts/qa/scenario.py" seed "$id" > /dev/null # the scenario exists
    seed_json="$(python3 -I "$root/scripts/qa/scenario.py" seed "$id")"
    if echo "$seed_json" | grep -q '"first_run": true' || echo "$seed_json" | grep -q '"replica": false'; then
      # A persona's first run (AJ-E3307), or a lab whose counts are all its
      # own (PL1): only the folders its scenario names, never the replica's
      # heavy Mac (its accounts and sessions would contradict them).
      mkdir -p "$lab"
    else
      python3 -I "$root/scripts/replica.py" build --home "$lab"
    fi
    # The lab is this user's alone (ADR-0027: the write mode refuses a lab
    # others can write in).
    chmod 700 "$lab"
    # 0.4.7: the tools' stand-ins (lab_standin.sh) as `claude` and `codex`,
    # unless the scenario runs the real ones (live-claude, live-codex): a
    # session started in the lab runs a stand-in, never a real tool.
    if ! python3 -I "$root/scripts/qa/scenario.py" list | grep -E "^$id " | grep -q 'live-'; then
      mkdir -p "$lab/.aij-lab-bin"
      for tool in claude codex; do cp "$root/scripts/qa/lab_standin.sh" "$lab/.aij-lab-bin/$tool"; chmod 755 "$lab/.aij-lab-bin/$tool"; done
      # `standin = "asks"` (K3): a session started in the lab asks at once.
      if echo "$seed_json" | grep -q '"standin": "asks"'; then touch "$lab/.aij-lab-bin/asks-at-start"; fi
    fi
    # Every program the lab runs gets only the lab's environment
    # (lab_env.sh): never this shell's tool folders (CLAUDE_CONFIG_DIR…).
    if [ "$import" = 1 ]; then
      (cd "$root" && HOME="$lab" "$root/scripts/qa/lab_env.sh" cargo run -q --offline -p juggler-app --features dev-bridge --example dev_walk_data)
    fi
    mkdir -p "$data"
    python3 -I "$root/scripts/qa/scenario.py" seed "$id" > "$lab/scenario-seed.json"
    if grep -q '"optional_apps"' "$lab/scenario-seed.json"; then
      # T16's Hermes Apps (D-123): seeded with --with-hermes, or when
      # Hermes is installed (on stand-ins, only when asked).
      if [ "$hermes" = auto ]; then
        hermes=0
        if [ "$standins" = 0 ] && { command -v hermes >/dev/null 2>&1 || [ -x "$real_home/.local/bin/hermes" ]; }; then hermes=1; fi
      fi
      [ "$hermes" = 1 ] && : > "$lab/with-hermes"
      python3 -I -c 'import json, sys
path, wanted = sys.argv[1], sys.argv[2] == "1"
seed = json.load(open(path))
extra = seed.pop("optional_apps", [])
seed["apps"] = seed.get("apps", []) + [dict((k, v) for k, v in app.items() if k != "with") for app in extra if wanted and app.get("with") == "hermes"]
json.dump(seed, open(path, "w"), indent=2)' "$lab/scenario-seed.json" "$hermes"
      echo "lab-scenario: Hermes Apps $([ "$hermes" = 1 ] && echo "seeded" || echo "left out (--with-hermes adds them)")"
    fi
    if [ "$standins" = 1 ]; then
      # The dry run's stand-in tools, before the seed asks each adapter for
      # its Sign in (the overrides decide which program that is).
      python3 -I "$root/scripts/qa/standins/install.py" --lab "$lab" $([ "$hermes" = 1 ] && echo --with-hermes)
      # shellcheck source=/dev/null
      . "$lab/standins/env.sh"
    fi
    if grep -q '"from"' "$lab/scenario-seed.json"; then
      # P9: the lab starts as a 0.3 data folder (schema v39), for 0.4 to
      # move on first open; seeding through 0.4 would move it already.
      (cd "$root" && AIJ_P9_DB="$data/juggler.sqlite" AIJ_P9_LAB="$lab" \
        cargo test -q --offline -p juggler-core --test pools_v45 "$(grep -q '"from": ".*disagree' "$lab/scenario-seed.json" && echo write_the_a9_labs_0_3_data_folder || echo write_the_p9_labs_0_3_data_folder)" -- --ignored --exact --nocapture 2>&1 | grep '^seed:' || true)
    else
      # AIJ_LAB_ADOPT=<tool> (ADR-0027 condition 7): that tool's App uses
      # its real default folder where it is; the bridge checks it again.
      (cd "$root" && HOME="$lab" AIJ_QA_SEED="$lab/scenario-seed.json" AIJ_QA_DATA="$data" AIJ_QA_HOME="$lab" AIJ_QA_REAL_HOME="$real_home" \
        "$root/scripts/qa/lab_env.sh" cargo test -q --offline -p juggler-app ${features[@]+"${features[@]}"} --test qa_scenario_seed -- --ignored --nocapture > "$lab/seed.log" 2>&1) \
        || { tail -20 "$lab/seed.log" >&2; echo "lab-scenario: the seed failed (whole log: $lab/seed.log)" >&2; exit 1; }
      grep '^seed:' "$lab/seed.log" || true
    fi
    if [ -e "$lab/sign-ins.json" ]; then
      # Every fresh App's Sign in held to the lab (T16).
      python3 -I "$root/scripts/qa/lab_sign_ins.py" check --home "$lab"
    fi
    # The walk sheet is tracked (`scenario.py check`); setup never writes
    # into the repo (the rc26 walk: PM3's and PF1's were left untracked).
    sheet="$(python3 -I "$root/scripts/qa/scenario.py" sheet-path "$id")"
    if [ -e "$root/$sheet" ]; then echo "lab-scenario: walk sheet $sheet"
    else echo "lab-scenario: $id has no walk sheet: make it with scripts/qa/scenario.py sheet $id --out $sheet and commit it" >&2; fi
    echo "lab-scenario: $id is set up in $lab — open it with: scripts/qa/lab_scenario.sh open $id"
    ;;
  open)
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    if [ "$lab_sign_ins" = 1 ]; then
      # The lab's Dev build (debug, lab-bridge: the Muse switch compiles
      # only there); a release-mode Dev .app would sign Muse in nowhere.
      # The lab's Dev app is kept at one stable path outside any build
      # folder (a build folder is trimmed to save the disk; a copy there
      # costs a whole target, about 25 GB): scripts/build_dev_app.sh lab
      # builds it in the shared target and copies the bundle here.
      app="${AIJ_DEV_APP:-/tmp/aij-lab-dev/AI Juggler Dev.app}"
      [ -d "$app" ] || { echo "lab-scenario: no lab Dev build (scripts/build_dev_app.sh lab, or set AIJ_DEV_APP)" >&2; exit 1; }
    else
      app="${AIJ_DEV_APP:-$root/target/release/bundle/macos/AI Juggler Dev.app}"
      [ -d "$app" ] || app="${CARGO_TARGET_DIR:-$root/target}/release/bundle/macos/AI Juggler Dev.app"
      [ -d "$app" ] || { echo "lab-scenario: no AI Juggler Dev build (just build-dev, or set AIJ_DEV_APP)" >&2; exit 1; }
    fi
    # A bundle whose signature doesn't hold (a debug build's linker-only
    # signature: "code has no resources but signature indicates they must be
    # present") is what made macOS call a replica app damaged and offer the
    # Trash (2026-11-13): signed ad hoc as a whole, then verified strictly,
    # before it runs. Launched only by its binary, never through `open`.
    if ! codesign --verify --strict "$app" 2>/dev/null; then
      codesign --force --deep --sign - "$app" >/dev/null 2>&1 || true
      codesign --verify --strict "$app" || { echo "lab-scenario: $app doesn't verify after an ad-hoc signature" >&2; exit 1; }
    fi
    # Its own HOME: the data folder, the vendors' folders and the sign-ins are the lab's;
    # its `claude` and `codex` are the lab's stand-ins when `setup` put them there.
    # Its log in the lab, so this command returns (and a pipe after it ends).
    HOME="$lab" "${standin_bins[@]}" "$root/scripts/qa/lab_env.sh" "$app/Contents/MacOS/$(defaults read "$app/Contents/Info" CFBundleExecutable)" > "$lab/dev.log" 2>&1 &
    pid=$!
    # AI Juggler Dev runs once per Mac: a second one hands over to the
    # running copy and quits. Still running after 8 s, it is the lab's.
    sleep 8
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "lab-scenario: AI Juggler Dev quit at once: another AI Juggler Dev runs on this Mac and took over (quit it first)" >&2
      exit 1
    fi
    echo "lab-scenario: AI Juggler Dev on $lab (pid $pid; drive it by this pid only; its log: $lab/dev.log)"
    if [ -e "$lab/.aij-lab-bin/asks-at-start" ]; then
      echo "lab-scenario: a session started here asks at once (its stand-in): start the one that asks you from New session"
    fi
    ;;
  bridge)
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    # The Dev backend (a debug build is always Dev) on the lab's HOME, with
    # the read-only bridge; its page needs the UI on localhost:5173.
    (cd "$root" && HOME="$lab" AIJ_DEV_BRIDGE=1 "${standin_bins[@]}" "$root/scripts/qa/lab_env.sh" cargo run -q --offline -p juggler-app ${AIJ_LAB_PROFILE:+--profile "$AIJ_LAB_PROFILE"} --features dev-bridge) &
    echo "lab-scenario: the lab's backend is starting (pid $!); its page: HOME=$lab python3 scripts/dev_bridge_url.py"
    ;;
  lab-bridge)
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    # ADR-0027: the same, with the lab write mode — the walk makes pools,
    # starts, switches and stops sessions from the browser. The app checks
    # the HOME itself and refuses to start the bridge on anything but a lab.
    # The pool tools' program (juggler-agent) must sit beside the app's own
    # executable, so it is built for the same profile first (0.4.6: a walk
    # build without it read as "not in this build").
    (cd "$root" && "$root/scripts/qa/lab_env.sh" cargo build -q --offline -p juggler-agentbridge --bin juggler-agent ${AIJ_LAB_PROFILE:+--profile "$AIJ_LAB_PROFILE"}) || { echo "lab-scenario: juggler-agent didn't build" >&2; exit 1; }
    (cd "$root" && HOME="$lab" AIJ_DEV_BRIDGE=1 AIJ_LAB_BRIDGE=1 "${standin_bins[@]}" "$root/scripts/qa/lab_env.sh" cargo run -q --offline -p juggler-app ${AIJ_LAB_PROFILE:+--profile "$AIJ_LAB_PROFILE"} --features lab-bridge) &
    echo "lab-scenario: the lab's backend (LAB WRITE MODE) is starting (pid $!); its page: HOME=$lab python3 scripts/dev_bridge_url.py"
    ;;
  start-apps)
    # PL1: the Claude app profiles its scenario says run, started on their
    # lab folders with the launch settings the seeder stored, as AI
    # Juggler's Open starts them (each shows Claude's sign-in window: never
    # type or click in it).
    [ -d "$data" ] || { echo "lab-scenario: $lab isn't set up" >&2; exit 1; }
    python3 -I "$root/scripts/qa/lab_apps.py" start --home "$lab"
    ;;
  stop-apps)
    [ -d "$lab" ] || { echo "lab-scenario: nothing at $lab"; exit 0; }
    python3 -I "$root/scripts/qa/lab_apps.py" stop --home "$lab"
    ;;
  teardown)
    [ -d "$lab" ] || { echo "lab-scenario: nothing at $lab"; exit 0; }
    if [ "$lab_sign_ins" = 1 ] && [ -f "$lab/sign-ins.json" ]; then
      if [ "$force_abandon" = 1 ]; then
        echo "lab-scenario: --force-abandon: removing $lab and every sign-in in it, steps recorded or not" >&2
      elif ! python3 -I "$root/scripts/qa/lab_sign_ins.py" complete --home "$lab"; then
        echo "lab-scenario: refusing to tear down $lab: run-sheet steps have no result (the sign-ins are the maintainer's time). Record them with lab_sign_ins.py step, or pass --force-abandon." >&2
        exit 1
      fi
    fi
    # The lab's Claude app instances first, by their --user-data-dir under
    # the lab: asked to quit, with their helpers.
    python3 -I "$root/scripts/qa/lab_apps.py" stop --home "$lab" || true
    pids="$(lsof -t +D "$lab" 2>/dev/null | sort -u || true)"
    for pid in $pids; do
      [ "$pid" = "$$" ] && continue
      echo "lab-scenario: stopping $(ps -o comm= -p "$pid" 2>/dev/null || echo "pid $pid")"
      kill "$pid" 2>/dev/null || true
    done
    sleep 1
    for pid in $pids; do kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null || true; done
    # The replica's stand-in apps out of LaunchServices' records first: one
    # left registered could be started by the system outside the lab (a
    # "damaged app" dialog on the maintainer's screen, 2026-11-13).
    python3 -I "$root/scripts/replica.py" teardown --home "$lab" || true
    rm -rf "$lab"
    echo "lab-scenario: $lab removed; nothing of it runs"
    ;;
  *) sed -n '2,31p' "$0"; exit 2 ;;
esac
