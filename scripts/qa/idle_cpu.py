#!/usr/bin/env python3
"""Idle CPU of a lab AI Juggler build, read by pid only (AJ-E805, E812, E814).

    idle_cpu.py pids APP_PID
        the app and the WebKit processes it is responsible for (web
        content, GPU, networking)
    idle_cpu.py watch APP_PID OUT.csv [--every S] [--seconds N]
        `ps -o time=` of each every S seconds (default 1) with the 1-minute
        load average, one CSV line per process: epoch, role, pid, cpu s, load
    idle_cpu.py windows OUT.csv T0 T1 [--window S]
        % of one core of the app, its web content and its GPU process (and
        the three together) in each S-second window from T0 to T1 (epoch
        seconds), with the highest load seen in it
    idle_cpu.py events APP_PID T0 T1
        what happened to the window from outside in T0..T1, from the app's
        own system log: display reconfigurations, the app made active, the
        window occluded or shown again, a close pressed from outside, and
        accessibility clusters. A reading with any of these but the steady
        accessibility poll (at most 3 clusters a minute) is dropped and
        recorded, never by its number (AJ-E805 round 4's rule, widened in
        AJ-E812 with the display, activation and occlusion events)

Nothing is attached to any process: it reads ids, names and CPU times
(`ps`), the responsible pid (libsystem's responsibility call, as `ps`
reads its own fields) and the log (`log show`). The installed AI Juggler
and a Dev build share the process name juggler-app; this takes a pid.
"""
from __future__ import annotations

import ctypes
import os
import re
import subprocess
import sys
import time

_lib = ctypes.CDLL(None)
_responsible = _lib.responsibility_get_pid_responsible_for_pid
_responsible.restype = ctypes.c_int
_responsible.argtypes = [ctypes.c_int]

ROLES = {
    "com.apple.WebKit.WebContent": "web",
    "com.apple.WebKit.GPU": "gpu",
    "com.apple.WebKit.Networking": "net",
}

# The system log lines that say something outside moved the window, and
# what each is called in a record.
EVENTS = [
    ("kCGSDisplayWillReconfigure", "display reconfigured"),
    ("setApplicationIsActive: (isActive=1)", "app made active"),
    ("view visibility state changed 1 -> 0", "window occluded or hidden"),
    ("view visibility state changed 0 -> 1", "window shown again"),
    ("performClick", "close pressed from outside"),
    ("windowShouldClose", "close pressed from outside"),
    ("WebProcessPool::initializeAccessibility", "accessibility"),
]


def pids(app: int) -> dict[int, str]:
    """The app and the WebKit processes whose responsible pid is it."""
    found = {app: "app"}
    out = subprocess.run(["ps", "-A", "-o", "pid=,comm="], capture_output=True, text=True, check=False).stdout
    for line in out.splitlines():
        parts = line.strip().split(None, 1)
        if len(parts) != 2:
            continue
        pid, comm = int(parts[0]), parts[1]
        role = next((r for name, r in ROLES.items() if comm.endswith(name)), None)
        if role and _responsible(pid) == app:
            found[pid] = role
    return found


def seconds(text: str) -> float:
    """`ps` time ([[dd-]hh:]mm:ss.cc) in seconds."""
    text = text.strip()
    days = 0
    if "-" in text:
        day, text = text.split("-", 1)
        days = int(day)
    total = 0.0
    for part in text.split(":"):
        total = total * 60 + float(part)
    return days * 86400 + total


def cpu_times(found: dict[int, str]) -> dict[int, float]:
    out = subprocess.run(["ps", "-o", "pid=,time=", "-p", ",".join(map(str, found))],
                         capture_output=True, text=True, check=False).stdout
    got = {}
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 2:
            got[int(parts[0])] = seconds(parts[1])
    return got


def watch(app: int, out: str, every: float, total: float) -> None:
    found = pids(app)
    end = time.time() + total
    scanned = time.time()
    with open(out, "a", encoding="utf-8") as sink:
        while time.time() < end:
            now = time.time()
            if now - scanned > 5:  # the web view's processes start after the app
                found = pids(app)
                scanned = now
            got = cpu_times(found)
            if app not in got:
                sink.write(f"{now:.3f},gone,{app},0,0\n")
                return
            load = os.getloadavg()[0]
            for pid, cpu in got.items():
                sink.write(f"{now:.3f},{found.get(pid, '?')},{pid},{cpu:.2f},{load:.1f}\n")
            sink.flush()
            time.sleep(max(0.0, every - (time.time() - now)))


