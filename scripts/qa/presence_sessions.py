#!/usr/bin/env python3
"""0.4.3: what each App on the rail unfolds to, against the apps' own records.

`presence_check.py` (ADR-0032) checked only which Apps sit in Open now. The
0.4.2 rail passed it while three running Claude app profiles unfolded to
nothing, none wore its Account's Out mark, and two CLI Apps listed our own
lab runs. This reads the same accessibility lines and checks, read-only:

1. **Unfold.** Every header with a fold, unfolded, has a row under it; a
   header with nothing to show has no fold. A running App with nothing
   open shows its last ones (up to 3), then "All N sessions ›" when it has
   more ("Show all N" before 0.4.4, D-103.2).
2. **Records.** A Claude app profile's sessions are its Code tab's records
   (`<instance>/claude-code-sessions/*/*/local_*.json`, by `cliSessionId`,
   with a transcript in its folder): the rows and counts the rail shows
   for it come from them, nothing else. A Claude Code CLI App's are its
   folder's transcripts no desktop entrypoint started and no profile's
   records hold.
3. **Out marks.** Every App whose Account is out (its latest measured
   five-hour or weekly reading at 100%) says so in its name on the rail
   (the header's, the Inactive row's), and none whose Account has room.
4. **Our labs stay out (a release gate).** No terminal transcript (`cli`,
   `sdk-cli`) in a real profile's folder runs in a lab folder (`~/lab/`,
   `~/Dev/aij-`), however old: our tests and walks use the sanctioned test
   profile and the Dev build's Lab CLI App only. On 2026-11-12 the
   maintainer moved the 62 our labs had left out of two profiles; one more
   is a failed gate.
5. **Scripts' runs and Codex (0.4.3, D-102).** A script's run (`claude -p`:
   first entrypoint `sdk-cli` or `sdk-py`; `codex exec`) is in no App's
   count, as neither tool's own picker lists it. Each Codex thread a Codex
   surface lists (`state_5.sqlite`: source `cli` or `vscode`, not
   archived) is in one App by its originator: the Codex app's count plus
   Codex CLI's is the store's listed total, and Codex CLI's is the
   threads no desktop originator started. The store is opened read-only
   (or a copy is read); nothing in it is written.

Nothing here clicks, types or writes outside the report.
"""

from __future__ import annotations

import json
import mmap
import re
import shutil
import sqlite3
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

HOME = Path.home()
DESKTOP_ENTRYPOINTS = (b'"entrypoint":"claude-desktop"', b'"entrypoint":"sdk-ts"')
# D-102: a transcript whose first entrypoint is one of these is a script's run.
SCRIPT_ENTRYPOINTS = ("sdk-cli", "sdk-py")
# D-102: the sources of a Codex thread a Codex surface lists.
CODEX_LISTED_SOURCES = ("cli", "vscode")
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
        elif m := re.match(r"^(?:All (\d+) sessions? ›|Show all (\d+))$", words):
            cur.show_all = int(m.group(1) or m.group(2))
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


def first_entrypoint(path: Path) -> str | None:
    """The first `entrypoint` a transcript's lines name (D-102)."""
    try:
        with path.open(errors="replace") as handle:
            for n, line in enumerate(handle):
                if n > 400:
                    break
                if '"entrypoint"' in line:
                    return json.loads(line).get("entrypoint")
    except (OSError, ValueError):
        return None
    return None


def cli_records(state_dir: Path, held: set[str], own: dict[str, str | None]) -> Records:
    """A Claude Code CLI App's sessions: its folder's transcripts no desktop
    app started, no profile's records hold and no script ran (D-102), and
    AI Juggler's own sessions on its folder that no such transcript is
    (`own`: session id → its conversation's id; AI Juggler's runs are
    `sdk-cli` too, and counted as its sessions)."""
    terminal = {
        sid
        for sid, path in transcripts(state_dir).items()
        if sid not in held and not started_by_desktop(path) and first_entrypoint(path) not in SCRIPT_ENTRYPOINTS
    }
    n = len(terminal) + sum(1 for conversation in own.values() if conversation not in terminal)
    return Records(listed=n, shown=n)


