#!/usr/bin/env python3
"""0.4.3: what each App on the rail unfolds to, against the apps' own records.

`presence_check.py` (ADR-0032) checked only which Apps sit in Open now. The
0.4.2 rail passed it while three running Claude app profiles unfolded to
nothing, none wore its Account's Out mark, and two CLI Apps listed our own
lab runs. This reads the same accessibility lines and checks, read-only:

1. **Unfold.** Every header with a fold, unfolded, has a row under it; a
   header with nothing to show has no fold. A running App with nothing
   open shows its last ones (up to 3), then "Show all N" when it has more.
2. **Records.** A Claude app profile's sessions are its Code tab's records
   (`<instance>/claude-code-sessions/*/*/local_*.json`, by `cliSessionId`,
   with a transcript in its folder): the rows and counts the rail shows
   for it come from them, nothing else. A Claude Code CLI App's are its
   folder's transcripts no desktop entrypoint started and no profile's
   records hold.
3. **Out marks.** Every App whose Account is out (its latest measured
   five-hour or weekly reading at 100%) says so in its name on the rail
   (the header's, the Inactive row's), and none whose Account has room.
4. **Our labs stay out.** No transcript made since `since` in a real
   profile's folder runs in a lab folder (`~/lab/`, `~/Dev/aij-`): our
   tests and walks use the sanctioned test profile and the Dev build's Lab
   CLI App only.

Nothing here clicks, types or writes outside the report.
"""

from __future__ import annotations

import json
import mmap
import re
import sqlite3
from dataclasses import dataclass, field
from pathlib import Path

HOME = Path.home()
DESKTOP_ENTRYPOINTS = (b'"entrypoint":"claude-desktop"', b'"entrypoint":"sdk-ts"')
LAB_ROOTS = (str(HOME / "lab") + "/", str(HOME / "Dev/aij-"))
# The folders our tests may use: the sanctioned test profile's.
SANCTIONED = ("aij-test",)
ROW = re.compile(r"^(?:Probably )?(Asks you|Working|Idle|Done|Ended|Starting|Stopped|Failed|Finished)\b.*?, ")


@dataclass
class RailApp:
    """One App as the rail draws it."""

    name: str
    section: str
    foldable: bool = False
    folded: bool = False
    nothing_open: bool = False
    open_count: int = 0
    rows: int = 0
    pinned: int = 0
    recent: int | None = None
    show_all: int | None = None
    sessions: int | None = None
    out: bool = False
    words: list[str] = field(default_factory=list)


def parse_rail(lines: list[tuple[str, str, str]]) -> list[RailApp]:
    """The Apps under Open now and Inactive apps, in order, with what's
    drawn under each (`lines`: ax_texts.swift's role, label, value)."""
    apps: list[RailApp] = []
    section: str | None = None
    cur: RailApp | None = None

    def start(name: str, foldable: bool, folded: bool) -> RailApp:
        nonlocal cur
        cur = RailApp(name=name, section=section or "", foldable=foldable, folded=folded)
        apps.append(cur)
        return cur

    for role, label, value in lines:
        words = label or value
        if re.match(r"^Open now \d+", words):
            section, cur = "open", None
            continue
        if re.match(r"^Inactive apps \d+", words):
            section, cur = "idle", None
            continue
        if re.match(r"^(Running, not added|Also:|Ended today|History)\b", words):
            section, cur = None, None
            continue
        if section is None or role != "AXButton":
            continue
        if m := re.match(r"^(Fold|Unfold) (.+)$", words):
            start(m.group(2), True, m.group(1) == "Unfold")
            continue
        head = re.match(r"^(.+?), (nothing open|(\d+) open)(?:, \d+ need you)?(?:, (.*))?$", words) if section == "open" else None
        idle = re.match(r"^(.+?), (Desktop app|CLI app)(?:, (.*))?$", words) if section == "idle" else None
        folded_head = cur is not None and cur.folded and not cur.words and " · " in words and not ROW.match(words)
        if head or idle or folded_head:
            name = (head or idle).group(1) if (head or idle) else cur.name  # type: ignore[union-attr]
            app = cur if cur is not None and cur.name == name and not cur.words else start(name, False, False)
            app.words.append(words)
            app.out = "Out of room" in words
            app.nothing_open = "nothing open" in words
            if head and head.group(3):
                app.open_count = int(head.group(3))
            if idle and (n := re.search(r"(\d+) sessions?\b", words)):
                app.sessions = int(n.group(1))
            continue
        if cur is None:
            continue
        if ROW.match(words):
            cur.rows += 1
            cur.pinned += ", pinned," in words or words.endswith(", pinned")
        elif m := re.match(r"^Last (?:(\d+) )?inactive$", words):
            cur.recent = int(m.group(1) or 1)
        elif m := re.match(r"^Show all (\d+)$", words):
            cur.show_all = int(m.group(1))
    return apps


