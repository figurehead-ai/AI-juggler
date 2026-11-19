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
   folder's transcripts no profile's records hold, no script ran and, when
   one profile holds the folder, no desktop entrypoint started. 0.4.6: as
   the app's one order has it (`places::app_of`,
   `ui/src/lib/fixtures/session-apps.json`): each AI Juggler session is a
   row in the App it ran in (two on one transcript are two), and a
   transcript one is on is that session, not a profile's record too; a
   desktop-started transcript on a folder no profile holds is its CLI
   App's (`claude --resume` lists it: its first entrypoint isn't a
   script's).
3. **Out marks.** Every App whose Account is out (its latest measured
   five-hour or weekly reading at 100%) says so in its name on the rail
   (the header's, the Inactive row's), and none whose Account has room.
4. **Our labs stay out (a release gate).** No terminal transcript (`cli`,
   `sdk-cli`) in a real profile's folder runs in a lab folder (`~/lab/`,
   `~/Dev/aij-`), however old: our tests and walks use the sanctioned test
   profile and the Dev build's Lab CLI App only. On 2026-11-12 the
   maintainer moved the 62 our labs had left out of two profiles; one more
   is a failed gate. 0.4.6: the Lab App's folder is exactly the one the
   Dev build's own database names ([`lab_app_folder`]), by its resolved
   path; every other folder, another Dev-managed one too, is checked.
5. **Scripts' runs and Codex (0.4.3, D-102).** A script's run (`claude -p`:
   first entrypoint `sdk-cli` or `sdk-py`; `codex exec`) is in no App's
   count, as neither tool's own picker lists it. Each Codex thread a Codex
   surface lists (`state_5.sqlite`: source `cli` or `vscode`, not
   archived) is in one App by its originator: the Codex app's count plus
   Codex CLI's is the store's listed total, and Codex CLI's is the
   threads no desktop originator started. 0.4.6 (the rc3 presence run):
   as a Claude Code CLI App's, AI Juggler's own runs on the folder are
   rows of their own (two on one thread two, one that never made a thread
   one), and a thread one of them is on isn't counted again. The store is
   opened read-only (or a copy is read); nothing in it is written.

Nothing here clicks, types or writes outside the report.
"""

from __future__ import annotations

import json
import mmap
import os
import re
import shutil
import sqlite3
import subprocess
import tempfile
from dataclasses import dataclass, field, replace
from pathlib import Path

HOME = Path.home()
DESKTOP_ENTRYPOINTS = (b'"entrypoint":"claude-desktop"', b'"entrypoint":"sdk-ts"')
# D-102: a transcript whose first entrypoint is one of these is a script's run.
SCRIPT_ENTRYPOINTS = ("sdk-cli", "sdk-py")
# D-102: the sources of a Codex thread a Codex surface lists.
CODEX_LISTED_SOURCES = ("cli", "vscode")
LAB_ROOTS = (str(HOME / "lab") + "/", str(HOME / "Dev/aij-"))
# The folders our tests may use: the sanctioned test profile's (and the
# Dev build's Lab App folder, [`lab_app_folder`]).
SANCTIONED = ("aij-test",)
# The Dev build's data folder, under a HOME.
DEV_DATA = "Library/Application Support/AI Juggler Dev"
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


def desktop_records(instance_dir: Path, state_dir: Path, linked: frozenset[str] | set[str] = frozenset()) -> tuple[Records, set[str]]:
    """A Claude app profile's Code tab records with a transcript, and the
    ids they hold. 0.4.6: one an AI Juggler session on its folder is on
    (`linked`) is that session, counted in the App it ran in (the app's
    order: AI Juggler's own session is never placed by a profile's
    records), so it's left out here: never in two Apps."""
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
    ids = {sid for sid in newest if sid in have and sid not in linked}
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


