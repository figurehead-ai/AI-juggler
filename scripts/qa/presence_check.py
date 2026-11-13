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
App, Open now unfolded and every App in it unfolded. 0.4.4 (D-103.10): it
prints what the one-time pass that gives past limit hits back to the
Account each happened on counted, and fails when hits stay on the wrong
Account after it.

    python3 scripts/qa/presence_check.py --pid <Dev build pid> [--home DIR] [--data DIR] [--record FILE] [--since YYYY-MM-DD]

On a lab (`scripts/qa/lab_scenario.sh`, the build started with its HOME),
`--home <the lab>`: the build's data folder and the Claude app's default
instance are the lab's, the apps' records and Codex threads are read where
the lab's data says they are, and only the lab's own processes run for it,
as the build sees them (AJ-E3306): this Mac's own apps (its ChatGPT.app
too) aren't the lab's.
"""

from __future__ import annotations

import argparse
import json
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


def processes() -> list[tuple[int, int, str]]:
    """Every process: its pid, its parent's and its command line."""
    out = subprocess.run(["ps", "-axww", "-o", "pid=,ppid=,args="], capture_output=True, text=True, check=True).stdout
    rows = []
    for line in out.splitlines():
        parts = line.split(None, 2)
        if len(parts) >= 2 and parts[0].isdigit() and parts[1].isdigit():
            rows.append((int(parts[0]), int(parts[1]), parts[2] if len(parts) > 2 else ""))
    return rows


def lab_only(rows: list[tuple[int, int, str]], lab: Path, build: int) -> list[tuple[int, int, str]]:
    """AJ-E3306 (`crates/juggler-app/src/lab_scope.rs`): a build on a lab's
    HOME sees only the lab's processes: one whose program or an argument
    names a path in the lab, or one the build started. This Mac's own apps
    run beside the lab, but not for it."""
    bare = str(lab).removeprefix("/private")
    spellings = (bare, f"/private{bare}")
    parent = {pid: ppid for pid, ppid, _ in rows}

    def descends(pid: int | None) -> bool:
        for _ in range(64):
            if pid == build:
                return True
            pid = parent.get(pid or 0)
            if not pid:
                return False
        return False

    def names_lab(args: str) -> bool:
        # As the build reads each argument: one starting with the lab's path,
        # or naming it after an `=` (a joined command line: after a space).
        return any(args.startswith(s) or f"={s}" in args or f" {s}" in args for s in spellings)

    return [row for row in rows if descends(row[0]) or names_lab(row[2])]


def other_home(home: Path) -> bool:
    """`home` isn't this user's own (the user record's): a lab's."""
    import pwd

    own = pwd.getpwuid(os.getuid()).pw_dir
    return os.path.realpath(home) != os.path.realpath(own)


def running_ids(claude_default: Path = CLAUDE_DEFAULT, rows: list[tuple[int, int, str]] | None = None) -> set[str]:
    """The App ids of the desktop apps running (in `rows`, else every
    process), as the backend keys them (`claude_default`: the Claude app's
    default instance under the HOME the build runs with)."""
    ids: set[str] = set()
    for _, _, args in processes() if rows is None else rows:
        if args == CHATGPT_MAIN or args.startswith(CHATGPT_MAIN + " "):
            ids.add("app:codex-app:")
        elif args == CLAUDE_MAIN or args.startswith(CLAUDE_MAIN + " "):
            m = re.search(r"--user-data-dir[= ](.+?)(?= --|$)", args)
            ids.add(f"app:claude-app:{m.group(1) if m else claude_default}")
    return ids


def users_apps(data: Path, claude_default: Path = CLAUDE_DEFAULT) -> dict[str, str]:
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
        default = instance and Path(instance) == claude_default
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


PAST_HITS_KEY = "limits.past_hits_given_back"
PAST_HITS = "  limit hits given back (D-103.10): "