def script_runs(state_dir: Path) -> int:
    """How many of a Claude Code folder's transcripts a script ran (D-102)."""
    return sum(1 for path in transcripts(state_dir).values() if first_entrypoint(path) in SCRIPT_ENTRYPOINTS)


def rollout_originator(path: Path) -> str | None:
    """A Codex rollout's originator, from its first line (`session_meta`);
    a compressed one's first line through `zstd`, never the whole file."""
    try:
        if path.name.endswith(".zst"):
            with subprocess.Popen(["zstd", "-dcq", str(path)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL) as proc:
                first = proc.stdout.readline() if proc.stdout else b""
                proc.kill()
        else:
            with path.open("rb") as handle:
                first = handle.readline()
        return json.loads(first).get("payload", {}).get("originator")
    except (OSError, ValueError, AttributeError):
        return None


def codex_store(home: Path) -> Path | None:
    """The Codex home's thread store (`state_<n>.sqlite`, the newest)."""
    found = sorted(home.glob("state_*.sqlite"), key=lambda path: int(re.sub(r"\D", "", path.stem) or 0))
    return found[-1] if found else None


def codex_threads(store: Path) -> list[tuple[str | None, str]]:
    """Each listed thread's originator (`None` when the table has none) and
    rollout path, read-only: the store opened `mode=ro`, else (a closed WAL
    store has no shared memory to read through) a copy of it."""
    has = "select name from pragma_table_info('threads') where name = 'originator'"
    query = "select {originator}, rollout_path from threads where source in ({sources}) and archived = 0"
    sources = ", ".join(f"'{source}'" for source in CODEX_LISTED_SOURCES)

    def read(db: sqlite3.Connection) -> list[tuple[str | None, str]]:
        originator = "originator" if db.execute(has).fetchone() else "null"
        return list(db.execute(query.format(originator=originator, sources=sources)))

    try:
        db = sqlite3.connect(f"{store.as_uri()}?mode=ro", uri=True)
        try:
            return read(db)
        finally:
            db.close()
    except sqlite3.OperationalError:
        pass
    with tempfile.TemporaryDirectory(prefix="codex-listed-") as tmp:
        copy = Path(tmp) / store.name
        for suffix in ("", "-wal"):
            if Path(f"{store}{suffix}").exists():
                shutil.copyfile(f"{store}{suffix}", f"{copy}{suffix}")
        db = sqlite3.connect(copy)
        try:
            return read(db)
        finally:
            db.close()


def codex_listed(home: Path) -> tuple[int, int]:
    """0.4.3 (D-102): the threads a Codex surface lists (source `cli` or
    `vscode`, not archived; a subagent's has another source), as (the Codex
    app's: an originator naming desktop, the rest: Codex CLI's), from the
    `threads` table's originator, else each rollout's first line."""
    store = codex_store(home)
    if store is None:
        return 0, 0
    desktop = other = 0
    for originator, rollout in codex_threads(store):
        if originator is None:
            originator = rollout_originator(Path(rollout))
        if "desktop" in (originator or "").lower():
            desktop += 1
        else:
            other += 1
    return desktop, other


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


TERMINAL_ENTRYPOINTS = ("cli", "sdk-cli")


def lab_leftovers(state_dirs: list[Path], since: float = 0.0) -> int:
    """Terminal transcripts (`cli`, `sdk-cli`) in a real profile's folder
    that run in a lab folder: our tests on a real Account. `since` (epoch
    s, default always) counts only those made after it."""
    count = 0
    for state_dir in state_dirs:
        if state_dir.name in SANCTIONED:
            continue
        for path in transcripts(state_dir).values():
            try:
                if path.stat().st_birthtime < since:
                    continue
                cwd = entry = None
                with path.open(errors="replace") as handle:
                    for n, line in enumerate(handle):
                        if n > 400 or (cwd and entry):
                            break
                        if '"cwd"' in line or '"entrypoint"' in line:
                            value = json.loads(line)
                            cwd = cwd or value.get("cwd")
                            entry = entry or value.get("entrypoint")
            except (OSError, ValueError):
                continue
            if entry in TERMINAL_ENTRYPOINTS and cwd and cwd.startswith(LAB_ROOTS):
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


def drawn_count(app: RailApp) -> int | None:
    """How many sessions the rail says an App has: its "All N sessions ›" when
    drawn; an Inactive App's "N sessions"; else the rows under an unfolded
    App with nothing open (a header with no fold has none). `None`: not
    drawn (folded, or something open in it)."""
    if app.show_all is not None:
        return app.show_all
    if app.section == "idle" and app.sessions is not None:
        return app.sessions
    if app.foldable and app.folded:
        return None
    if app.section == "open" and not app.nothing_open:
        return None
    return app.rows


def codex_problems(rail: list[RailApp], apps: list[KnownApp], codex: dict[str, tuple[int, int]]) -> tuple[list[str], list[str]]:
    """D-102: each Codex home's listed threads (`codex`: folder → (the
    Codex app's, the rest)) against the rail: the Codex app's count plus
    Codex CLI's Inactive "N sessions" is the total, Codex CLI's the rest
    (alone when the Codex app's count isn't drawn)."""
    lines: list[str] = []
    problems: list[str] = []
    for folder, (desktop, other) in sorted(codex.items()):
        named = {(app.kind, app.name) for app in apps if app.tool == "codex" and app.folder == folder}
        app = next((one for one in rail if ("desktop", one.name) in named), None)
        cli = next((one for one in rail if ("cli", one.name) in named and one.section == "idle"), None)
        app_count = drawn_count(app) if app else None
        cli_count = (cli.sessions or 0) if cli else None
        lines.append(f"  codex store: {desktop} by the Codex app + {other} others = {desktop + other} listed; rail: app={app_count} cli={cli_count}")
        if cli_count is not None and cli_count != other:
            problems.append(f"Codex CLI says {cli_count} sessions, the Codex store lists {other} not the Codex app's: {cli.name}")  # type: ignore[union-attr]
        if app_count is not None and cli_count is not None and app_count + cli_count != desktop + other:
            problems.append(f"the Codex app's {app_count} and Codex CLI's {cli_count} aren't the store's {desktop + other} listed threads")
    return lines, problems


def check(
    rail: list[RailApp],
    apps: list[KnownApp],
    out: set[str],
    leftovers: int,
    own: dict[str, dict[str, str | None]] | None = None,
    codex: dict[str, tuple[int, int]] | None = None,
) -> tuple[list[str], list[str]]:
    """The lines to print and the problems: unfold, records, Out marks,
    labs, and (D-102) each Codex home's listed threads (`codex`, from
    [`codex_listed`] by folder)."""
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
                problems.append(f"All N sessions says {shown.show_all}, its records hold {mine.listed}: {shown.name}")
    if leftovers:
        problems.append(f"{leftovers} new transcript(s) of a lab run in a real profile's folder")
    more_lines, more = codex_problems(rail, apps, codex or {})
    return lines + more_lines, problems + more


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
        ("AXStaticText", "Recent", ""), ("AXButton", "Ended, A, proj · 1 d ago", ""), ("AXButton", "Ended, B, proj · 2 d ago", ""), ("AXButton", "Ended, C, proj · 3 d ago", ""), ("AXButton", "All 9 sessions ›", ""),
        ("AXButton", "Claude app · West, nothing open", ""), ("AXButton", "Inactive apps 1", ""), ("AXButton", "Claude app · Gone, Desktop app, 4 d ago, Out of room until Sun", ""),
    ]
    got = parse_rail(new)
    assert [(a.name, a.foldable, a.rows, a.show_all, a.out) for a in got] == [
        ("Claude app · East", True, 3, 9, True), ("Claude app · West", False, 0, None, False), ("Claude app · Gone", False, 0, None, True)
    ], got
    # An installed 0.4.3 says "Show all 9" (D-103.2 renamed it in 0.4.4).
    assert parse_rail([*new[:7], ("AXButton", "Show all 9", ""), *new[8:]])[0].show_all == 9
    east = KnownApp("Claude app · East", "desktop", "codex", "f-east", None, None)
    _, problems = check(got, [east], {"f-east"}, 0)
    assert problems == [], problems
    folded_idle = [RailApp(name="Claude Code CLI · East", section="idle", foldable=True, folded=True, sessions=51)]
    assert not check(folded_idle, [], set(), 0)[1]
    _, problems = check(parse_rail(old), [east], {"f-east"}, 2)
    assert any("no Out mark" in p for p in problems) and any("lab run" in p for p in problems), problems
    selftest_d102()
    print("presence_sessions: selftest passed")