@dataclass
class Records:
    """An App's own records: every session it lists, and the ones it shows
    (not archived), each with a transcript in its folder."""

    listed: int
    shown: int


def transcripts(state_dir: Path) -> dict[str, Path]:
    """A tool folder's top-level transcripts, by session id."""
    out: dict[str, Path] = {}
    for path in (state_dir / "projects").glob("*/*.jsonl"):
        out[path.stem] = path
    return out


def desktop_records(instance_dir: Path, state_dir: Path) -> tuple[Records, set[str]]:
    """A Claude app profile's Code tab records with a transcript, and the
    ids they hold."""
    newest: dict[str, tuple[int, bool]] = {}
    for path in (instance_dir / "claude-code-sessions").glob("*/*/local_*.json"):
        try:
            record = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        sid = record.get("cliSessionId")
        if not sid:
            continue
        at = int(record.get("lastActivityAt") or 0)
        if sid not in newest or at >= newest[sid][0]:
            newest[sid] = (at, bool(record.get("isArchived")))
    have = transcripts(state_dir)
    ids = {sid for sid in newest if sid in have}
    return Records(listed=len(ids), shown=sum(1 for sid in ids if not newest[sid][1])), set(newest)


def started_by_desktop(path: Path) -> bool:
    """Any line says a desktop app started it (the adapter's `started_in`)."""
    try:
        with path.open("rb") as handle, mmap.mmap(handle.fileno(), 0, access=mmap.ACCESS_READ) as data:
            return any(data.find(word) != -1 for word in DESKTOP_ENTRYPOINTS)
    except (OSError, ValueError):
        return False


def cli_records(state_dir: Path, held: set[str], own: dict[str, str | None]) -> Records:
    """A Claude Code CLI App's sessions: its folder's transcripts no desktop
    app started and no profile's records hold, and AI Juggler's own
    sessions on its folder that no such transcript is (`own`: session id
    → its conversation's id)."""
    terminal = {sid for sid, path in transcripts(state_dir).items() if sid not in held and not started_by_desktop(path)}
    n = len(terminal) + sum(1 for conversation in own.values() if conversation not in terminal)
    return Records(listed=n, shown=n)


def own_sessions(db: sqlite3.Connection) -> dict[str, dict[str, str | None]]:
    """AI Juggler's own sessions in no desktop App, by folder: session id →
    its conversation's id (the CLI App's records of AI Juggler's runs)."""
    out: dict[str, dict[str, str | None]] = {}
    for sid, folder, conversation in db.execute(
        "select i.id, r.account_id, i.conversation_id from instances i join launch_recipes r on r.id = i.launch_recipe_id "
        "where i.app_id is null or i.app_id not like 'app:%'"
    ):
        out.setdefault(folder, {})[sid] = conversation
    return out


