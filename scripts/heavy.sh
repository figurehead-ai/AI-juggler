#!/usr/bin/env bash
# One heavy run at a time on a shared Mac (the coordinating session's rule,
# 2026-11-23: our Rust builds disturbed another project's timings).
#
#   scripts/heavy.sh just keep-going verify-static
#   scripts/heavy.sh cargo test --offline -p juggler-app
#   scripts/heavy.sh scripts/visual_gate.sh --project=gallery --workers=1
#
# Waits for the lock beside the build output ($CARGO_TARGET_DIR, so every
# worktree sharing one target shares one lock), then runs the command at
# low priority (nice 10) with cargo's parallelism capped (CARGO_BUILD_JOBS,
# default 6). The lock is released when the command ends, however it ends.
# macOS has no flock(1); Python's fcntl.flock is the same kernel lock.
set -euo pipefail

if [[ $# -eq 0 ]]; then
  sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi

target="${CARGO_TARGET_DIR:-$(pwd)/target}"
mkdir -p "$target"
lock="$target/.heavy.lock"
# 2026-11-24: one lock for the whole repository too (git's common folder,
# shared by every worktree), so a run started without CARGO_TARGET_DIR (its
# own target, its own lock) still waits its turn. Both are taken, the
# common one first, so runs holding only the target's lock stay serialized.
common="$(git rev-parse --git-common-dir 2>/dev/null || true)"
common_lock="${common:+$common/heavy.lock}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"

exec python3 -I -c '
import fcntl, subprocess, sys, time
locks = [path for path in sys.argv[1:3] if path]
cmd = sys.argv[3:]
held = []
for lock in locks:
    handle = open(lock, "a")
    held.append(handle)
    try:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print(f"heavy.sh: waiting for the other heavy run ({lock})", file=sys.stderr, flush=True)
        started = time.monotonic()
        fcntl.flock(handle, fcntl.LOCK_EX)
        print(f"heavy.sh: got the lock after {time.monotonic() - started:.0f} s", file=sys.stderr, flush=True)
sys.exit(subprocess.call(["nice", "-n", "10", *cmd]))
' "$common_lock" "$lock" "$@"