def cli_records(state_dir: Path, held: set[str], own: dict[str, str | None], *, held_folder: bool = True) -> Records:
    """A Claude Code CLI App's sessions, as the app's one order counts
    them (`places::app_of`): every AI Juggler session on its folder that
    ran in no desktop App (`own`: session id → its conversation's id; AI
    Juggler's runs are `sdk-cli` too, and each is a row, two on one
    transcript two), and its folder's transcripts no such session is on,
    no profile's records hold and no script ran (D-102); one a desktop app
    started only when no profile holds the folder (`held_folder`: one
    does, and its records leave out the rest)."""
    linked = {conversation for conversation in own.values() if conversation}
    outside = {
        sid
        for sid, path in transcripts(state_dir).items()
        if sid not in held
        and sid not in linked
        and first_entrypoint(path) not in SCRIPT_ENTRYPOINTS
        and not (held_folder and started_by_desktop(path))
    }
    n = len(own) + len(outside)
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


def codex_threads(store: Path) -> list[tuple[str, str | None, str]]:
    """Each listed thread's id, originator (`None` when the table has none)
    and rollout path, read-only: the store opened `mode=ro`, else (a closed
    WAL store has no shared memory to read through) a copy of it."""
    has = "select name from pragma_table_info('threads') where name = 'originator'"
    query = "select id, {originator}, rollout_path from threads where source in ({sources}) and archived = 0"
    sources = ", ".join(f"'{source}'" for source in CODEX_LISTED_SOURCES)

    def read(db: sqlite3.Connection) -> list[tuple[str, str | None, str]]:
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


def codex_listed(home: Path) -> tuple[frozenset[str], frozenset[str]]:
    """0.4.3 (D-102): the threads a Codex surface lists (source `cli` or
    `vscode`, not archived; a subagent's has another source), by id, as
    (the Codex app's: an originator naming desktop, the rest: Codex CLI's),
    from the `threads` table's originator, else each rollout's first line."""
    store = codex_store(home)
    if store is None:
        return frozenset(), frozenset()
    desktop: set[str] = set()
    other: set[str] = set()
    for thread, originator, rollout in codex_threads(store):
        if originator is None:
            originator = rollout_originator(Path(rollout))
        if "desktop" in (originator or "").lower():
            desktop.add(thread)
        else:
            other.add(thread)
    return frozenset(desktop), frozenset(other)


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


def _has_table(db: sqlite3.Connection, name: str) -> bool:
    return db.execute("select 1 from sqlite_master where type = 'table' and name = ?", (name,)).fetchone() is not None


def out_accounts(db: sqlite3.Connection, now_ms: int) -> set[str]:
    """AJ-E3210 (D-135): the Apps whose own Account is out, by App id (a
    folder no Account lists, by its folder id). An Account is out when its
    newest measured five-hour or weekly reading, wherever it was read, is at
    100% and not past its reset (the newest is the truth: a lower one after
    a 100% means the window ended early). A reading is the Account it was
    read under (V55's tag), else its folder's. An App is on its own
    Account: a desktop App its records' (`app_accounts`), a CLI App its
    folder's; so a Claude app profile out never marks its CLI on another
    Account, nor the other way."""
    subs = dict(db.execute("select id, subscription_id from accounts"))
    tagged = _has_table(db, "quota_sample_sign_ins")
    rows = db.execute(
        "select q.account, q.window, q.ts, q.used_pct, q.resets_at, "
        + ("g.signed_in_as " if tagged else "null ")
        + "from quota_samples q "
        + ("left join quota_sample_sign_ins g on g.account = q.account and g.window = q.window and g.ts = q.ts " if tagged else "")
        + "where q.window in ('five_hour', 'weekly')"
    )
    newest: dict[tuple[str, str], tuple[int, float, int | None]] = {}
    for folder, window, ts, used, resets, under in rows:
        whose = under or subs.get(folder) or f"folder:{folder}"
        if (whose, window) not in newest or ts > newest[(whose, window)][0]:
            newest[(whose, window)] = (ts, used, resets)
    out = {whose for (whose, _), (_, used, resets) in newest.items() if used >= 100 and (resets is None or resets > now_ms)}
    own = dict(db.execute("select app_id, subscription_id from app_accounts")) if _has_table(db, "app_accounts") else {}
    apps: set[str] = set()
    for app_id, kind, folder in db.execute("select id, kind, folder_id from apps"):
        whose = own.get(app_id) if kind == "desktop" else (subs.get(folder) or (f"folder:{folder}" if folder else None))
        if whose in out:
            apps.add(app_id)
    return apps | {folder for folder in subs if f"folder:{folder}" in out}


