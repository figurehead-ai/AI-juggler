#!/usr/bin/env python3
"""AJ-E512 collateral fixture (SPEC §10 harness, pass2 §14).

A scriptable fake helper. Every trappable signal is appended to the
trap log BEFORE any action — the collateral suites assert on these
logs (never on UI state), so a fixture that received anything leaves
a line behind.

Usage:
  trap_helper.py --ready READY --trap-log LOG [--port N] [--setsid]
                 [--ignore-term] [decorative words...]

Modes:
  default        sleep; TERM/HUP/INT are logged then honoured (TERM
                 exits 0 — the cooperative dev-server shape).
  --ignore-term  log TERM and KEEP RUNNING (the TERM-ignorer — only
                 KILL can take it, after grace).
  --port N       bind 127.0.0.1:N before signalling readiness (the
                 vite-like port holder).
  --setsid       os.setsid() before anything else (the escapee —
                 leaves the root's process group; the ppid chain
                 stays intact so the tree walk still finds it).
Decorative words ride the argv for role needles (e.g. `mcp-server`).
"""
import argparse
import os
import signal
import socket
import sys
import time

parser = argparse.ArgumentParser()
parser.add_argument("--ready", required=True)
parser.add_argument("--trap-log", required=True)
parser.add_argument("--port", type=int, default=None)
parser.add_argument("--setsid", action="store_true")
parser.add_argument("--ignore-term", action="store_true")
args, _ = parser.parse_known_args()

if args.setsid:
    # The escapee: its OWN process group, one process earlier than
    # anything else (so no window where it is still group-shared).
    os.setsid()

TRAPPED = ("TERM", "HUP", "INT", "USR1", "USR2")


def log(name):
    with open(args.trap_log, "a") as handle:
        handle.write(name + "\n")
        handle.flush()


def handler(name):
    def inner(_signum, _frame):
        log(name)
        if name in ("TERM", "HUP", "INT") and not args.ignore_term:
            sys.exit(0)

    return inner


for name in TRAPPED:
    signal.signal(getattr(signal, "SIG" + name), handler(name))

bound_port = 0
if args.port is not None:
    # Port 0 = let the OS assign — the ready file then carries the
    # BOUND port, killing the probe-then-bind race entirely (CI
    # runners recycle ephemeral ports between the two syscalls).
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("127.0.0.1", args.port))
    sock.listen(8)
    bound_port = sock.getsockname()[1]

with open(args.ready, "w") as handle:
    handle.write(f"{bound_port}\n")

# 5-minute cap: fixtures outlive every suite, and a tree leaked by a
# failing test self-cleans instead of lingering for an hour.
for _ in range(1500):
    time.sleep(0.2)
