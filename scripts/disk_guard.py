#!/usr/bin/env python3
"""Free disk before heavy work: warn under 40 GB, refuse under 20 GB.

2026-11-21: the data volume fell to 180 MB free while two sessions'
builders, soak walkers and a snapshot target ran at once. Preflight
checked the disk, but nothing else did, so parallel sessions drove it to
zero between preflights. Every heavy entry point runs this first: the Dev
build (`scripts/build_dev_app.sh`), the visual and accessibility container
runs, the nextest recipes and preflight. One place for the rule.

    disk_guard.py WHAT        exit 0 (a warning under 40 GB), 1 under 20 GB
    disk_guard.py --selftest

WHAT names the step in the message ("the Dev build"). Under either line
it names the biggest shared build targets, the first thing to trim:
`python3 scripts/trim_target.py <target> --apply` (safe while another
build runs; it refuses while one uses that target). It never deletes
anything itself.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

WARN_GB = 40
REFUSE_GB = 20
SHARED = Path(os.path.expanduser("~/Dev/AIJuggler-shared-target"))
DATA_VOLUME = "/System/Volumes/Data" if Path("/System/Volumes/Data").is_dir() else "/"


def free_gb(path: str = DATA_VOLUME) -> int:
    """The data volume's free space, whole GB."""
    return shutil.disk_usage(path).free // 1_000_000_000


def biggest(root: Path = SHARED, top: int = 3) -> list[tuple[int, str]]:
    """The largest folders under the shared targets, (GB, path), largest
    first. Measured only when a warning prints (du takes seconds)."""
    if not root.is_dir():
        return []
    sized = []
    for child in sorted(root.iterdir()):
        if not child.is_dir():
            continue
        try:
            out = subprocess.run(["du", "-sk", str(child)], capture_output=True, text=True, timeout=90, check=False)
            kb = int(out.stdout.split()[0]) if out.stdout else 0
        except (subprocess.TimeoutExpired, ValueError, IndexError):
            continue
        sized.append((kb // 1_000_000, str(child)))
    return sorted(sized, reverse=True)[:top]


def verdict(what: str, free: int, targets: list[tuple[int, str]]) -> tuple[int, str]:
    """(exit code, message) for `free` GB before `what`."""
    if free >= WARN_GB:
        return 0, ""
    home = os.path.expanduser("~")
    named = ", ".join(f"{p.replace(home, '~')} ({gb} GB)" for gb, p in targets) or "none found"
    hint = f"Trim first: python3 scripts/trim_target.py <target> --apply. The biggest shared targets: {named}."
    if free < REFUSE_GB:
        return 1, f"disk-guard: only {free} GB free; refusing to start {what} under {REFUSE_GB} GB. {hint}"
    return 0, f"disk-guard: warning, only {free} GB free (under {WARN_GB} GB) before {what}. {hint}"


def selftest() -> int:
    targets = [(29, os.path.expanduser("~/Dev/AIJuggler-shared-target/feed"))]
    assert verdict("x", 60, targets) == (0, ""), "plenty: silent"
    code, msg = verdict("the Dev build", 30, targets)
    assert code == 0 and "warning, only 30 GB" in msg and "~/Dev/AIJuggler-shared-target/feed (29 GB)" in msg, msg
    code, msg = verdict("the Dev build", 12, targets)
    assert code == 1 and "refusing to start the Dev build under 20 GB" in msg and "trim_target.py" in msg, msg
    assert verdict("x", 20, [])[0] == 0 and verdict("x", 19, [])[0] == 1, "20 GB is the line"
    assert "none found" in verdict("x", 10, [])[1]
    assert free_gb() >= 0
    print("disk-guard selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) != 2:
        print("usage: disk_guard.py WHAT | --selftest", file=sys.stderr)
        return 2
    free = free_gb()
    if free >= WARN_GB:
        return 0
    code, msg = verdict(argv[1], free, biggest())
    print(msg, file=sys.stderr)
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv))