TERMINAL_ENTRYPOINTS = ("cli", "sdk-cli")


def lab_app_folder(home: Path) -> Path | None:
    """The Dev build's Lab App folder (ADR-0027 7b: "the 'Lab' Claude Code
    App", `docs/qa/0.4-walks/7b-live-claude.md`), its path resolved: the
    one Claude Code Account the Dev build's own database (under `home`)
    calls Lab that is a copy of its own (not in place, not removed),
    inside its own `accounts/` folder. Any other, or two, or none: `None`,
    and nothing is exempt. The installed AI Juggler's data is never read."""
    data = home / DEV_DATA
    try:
        db = sqlite3.connect(f"{(data / 'juggler.sqlite').as_uri()}?mode=ro", uri=True)
        try:
            rows = db.execute(
                "select state_dir from accounts where provider_id = 'claude-code' and label = 'Lab' and in_place = 0 and removed_at is null"
            ).fetchall()
        finally:
            db.close()
    except sqlite3.Error:
        return None
    own = os.path.realpath(data / "accounts") + os.sep
    found = {os.path.realpath(state) for (state,) in rows}
    if len(rows) != 1 or not all(path.startswith(own) for path in found):
        return None
    return Path(found.pop())


def lab_leftovers(state_dirs: list[Path], since: float = 0.0, sanctioned: Path | None = None) -> int:
    """Terminal transcripts (`cli`, `sdk-cli`) in a real profile's folder
    that run in a lab folder: our tests on a real Account. `since` (epoch
    s, default always) counts only those made after it. Each folder once,
    by its resolved path; `sanctioned` ([`lab_app_folder`]) is ours."""
    count = 0
    seen: set[str] = set()
    for state_dir in state_dirs:
        real = os.path.realpath(state_dir)
        if state_dir.name in SANCTIONED or real in seen or (sanctioned is not None and real == str(sanctioned)):
            continue
        seen.add(real)
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
    # Its app's default instance (the Claude app with no `--user-data-dir`).
    default: bool = False
    # AJ-E3210: its App id (its Out mark is its own Account's).
    app_id: str | None = None


def sole_holders(apps: list[KnownApp]) -> dict[str, str]:
    """Each Claude Code folder one desktop App holds → that App's name, as
    the app's `places::sole_holders`: one profile on it, else the one of
    them that's its app's default instance; two others: none."""
    held: dict[str, list[KnownApp]] = {}
    for app in apps:
        if app.kind == "desktop" and app.tool == "claude-code" and app.state_dir:
            on = held.setdefault(str(app.state_dir), [])
            if all(one.name != app.name for one in on):
                on.append(app)
    out = {}
    for folder, on in held.items():
        defaults = [app for app in on if app.default]
        one = on if len(on) == 1 else defaults
        if len(one) == 1:
            out[folder] = one[0].name
    return out


def known_apps(db: sqlite3.Connection, desktop_names: dict[str, str], claude_default: Path | None = None) -> list[KnownApp]:
    """Every App of the database by its rail name (`desktop_names`:
    presence_check's desktop App names by id; `claude_default`: the Claude
    app's default instance folder)."""
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
        default = bool(instance) and claude_default is not None and Path(instance) == claude_default
        out.append(KnownApp(name=name, kind=kind, tool=tool, folder=folder, instance=Path(instance) if instance else None, state_dir=state, default=default, app_id=app_id))
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


Listed = tuple[frozenset[str], frozenset[str]]


