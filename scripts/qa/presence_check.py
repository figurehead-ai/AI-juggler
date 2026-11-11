#!/usr/bin/env python3
"""ADR-0032 (D-100): does the rail show what runs? A read-only check on a
real Mac.

It compares three things, touching nothing:

1. the desktop Apps running now, from `ps` (each Claude app main process by
   its `--user-data-dir`, ChatGPT.app's main process as the Codex app);
2. the user's Apps, from the AI Juggler build's own database, opened
   read-only (D-84: an App the user added or whose Account they have);
3. what that build's rail shows under Open now and Inactive apps, read
   through the accessibility tree of the process with `--pid` (by pid,
   never by name: the installed app and the Dev build share one).

It fails when a running App of the user's is missing from Open now, when
Open now names a desktop App that isn't running, when an App is in both
sections, or when Inactive apps says "Running". Run it with the rail
grouped by App and Open now unfolded.

    python3 scripts/qa/presence_check.py --pid <Dev build pid> [--data DIR] [--record FILE]
"""

from __future__ import annotations

import argparse
import os
import re
import sqlite3
import subprocess
import sys
from pathlib import Path

HOME = Path.home()
CLAUDE_MAIN = "/Applications/Claude.app/Contents/MacOS/Claude"
CHATGPT_MAIN = "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT"
CLAUDE_DEFAULT = HOME / "Library/Application Support/Claude"
HERE = Path(__file__).resolve().parent


def running_ids() -> set[str]:
    """The App ids of the desktop apps running, as the backend keys them."""
    out = subprocess.run(["ps", "-axww", "-o", "pid=,args="], capture_output=True, text=True, check=True).stdout
    ids: set[str] = set()
    for line in out.splitlines():
        args = line.strip().split(" ", 1)[1] if " " in line.strip() else ""
        if args == CHATGPT_MAIN or args.startswith(CHATGPT_MAIN + " "):
            ids.add("app:codex-app:")
        elif args == CLAUDE_MAIN or args.startswith(CLAUDE_MAIN + " "):
            m = re.search(r"--user-data-dir[= ](.+?)(?= --|$)", args)
            ids.add(f"app:claude-app:{m.group(1) if m else CLAUDE_DEFAULT}")
    return ids


def users_apps(data: Path) -> dict[str, str]:
    """Each of the user's desktop Apps (id → its name on the rail)."""
    db = sqlite3.connect(f"file:{data / 'juggler.sqlite'}?mode=ro", uri=True)
    labels = dict(db.execute("select id, label from accounts"))
    names: dict[str, str] = {}
    for app_id, host, instance, folder, label, hidden, added in db.execute(
        "select id, host_app_id, instance_dir, folder_id, label, hidden, added from apps where kind = 'desktop'"
    ):
        if hidden and not added and not folder:
            continue
        if not (added or folder):
            continue
        word = "Codex app" if host == "codex-app" else "Claude app"
        # As the rail names it: an instance's profile is its Account's
        # name, else its folder's; an app that runs one instance has none.
        profile = (labels.get(folder) or Path(instance).name) if instance else None
        names[app_id] = label or (f"{word} · {profile}" if profile else word)
    return names


def rail(pid: int) -> tuple[list[str], list[str], str]:
    """The App names under Open now and under Inactive apps, and the
    Inactive section's own words."""
    out = subprocess.run(["swift", str(HERE / "ax_texts.swift"), str(pid)], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"presence-check: couldn't read the rail: {out.stderr.strip()}")
    open_now: list[str] = []
    inactive: list[str] = []
    inactive_words = ""
    at = None
    for line in out.stdout.splitlines():
        role, label, value = (line.split("\t") + ["", ""])[:3]
        words = label or value
        if re.match(r"^Open now\b", words):
            at = "open"
        elif re.match(r"^Inactive apps\b", words):
            at = "idle"
        elif re.match(r"^(Also:|Ended today|History)\b", words):
            at = None
        elif at == "open" and role == "AXButton" and words.startswith("Open "):
            open_now.append(words[len("Open "):])
        elif at == "idle":
            inactive_words += " " + words
            m = re.match(r"^[^:]+: (.+)$", words)
            if role == "AXButton" and m:
                inactive.append(m.group(1))
    return open_now, inactive, inactive_words


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pid", type=int, required=True, help="the AI Juggler build's pid (by pid only)")
    parser.add_argument("--data", type=Path, default=HOME / "Library/Application Support/AI Juggler Dev")
    parser.add_argument("--record", type=Path, help="write the report here too")
    args = parser.parse_args()

    running = running_ids()
    mine = users_apps(args.data)
    open_now, inactive, inactive_words = rail(args.pid)
    desktop_words = ("Claude app", "Codex app")
    shown_open = {name for name in open_now if name.startswith(desktop_words)}
    want_open = {mine[i] for i in running if i in mine}
    lines = [
        "presence-check (ADR-0032)",
        f"running desktop apps (ps): {len(running)}",
        *[f"  {i.replace(str(HOME), '~')}{'' if i in mine else '  (not one of your Apps)'}" for i in sorted(running)],
        f"your running Apps: {sorted(want_open)}",
        f"Open now shows:    {sorted(shown_open)}",
        f"Inactive apps shows: {sorted(inactive) or '(folded)'}",
    ]
    problems = []
    for name in sorted(want_open - shown_open):
        problems.append(f"runs but isn't in Open now: {name}")
    for name in sorted(shown_open - want_open):
        problems.append(f"in Open now but not running: {name}")
    for name in sorted(set(open_now) & set(inactive)):
        problems.append(f"in Open now and Inactive apps: {name}")
    for name in sorted(set(inactive) & want_open):
        problems.append(f"runs but is under Inactive apps: {name}")
    if re.search(r"\bRunning\b", inactive_words):
        problems.append("Inactive apps says Running")
    lines += [f"FAIL {p}" for p in problems] or ["PASS Open now is exactly your running Apps; none is in two sections"]
    report = "\n".join(lines)
    print(report)
    if args.record:
        args.record.parent.mkdir(parents=True, exist_ok=True)
        args.record.write_text(report + "\n")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
