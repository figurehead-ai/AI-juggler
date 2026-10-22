#!/usr/bin/env bash
# QA batch 9 (the coordinator's check): a launch alert can't outlive AI
# Juggler. Runs the real debug binary against a throwaway home whose data
# folder is held (as by an AI Juggler that isn't answering), waits until
# its "can't start" alert is up, kills the app with SIGKILL while the
# alert shows, and checks that no window of it remains, neither its own
# nor one the system shows for it (rfd's CFUserNotification was the
# system's UserNotificationCenter, which stayed up).
#
# macOS, by hand only: it shows a real alert for a few seconds, so it
# runs only with AIJ_ALLOW_WINDOWS=1 and never from the gates. It never
# runs while AI Juggler itself is open (a plain build shares its single-
# instance name), and it needs no permission (window owners are listed
# without Screen Recording). Every process it starts carries a tag in its
# environment; on any exit all of them are killed, and it fails if one
# is left.
#
# Usage: AIJ_ALLOW_WINDOWS=1 [AIJ_ALERT_SHOT=alert.png] scripts/qa/launch_alert_check.sh [path/to/juggler-app]
# (AIJ_ALERT_SHOT: a capture of the alert window alone, to check it draws.)
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "macOS only"; exit 2; }
[ "${AIJ_ALLOW_WINDOWS:-}" = 1 ] || { echo "shows a real alert: run it by hand with AIJ_ALLOW_WINDOWS=1"; exit 2; }
tag="AIJ_ALERT_CHECK=$$-$RANDOM"
# This run's processes: the app, any launch it starts, the lock holder.
tagged() { ps eww -axo pid=,command= | { grep -F "$tag" || true; } | { grep -v grep || true; } | awk '{print $1}'; }
reap() {
  for pid in $(tagged); do kill -9 "$pid" 2>/dev/null || true; done
  rm -rf "${work:-/nonexistent}"
}
bin=${1:-${CARGO_TARGET_DIR:-target}/debug/juggler-app}
[ -x "$bin" ] || { echo "no binary at $bin (cargo build -p juggler-app)"; exit 2; }
if ps -axo comm= | grep -q 'AI Juggler.app/Contents/MacOS/'; then
  echo "skipped: AI Juggler is open, and this build shares its single-instance name"
  exit 0
fi

work=$(mktemp -d)
trap reap EXIT INT TERM
cat > "$work/windows.swift" <<'SWIFT'
import CoreGraphics
let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] ?? []
for window in list {
    let pid = window[kCGWindowOwnerPID as String] as? Int ?? 0
    let owner = window[kCGWindowOwnerName as String] as? String ?? ""
    let layer = window[kCGWindowLayer as String] as? Int ?? 0
    let onscreen = window[kCGWindowIsOnscreen as String] as? Bool ?? false
    let number = window[kCGWindowNumber as String] as? Int ?? 0
    if onscreen { print("\(pid)\t\(layer)\t\(owner)\t\(number)") }
}
SWIFT
swiftc -O "$work/windows.swift" -o "$work/windows" 2>/dev/null
windows_of() { "$work/windows" | awk -F'\t' -v who="$1" '$1 == who || $3 == who' | wc -l | tr -d ' '; }
# The pid owning a window named like the binary (the launch that shows
# the alert may be a fresh one: launch_alert::in_fresh_process).
alert_pid() { "$work/windows" | awk -F'\t' -v who="$(basename "$bin")" '$3 == who {print $1; exit}'; }

home="$work/home"
data="$home/Library/Application Support/AI Juggler Dev"
mkdir -p "$data"
# The AI Juggler that holds the data and doesn't answer.
env "$tag" python3 -c 'import fcntl,sys,time; f=open(sys.argv[1],"a"); fcntl.flock(f, fcntl.LOCK_EX); time.sleep(300)' "$data/juggler.lock" &
holder=$!
sleep 0.5

unc_before=$(windows_of UserNotificationCenter)
env "$tag" HOME="$home" "$bin" > "$work/app.log" 2>&1 &
app=$!
up=""
for _ in $(seq 1 160); do
  owner=$(alert_pid)
  if [ -n "$owner" ]; then up=yes; app=$owner; break; fi
  sleep 0.25
done
if [ -z "$up" ]; then
  echo "FAIL: no alert window owned by the app appeared (log: $(tail -3 "$work/app.log"))"
  exit 1
fi
unc_during=$(windows_of UserNotificationCenter)
if [ -n "${AIJ_ALERT_SHOT:-}" ]; then
  sleep 1 # drawn
  number=$("$work/windows" | awk -F'\t' -v pid="$app" '$1 == pid {print $4; exit}')
  screencapture -x -o -l "$number" "$AIJ_ALERT_SHOT" && echo "captured the alert window to $AIJ_ALERT_SHOT"
fi
echo "alert up: $(windows_of "$app") window(s) owned by AI Juggler itself ($(basename "$bin"), pid $app); system alert windows $unc_before -> $unc_during"
kill -9 "$app"
wait "$app" 2>/dev/null || true
sleep 1
left_own=$(windows_of "$app")
unc_after=$(windows_of UserNotificationCenter)
echo "after SIGKILL: its windows $left_own; system alert windows $unc_after"
kill "$holder" 2>/dev/null || true
sleep 0.5
left=$(tagged | wc -l | tr -d ' ')
echo "processes of this run left: $left"
if [ "$left_own" -eq 0 ] && [ "$left" -eq 0 ] && [ "$unc_during" -le "$unc_before" ] && [ "$unc_after" -le "$unc_before" ]; then
  echo "PASS: the alert was the app's own and went with it"
else
  echo "FAIL: an alert outlived the app"
  exit 1
fi