def codex_problems(
    rail: list[RailApp],
    apps: list[KnownApp],
    codex: dict[str, Listed],
    own: dict[str, dict[str, str | None]] | None = None,
) -> tuple[list[str], list[str]]:
    """D-102: each Codex home's listed threads (`codex`: folder → (the
    Codex app's ids, the rest)) against the rail: the Codex app's count plus
    Codex CLI's Inactive "N sessions" is the total, Codex CLI's the rest
    (alone when the Codex app's count isn't drawn). 0.4.6: AI Juggler's own
    runs on the folder (`own`: session id → its thread) are Codex CLI's
    rows, each one, and a thread one is on is theirs, not counted again
    (`cli_records`'s rule, `features::relations::build`)."""
    lines: list[str] = []
    problems: list[str] = []
    for folder, (desktop_ids, other_ids) in sorted(codex.items()):
        runs = (own or {}).get(folder, {})
        ran = {thread for thread in runs.values() if thread}
        desktop = len(desktop_ids - ran)
        other = len(runs) + len(other_ids - ran)
        total = desktop + other
        named = {(app.kind, app.name) for app in apps if app.tool == "codex" and app.folder == folder}
        app = next((one for one in rail if ("desktop", one.name) in named), None)
        cli = next((one for one in rail if ("cli", one.name) in named and one.section == "idle"), None)
        app_count = drawn_count(app) if app else None
        cli_count = (cli.sessions or 0) if cli else None
        listed = len(desktop_ids) + len(other_ids)
        lines.append(
            f"  codex store: {len(desktop_ids)} by the Codex app + {len(other_ids)} others = {listed} listed;"
            f" AI Juggler's runs: {len(runs)}, on {len(ran & (desktop_ids | other_ids))} of them;"
            f" so app={desktop} cli={other}; rail: app={app_count} cli={cli_count}"
        )
        if cli_count is not None and cli_count != other:
            problems.append(f"Codex CLI says {cli_count} sessions, its runs and the store's threads not the Codex app's are {other}: {cli.name}")  # type: ignore[union-attr]
        if app_count is not None and cli_count is not None and app_count + cli_count != total:
            problems.append(f"the Codex app's {app_count} and Codex CLI's {cli_count} aren't the {total} of the store's threads and AI Juggler's runs")
    return lines, problems


def check(
    rail: list[RailApp],
    apps: list[KnownApp],
    out: set[str],
    leftovers: int,
    own: dict[str, dict[str, str | None]] | None = None,
    codex: dict[str, Listed] | None = None,
) -> tuple[list[str], list[str]]:
    """The lines to print and the problems: unfold, records, Out marks,
    labs, and (D-102) each Codex home's listed threads (`codex`, from
    [`codex_listed`] by folder)."""
    by_name = {app.name: app for app in apps}
    # A profile's records hold its own folder's copy only: a switch copies
    # a conversation into another folder under the same id. 0.4.6: one AI
    # Juggler's session on that folder is on is that session's.
    held: dict[str, set[str]] = {}
    records: dict[str, Records] = {}
    holders = sole_holders(apps)
    for app in apps:
        if app.kind == "desktop" and app.tool == "claude-code" and app.instance and app.state_dir:
            linked = {conversation for conversation in (own or {}).get(app.folder or "", {}).values() if conversation}
            records[app.name], ids = desktop_records(app.instance, app.state_dir, linked)
            held.setdefault(str(app.state_dir), set()).update(ids)
    for app in apps:
        if app.kind == "cli" and app.tool == "claude-code" and app.state_dir:
            records[app.name] = cli_records(
                app.state_dir,
                held.get(str(app.state_dir), set()),
                (own or {}).get(app.folder or "", {}),
                held_folder=str(app.state_dir) in holders,
            )
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
        # AJ-E3210: by the App's own Account (its id), else its folder's.
        key = (known.app_id if known and known.app_id else None) or (known.folder if known else None)
        if known and key:
            if key in out and not shown.out:
                problems.append(f"its Account is out, no Out mark: {shown.name}")
            if key not in out and shown.out:
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
    more_lines, more = codex_problems(rail, apps, codex or {}, own)
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
    selftest_046()
    selftest_split()
    print("presence_sessions: selftest passed")