def out_accounts(db: sqlite3.Connection, now_ms: int) -> set[str]:
    """The folders whose Account is out: its latest measured five-hour or
    weekly reading at 100%, not past its reset (the newest reading is the
    truth: a lower one after a 100% means the window ended early); one
    folder out puts every folder of its subscription out."""
    out_folders: set[str] = set()
    for folder, window, used, resets in db.execute(
        "select account, window, used_pct, resets_at from quota_samples q where window in ('five_hour', 'weekly') "
        "and ts = (select max(ts) from quota_samples where account = q.account and window = q.window)"
    ):
        if used >= 100 and (resets is None or resets > now_ms):
            out_folders.add(folder)
    subs = dict(db.execute("select id, subscription_id from accounts"))
    out_subs = {subs.get(folder) for folder in out_folders if subs.get(folder)}
    return out_folders | {folder for folder, sub in subs.items() if sub in out_subs}


def lab_leftovers(state_dirs: list[Path], since: float) -> int:
    """Transcripts made since `since` in a real profile's folder that run
    in a lab folder: our tests on a real Account."""
    count = 0
    for state_dir in state_dirs:
        if state_dir.name in SANCTIONED:
            continue
        for path in transcripts(state_dir).values():
            try:
                if path.stat().st_birthtime < since:
                    continue
                with path.open(errors="replace") as handle:
                    cwd = next((json.loads(line).get("cwd") for line in handle if '"cwd"' in line), None)
            except (OSError, ValueError, StopIteration):
                continue
            if cwd and cwd.startswith(LAB_ROOTS):
                count += 1
    return count


@dataclass
class KnownApp:
    """An App of the database: its name on the rail, kind, folder, and its
    instance's and tool folder's paths."""

    name: str
    kind: str
    tool: str
    folder: str | None
    instance: Path | None
    state_dir: Path | None


def known_apps(db: sqlite3.Connection, desktop_names: dict[str, str]) -> list[KnownApp]:
    """Every App of the database by its rail name (`desktop_names`:
    presence_check's desktop App names by id)."""
    folders = {fid: (label, Path(state)) for fid, label, state in db.execute("select id, label, state_dir from accounts where removed_at is null")}
    out = []
    for app_id, kind, tool, instance, folder in db.execute("select id, kind, tool, instance_dir, folder_id from apps"):
        label, state = folders.get(folder, (None, None))
        if kind == "desktop":
            name = desktop_names.get(app_id)
            if name is None:
                continue
        else:
            word = "Codex CLI" if tool == "codex" else "Claude Code CLI"
            name = f"{word} · {label}" if label else word
        out.append(KnownApp(name=name, kind=kind, tool=tool, folder=folder, instance=Path(instance) if instance else None, state_dir=state))
    return out


def check(rail: list[RailApp], apps: list[KnownApp], out: set[str], leftovers: int, own: dict[str, dict[str, str | None]] | None = None) -> tuple[list[str], list[str]]:
    """The lines to print and the problems: unfold, records, Out marks, labs."""
    by_name = {app.name: app for app in apps}
    # A profile's records hold its own folder's copy only: a switch copies
    # a conversation into another folder under the same id.
    held: dict[str, set[str]] = {}
    records: dict[str, Records] = {}
    for app in apps:
        if app.kind == "desktop" and app.tool == "claude-code" and app.instance and app.state_dir:
            records[app.name], ids = desktop_records(app.instance, app.state_dir)
            held.setdefault(str(app.state_dir), set()).update(ids)
    for app in apps:
        if app.kind == "cli" and app.tool == "claude-code" and app.state_dir:
            records[app.name] = cli_records(app.state_dir, held.get(str(app.state_dir), set()), (own or {}).get(app.folder or "", {}))
    lines: list[str] = []
    problems: list[str] = []
    for shown in rail:
        known = by_name.get(shown.name)
        mine = records.get(shown.name)
        drawn = f"{shown.section}: {shown.name}: fold={'folded' if shown.folded else shown.foldable} rows={shown.rows} pinned={shown.pinned} last={shown.recent} all={shown.show_all} count={shown.sessions} out={shown.out}"
        lines.append(f"  {drawn}" + (f"  records={mine.listed}/{mine.shown} shown" if mine else ""))
        if shown.foldable and not shown.folded and shown.section == "open" and shown.rows == 0 and shown.recent is None:
            problems.append(f"its fold unfolds to nothing: {shown.name}")
        if shown.section == "open" and not shown.foldable and (shown.rows or shown.recent):
            problems.append(f"rows under a header with no fold: {shown.name}")
        if known and known.folder:
            if known.folder in out and not shown.out:
                problems.append(f"its Account is out, no Out mark: {shown.name}")
            if known.folder not in out and shown.out:
                problems.append(f"an Out mark, its Account has room: {shown.name}")
        if mine is None:
            continue
        # Its count is in its name, folded or not.
        if shown.section == "idle" and (shown.sessions or 0) != mine.listed:
            problems.append(f"says {shown.sessions or 0} sessions, its records hold {mine.listed}: {shown.name}")
        if shown.folded:
            continue
        rest = mine.shown - shown.pinned - shown.open_count
        if shown.section == "open" and shown.nothing_open:
            want = min(3, max(rest, 0))
            if shown.rows - shown.pinned != want:
                problems.append(f"{shown.rows - shown.pinned} of its last ones under it, its records say {want}: {shown.name}")
            if rest > 3 and shown.show_all != mine.listed:
                problems.append(f"Show all says {shown.show_all}, its records hold {mine.listed}: {shown.name}")
    if leftovers:
        problems.append(f"{leftovers} new transcript(s) of a lab run in a real profile's folder")
    return lines, problems