def selftest_d102() -> None:
    """D-102: a Codex store's listed threads (each kind, the store closed in
    WAL mode, with and without an originator column), scripts' runs out of
    a CLI App's records, and the rail's Codex counts against the store."""
    kinds = [  # source, originator, archived: listed by a Codex surface?
        ("vscode", "Codex Desktop", 0), ("vscode", "codex_work_desktop", 0), ("vscode", "ai_juggler", 0), ("cli", "codex_cli_rs", 0),
        ("exec", "codex_exec", 0), ('{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}', "Codex Desktop", 0), ("vscode", "Codex Desktop", 1),
    ]
    with tempfile.TemporaryDirectory() as tmp:
        for with_column in (True, False):
            home = Path(tmp) / f"codex-{with_column}"
            home.mkdir()
            db = sqlite3.connect(home / "state_5.sqlite")
            db.execute("pragma journal_mode=wal")
            db.execute(f"create table threads (id text, rollout_path text, source text, archived integer{', originator text' if with_column else ''})")
            for n, (source, originator, archived) in enumerate(kinds):
                rollout = home / f"rollout-{n}.jsonl"
                rollout.write_text(json.dumps({"type": "session_meta", "payload": {"id": f"t{n}", "originator": originator}}) + "\n")
                row = (f"t{n}", str(rollout), source, archived) + ((originator,) if with_column else ())
                db.execute(f"insert into threads values ({', '.join('?' * len(row))})", row)
            db.commit()
            db.execute("pragma wal_checkpoint(truncate)")
            db.close()
            # As Codex leaves it closed: no shared memory to read through.
            for suffix in ("-wal", "-shm"):
                Path(f"{home}/state_5.sqlite{suffix}").unlink(missing_ok=True)
            assert codex_listed(home) == (2, 2), (with_column, codex_listed(home))
        assert codex_listed(Path(tmp)) == (0, 0), "no store"
        folder = Path(tmp) / "claude"
        (folder / "projects/-w").mkdir(parents=True)
        for sid, entry in (("term", "cli"), ("run", "sdk-cli"), ("py", "sdk-py"), ("ours", "sdk-cli")):
            (folder / f"projects/-w/{sid}.jsonl").write_text(json.dumps({"type": "user", "sessionId": sid, "entrypoint": entry}) + "\n")
        assert script_runs(folder) == 3
        assert cli_records(folder, set(), {"s-1": "ours"}).listed == 2, "the terminal's and AI Juggler's own, once"
    codex_apps = [KnownApp("Codex app", "desktop", "codex", "f-codex", None, None), KnownApp("Codex CLI · Codex home", "cli", "codex", "f-codex", None, None)]

    def rail(cli_sessions: int, app_folded: bool) -> list[RailApp]:
        return parse_rail([
            ("AXButton", "Open now 1", ""), ("AXButton", f"{'Unfold' if app_folded else 'Fold'} Codex app", ""), ("AXButton", "Codex app, nothing open", ""),
            *([] if app_folded else [("AXButton", f"Ended, T{n}, proj · {n} d ago", "") for n in (1, 2, 3)] + [("AXButton", "All 15 sessions ›", "")]),
            ("AXButton", "Inactive apps 1", ""), ("AXButton", "Unfold Codex CLI · Codex home", ""),
            ("AXButton", f"Codex CLI · Codex home, CLI app, 2 d ago · {cli_sessions} sessions", ""),
        ])

    store = {"f-codex": (15, 33)}
    assert not check(rail(33, False), codex_apps, set(), 0, None, store)[1]
    _, problems = check(rail(92, False), codex_apps, set(), 0, None, store)
    assert len(problems) == 2 and "says 92 sessions" in problems[0] and "aren't the store's 48" in problems[1], problems
    _, problems = check(rail(92, True), codex_apps, set(), 0, None, store)
    assert len(problems) == 1 and "says 92 sessions" in problems[0], "the app's count not drawn: Codex CLI's alone"
    assert not check(rail(33, True), codex_apps, set(), 0, None, store)[1]


if __name__ == "__main__":
    selftest()