def selftest_split() -> None:
    """AJ-E3210 (F-rc3c-3): aij-test's folder holds the Team seat's CLI
    sign-in; its Claude app profile is on the personal plan, which is out.
    The profile is Out, its CLI App isn't, and the default app (Team) isn't;
    a check that read by folder would mark all three, or none."""
    db = sqlite3.connect(":memory:")
    db.executescript(
        "create table accounts (id text, subscription_id text, label text, state_dir text, removed_at integer);"
        "create table apps (id text, kind text, tool text, instance_dir text, folder_id text);"
        "create table app_accounts (app_id text, subscription_id text);"
        "create table quota_samples (account text, window text, ts integer, used_pct real, resets_at integer);"
        "create table quota_sample_sign_ins (account text, window text, ts integer, signed_in_as text);"
        "insert into accounts values ('shared', 'team', 'aij-test', '/s', null), ('default', 'team', 'Default', '/d', null);"
        "insert into apps values ('cli:shared', 'cli', 'claude-code', null, 'shared'), ('app:claude-app:/p/aij-test', 'desktop', 'claude-code', '/p/aij-test', 'shared'),"
        " ('app:claude-app:', 'desktop', 'claude-code', null, 'default');"
        "insert into app_accounts values ('cli:shared', 'team'), ('app:claude-app:/p/aij-test', 'personal'), ('app:claude-app:', 'team');"
        "insert into quota_samples values ('shared', 'weekly', 10, 7, 9000), ('shared', 'weekly', 20, 100, 9000), ('default', 'weekly', 12, 7, 9000);"
        "insert into quota_sample_sign_ins values ('shared', 'weekly', 10, 'team'), ('shared', 'weekly', 20, 'personal');"
    )
    out = out_accounts(db, 5_000)
    assert out == {"app:claude-app:/p/aij-test"}, out
    apps = [
        KnownApp("Claude app · aij-test", "desktop", "claude-code", "shared", None, None, app_id="app:claude-app:/p/aij-test"),
        KnownApp("Claude Code CLI · aij-test", "cli", "claude-code", "shared", None, None, app_id="cli:shared"),
        KnownApp("Claude app", "desktop", "claude-code", "default", None, None, app_id="app:claude-app:"),
    ]

    def rail(profile_out: bool, cli_out: bool) -> list[RailApp]:
        return [
            RailApp(name="Claude app · aij-test", section="idle", foldable=False, folded=False, out=profile_out),
            RailApp(name="Claude Code CLI · aij-test", section="idle", foldable=False, folded=False, out=cli_out),
            RailApp(name="Claude app", section="idle", foldable=False, folded=False, out=False),
        ]

    assert check(rail(True, False), apps, out, 0)[1] == []
    _, problems = check(rail(True, True), apps, out, 0)
    assert problems == ["an Out mark, its Account has room: Claude Code CLI · aij-test"], problems
    _, problems = check(rail(False, False), apps, out, 0)
    assert problems == ["its Account is out, no Out mark: Claude app · aij-test"], problems
    # Before V55 (no tags, no per-App table): by folder, as 0.4.6 read it.
    old = sqlite3.connect(":memory:")
    old.executescript(
        "create table accounts (id text, subscription_id text);"
        "create table apps (id text, kind text, tool text, instance_dir text, folder_id text);"
        "create table quota_samples (account text, window text, ts integer, used_pct real, resets_at integer);"
        "insert into accounts values ('shared', 'team');"
        "insert into apps values ('cli:shared', 'cli', 'claude-code', null, 'shared');"
        "insert into quota_samples values ('shared', 'weekly', 20, 100, 9000);"
    )
    assert out_accounts(old, 5_000) == {"cli:shared"}


def _transcript(folder: Path, sid: str, entry: str, cwd: str = "/w") -> None:
    (folder / "projects/-w").mkdir(parents=True, exist_ok=True)
    (folder / f"projects/-w/{sid}.jsonl").write_text(json.dumps({"type": "user", "sessionId": sid, "entrypoint": entry, "cwd": cwd}, separators=(",", ":")) + "\n")