def read_csv(path: str) -> dict[str, list[tuple[float, float, float]]]:
    rows: dict[str, list[tuple[float, float, float]]] = {}
    with open(path, encoding="utf-8") as source:
        for line in source:
            parts = line.strip().split(",")
            if len(parts) < 5 or parts[1] == "gone":
                continue
            rows.setdefault(parts[1], []).append((float(parts[0]), float(parts[3]), float(parts[4])))
    return rows


def share(rows: dict[str, list[tuple[float, float, float]]], role: str, t0: float, t1: float) -> float | None:
    """% of one core between the last sample at or before t0 and the first
    at or after t1."""
    samples = rows.get(role, [])
    before = [s for s in samples if s[0] <= t0]
    after = [s for s in samples if s[0] >= t1]
    if not before or not after or after[0][0] <= before[-1][0]:
        return None
    return (after[0][1] - before[-1][1]) * 100 / (after[0][0] - before[-1][0])


def windows(path: str, t0: float, t1: float, step: float) -> None:
    rows = read_csv(path)
    at = t0
    while at + step <= t1 + 0.01:
        parts = {role: share(rows, role, at, at + step) for role in ("app", "web", "gpu")}
        loads = [s[2] for s in rows.get("app", []) if at <= s[0] <= at + step]
        known = [v for v in parts.values() if v is not None]
        total = sum(known) if len(known) == 3 else None
        shown = " ".join(f"{role} {'-' if v is None else f'{v:.2f}'}" for role, v in parts.items())
        print(f"{at - t0:6.0f}s {shown} all {'-' if total is None else f'{total:.2f}'} load {max(loads) if loads else 0:.1f}")
        at += step


def events(app: int, t0: float, t1: float) -> None:
    start = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t0 - 1))
    end = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t1 + 1))
    wanted = " OR ".join(f"eventMessage CONTAINS '{key}'" for key, _ in EVENTS)
    out = subprocess.run(["/usr/bin/log", "show", "--style", "compact", "--start", start, "--end", end,
                          "--predicate", f"processID == {app} AND ({wanted})"],
                         capture_output=True, text=True, check=False).stdout
    seen: list[tuple[float, str]] = []
    for line in out.splitlines():
        stamp = re.match(r"(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\.(\d+)", line)
        if not stamp:
            continue
        at = time.mktime(time.strptime(stamp.group(1), "%Y-%m-%d %H:%M:%S")) + float("0." + stamp.group(2))
        if not t0 <= at <= t1:
            continue
        what = next((name for key, name in EVENTS if key in line), None)
        if what:
            seen.append((at, what))
    # Accessibility comes in clusters (a few lines within a second).
    clusters = 0
    last = -10.0
    outside = []
    for at, what in seen:
        if what == "accessibility":
            if at - last > 2:
                clusters += 1
            last = at
        elif not outside or outside[-1] != (round(at), what):
            outside.append((round(at), what))
    for at, what in outside:
        print(f"{at - t0:6.0f}s {what}")
    minutes = max((t1 - t0) / 60, 1 / 60)
    print(f"accessibility clusters: {clusters} ({clusters / minutes:.1f} a minute)")
    drop = bool(outside) or clusters / minutes > 3
    print("DROP" if drop else "KEEP")


def main() -> None:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    flag = lambda name, default: float(args[args.index(name) + 1]) if name in args else default  # noqa: E731
    if args[0] == "pids":
        for pid, role in pids(int(args[1])).items():
            print(pid, role)
    elif args[0] == "watch":
        watch(int(args[1]), args[2], flag("--every", 1.0), flag("--seconds", 3600.0))
    elif args[0] == "windows":
        windows(args[1], float(args[2]), float(args[3]), flag("--window", 60.0))
    elif args[0] == "events":
        events(int(args[1]), float(args[2]), float(args[3]))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