def selftest() -> None:
    """The parser reads 0.4.2's and 0.4.3's rails."""
    old = [
        ("AXButton", "Open now 2", ""), ("AXButton", "Unfold Claude app · East", ""), ("AXButton", "East · nothing open", ""),
        ("AXButton", "Ended, Talk, pinned, proj · 1 d ago", ""), ("AXButton", "Unfold Claude app · West", ""), ("AXButton", "West · nothing open", ""),
        ("AXButton", "Inactive apps 1", ""), ("AXButton", "Unfold Claude Code CLI · East", ""), ("AXButton", "Claude Code CLI · East, CLI app, 5 d ago · 51 sessions", ""),
    ]
    got = parse_rail(old)
    assert [(a.name, a.section, a.folded, a.rows, a.sessions) for a in got] == [
        ("Claude app · East", "open", True, 1, None), ("Claude app · West", "open", True, 0, None), ("Claude Code CLI · East", "idle", True, 0, 51)
    ], got
    new = [
        ("AXButton", "Open now 2", ""), ("AXButton", "Fold Claude app · East", ""), ("AXButton", "Claude app · East, nothing open, Out of room until Sun 6:00 AM", ""),
        ("AXButton", "Ended, A, proj · 1 d ago", ""), ("AXButton", "Ended, B, proj · 2 d ago", ""), ("AXButton", "Ended, C, proj · 3 d ago", ""), ("AXButton", "Show all 9", ""),
        ("AXButton", "Claude app · West, nothing open", ""), ("AXButton", "Inactive apps 1", ""), ("AXButton", "Claude app · Gone, Desktop app, 4 d ago, Out of room until Sun", ""),
    ]
    got = parse_rail(new)
    assert [(a.name, a.foldable, a.rows, a.show_all, a.out) for a in got] == [
        ("Claude app · East", True, 3, 9, True), ("Claude app · West", False, 0, None, False), ("Claude app · Gone", False, 0, None, True)
    ], got
    east = KnownApp("Claude app · East", "desktop", "codex", "f-east", None, None)
    _, problems = check(got, [east], {"f-east"}, 0)
    assert problems == [], problems
    folded_idle = [RailApp(name="Claude Code CLI · East", section="idle", foldable=True, folded=True, sessions=51)]
    assert not check(folded_idle, [], set(), 0)[1]
    _, problems = check(parse_rail(old), [east], {"f-east"}, 2)
    assert any("no Out mark" in p for p in problems) and any("lab run" in p for p in problems), problems
    print("presence_sessions: selftest passed")


if __name__ == "__main__":
    selftest()