def selftest_046() -> None:
    """The 0.4.6 RC's three failures, each a case the check counted
    against the app's one order (`places::app_of`,
    `ui/src/lib/fixtures/session-apps.json`): two AI Juggler sessions on
    one terminal transcript are two rows; a desktop-started transcript on
    a folder no profile holds is its CLI App's (`claude --resume` lists
    it); a profile's record that an AI Juggler terminal session resumed is
    that session, in its CLI App, never in both. And the Dev build's Lab
    App folder, exactly, is where our lab runs belong: every other
    folder, another Dev-managed one too, still fails."""
    with tempfile.TemporaryDirectory() as tmp:
        lab, prof, inst = Path(tmp) / "lab", Path(tmp) / "prof", Path(tmp) / "inst"
        for sid, entry in (("t-term", "cli"), ("t-twice", "cli"), ("t-desk", "claude-desktop"), ("t-run", "sdk-cli")):
            _transcript(lab, sid, entry)
        for sid, entry in (("p-rec", "claude-desktop"), ("p-res", "cli"), ("p-loose", "claude-desktop"), ("p-term", "cli")):
            _transcript(prof, sid, entry)
        (inst / "claude-code-sessions/a/o").mkdir(parents=True)
        for record, sid in (("local_r1", "p-rec"), ("local_p-res", "p-res")):
            (inst / f"claude-code-sessions/a/o/{record}.json").write_text(json.dumps({"cliSessionId": sid, "lastActivityAt": 1}))
        apps = [
            KnownApp("Claude Code CLI · Lab", "cli", "claude-code", "f-lab", None, lab),
            KnownApp("Claude app · P", "desktop", "claude-code", "f-prof", inst, prof),
            KnownApp("Claude Code CLI · P", "cli", "claude-code", "f-prof", None, prof),
        ]
        own = {"f-lab": {"s-1": "t-twice", "s-2": "t-twice", "s-3": "gone"}, "f-prof": {"s-9": "p-res"}}

        def idle(lab_n: int, app_n: int, cli_n: int) -> list[RailApp]:
            return parse_rail([
                ("AXButton", "Inactive apps 3", ""),
                ("AXButton", f"Claude Code CLI · Lab, CLI app, 2 d ago · {lab_n} sessions", ""),
                ("AXButton", f"Claude app · P, Desktop app, 3 d ago · {app_n} sessions", ""),
                ("AXButton", f"Claude Code CLI · P, CLI app, 4 d ago · {cli_n} sessions", ""),
            ])

        # The Lab CLI App: t-term, t-twice's two sessions, t-desk, s-3 (no
        # transcript); not t-run. The profile: p-rec; p-loose is in no
        # App (its records leave it out). Its CLI App: s-9 and p-term.
        lines, problems = check(idle(5, 1, 2), apps, set(), 0, own)
        assert problems == [], (problems, lines)
        _, problems = check(idle(3, 2, 2), apps, set(), 0, own)
        assert any("says 3 sessions, its records hold 5" in p for p in problems), problems
        assert any("says 2 sessions, its records hold 1: Claude app · P" in p for p in problems), problems
        # Two profiles on one folder, neither its app's default: neither
        # holds it, so a desktop-started transcript there is its CLI App's;
        # one of them the default instance: that one holds it.
        other = KnownApp("Claude app · Q", "desktop", "claude-code", "f-prof", Path(tmp) / "inst-q", prof)
        assert sole_holders([apps[1], other]) == {}
        assert sole_holders([apps[1], replace(other, default=True)]) == {str(prof): "Claude app · Q"}
        assert sole_holders(apps) == {str(prof): "Claude app · P"}
        assert cli_records(prof, {"p-rec", "p-res"}, {}, held_folder=False).listed == 2, "p-loose and p-term"
        selftest_lab_app(Path(tmp))