def past_hits(db: sqlite3.Connection) -> tuple[str, int]:
    """D-103.10: the counts the build kept when it gave limit hits an
    earlier version credited to the wrong Account back to the one each
    happened on (once, at its first start), and how many stayed wrong."""
    try:
        row = db.execute("select value from app_settings where key = ?", (PAST_HITS_KEY,)).fetchone()
    except sqlite3.Error:
        row = None
    if not row:
        return PAST_HITS + "not run on this data yet (a build before 0.4.4)", 0
    try:
        c = json.loads(row[0])
        words = (
            f"{c['wrong_before']} on the wrong Account before, {c['wrong_after']} after; "
            f"{c['moved']} moved, {c['cleared']} cleared (their Account is gone), "
            f"{c['left']} left where they happened"
        )
        return PAST_HITS + words, int(c["wrong_after"])
    except (ValueError, KeyError, TypeError):
        return PAST_HITS + "its record can't be read", 0


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
    ] + [line for line in lines if line.startswith(("FAIL", "PASS", "(", "  open: ", "  idle: ", "  codex store: ", PAST_HITS))]
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
    # D-103.10: the past-hits line reaches the record, and a hit still on
    # the wrong Account after the pass is a failure.
    db = sqlite3.connect(":memory:")
    assert past_hits(db) == (PAST_HITS + "not run on this data yet (a build before 0.4.4)", 0)
    db.execute("create table app_settings (key text primary key, value text)")
    counts = {"wrong_before": 3, "moved": 2, "cleared": 1, "left": 4, "wrong_after": 0}
    db.execute("insert into app_settings values (?, ?)", (PAST_HITS_KEY, json.dumps(counts)))
    line, wrong = past_hits(db)
    assert wrong == 0 and "3 on the wrong Account before, 0 after; 2 moved, 1 cleared" in line, line
    assert line in recorded([line], running, {app}, set(), [], {}, [app]), "kept in the record"
    # On a lab, only the lab's processes run for the build (AJ-E3306).
    lab = Path("/tmp/aij-lab-z1")
    rows = [
        (10, 1, f"{CLAUDE_MAIN} --user-data-dir=/tmp/aij-lab-z1/Library/Application Support/ClaudeProfiles/work"),
        (11, 1, f"{CLAUDE_MAIN} --user-data-dir={home}/Library/Application Support/ClaudeProfiles/b-work"),
        (12, 1, CHATGPT_MAIN),
        (13, 1, "/private/tmp/aij-lab-z1/bin/tool --x"),
        (20, 1, "juggler-app"),
        (21, 20, "/usr/local/bin/claude --resume x"),
        (22, 1, "/usr/bin/sleep 5"),
    ]
    assert [row[0] for row in lab_only(rows, lab, 20)] == [10, 13, 20, 21], lab_only(rows, lab, 20)
    assert running_ids(Path("/x"), lab_only(rows, lab, 20)) == {
        "app:claude-app:/tmp/aij-lab-z1/Library/Application Support/ClaudeProfiles/work"
    }
    assert running_ids(Path("/x"), rows) >= {"app:codex-app:"}, "on the user's own home, every process"
    print("presence_check: selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pid", type=int, required=True, help="the AI Juggler build's pid (by pid only)")
    parser.add_argument("--home", type=Path, default=HOME, help="the HOME the build runs with (a lab's: lab_scenario.sh)")
    parser.add_argument("--data", type=Path, help="its data folder (default: AI Juggler Dev's under --home)")
    parser.add_argument("--record", type=Path, help="write the report here too")
    parser.add_argument("--since", default=None, help="count lab runs in a real profile's folder only from this day (default: all, the release gate)")
    args = parser.parse_args()
    home = args.home.expanduser()
    args.data = args.data or home / "Library/Application Support/AI Juggler Dev"
    claude_default = home / "Library/Application Support/Claude"

    rows = processes()
    lab = other_home(home)
    if lab:
        rows = lab_only(rows, home, args.pid)
    running = running_ids(claude_default, rows)
    mine = users_apps(args.data, claude_default)
    open_now, inactive, inactive_words, counts, drawn = rail(args.pid)
    desktop_words = ("Claude app", "Codex app")
    shown_open = {name for name in open_now if name.startswith(desktop_words)}
    want_open = {mine[i] for i in running if i in mine}
    lines = [
        "presence-check (ADR-0032)",
        *(["(a lab: only its own processes count, as the build sees them, AJ-E3306)"] if lab else []),
        f"running desktop apps (ps): {len(running)}",
        *[f"  {i.replace(str(home), '~').replace(str(HOME), '~')}{'' if i in mine else '  (not one of your Apps)'}" for i in sorted(running)],
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
    hits_line, hits_wrong = past_hits(db)
    lines.append(hits_line)
    if hits_wrong:
        problems.append(f"{hits_wrong} limit hit(s) still on an Account they didn't happen on after the one-time pass (D-103.10)")
    lines += [f"FAIL {p}" for p in problems] or [
        "PASS Open now is exactly your running Apps; none is in two sections; each unfolds to its own records; every Out Account marked; no lab run in a real profile; Codex counts are the store's listed threads; no past limit hit left on the wrong Account"
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
