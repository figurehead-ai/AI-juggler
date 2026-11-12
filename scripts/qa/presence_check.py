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
sections, or when Inactive apps says "Running". 0.4.3: and when what an
App unfolds to, its session counts or its Out mark disagree with the apps'
own records and the Accounts' readings, or a lab run wrote into a real
profile's folder (`presence_sessions.py`); (D-102) when the Codex app's and
Codex CLI's counts aren't the Codex store's listed threads. Run it with the rail grouped by
App, Open now unfolded and every App in it unfolded.

    python3 scripts/qa/presence_check.py --pid <Dev build pid> [--data DIR] [--record FILE] [--since YYYY-MM-DD]
"""

from __future__ import annotations

import argparse
import os
import re
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from anonymize import anonymize  # noqa: E402  (scripts/qa, beside this file)
import presence_sessions  # noqa: E402

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
        # name, else its folder's; the default instance and an app that
        # runs one instance have none.
        default = instance and Path(instance) == CLAUDE_DEFAULT
        profile = (labels.get(folder) or Path(instance).name) if instance and not default else None
        names[app_id] = label or (f"{word} · {profile}" if profile else word)
    return names


def rail(pid: int) -> tuple[list[str], list[str], str, dict[str, int], list[presence_sessions.RailApp]]:
    """The App names under Open now and under Inactive apps, the Inactive
    section's own words, the counts, and each App as drawn. 0.4.3: an App
    is found by its header's name as well as its fold (a header with
    nothing to unfold has none)."""
    out = subprocess.run(["swift", str(HERE / "ax_texts.swift"), str(pid)], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"presence-check: couldn't read the rail: {out.stderr.strip()}")
    counts: dict[str, int] = {}
    inactive_words = ""
    at = None
    lines = []
    for line in out.stdout.splitlines():
        role, label, value = (line.split("\t") + ["", ""])[:3]
        lines.append((role, label, value))
        words = label or value
        if m := re.match(r"^Open now (\d+)", words):
            counts["open"] = int(m.group(1))
        if re.match(r"^Open now\b", words):
            at = "open"
        elif re.match(r"^Inactive apps\b", words):
            at = "idle"
        elif re.match(r"^(Also:|Ended today|History)\b", words):
            at = None
        elif at == "idle" and not re.match(r"^(?:Fold|Unfold) ", words):
            inactive_words += " " + words
    drawn = presence_sessions.parse_rail(lines)
    open_now = [app.name for app in drawn if app.section == "open"]
    inactive = [app.name for app in drawn if app.section == "idle"]
    return open_now, inactive, inactive_words, counts, drawn


def recorded(lines, running, want_open, shown_open, inactive, counts, names) -> str:
    """The report as written into the repository, which is public: it
    names no App, no profile, no path and no address (each App is
    "App N"; scripts/qa/anonymize.py), only what was checked."""
    kinds = sorted({"desktop" if i.startswith("app:") else "cli" for i in running})
    out = [
        "presence-check (ADR-0032)",
        f"running desktop apps (ps): {len(running)} ({', '.join(kinds)})",
        f"your running Apps: {len(want_open)}",
        f"Open now shows: {len(shown_open)} desktop App(s), counted {counts.get('open', '?')}",
        f"Inactive apps shows: {len(inactive)} named on screen",
    ] + [line for line in lines if line.startswith(("FAIL", "PASS", "(", "  open: ", "  idle: ", "  codex store: "))]
    return anonymize("\n".join(out), names) + "\n"


def selftest() -> int:
    """A real-looking name and home folder never reach the record."""
    home = "/Us" + "ers/janedoe"
    app = "Claude app · janecorp-jane"
    running = {f"app:claude-app:{home}/Library/Application Support/Claude" "Profiles/b-jane"}
    lines = [f"FAIL runs but isn't in Open now: {app}", f"(Open now 2: 1 below the visible rows, in {home}/x)"]
    text = recorded(lines, running, {app}, set(), ["janecorp-jane"], {"open": 2}, [app, "janecorp-jane"])
    for leaked in ("jane" + "doe", "janecorp", "b-jane"):
        assert leaked not in text, f"{leaked!r} reached the record:\n{text}"
    assert "App 1" in text, text
    print("presence_check: selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pid", type=int, required=True, help="the AI Juggler build's pid (by pid only)")
    parser.add_argument("--data", type=Path, default=HOME / "Library/Application Support/AI Juggler Dev")
    parser.add_argument("--record", type=Path, help="write the report here too")
    parser.add_argument("--since", default=None, help="count lab runs in a real profile's folder only from this day (default: all, the release gate)")
    args = parser.parse_args()

    running = running_ids()
    mine = users_apps(args.data)
    open_now, inactive, inactive_words, counts, drawn = rail(args.pid)
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
    # The rail draws only the rows on screen; its heading counts them all.
    # By App, Open now counts every App open: the user's running ones and
    # the CLI Apps with a session open (named on screen when drawn).
    shown_cli = {name for name in open_now if not name.startswith(desktop_words)}
    problems = []
    if "open" in counts and counts["open"] < len(want_open):
        problems.append(f"Open now counts {counts['open']}, fewer than your {len(want_open)} running Apps")
    if len(shown_open) + len(shown_cli) < counts.get("open", 0):
        lines.append(f"(Open now {counts['open']}: {counts['open'] - len(shown_open) - len(shown_cli)} below the visible rows, checked by count)")
    seen_all = len(shown_open) + len(shown_cli) >= counts.get("open", 0)
    for name in sorted(want_open - shown_open):
        if seen_all:
            problems.append(f"runs but isn't in Open now: {name}")
    for name in sorted(shown_open - want_open):
        problems.append(f"in Open now but not running: {name}")
    for name in sorted(set(open_now) & set(inactive)):
        problems.append(f"in Open now and Inactive apps: {name}")
    for name in sorted(set(inactive) & want_open):
        problems.append(f"runs but is under Inactive apps: {name}")
    if re.search(r"\bRunning\b", inactive_words):
        problems.append("Inactive apps says Running")
    # 0.4.3: a folded Open now proves nothing about what its Apps unfold to.
    if counts.get("open", 0) > 0 and not any(app.section == "open" for app in drawn):
        problems.append("Open now is folded or not drawn: unfold it and run again")
    # 0.4.3: what each unfolds to, its counts and its Out mark.
    db = sqlite3.connect(f"file:{args.data / 'juggler.sqlite'}?mode=ro", uri=True)
    known = presence_sessions.known_apps(db, mine)
    since = time.mktime(time.strptime(args.since, "%Y-%m-%d")) if args.since else 0.0
    leftovers = presence_sessions.lab_leftovers([app.state_dir for app in known if app.state_dir], since)
    # 0.4.3 (D-102): each Codex home's listed threads, read-only from its store.
    homes = {app.folder: app.state_dir for app in known if app.tool == "codex" and app.folder and app.state_dir}
    codex = {folder: presence_sessions.codex_listed(home) for folder, home in homes.items()}
    shown, more = presence_sessions.check(
        drawn, known, presence_sessions.out_accounts(db, int(time.time() * 1000)), leftovers, presence_sessions.own_sessions(db), codex
    )
    lines += ["each App as drawn:", *shown]
    problems += more
    lines += [f"FAIL {p}" for p in problems] or [
        "PASS Open now is exactly your running Apps; none is in two sections; each unfolds to its own records; every Out Account marked; no lab run in a real profile; Codex counts are the store's listed threads"
    ]
    report = "\n".join(lines)
    print(report)
    if args.record:
        names = sorted(want_open | shown_open | set(inactive) | set(mine.values()) | {app.name for app in known} | {app.name for app in drawn})
        args.record.parent.mkdir(parents=True, exist_ok=True)
        args.record.write_text(recorded(lines, running, want_open, shown_open, inactive, counts, names))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(selftest() if sys.argv[1:] == ["--selftest"] else main())