def selftest_lab_app(tmp: Path) -> None:
    """FAIL 3 of the 0.4.6 RC: the Dev build's Lab App folder (ADR-0027
    7b, `docs/qa/0.4-walks/7b-live-claude.md`) is found in the Dev build's
    own database by what it is, its path resolved, and only it is ours."""
    home = tmp / "home"
    dev, installed = home / DEV_DATA, home / "Library/Application Support/AI Juggler"
    lab_dir, other_dir = dev / "accounts/claude-code/claude-code-1", dev / "accounts/claude-code/claude-code-2"
    real_dir, installed_dir = home / "Library/Application Support/ClaudeCodeProfiles/b-east", installed / "accounts/claude-code/claude-code-3"
    run = str(HOME / "lab/web")
    for folder in (lab_dir, other_dir, real_dir, installed_dir):
        _transcript(folder, "lab-run", "cli", run)
        _transcript(folder, "own-cwd", "cli", "/elsewhere")

    def database(data: Path, rows: list[tuple[str, str, str, Path, int, int | None]]) -> None:
        data.mkdir(parents=True, exist_ok=True)
        (data / "juggler.sqlite").unlink(missing_ok=True)
        db = sqlite3.connect(data / "juggler.sqlite")
        db.execute("create table accounts (id text, provider_id text, label text, state_dir text, in_place integer, removed_at integer)")
        db.executemany("insert into accounts values (?, ?, ?, ?, ?, ?)", [(i, p, label, str(d), place, gone) for i, p, label, d, place, gone in rows])
        db.commit()
        db.close()

    lab_row = ("claude-code-1", "claude-code", "Lab", lab_dir, 0, None)
    database(installed, [("claude-code-3", "claude-code", "Lab", installed_dir, 0, None)])
    database(dev, [lab_row, ("claude-code-2", "claude-code", "R3", other_dir, 0, None), ("claude-code-9", "claude-code", "Lab", real_dir, 1, None)])
    found = lab_app_folder(home)
    assert found == Path(os.path.realpath(lab_dir)), found
    folders = [lab_dir, other_dir, real_dir, installed_dir]
    assert lab_leftovers(folders) == 4, "nothing exempt unless named"
    assert lab_leftovers(folders, sanctioned=found) == 3, "only the Lab App's folder: another Dev-managed one, a real profile's and the installed app's still fail"
    link = tmp / "lab-link"
    link.symlink_to(lab_dir)
    assert lab_leftovers([link, other_dir, other_dir], sanctioned=found) == 1, "by its resolved path; a folder counted once"
    # Not found (none exempt): two Lab App copies, or none of Dev's own.
    database(dev, [lab_row, ("claude-code-4", "claude-code", "Lab", other_dir, 0, None)])
    assert lab_app_folder(home) is None
    for gone in (("claude-code-1", "claude-code", "Lab", lab_dir, 0, 5), ("claude-code-1", "codex", "Lab", lab_dir, 0, None), ("claude-code-1", "claude-code", "Lab", real_dir, 0, None)):
        database(dev, [gone])
        assert lab_app_folder(home) is None, gone
    assert lab_app_folder(tmp / "nowhere") is None, "no Dev build here"


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
            listed = codex_listed(home)
            assert listed == (frozenset({"t0", "t1"}), frozenset({"t2", "t3"})), (with_column, listed)
        assert codex_listed(Path(tmp)) == (frozenset(), frozenset()), "no store"
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

    store = {"f-codex": (frozenset(f"d{n}" for n in range(15)), frozenset(f"o{n}" for n in range(33)))}
    assert not check(rail(33, False), codex_apps, set(), 0, None, store)[1]
    _, problems = check(rail(92, False), codex_apps, set(), 0, None, store)
    assert len(problems) == 2 and "says 92 sessions" in problems[0] and "aren't the 48 of" in problems[1], problems
    _, problems = check(rail(92, True), codex_apps, set(), 0, None, store)
    assert len(problems) == 1 and "says 92 sessions" in problems[0], "the app's count not drawn: Codex CLI's alone"
    assert not check(rail(33, True), codex_apps, set(), 0, None, store)[1]
    # 0.4.6 (the rc3 presence run): AI Juggler's runs are Codex CLI's rows,
    # each one: two on one thread two, one that made no thread one, one on
    # a thread the store doesn't list one; the thread they're on isn't
    # counted again.
    runs = {"f-codex": {"s-1": "o0", "s-2": "o0", "s-3": None, "s-4": "gone"}}
    assert not check(rail(36, False), codex_apps, set(), 0, runs, store)[1], "4 runs + 32 threads"
    _, problems = check(rail(33, True), codex_apps, set(), 0, runs, store)
    assert len(problems) == 1 and "says 33 sessions" in problems[0] and "are 36" in problems[0], problems
    # A run on a thread the Codex app started is Codex CLI's row: the app
    # lists one fewer.
    runs = {"f-codex": {"s-5": "d0"}}
    assert not check(rail(34, True), codex_apps, set(), 0, runs, store)[1]
    _, problems = check(rail(34, False), codex_apps, set(), 0, runs, store)
    assert len(problems) == 1 and "the Codex app's 15 and Codex CLI's 34 aren't the 48" in problems[0], problems


if __name__ == "__main__":
    selftest()
