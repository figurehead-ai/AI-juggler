#!/usr/bin/env python3
"""AJ-E3013 lab probes C2 and C3, headless, at one shared `codex app-server`.

The question (docs/research/desktop-pool-members.md §4.4, §7.4): when several
threads share one `codex app-server`, can the MCP server it spawns tell which
thread is calling? This answers it at its source, with the `codex` CLI only
(the ChatGPT desktop app is never launched), on a lab home that is never a
real profile. Runbook: docs/qa/evidence/AJ-E3013/c2-runbook.md.

Run it with the standard library only: `python3 -I scripts/qa/c2_probe.py STEP`

    stage      make the lab (0700), pin the probe binary, write config.toml,
               record the names, sizes and dates of ~/.codex and of
               ~/Library/Application Support/Codex as the baseline
    status     what is staged; signed in or not (by file name only)
    features   `codex features list` on the lab: anything on beyond 0.163's
    dry        one app-server: start, initialize, thread/list,
               mcpServerStatus/list, stop; ps and lsof evidence
    preflight  no model, no sign-in: two threads, the probe called directly
               on each (mcpServer/tool/call), --probe-shell through
               thread/shellCommand on each
    login      the maintainer's sign-in: `codex login` on the lab home, its
               page opened on screen, 15 minutes at most
    run        C2 and C3 with the model (needs the sign-in): two threads each
               asked to call the probe, then one asked to run --probe-shell
    reap       stop anything the lab left running
    compare    two report .json files (one per codex version) side by side
    selftest   the checks, the listings, lsof's reading, the guard, --codex,
               the stop-at-once (on a bounded sleep), no codex run

`--codex` picks the binary: /opt/homebrew/bin/codex (default), or the
ChatGPT app's bundled CLI run in place, read-only, through its own wrapper
(Contents/Resources/codex-cli/bin/codex: pinned by sha256, it sets no
environment and execs ../CodexCLI.app/Contents/MacOS/codex; the bundle's
`pathDir` goes first on PATH). Never a copy, and the app is never launched.

Every step that runs codex (stage's --version too) refuses a lab config.toml
that doesn't set every REQUIRED key (the sign-in as a file; plugins, remote
plugins, Apps, dependency installs, tool suggestions, in-app updates, the
daemon, the shell snapshot, the update check and analytics off), naming the
missing key; and it refuses when CODEX_HOME names another folder, when
~/.codex differs from the staged baseline (names and dates only; nothing in
it is ever opened), or when another Codex is running; it runs codex with a
small environment and inside a macOS sandbox that denies every read and
write under ~/.codex and ~/Library/Application Support/Codex and any write
inside the ChatGPT app (`--no-guard` drops only that sandbox), checks the
server's own `codexHome`, samples ps and lsof while it runs (and stops the
whole tree at once on a file open under ~/.codex, the app's Codex data or
~/Library/Keychains, or on an internet socket in a step that expects none),
compares both listings afterwards, and reaps every process it started. It never reads
auth.json, a token or the Keychain; the sign-in is stored in the lab's own
auth.json (`cli_auth_credentials_store = "file"`).
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pwd
import queue
import re
import shutil
import signal
import stat
import subprocess
import sys
import threading
import time
from pathlib import Path

LAB = Path("/tmp/aij-lab-c2")
CODEX_HOME = LAB / "codex-home"
WORK = LAB / "work"
EVIDENCE = LAB / "evidence"
BIN = LAB / "bin"
AGENT = BIN / "juggler-agent"
PROBE_OUT = LAB / "probe.jsonl"
BASELINE = EVIDENCE / "guarded-baseline.json"
OLD_BASELINE = EVIDENCE / "codex-dir-baseline.json"  # the first stage's, ~/.codex only
GROUPS = EVIDENCE / "process-groups.txt"
HOME = Path.home()
REAL_CODEX_DIR = HOME / ".codex"
APP_SUPPORT_CODEX = HOME / "Library/Application Support/Codex"  # the ChatGPT app's Codex data
KEYCHAINS = HOME / "Library/Keychains"
# Listed before and after every codex step (names, sizes, dates; never a
# file's content), and denied to the whole codex tree by the guard.
LISTED = (("~/.codex", REAL_CODEX_DIR), ("~/Library/Application Support/Codex", APP_SUPPORT_CODEX))
# Nothing of the codex tree may have a file open under these (lsof).
GUARDED = LISTED + (("~/Library/Keychains", KEYCHAINS),)
HOMEBREW_CODEX = Path("/opt/homebrew/bin/codex")
APP_BUNDLE = Path("/Applications/ChatGPT.app")
# The app's bundled CLI, run in place through its own wrapper, read-only:
# never copied or changed, and the app itself is never launched. The
# wrapper as read on 2026-11-16 (328 bytes, sha256 below) sets no
# environment: it resolves its own links and runs
# `exec "$bin_dir/../CodexCLI.app/Contents/MacOS/codex" "$@"`. Another
# wrapper is refused until it is read again.
BUNDLED_WRAPPER = APP_BUNDLE / "Contents/Resources/codex-cli/bin/codex"
BUNDLED_TARGET = APP_BUNDLE / "Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
BUNDLED_WRAPPER_SHA256 = "50ab38ba21d0d9f8346f32f41848382f15b556190f3c7a07e885a4fb73e379c8"
CODEX = HOMEBREW_CODEX  # --codex
AGENT_SOURCE = HOME / "Dev/AIJuggler-shared-target/feed/debug/juggler-agent"
# Read for its version only; nothing in the ChatGPT app is ever run.
APP_CODEX_PACKAGE = Path(
    "/Applications/ChatGPT.app/Contents/Resources/codex-cli/codex-package.json"
)
MARKER = b"AIJ_PROBE_V1"
# What a probe build must hold to be pinned: this commit's probe (nested
# _meta, the sandboxed chain), not an older one another worktree built.
PROBE_NEEDS = (MARKER, b"meta_nested", b"(ps can't run here)")
SERVER = "juggler-probe"
CLIENT = {"name": "ai_juggler_lab", "title": "AI Juggler lab probe C2", "version": "1"}
LOGIN_CAP_S = 15 * 60
TURN_WAIT_S = 240
PATH = "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"

PROMPT_TOOL = (
    "Call the `probe` tool of the juggler-probe MCP server exactly once, with "
    "no arguments. Run no shell command. Then reply with the single word: done."
)
PROMPT_SHELL = (
    "Run exactly this shell command once, unchanged, and nothing else: {cmd}\n"
    "Then reply with its output, verbatim."
)


def config_text() -> str:
    """The lab home's config.toml. Every key was checked against codex 0.163.0
    (the binary's own strings and `codex app-server generate-json-schema`),
    and every run starts the server with --strict-config, so a wrong key
    stops it instead of being ignored."""
    return f"""# AJ-E3013 lab probe C2: written by scripts/qa/c2_probe.py stage.
# A lab home, never a real profile. The sign-in is stored in this folder's
# auth.json only, never the Keychain (AuthCredentialsStoreMode: file).
cli_auth_credentials_store = "file"
mcp_oauth_credentials_store = "file"
check_for_update_on_startup = false
approval_policy = "on-request"
sandbox_mode = "read-only"

[analytics]
enabled = false

# No downloads, nothing started on its own: the first dry run (2026-11-16)
# found the app-server cloning github.com/openai/plugins (93 MB) into this
# home at start. These flags (names from `codex features list`) turn off
# that sync, remote plugins and Apps connectors, MCP dependency installs,
# tool suggestions, in-app updates, the shared local app-server daemon and
# the login-shell snapshot each thread started. Every step that runs codex
# refuses a config without them (REQUIRED).
[features]
plugins = false
remote_plugin = false
apps = false
skill_mcp_dependency_install = false
tool_suggest = false
in_app_updates = false
daemon_auto_start = false
shell_snapshot = false

# The probe (debug-only juggler-agent, pinned into the lab by `stage`).
[mcp_servers.{SERVER}]
command = "{AGENT}"
args = ["--mcp", "--probe", "{PROBE_OUT}"]
enabled_tools = ["probe"]
startup_timeout_sec = 20
tool_timeout_sec = 30

# AppToolApproval: auto | prompt | writes | approve. "approve" skips the prompt.
[mcp_servers.{SERVER}.tools.probe]
approval_mode = "approve"
"""


# What every lab CODEX_HOME sets before codex first runs on it: the sign-in
# and MCP tokens as files, never the Keychain, and off everything that can
# download or start things on its own (the lesson of 2026-11-16: a dry
# app-server cloned github.com/openai/plugins, 93 MB, into the lab at
# start). A step that runs codex refuses a config without one, naming it.
REQUIRED: tuple = (
    ("cli_auth_credentials_store", "file"),
    ("mcp_oauth_credentials_store", "file"),
    ("check_for_update_on_startup", False),
    ("analytics.enabled", False),
    ("features.plugins", False),
    ("features.remote_plugin", False),
    ("features.apps", False),
    ("features.skill_mcp_dependency_install", False),
    ("features.tool_suggest", False),
    ("features.in_app_updates", False),
    ("features.daemon_auto_start", False),
    ("features.shell_snapshot", False),
)
TWICE = object()


def strip_comment(line: str) -> str:
    """`line` up to a `#` outside quotes."""
    quote = ""
    for at, char in enumerate(line):
        if quote:
            quote = "" if char == quote else quote
        elif char in "\"'":
            quote = char
        elif char == "#":
            return line[:at]
    return line


def toml_value(text: str):
    """A plain TOML value: true/false, a one-line string, else its text."""
    text = text.strip()
    if text in ("true", "false"):
        return text == "true"
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "\"'":
        return text[1:-1]
    return text


def toml_settings(text: str) -> dict:
    """A config.toml's plain `key = value` lines by dotted name
    (`[features]` then `plugins = false` is `features.plugins`; a dotted
    key counts in its table). Only what REQUIRED needs: anything else (an
    inline table, a multi-line value) is just not found, so it is refused.
    A name set twice is TWICE (TOML forbids it; refused too)."""
    settings: dict = {}
    table = ""
    for raw in text.splitlines():
        line = strip_comment(raw).strip()
        if not line:
            continue
        header = re.fullmatch(r"\[\[?([^\[\]]+)\]\]?", line)
        if header:
            table = ".".join(part.strip().strip("\"'") for part in header.group(1).split("."))
            continue
        pair = re.fullmatch(r"([A-Za-z0-9_\-.\"' ]+?)\s*=\s*(.+)", line)
        if not pair:
            continue
        key = ".".join(part.strip().strip("\"'") for part in pair.group(1).split("."))
        name = f"{table}.{key}" if table else key
        settings[name] = TWICE if name in settings else toml_value(pair.group(2))
    return settings


def missing_settings(text: str) -> list:
    """The REQUIRED settings a config lacks, each named with what it needs."""
    settings = toml_settings(text)
    out = []
    for name, want in REQUIRED:
        shown = f'"{want}"' if isinstance(want, str) else str(want).lower()
        got = settings.get(name)
        if got is TWICE:
            out.append(f"{name} = {shown} (it is set twice)")
        elif name not in settings:
            out.append(f"{name} = {shown} (missing)")
        elif got != want:
            out.append(f"{name} = {shown} (it says {got!r})")
    return out


def check_config(path: Path) -> None:
    """Refuses a lab config.toml without every REQUIRED setting."""
    try:
        text = path.read_text()
    except OSError:
        raise Refused(f"the lab config.toml can't be read ({path}): run `stage`") from None
    missing = missing_settings(text)
    if missing:
        raise Refused(
            "the lab config.toml doesn't turn off everything that can download or start "
            "things on its own (or keep the sign-in in a file), so codex won't run on it:\n  "
            + "\n  ".join(missing)
        )


# ---------------------------------------------------------------- small helpers


def now_ms() -> int:
    return int(time.time() * 1000)


def stamp() -> str:
    return dt.datetime.now().strftime("%Y%m%d-%H%M%S")


def tilde(text: str) -> str:
    """Home shown as ~ in everything written (no real names in outputs)."""
    return text.replace(str(HOME), "~")


def say(text: str) -> None:
    print(tilde(text), flush=True)


class Refused(Exception):
    """Why a step won't run."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def real(path: Path) -> str:
    return os.path.realpath(path)


def private_dir(path: Path) -> None:
    """A folder of ours, 0700, never a link (made when missing)."""
    if not os.path.lexists(path):
        os.mkdir(path, 0o700)
    meta = os.lstat(path)
    if stat.S_ISLNK(meta.st_mode) or not stat.S_ISDIR(meta.st_mode):
        raise Refused(f"{path} is a link or not a folder")
    if meta.st_uid != os.getuid():
        raise Refused(f"{path} isn't this user's")
    if stat.S_IMODE(meta.st_mode) != 0o700:
        os.chmod(path, 0o700)


def write_private(path: Path, text: str) -> None:
    if os.path.islink(path):
        raise Refused(f"{path} is a link")
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as handle:
        handle.write(text)
    os.chmod(path, 0o600)


def has_auth_file() -> bool:
    """Signed in, by the file's name only: it is never opened."""
    try:
        return stat.S_ISREG(os.lstat(CODEX_HOME / "auth.json").st_mode)
    except FileNotFoundError:
        return False


IDENTITY: dict = {}


def codex_identity() -> dict:
    """Which codex this run uses (`--codex`), checked once: the homebrew
    CLI, or the ChatGPT app's bundled CLI through its pinned wrapper. The
    bundle's own `codex-package.json` names a `pathDir` (its `rg`), which
    the app presumably puts on PATH: it is put first on PATH here too."""
    if IDENTITY.get("path") == str(CODEX):
        return IDENTITY
    chosen = Path(os.path.abspath(CODEX))
    if chosen == HOMEBREW_CODEX:
        if not os.access(chosen, os.X_OK):
            raise Refused(f"{chosen} isn't there or can't run")
        found = {"kind": "homebrew", "path": str(chosen), "runs": real(chosen), "path_dir": None}
    elif chosen == BUNDLED_WRAPPER:
        if not BUNDLED_WRAPPER.is_file() or not os.access(BUNDLED_TARGET, os.X_OK):
            raise Refused("the ChatGPT app's bundled codex isn't there")
        digest = sha256(BUNDLED_WRAPPER)
        if digest != BUNDLED_WRAPPER_SHA256:
            raise Refused(
                f"the bundled wrapper changed (sha256 {digest}, read {BUNDLED_WRAPPER_SHA256[:16]}…): "
                "read it again, and what environment it sets, before running it"
            )
        package = json.loads(APP_CODEX_PACKAGE.read_text())
        name = package.get("pathDir")
        folder = APP_CODEX_PACKAGE.parent / name if isinstance(name, str) else None
        plain = isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_-][A-Za-z0-9._-]*", name)
        found = {
            "kind": "bundled",
            "path": str(chosen),
            "runs": str(BUNDLED_TARGET),
            "package_version": package.get("version"),
            "path_dir": str(folder) if plain and folder.is_dir() else None,
        }
    else:
        raise Refused(f"--codex takes {HOMEBREW_CODEX} or {BUNDLED_WRAPPER}, not {chosen}")
    IDENTITY.clear()
    IDENTITY.update(found)
    return IDENTITY


def codex_version() -> str:
    """`codex --version`, in the guard and watched like everything else."""
    _, out, err, watched = run_watched(guarded([str(CODEX), "--version"], True), stop_on_net=True)
    if watched["alarm"]:
        raise Refused("STOPPED AT ONCE, `codex --version` touched outside the lab: " + "; ".join(watched["alarm"]))
    return out.strip() or err.strip()


def app_codex_version() -> str:
    try:
        return json.loads(APP_CODEX_PACKAGE.read_text()).get("version", "?")
    except (OSError, ValueError):
        return "not installed"


# ------------------------------------- ~/.codex and the app's data, by names


def folder_snapshot(folder: Path) -> dict:
    """A folder's names, sizes and modification dates, every entry, links
    not followed (never a file's content)."""
    try:
        entries = sorted(os.scandir(folder), key=lambda e: e.name)
    except FileNotFoundError:
        return {"exists": False, "top": [], "deep": {}, "deep_sha256": "", "entries": 0}
    top = []
    for entry in entries:
        try:
            meta = entry.stat(follow_symlinks=False)
        except OSError:
            continue
        top.append([entry.name, meta.st_mtime_ns, meta.st_size])
    root = os.lstat(folder)
    deep = {".": [root.st_mtime_ns, root.st_size]}
    for parent, dirs, files in os.walk(folder, followlinks=False):
        for name in dirs + files:
            path = os.path.join(parent, name)
            try:
                meta = os.lstat(path)
            except OSError:
                continue
            deep[os.path.relpath(path, folder)] = [meta.st_mtime_ns, meta.st_size]
    blob = json.dumps(sorted(deep.items())).encode()
    return {
        "exists": True,
        "top": top,
        "deep": deep,
        "deep_sha256": hashlib.sha256(blob).hexdigest(),
        "entries": len(deep),
    }


def guarded_snapshot() -> dict:
    """Every LISTED folder's snapshot, by its label."""
    return {label: folder_snapshot(path) for label, path in LISTED}


def top_lines(snapshot: dict) -> list:
    """`ls -la`, names, sizes and dates only."""
    lines = []
    for row in snapshot["top"]:
        name, ns, size = (row + [None])[:3]
        shown = "" if size is None else f"{size:>12}  "
        lines.append(f"{dt.datetime.fromtimestamp(ns / 1e9):%Y-%m-%d %H:%M:%S}  {shown}{name}")
    return lines


def folder_changes(before: dict, after: dict) -> list:
    """Names added, removed, or with another date or size."""
    old, new = before.get("deep", {}), after.get("deep", {})
    changed = [name for name in new if name in old and new[name] != old[name]]
    added = [name for name in new if name not in old]
    removed = [name for name in old if name not in new]
    return [f"changed {n}" for n in sorted(changed)] + [
        f"added {n}" for n in sorted(added)
    ] + [f"removed {n}" for n in sorted(removed)]


def guarded_changes(baseline: dict, now: dict) -> dict:
    """Each LISTED folder's changes since `baseline` (empty when none)."""
    folders = baseline.get("folders", {})
    return {label: folder_changes(folders.get(label, {}), now.get(label, {})) for label, _ in LISTED}


def load_baseline() -> dict:
    if not BASELINE.exists():
        raise Refused("no baseline of ~/.codex and the app's Codex data: run `stage` first")
    baseline = json.loads(BASELINE.read_text())
    if baseline.get("version") != 2:
        raise Refused("the baseline is of an older kind: run `stage --rebaseline`")
    return baseline


def save_baseline() -> dict:
    baseline = {
        "version": 2,
        "taken": dt.datetime.now().isoformat(timespec="seconds"),
        "folders": guarded_snapshot(),
    }
    write_private(BASELINE, json.dumps(baseline))
    return baseline


def say_changes(changes: dict) -> list:
    """One line per LISTED folder: unchanged, or its first changed names."""
    return [
        f"{label}: " + ("UNCHANGED (names, sizes and dates)" if not found else f"CHANGED: {'; '.join(found[:10])}")
        for label, found in changes.items()
    ]


# ----------------------------------------------------------------- processes


def ps_table() -> dict:
    out = subprocess.run(
        ["/bin/ps", "-axww", "-o", "pid=,ppid=,pgid=,command="],
        capture_output=True, text=True, check=False,
    ).stdout
    rows = {}
    for line in out.splitlines():
        parts = line.split(None, 3)
        if len(parts) == 4 and parts[0].isdigit():
            rows[int(parts[0])] = (int(parts[1]), int(parts[2]), parts[3])
    return rows


def descendants(root: int, table: dict) -> list:
    out, todo = [], [root]
    while todo:
        pid = todo.pop()
        if pid in table and pid not in out:
            out.append(pid)
            todo.extend(child for child, row in table.items() if row[0] == pid)
    return sorted(out)


ENV_KEYS = ("CODEX_HOME", "CODEX_SQLITE_HOME", "HOME")


def env_marks(pid: int) -> dict:
    """The values of CODEX_HOME, CODEX_SQLITE_HOME and HOME in a process's
    starting environment (`ps -E`); no other value is kept."""
    out = subprocess.run(
        ["/bin/ps", "-wwE", "-o", "command=", "-p", str(pid)],
        capture_output=True, text=True, check=False,
    ).stdout
    marks = {}
    for key in ENV_KEYS:
        found = re.search(rf"(?:^|\s){key}=(\S+)", out)
        marks[key] = found.group(1) if found else ("(gone)" if not out.strip() else None)
    return marks


def other_codex(own: set) -> list:
    """A Codex this script didn't start (then ~/.codex changes can't be
    told apart): the ChatGPT app's main process or any codex program."""
    found = []
    for pid, (_, _, command) in ps_table().items():
        if pid in own or pid == os.getpid():
            continue
        program = command.split(" ")[0]
        base = os.path.basename(program)
        words = command.split()
        is_codex_js = len(words) > 1 and words[1].endswith("/bin/codex") and base == "node"
        if program.endswith("/Contents/MacOS/ChatGPT") or base == "codex" or is_codex_js:
            if not walled_off(pid):
                found.append(f"{pid} {command[:120]}")
    return found


def walled_off(pid: int) -> bool:
    """A process that can't reach ~/.codex by its own defaults: both its
    CODEX_HOME and its HOME set and away from the real home (another
    lab's replica fixture, for one)."""
    marks = env_marks(pid)
    home, codex_home = marks["HOME"], marks["CODEX_HOME"]
    if not home or not codex_home:
        return False
    inside = lambda path: real(Path(path)) == real(HOME) or real(Path(path)).startswith(real(HOME) + "/")
    return not inside(home) and not inside(codex_home)


def lsof_parse(out: str) -> dict:
    """lsof's `-F ptn` lines: the names open under each GUARDED place (its
    real path or as written), the internet sockets, the count, the lab's."""
    places = [(label, {real(path), str(path)}) for label, path in GUARDED]
    hits: dict = {label: [] for label, _ in GUARDED}
    pid, kind, net, files, lab = 0, "", [], 0, set()
    for line in out.splitlines():
        if line.startswith("p"):
            pid = int(line[1:])
        elif line.startswith("t"):
            kind = line[1:]
        elif line.startswith("n"):
            name = line[1:]
            files += 1
            if kind in ("IPv4", "IPv6"):
                net.append(f"{pid} {name}")
            for label, roots in places:
                if any(name == root or name.startswith(root + "/") for root in roots):
                    hits[label].append(f"{pid} {name}")
            if "aij-lab-c2" in name:
                lab.add(name)
    return {"hits": hits, "net": net, "files": files, "lab": sorted(lab)}


def lsof(pids: list) -> dict:
    """Open files of `pids`, by lsof_parse (names only, never contents)."""
    if not pids:
        return lsof_parse("")
    out = subprocess.run(
        ["/usr/sbin/lsof", "-nP", "-w", "-p", ",".join(map(str, pids)), "-F", "ptn"],
        capture_output=True, text=True, check=False,
    ).stdout
    return lsof_parse(out)


def alarm_reasons(found: dict, stop_on_net: bool) -> list:
    """Why a run must stop at once: a file open under a GUARDED place, or
    an internet socket where none is expected (no sign-in, no model)."""
    reasons = [f"open under {label}: {', '.join(names[:3])}" for label, names in found["hits"].items() if names]
    if stop_on_net and found["net"]:
        reasons.append(f"internet sockets: {', '.join(found['net'][:3])}")
    return reasons


def kill_tree(root: int) -> None:
    """SIGKILL `root` and everything under it, children first (codex starts
    its children in groups of their own, so the root's group isn't enough)."""
    table = ps_table()
    depth = {}
    for pid in descendants(root, table):
        steps, at = 0, pid
        while at != root and at in table and steps < 64:
            at, steps = table[at][0], steps + 1
        depth[pid] = steps
    for pid in sorted(depth, key=lambda pid: -depth[pid]):
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def describe_tree(root: int) -> list:
    """Each process the server started: pid, parent, its program and args,
    and its CODEX_HOME / CODEX_SQLITE_HOME / HOME."""
    table = ps_table()
    rows = []
    for pid in descendants(root, table):
        ppid, pgid, command = table[pid]
        rows.append({"pid": pid, "ppid": ppid, "pgid": pgid, "command": command[:200], "env": env_marks(pid)})
    return rows


def remember_group(pgid: int) -> None:
    with open(GROUPS, "a") as handle:
        handle.write(f"{pgid}\n")


def lab_processes() -> list:
    """Processes that are the lab's: running the pinned probe (its path is
    in its arguments), or in a group this script started AND carrying the
    lab home (a group number reused by another program is never taken)."""
    groups = set()
    if GROUPS.exists():
        groups = {int(x) for x in GROUPS.read_text().split() if x.isdigit()}
    lab_home = real(CODEX_HOME)
    found = []
    for pid, (_, pgid, command) in ps_table().items():
        if pid == os.getpid():
            continue
        if str(AGENT) in command or real(AGENT) in command:
            found.append((pid, pgid, command))
        elif pgid in groups:
            home = env_marks(pid)["CODEX_HOME"]
            if home and real(Path(home)) == lab_home:
                found.append((pid, pgid, command))
    return found


def reap(quiet: bool = False) -> list:
    """Stops the lab's processes (TERM, then KILL); what is left."""
    for signum in (signal.SIGTERM, signal.SIGKILL):
        left = lab_processes()
        if not left:
            break
        for pid, _, _ in left:
            try:
                os.kill(pid, signum)
            except ProcessLookupError:
                pass
        time.sleep(2)
    left = lab_processes()
    if not left and GROUPS.exists():
        write_private(GROUPS, "")
    if not quiet:
        say("reaped: nothing of the lab's is running" if not left else f"STILL RUNNING: {left}")
    return left


# -------------------------------------------------------------- environment


def child_env() -> dict:
    """The small environment every codex process starts with: no keys, no
    tokens, the lab home named twice."""
    user = pwd.getpwuid(os.getuid())
    return {
        "HOME": str(HOME),
        "USER": user.pw_name,
        "LOGNAME": user.pw_name,
        "SHELL": user.pw_shell or "/bin/zsh",
        "TMPDIR": os.environ.get("TMPDIR", "/tmp/"),
        "LANG": "en_US.UTF-8",
        "PATH": f"{codex_identity()['path_dir']}:{PATH}" if codex_identity()["path_dir"] else PATH,
        "CODEX_HOME": str(CODEX_HOME),
        "CODEX_SQLITE_HOME": str(CODEX_HOME),
    }


def guard_profile() -> str:
    """A sandbox for the whole codex tree: everything allowed but any read
    or write under ~/.codex or the app's Codex data (an attempt fails
    instead of happening), and any write inside the ChatGPT app."""
    denied = sorted({spelling for _, path in LISTED for spelling in (real(path), str(path))})
    written = sorted({real(APP_BUNDLE), str(APP_BUNDLE)})
    for path in denied + written:
        if '"' in path or "\\" in path:
            raise Refused(f"{path} can't be quoted for the sandbox")
    subpaths = " ".join(f'(subpath "{path}")' for path in denied)
    bundle = " ".join(f'(subpath "{path}")' for path in written)
    return f"(version 1)(allow default)(deny file-read* file-write* {subpaths})(deny file-write* {bundle})"


def guarded(command: list, guard: bool) -> list:
    return (["/usr/bin/sandbox-exec", "-p", guard_profile()] if guard else []) + command


def preconditions(rebaseline: bool, need_auth: bool = False) -> dict:
    """The checks before any codex runs; the baseline it is held to."""
    named = os.environ.get("CODEX_HOME")
    if named and real(Path(named)) != real(CODEX_HOME):
        raise Refused(f"CODEX_HOME is {named}, not the lab home {CODEX_HOME}")
    for folder in (LAB, CODEX_HOME, WORK, EVIDENCE, BIN):
        if not os.path.isdir(folder) or os.path.islink(folder):
            raise Refused(f"{folder} is missing: run `stage` first")
        meta = os.lstat(folder)
        if meta.st_uid != os.getuid() or stat.S_IMODE(meta.st_mode) != 0o700:
            raise Refused(f"{folder} must be this user's, 0700")
    if real(CODEX_HOME).startswith(real(REAL_CODEX_DIR)):
        raise Refused("the lab home is inside ~/.codex")
    check_config(CODEX_HOME / "config.toml")
    if not os.access(AGENT, os.X_OK) or not all(need in AGENT.read_bytes() for need in PROBE_NEEDS):
        raise Refused(f"{AGENT} is missing or isn't this commit's probe build: run `stage`")
    if (CODEX_HOME / "config.toml").read_text() != config_text():
        raise Refused("the lab config.toml isn't the staged one: run `stage` again")
    others = other_codex(set())
    if others:
        raise Refused(
            "another Codex is running, so a change to ~/.codex couldn't be told apart; "
            "quit it first:\n  " + "\n  ".join(others)
        )
    if lab_processes():
        raise Refused("the lab still has processes running: run `reap`")
    codex_identity()
    baseline = save_baseline() if rebaseline else load_baseline()
    changes = guarded_changes(baseline, guarded_snapshot())
    if any(changes.values()):
        listed = [f"{label}: {name}" for label, found in changes.items() for name in found[:20]]
        raise Refused(
            "~/.codex or the app's Codex data differs from the staged baseline (names, sizes and "
            "dates). If that was the maintainer's own Codex or app, look at the names and run "
            "again with --rebaseline:\n  " + "\n  ".join(listed)
        )
    if need_auth and not has_auth_file():
        raise Refused("the lab home isn't signed in (no auth.json by name): run `login` first")
    return baseline


# ------------------------------------------------------------ the app-server


class RpcError(Exception):
    pass


class Watcher:
    """ps and lsof over a process tree every half second while it lives:
    every process seen once (its program, parent and lab-home variables),
    every name open under a GUARDED place and every internet socket, across
    all samples (short-lived children such as the probe's first start are
    caught too). On the first alarm (alarm_reasons) it kills the whole
    tree at once and keeps the reason."""

    def __init__(self, root: int, stop_on_net: bool = False, every: float = 0.5):
        self.root, self.every, self.stop_on_net = root, every, stop_on_net
        self.seen: dict = {}
        self.hits: dict = {label: set() for label, _ in GUARDED}
        self.net: set = set()
        self.alarm: list = []
        self.samples = 0
        self.halt = threading.Event()
        self.thread = threading.Thread(target=self._loop, daemon=True)
        self.thread.start()

    def _loop(self) -> None:
        while not self.halt.is_set():
            table = ps_table()
            pids = descendants(self.root, table)
            for pid in pids:
                if pid not in self.seen:
                    ppid, pgid, command = table[pid]
                    self.seen[pid] = {"pid": pid, "ppid": ppid, "pgid": pgid, "command": command[:200], "env": env_marks(pid)}
            if pids:
                found = lsof(pids)
                for label, names in found["hits"].items():
                    self.hits[label].update(names)
                self.net.update(found["net"])
                self.samples += 1
                reasons = alarm_reasons(found, self.stop_on_net)
                if reasons and not self.alarm:
                    self.alarm = reasons
                    kill_tree(self.root)
            self.halt.wait(self.every)

    def stop(self) -> dict:
        self.halt.set()
        self.thread.join(timeout=10)
        return {
            "samples": self.samples,
            "processes": sorted(self.seen.values(), key=lambda row: row["pid"]),
            "hits": {label: sorted(names) for label, names in self.hits.items()},
            "net": sorted(self.net),
            "alarm": self.alarm,
        }


class AppServer:
    """One `codex app-server` on the lab home, over stdio (newline JSON-RPC),
    as AI Juggler's adapter drives it (crates/juggler-adapters/src/
    codex_appserver)."""

    def __init__(self, guard: bool, tag: str, stop_on_net: bool = False):
        command = guarded(
            [str(CODEX), "app-server", "--listen", "stdio://", "--strict-config"], guard
        )
        self.stderr_path = EVIDENCE / f"{tag}-app-server.stderr"
        self.stderr = open(self.stderr_path, "wb")
        os.chmod(self.stderr_path, 0o600)
        self.proc = subprocess.Popen(
            command, cwd=WORK, env=child_env(), stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=self.stderr, start_new_session=True,
        )
        remember_group(self.proc.pid)
        self.watch = Watcher(self.proc.pid, stop_on_net)
        self.watched: dict = {}
        self.lines: queue.Queue = queue.Queue()
        self.next_id = 0
        self.responses: dict = {}
        self.events: list = []  # (ms, kind, details)
        self.prompts: list = []  # server requests: approvals, questions
        self.items: list = []  # completed items
        self.turns: dict = {}  # turn id -> (status, error)
        self.methods: dict = {}
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        for raw in self.proc.stdout:
            self.lines.put(raw)
        self.lines.put(None)

    def send(self, message: dict) -> None:
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()

    def pump(self, timeout: float) -> None:
        if self.watch.alarm:
            raise Refused("STOPPED AT ONCE, something touched outside the lab: " + "; ".join(self.watch.alarm))
        try:
            raw = self.lines.get(timeout=timeout)
        except queue.Empty:
            return
        if raw is None:
            raise RpcError("the app-server closed its output (see the .stderr file)")
        try:
            message = json.loads(raw)
        except ValueError:
            return
        method = message.get("method")
        if method is None and "id" in message:
            self.responses[message["id"]] = message
        elif method is not None and "id" in message:
            self._answer(message)
        elif method is not None:
            self._note(method, message.get("params") or {})

    def request(self, method: str, params: dict, timeout: float = 30) -> dict:
        self.next_id += 1
        ident = self.next_id
        self.send({"jsonrpc": "2.0", "id": ident, "method": method, "params": params})
        deadline = time.time() + timeout
        while ident not in self.responses:
            if time.time() > deadline:
                raise RpcError(f"{method}: no answer in {timeout:.0f} s")
            if self.proc.poll() is not None and self.lines.empty():
                raise RpcError(f"{method}: the app-server exited ({self.proc.returncode})")
            self.pump(0.2)
        answer = self.responses.pop(ident)
        if "error" in answer:
            raise RpcError(f"{method}: {answer['error'].get('message', answer['error'])}")
        return answer.get("result") or {}

    def _note(self, method: str, params: dict) -> None:
        self.methods[method] = self.methods.get(method, 0) + 1
        if method == "item/completed":
            item = params.get("item") or {}
            kept = {"type": item.get("type"), "thread": params.get("threadId"), "turn": params.get("turnId"), "ms": now_ms()}
            if item.get("type") == "mcpToolCall":
                kept.update(server=item.get("server"), tool=item.get("tool"), status=item.get("status"), error=item.get("error"))
            elif item.get("type") == "commandExecution":
                kept.update(command=item.get("command"), status=item.get("status"), exit=item.get("exitCode"), source=item.get("source"), output=(item.get("aggregatedOutput") or "")[:4000])
            elif item.get("type") == "agentMessage":
                kept.update(text=(item.get("text") or "")[:300])
            self.items.append(kept)
        elif method == "turn/completed":
            turn = params.get("turn") or {}
            error = (turn.get("error") or {}).get("message") if isinstance(turn.get("error"), dict) else turn.get("error")
            self.turns[turn.get("id")] = (turn.get("status"), error)
        elif method in ("mcpServer/startupStatus/updated", "error", "warning", "configWarning"):
            self.events.append((now_ms(), method, json.dumps(params)[:400]))

    def _answer(self, message: dict) -> None:
        """The driver answers what the person would: yes only for the
        probe's own tool and the exact --probe-shell command, no to all
        else. Every request is recorded: it is C2's "approval prompt"."""
        method, params, ident = message["method"], message.get("params") or {}, message["id"]
        record = {"ms": now_ms(), "method": method, "thread": params.get("threadId"), "turn": params.get("turnId")}
        result, error = None, None
        if method == "mcpServer/elicitation/request":
            mine = params.get("serverName") == SERVER
            record.update(server=params.get("serverName"), message=str(params.get("message", ""))[:200])
            result = {"action": "accept", "content": {}} if mine else {"action": "decline"}
        elif method == "item/commandExecution/requestApproval":
            command = params.get("command") or ""
            ours = is_probe_shell(command)
            record.update(command=command[:300], reason=str(params.get("reason", ""))[:200])
            result = {"decision": "accept" if ours else "decline"}
        elif method == "item/fileChange/requestApproval":
            result = {"decision": "decline"}
        elif method == "item/tool/requestUserInput":
            result = {"answers": {}}
        elif method == "item/tool/call":
            result = {"contentItems": [], "success": False}
        elif method in ("applyPatchApproval", "execCommandApproval"):
            result = {"decision": "denied"}
        else:
            error = {"code": -32601, "message": "the lab driver doesn't answer this"}
        record["answer"] = result if error is None else "refused"
        self.prompts.append(record)
        reply = {"jsonrpc": "2.0", "id": ident}
        reply.update({"result": result} if error is None else {"error": error})
        self.send(reply)

    def initialize(self) -> dict:
        result = self.request(
            "initialize", {"clientInfo": CLIENT, "capabilities": {"experimentalApi": False}}, 20
        )
        served = result.get("codexHome") or ""
        if real(Path(served)) != real(CODEX_HOME):
            self.stop()
            raise Refused(f"the app-server runs on {served}, not the lab home: stopped")
        self.send({"jsonrpc": "2.0", "method": "initialized"})
        return result

    def thread(self) -> str:
        result = self.request(
            "thread/start", {"cwd": str(WORK), "ephemeral": False}, 60
        )
        return (result.get("thread") or {}).get("id") or ""

    def turn(self, thread: str, text: str, timeout: float = TURN_WAIT_S) -> dict:
        start = now_ms()
        result = self.request(
            "turn/start",
            {"threadId": thread, "input": [{"type": "text", "text": text, "text_elements": []}]},
            60,
        )
        turn = (result.get("turn") or {}).get("id")
        deadline = time.time() + timeout
        while turn not in self.turns:
            if time.time() > deadline:
                try:
                    self.request("turn/interrupt", {"threadId": thread, "turnId": turn}, 10)
                except RpcError:
                    pass
                return {"turn": turn, "start": start, "end": now_ms(), "status": "timed out"}
            self.pump(0.5)
        status, error = self.turns[turn]
        return {"turn": turn, "start": start, "end": now_ms(), "status": status, "error": error}

    def shell(self, thread: str, command: str, wait: float = 30) -> dict:
        start = now_ms()
        seen = len(self.items)
        self.request("thread/shellCommand", {"threadId": thread, "command": command, "timeoutMs": 20000}, 30)
        deadline = time.time() + wait
        while time.time() < deadline:
            done = [i for i in self.items[seen:] if i["type"] == "commandExecution" and i.get("thread") == thread]
            if done:
                return {"start": start, "end": now_ms(), **done[-1]}
            self.pump(0.3)
        return {"start": start, "end": now_ms(), "status": "no output item"}

    def drain(self, seconds: float) -> None:
        deadline = time.time() + seconds
        while time.time() < deadline:
            self.pump(0.2)

    def stop(self) -> None:
        try:
            self.proc.stdin.close()
        except OSError:
            pass
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            for signum in (signal.SIGTERM, signal.SIGKILL):
                try:
                    os.killpg(self.proc.pid, signum)
                except ProcessLookupError:
                    break
                try:
                    self.proc.wait(timeout=5)
                    break
                except subprocess.TimeoutExpired:
                    continue
        if not self.watched:
            self.watched = self.watch.stop()
        self.stderr.close()


def probe_shell_command() -> str:
    return f"{AGENT} --probe-shell"


def is_probe_shell(command: str) -> bool:
    """Exactly the pinned probe's --probe-shell, maybe inside one `sh -lc`."""
    wanted = {probe_shell_command(), f"'{AGENT}' --probe-shell", f'"{AGENT}" --probe-shell'}
    text = command.strip()
    wrapped = re.match(r"^(?:/bin/)?(?:zsh|bash|sh) -l?c (['\"])(.*)\1$", text)
    if wrapped:
        text = wrapped.group(2).strip()
    return text in wanted


# -------------------------------------------------------------- the evidence


def sample(server: AppServer, label: str, samples: list) -> None:
    tree = describe_tree(server.proc.pid)
    pids = [row["pid"] for row in tree]
    samples.append({"label": label, "ms": now_ms(), "tree": tree, "lsof": lsof(pids)})


def probe_records(since_ms: int) -> list:
    if not PROBE_OUT.exists():
        return []
    rows = []
    for line in PROBE_OUT.read_text().splitlines():
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if row.get("at", 0) >= since_ms:
            rows.append(row)
    return rows


def parse_probe_shell(output: str) -> dict:
    thread = re.search(r"^CODEX_THREAD_ID=(\S+)$", output, re.M)
    chain = [
        {"pid": int(m.group(1)), "name": m.group(2)}
        for m in re.finditer(r"^(\d+) (.+)$", output, re.M)
    ]
    return {"CODEX_THREAD_ID": thread.group(1) if thread else None, "chain": chain}


def app_server_pid(watched: dict) -> int:
    """The native `codex app-server` process (node's child)."""
    for row in watched.get("processes", []):
        words = row["command"].split()
        if words and os.path.basename(words[0]) == "codex" and "app-server" in words:
            return row["pid"]
    return 0


def ancestry(pid: int, seen: dict, limit: int = 8) -> list:
    """`pid` and its parents as the driver's own ps saw them (outside any
    sandbox), nearest first, up to the server's root."""
    chain = []
    while pid in seen and len(chain) < limit:
        row = seen[pid]
        chain.append({"pid": pid, "name": os.path.basename(row["command"].split(" ")[0]) or row["command"][:40]})
        pid = row["ppid"]
    return chain


def window_of(at: int, windows: list):
    """The step whose time holds `at`: the latest that started by then."""
    started = [w for w in windows if w["start"] <= at <= w["end"] + 2000]
    return max(started, key=lambda w: w["start"]) if started else None


def analyse(records: list, windows: list, watched: dict, shells: list) -> dict:
    """C2's five answers and C3's, from the probe's own records and the
    driver's own ps (the probe can't run the setuid /bin/ps when sandboxed)."""
    server = app_server_pid(watched)
    seen = {row["pid"]: row for row in watched.get("processes", [])}
    calls = [r for r in records if "env_names" in r]
    starts = [r for r in records if r.get("request") == "initialize"]
    per_call = []
    for record in calls:
        window = window_of(record["at"], windows)
        own = record.get("chain") or []
        parent_pid = own[1]["pid"] if len(own) > 1 else seen.get(record["pid"], {}).get("ppid", 0)
        chain = [{"pid": record["pid"], "name": "juggler-agent"}] + ancestry(parent_pid, seen)
        ids = record.get("meta_ids") or {}
        nested = record.get("meta_nested") or {}
        keys = list(record.get("meta_keys") or []) + [k for v in nested.values() for k in v.get("keys", [])]
        per_call.append({
            "thread": window["thread"] if window else None,
            "how": window["how"] if window else None,
            "pid": record["pid"],
            "chain": chain,
            "parent": f"{chain[1]['pid']} {chain[1]['name']}" if len(chain) > 1 else f"{parent_pid} (not seen)",
            "parent_is_app_server": bool(server) and parent_pid == server,
            "env_thread_id": (record.get("ids") or {}).get("CODEX_THREAD_ID"),
            "env_has_codex_home": "CODEX_HOME" in record.get("env_names", []),
            "env_names": record.get("env_names", []),
            "meta_keys": record.get("meta_keys", []),
            "meta_nested": nested,
            "meta_ids": ids,
            "meta_names_thread": bool(window) and window["thread"] in ids.values(),
            "lease": any("connectionlease" in k.lower() for k in keys),
            "client_info": record.get("client_info"),
        })
    threads = sorted({w["thread"] for w in windows if w.get("thread")})
    named = {c["thread"] for c in per_call if c["meta_names_thread"]}
    by_thread = {}
    for call in per_call:
        by_thread.setdefault(call["thread"], set()).add(call["pid"])
    c3 = []
    for shell in shells:
        parsed = parse_probe_shell(shell.get("output") or "")
        own = parsed["chain"]
        above = ancestry(own[1]["pid"], seen) if len(own) > 1 else []
        pids = [step["pid"] for step in own] + [step["pid"] for step in above]
        c3.append({
            "thread": shell.get("thread"),
            "how": shell.get("how"),
            "status": shell.get("status"),
            "CODEX_THREAD_ID": parsed["CODEX_THREAD_ID"],
            "matches_thread": parsed["CODEX_THREAD_ID"] == shell.get("thread"),
            "reaches_app_server": bool(server) and server in pids,
            "chain": own[:1] + (above or own[1:]),
        })
    return {
        "app_server_pid": server,
        "threads": threads,
        "probe_processes": sorted({r["pid"] for r in records}),
        "probe_pids_by_thread": {k: sorted(v) for k, v in by_thread.items() if k},
        "probe_starts": len(starts),
        "calls": per_call,
        "every_thread_named_in_meta": bool(threads) and set(threads) <= named,
        "env_thread_id_seen": any(c["env_thread_id"] for c in per_call),
        "lease_seen": any(c["lease"] for c in per_call),
        "c3": c3,
    }


def verdict(found: dict, prompts: list, final: bool = True) -> list:
    """The report's answers. `final` only for the model's own calls (`run`);
    the direct calls of `preflight` are a preview."""
    result = "C2 RESULT" if final else "PREVIEW (direct calls, no model; `run` decides)"
    lines = []
    calls = found["calls"]
    if not calls:
        return ["NO PROBE CALL was recorded: C2 has no answer from this run (see the turns and the .stderr file)."]
    pids = found["probe_processes"]
    lines.append(f"Probe processes: {len(pids)} ({', '.join(map(str, pids))}) for {len(found['threads'])} threads.")
    lines.append(
        "Parent: " + ("every call's parent is the app-server" if all(c["parent_is_app_server"] for c in calls)
                      else "NOT always the app-server: " + "; ".join(c["parent"] for c in calls))
    )
    lines.append("CODEX_THREAD_ID in the MCP server's environment: " + ("yes" if found["env_thread_id_seen"] else "no"))
    lines.append("A connection lease key in _meta: " + ("yes" if found["lease_seen"] else "no"))
    lines.append(
        "Approval prompts during the calls: "
        + (", ".join(f"{p['method']} ({p.get('server') or p.get('command', '')[:60]})" for p in prompts) or "none")
    )
    if found["every_thread_named_in_meta"]:
        lines.append(
            f"{result}: each call's _meta names its own thread. Codex app threads CAN be members "
            "(an attestation design over the app-server's _meta is the next step, Opus)."
        )
    else:
        lines.append(
            f"{result}: _meta doesn't name the calling thread for every thread. Codex app threads stay "
            "Watched (D-119, D-122)."
        )
    for shell in found["c3"]:
        lines.append(
            f"C3 ({shell['how']}, thread {shell['thread']}): CODEX_THREAD_ID="
            f"{shell['CODEX_THREAD_ID']} -> {'matches the thread' if shell['matches_thread'] else 'NO MATCH'}; "
            f"ancestry {'reaches' if shell['reaches_app_server'] else 'does NOT reach'} the app-server."
        )
    return lines


def hits_line(hits: dict, net: list) -> str:
    """lsof's counts under each GUARDED place, and the internet sockets."""
    parts = [f"under {label} {len(names)}{' ' + str(names[:3]) if names else ''}" for label, names in hits.items()]
    return "; ".join(parts) + f"; internet sockets {len(net)}{' ' + str(net[:3]) if net else ''}"


def write_report(tag: str, body: dict, summary: list) -> Path:
    path = EVIDENCE / f"{tag}.md"
    json_path = EVIDENCE / f"{tag}.json"
    write_private(json_path, tilde(json.dumps(body, indent=1, default=str)))
    out = [f"# AJ-E3013 lab probe: {tag}", ""]
    out += [f"- {line}" for line in summary]
    out += ["", f"Full record: `{tilde(str(json_path))}`", ""]
    watched = body.get("watch") or {}
    if watched:
        out.append(f"## Every process the server started ({watched['samples']} ps and lsof samples, every 0.5 s)")
        out.append("")
        out.append("| pid | ppid | CODEX_HOME | CODEX_SQLITE_HOME | HOME | command |")
        out.append("|---|---|---|---|---|---|")
        for row in watched["processes"]:
            env = row["env"]
            out.append(
                f"| {row['pid']} | {row['ppid']} | {env['CODEX_HOME'] or '(none)'} | "
                f"{env['CODEX_SQLITE_HOME'] or '(none)'} | {env['HOME'] or '(none)'} | `{row['command'][:110]}` |"
            )
        out.append("")
        out.append(f"lsof across all samples: {hits_line(watched['hits'], watched['net'])}")
        out.append("")
    for label, lines in (body.get("listed_after") or {}).items():
        out.append(f"## {label} after (names, sizes, dates)")
        out.append("")
        out += [f"    {line}" for line in lines["top"]] or ["    (not there)"]
        out.append("")
    for entry in body.get("samples", []):
        out.append(f"## ps and lsof: {entry['label']}")
        out.append("")
        out.append("| pid | ppid | pgid | CODEX_HOME | CODEX_SQLITE_HOME | command |")
        out.append("|---|---|---|---|---|---|")
        for row in entry["tree"]:
            env = row["env"]
            out.append(
                f"| {row['pid']} | {row['ppid']} | {row['pgid']} | {env['CODEX_HOME'] or '(none)'} | "
                f"{env['CODEX_SQLITE_HOME'] or '(none)'} | `{row['command'][:110]}` |"
            )
        found = entry["lsof"]
        out.append("")
        out.append(
            f"lsof: {found['files']} open names; {hits_line(found['hits'], found['net'])};"
            f" lab paths: {', '.join(found.get('lab', [])) or 'none'}"
        )
        out.append("")
    write_private(path, tilde("\n".join(out) + "\n"))
    return path


def finish(server: AppServer, baseline: dict, body: dict, summary: list, tag: str) -> int:
    server.stop()
    left = reap(quiet=True)
    after = guarded_snapshot()
    changes = guarded_changes(baseline, after)
    body["listed_after"] = {
        label: {"top": top_lines(snap), "deep_sha256": snap["deep_sha256"], "entries": snap["entries"], "changes": changes[label]}
        for label, snap in after.items()
    }
    body["left_running"] = [f"{p} {c[:100]}" for p, _, c in left]
    watched = server.watched
    body["watch"] = watched
    hits = {label: list(names) for label, names in watched.get("hits", {}).items()}
    for entry in body.get("samples", []):
        for label, names in entry["lsof"]["hits"].items():
            hits.setdefault(label, []).extend(n for n in names if n not in hits[label])
    net = list(watched.get("net", []))
    for entry in body.get("samples", []):
        net += [n for n in entry["lsof"]["net"] if n not in net]
    homes = {row["env"]["CODEX_HOME"] for row in watched.get("processes", [])}
    samples = len(body.get("samples", [])) + watched.get("samples", 0)
    alarm = watched.get("alarm") or []
    head = [f"ALARM, stopped at once: {'; '.join(alarm)}"] if alarm else []
    head += [f"codex: {body.get('codex')} ({body.get('codex_identity', {}).get('kind')}, {tilde(body.get('codex_identity', {}).get('runs', ''))})"]
    head += say_changes(changes)
    head.append(f"lsof across {samples} samples: {hits_line(hits, net)}")
    head.append(f"processes started: {len(watched.get('processes', []))}; their CODEX_HOME values: {sorted(h or '(none)' for h in homes)}")
    head.append("every process reaped" if not left else f"LEFT RUNNING: {body['left_running']}")
    summary[:0] = head
    path = write_report(tag, body, summary)
    for line in summary:
        say(line)
    say(f"report: {path}")
    if alarm:
        return 5
    touched = any(hits.values()) or any(changes.values()) or left
    return 0 if not touched else 4


def begin(args, tag: str, need_auth: bool = False):
    baseline = preconditions(args.rebaseline, need_auth)
    identity = dict(codex_identity())
    identity["runs_sha256"] = sha256(Path(identity["runs"]))
    body = {
        "tag": tag,
        "codex": codex_version(),
        "codex_identity": identity,
        "app_bundled_codex": app_codex_version(),
        "guard": not args.no_guard,
        "agent_sha256": sha256(AGENT),
        "config_sha256": sha256(CODEX_HOME / "config.toml"),
        "listed_before": {
            label: {"top": top_lines(snap), "deep_sha256": snap["deep_sha256"], "entries": snap["entries"]}
            for label, snap in baseline["folders"].items()
        },
        "baseline_taken": baseline.get("taken"),
        "lab_home_names_before": sorted(os.listdir(CODEX_HOME)),
        "samples": [],
    }
    return baseline, body


# ------------------------------------------------------------------- the steps


def cmd_stage(args) -> int:
    named = os.environ.get("CODEX_HOME")
    if named and real(Path(named)) != real(CODEX_HOME):
        raise Refused(f"CODEX_HOME is {named}, not the lab home")
    for folder in (LAB, CODEX_HOME, WORK, EVIDENCE, BIN):
        private_dir(folder)
    source = Path(args.agent)
    fresh = source.is_file() and all(need in source.read_bytes() for need in PROBE_NEEDS)
    pinned = AGENT.is_file() and all(need in AGENT.read_bytes() for need in PROBE_NEEDS)
    if fresh:
        if os.path.lexists(AGENT):
            os.unlink(AGENT)
        shutil.copy2(source, AGENT)
        os.chmod(AGENT, 0o700)
        say(f"probe: {AGENT} sha256 {sha256(AGENT)} (from {source})")
    elif pinned:
        say(f"probe: kept the pinned {AGENT} (sha256 {sha256(AGENT)}); {source} isn't this commit's probe build")
    else:
        raise Refused(f"{source} isn't this commit's probe build (debug juggler-agent): build it from this worktree")
    write_private(CODEX_HOME / "config.toml", config_text())
    check_config(CODEX_HOME / "config.toml")  # before codex first runs on it
    identity = codex_identity()
    if OLD_BASELINE.exists():
        first = json.loads(OLD_BASELINE.read_text())
        old = {name: ns for name, ns in first.get("deep", {}).items()}
        now = {name: row[0] for name, row in folder_snapshot(REAL_CODEX_DIR)["deep"].items()}
        since = folder_changes({"deep": old}, {"deep": now})
        say(f"~/.codex since the first baseline ({first.get('taken')}), names and dates: "
            + ("unchanged" if not since else f"{len(since)} changes: {since[:10]}"))
    if not BASELINE.exists() or args.rebaseline:
        baseline = save_baseline()
    else:
        baseline = load_baseline()
    say(f"lab: {LAB} (0700); home {CODEX_HOME}; names: {sorted(os.listdir(CODEX_HOME))}")
    say(f"config.toml sha256 {sha256(CODEX_HOME / 'config.toml')}: every required setting is there")
    for label, snap in baseline["folders"].items():
        say(f"{label} baseline taken {baseline.get('taken')}: {snap['entries']} names, digest {snap['deep_sha256'][:16]}")
        for line in top_lines(snap):
            say(f"    {line}")
    say(f"codex ({identity['kind']}, runs {identity['runs']}): {codex_version()}"
        f" (the ChatGPT app bundles {app_codex_version()})")
    say("signed in: " + ("yes (auth.json present, by name)" if has_auth_file() else "no"))
    return 0


def cmd_status(args) -> int:
    say(f"lab home: {CODEX_HOME} names {sorted(os.listdir(CODEX_HOME)) if CODEX_HOME.exists() else 'missing'}")
    say("signed in: " + ("yes (auth.json present, by name)" if has_auth_file() else "no"))
    try:
        identity = codex_identity()
        say(f"codex ({identity['kind']}): runs {identity['runs']}"
            + (f", package {identity['package_version']}, PATH first {identity['path_dir']}" if identity["kind"] == "bundled" else ""))
    except Refused as why:
        say(f"codex: {why}")
    if AGENT.exists():
        ok = all(need in AGENT.read_bytes() for need in PROBE_NEEDS)
        say(f"probe: sha256 {sha256(AGENT)}, {'this commit' if ok else 'NOT this commit'}'s probe build")
    try:
        check_config(CODEX_HOME / "config.toml")
        say("config.toml: every required setting is there")
    except Refused as why:
        say(f"config.toml: {why}")
    try:
        for line in say_changes(guarded_changes(load_baseline(), guarded_snapshot())):
            say(f"vs baseline: {line}")
    except Refused as why:
        say(str(why))
    say(f"lab processes: {lab_processes() or 'none'}; other Codex: {other_codex(set()) or 'none'}")
    return 0


def cmd_dry(args) -> int:
    tag = f"dry-{stamp()}"
    baseline, body = begin(args, tag)
    rotate_probe_file(tag)
    since = now_ms()
    server = AppServer(not args.no_guard, tag, stop_on_net=True)
    summary = []
    try:
        init = server.initialize()
        body["initialize"] = {"codexHome": init.get("codexHome"), "userAgent": init.get("userAgent")}
        sample(server, "after initialize", body["samples"])
        threads = server.request("thread/list", {"limit": 5}, 30)
        body["thread_list"] = len(threads.get("data") or [])
        status = server.request("mcpServerStatus/list", {"detail": "toolsAndAuthOnly"}, 60)
        body["mcp_status"] = [
            {"name": s.get("name"), "runtime": s.get("runtimeStatus"), "tools": sorted((s.get("tools") or {}).keys()) if isinstance(s.get("tools"), dict) else s.get("tools")}
            for s in status.get("data") or []
        ]
        server.drain(2)
        sample(server, "after thread/list and mcpServerStatus/list", body["samples"])
        body["events"] = server.events
        summary.append(f"initialize: codexHome {init.get('codexHome')} (the lab home)")
        summary.append(f"thread/list: {body['thread_list']} threads; mcpServerStatus/list: {json.dumps(body['mcp_status'])[:300]}")
        records = probe_records(since)
        body["probe_records"] = records
        starts = sorted({r["pid"] for r in records})
        summary.append(
            f"the probe (MCP server) started {len(starts)} time(s), pids {starts}: "
            f"{[r['request'] for r in records]} (it lives only while mcpServerStatus/list asks)"
        )
        summary.append("no sign-in, no turn, no model call")
    except (RpcError, Refused) as why:
        summary.append(f"STOPPED: {why}")
    return finish(server, baseline, body, summary, tag)


def cmd_preflight(args) -> int:
    tag = f"preflight-{stamp()}"
    baseline, body = begin(args, tag)
    rotate_probe_file(tag)
    since = now_ms()
    server = AppServer(not args.no_guard, tag, stop_on_net=True)
    summary, windows, shells = [], [], []
    try:
        server.initialize()
        threads = [server.thread(), server.thread()]
        body["threads"] = threads
        server.drain(3)
        sample(server, "after two thread/start", body["samples"])
        for thread in threads:
            start = now_ms()
            result = server.request(
                "mcpServer/tool/call", {"threadId": thread, "server": SERVER, "tool": "probe", "arguments": {}}, 60
            )
            windows.append({"thread": thread, "how": "mcpServer/tool/call (no model)", "start": start, "end": now_ms()})
            body.setdefault("direct_results", []).append(json.dumps(result)[:200])
        sample(server, "after the direct probe calls", body["samples"])
        for thread in threads:
            shell = server.shell(thread, probe_shell_command())
            shells.append({**shell, "thread": thread, "how": "thread/shellCommand (no model)"})
        server.drain(1)
        sample(server, "after thread/shellCommand", body["samples"])
    except (RpcError, Refused) as why:
        summary.append(f"STOPPED: {why}")
    body["prompts"], body["events"], body["items"] = server.prompts, server.events, server.items
    server.stop()
    found = analyse(probe_records(since), windows, server.watched, shells)
    body["analysis"] = found
    summary += verdict(found, server.prompts, final=False)
    summary.append("Direct calls (mcpServer/tool/call) are the client's path, not the model's: `run` answers C2 itself.")
    return finish(server, baseline, body, summary, tag)


def cmd_run(args) -> int:
    tag = f"run-{stamp()}"
    baseline, body = begin(args, tag, need_auth=True)
    rotate_probe_file(tag)
    since = now_ms()
    server = AppServer(not args.no_guard, tag)
    summary, windows, shells = [], [], []
    try:
        server.initialize()
        account = server.request("account/read", {}, 30)
        kind = (account.get("account") or {}).get("type")
        body["account_type"] = kind  # never the email
        if not kind:
            raise Refused("the app-server sees no sign-in on the lab home")
        threads = [server.thread(), server.thread()]
        body["threads"] = threads
        server.drain(3)
        sample(server, "after two thread/start", body["samples"])
        for thread in threads:
            asked = len(server.prompts)
            window = server.turn(thread, PROMPT_TOOL)
            windows.append({**window, "thread": thread, "how": "the model's tool call (C2)"})
            body.setdefault("c2_turns", []).append({**window, "prompts": server.prompts[asked:]})
        sample(server, "after the C2 turns (probe processes still up)", body["samples"])
        window = server.turn(threads[0], PROMPT_SHELL.format(cmd=probe_shell_command()))
        body["c3_turn"] = window
        ran = [i for i in server.items if i["type"] == "commandExecution" and i.get("turn") == window["turn"]]
        for item in ran or [{"status": "the model ran no command", "output": ""}]:
            shells.append({**item, "thread": threads[0], "how": "the model's shell command (C3)"})
        shell = server.shell(threads[1], probe_shell_command())
        shells.append({**shell, "thread": threads[1], "how": "thread/shellCommand (no model)"})
        server.drain(1)
        sample(server, "after C3", body["samples"])
    except (RpcError, Refused) as why:
        summary.append(f"STOPPED: {why}")
    body["prompts"], body["events"], body["items"] = server.prompts, server.events, server.items
    server.stop()
    found = analyse(probe_records(since), windows, server.watched, shells)
    body["analysis"] = found
    c2_prompts = [p for t in body.get("c2_turns", []) for p in t["prompts"]]
    summary += verdict(found, c2_prompts)
    return finish(server, baseline, body, summary, tag)


# Features on by default in codex 0.163.0 under this lab's config (`codex
# features list`, 2026-11-16). A version that turns on anything else is
# looked at (`features`) before its first app-server run: a new feature can
# download or start things on its own, as the plugin sync did.
KNOWN_ON = frozenset(
    """api_key_cyber_access_programs api_key_model_discovery auth_elicitation
    browser_annotation_api browser_use browser_use_external browser_use_full_cdp_access
    cli_daybreak code_mode_host code_mode_interrupt collaboration_modes compaction_image_budget
    computer_use content_item_kinds enable_request_compression fast_mode goals guardian_approval
    guardian_reuse_parent_compaction hooks image_generation in_app_browser in_app_chat
    in_app_dictation in_app_local_automation in_app_voice instant_interrupt item_ids mentions_v2
    multi_agent plugin_sharing realtime_conversation resize_all_images shell_tool skill_search
    sleep_tool sqlite steer system_proxy_fallback terminal_resize_reflow tool_call_mcp_elicitation
    tool_search_always_defer_mcp_tools tui_app_server ultrafast_mode unbounded_connection_retries
    unified_exec unified_exec_tty view_image workspace_dependencies worktrees
    write_stdin_approval""".split()
)


def features_on(listing: str) -> list:
    """The features `codex features list` shows as on (`name stage true`)."""
    return sorted(
        words[0] for words in (line.split() for line in listing.splitlines()) if len(words) >= 2 and words[-1] == "true"
    )


def run_watched(command: list, stop_on_net: bool) -> tuple:
    """A short codex command in the lab, watched like the app-server (its
    exit, output, error text and what the watcher saw)."""
    proc = subprocess.Popen(
        command, cwd=WORK, env=child_env(), stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
    )
    remember_group(proc.pid)
    watch = Watcher(proc.pid, stop_on_net)
    try:
        out, err = proc.communicate(timeout=60)
    except subprocess.TimeoutExpired:
        kill_tree(proc.pid)
        out, err = proc.communicate()
    return proc.returncode, out.decode(errors="replace"), err.decode(errors="replace"), watch.stop()


def cmd_features(args) -> int:
    """`codex features list` on the lab: what is on beyond KNOWN_ON, and
    any REQUIRED feature still on (the config not taken)."""
    tag = f"features-{stamp()}"
    baseline = preconditions(args.rebaseline)
    command = guarded([str(CODEX), "features", "list"], not args.no_guard)
    code, out, err, watched = run_watched(command, stop_on_net=True)
    left = reap(quiet=True)
    changes = guarded_changes(baseline, guarded_snapshot())
    on = features_on(out)
    new = sorted(set(on) - KNOWN_ON)
    still = [name for name, _ in REQUIRED if name.startswith("features.") and name.split(".", 1)[1] in on]
    summary = ([f"ALARM, stopped at once: {'; '.join(watched['alarm'])}"] if watched["alarm"] else []) + [
        f"codex ({codex_identity()['kind']}): exit {code}, {len(on)} features on",
        "on beyond 0.163.0's: " + (", ".join(new) if new else "none") + (" (look at each before the dry run)" if new else ""),
        "REQUIRED features still on: " + (", ".join(still) if still else "none"),
        *(["its errors: " + err.strip().replace("\n", " | ")[:400]] if err.strip() else []),
        *say_changes(changes),
        f"lsof: {hits_line(watched['hits'], watched['net'])}",
        "every process reaped" if not left else f"LEFT RUNNING: {left}",
    ]
    write_private(EVIDENCE / f"{tag}.md", tilde("\n".join(f"- {line}" for line in summary) + "\n\n" + out))
    for line in summary:
        say(line)
    if watched["alarm"]:
        return 5
    bad = code != 0 or new or still or any(changes.values()) or any(watched["hits"].values()) or left
    return 4 if bad else 0


def cmd_compare(args) -> int:
    """Two reports (one per codex version) side by side; the ship claim
    covers only a version a `run` report proves."""
    if len(args.paths) != 2:
        raise Refused("compare takes two report .json files from the lab's evidence folder, one per codex version")
    reports = [json.loads(Path(path).read_text()) for path in args.paths]

    def answer(report: dict, key: str):
        found = report.get("analysis") or {}
        calls = found.get("calls") or []
        listed = report.get("listed_after") or {}
        watched = report.get("watch") or {}
        if key == "codex":
            return f"{report.get('codex')} ({(report.get('codex_identity') or {}).get('kind', '?')})"
        if key == "step":
            return f"{report.get('tag', '?').split('-')[0]} (guard {'on' if report.get('guard') else 'off'})"
        if key in dict(LISTED):
            changes = (listed.get(key) or {}).get("changes")
            return "n/a" if changes is None else ("unchanged" if not changes else f"CHANGED ({len(changes)})")
        if key == "lsof":
            return hits_line(watched.get("hits", {}), watched.get("net", []))
        if key == "alarm":
            return "; ".join(watched.get("alarm") or []) or "none"
        if not found:
            return "n/a"
        if key == "processes":
            return ", ".join(f"{len(pids)} for {thread[:8]}" for thread, pids in found["probe_pids_by_thread"].items()) or "none"
        if key == "parent":
            return "app-server" if calls and all(c["parent_is_app_server"] for c in calls) else f"{[c['parent'] for c in calls]}"
        if key == "env":
            return "yes" if found.get("env_thread_id_seen") else "no"
        if key == "meta":
            return ", ".join(sorted({k for c in calls for k in c["meta_keys"]})) or "none"
        if key == "named":
            return "yes, each its own" if found.get("every_thread_named_in_meta") else "no"
        if key == "lease":
            return "yes" if found.get("lease_seen") else "no"
        if key == "prompts":
            return str(len(report.get("prompts") or []))
        if key == "c3":
            return "; ".join(f"{s['how'].split(' ')[0]} {'matches' if s['matches_thread'] else 'NO'}" for s in found.get("c3", [])) or "none"
        return "?"

    rows = [
        ("codex", "codex"), ("step", "step"),
        *[(label, label) for label, _ in LISTED],
        ("lsof (guarded places, sockets)", "lsof"), ("alarm", "alarm"),
        ("probe processes per thread", "processes"), ("their parent", "parent"),
        ("CODEX_THREAD_ID in the MCP server's environment", "env"), ("_meta keys", "meta"),
        ("_meta names the calling thread", "named"), ("connection lease key", "lease"),
        ("approval prompts", "prompts"), ("C3: CODEX_THREAD_ID in the thread's shell", "c3"),
    ]
    out = ["| | " + " | ".join(answer(r, "codex") for r in reports) + " |", "|---|---|---|"]
    out += [f"| {title} | " + " | ".join(str(answer(r, key)) for r in reports) + " |" for title, key in rows[1:]]
    proven = [answer(r, "codex") for r in reports if r.get("tag", "").startswith("run-") and (r.get("analysis") or {}).get("calls")]
    out += ["", "The ship claim covers only what a `run` (the model's own path) proved: "
            + (", ".join(proven) if proven else "none of these (no `run` report)")]
    path = EVIDENCE / f"compare-{stamp()}.md"
    write_private(path, tilde("\n".join(out) + "\n"))
    for line in out:
        say(line)
    say(f"written: {path}")
    return 0


def rotate_probe_file(tag: str) -> None:
    if PROBE_OUT.exists() and not PROBE_OUT.is_symlink():
        os.rename(PROBE_OUT, EVIDENCE / f"probe-before-{tag}.jsonl")


def cmd_login(args) -> int:
    tag = f"login-{stamp()}"
    baseline = preconditions(args.rebaseline)
    if has_auth_file():
        say("the lab home is already signed in (auth.json present, by name); nothing to do")
        return 0
    names_before = sorted(os.listdir(CODEX_HOME))
    command = guarded([str(CODEX), "login", "-c", 'cli_auth_credentials_store="file"'], not args.no_guard)
    say(f"starting: CODEX_HOME={CODEX_HOME} codex login   (cap {LOGIN_CAP_S // 60} minutes)")
    proc = subprocess.Popen(
        command, cwd=WORK, env=child_env(), stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True,
    )
    remember_group(proc.pid)
    watch = Watcher(proc.pid)  # a file under a GUARDED place stops it at once
    opened = threading.Event()

    def relay() -> None:
        for raw in proc.stdout:
            line = raw.decode(errors="replace").rstrip()
            print(line, flush=True)
            found = re.search(r"https://\S+", line)
            if found and not opened.is_set():
                opened.set()
                if args.no_open:
                    say("--no-open: open the page above yourself")
                else:
                    # Always opened here too: codex's own attempt may not
                    # reach the screen, and the maintainer can't see this
                    # shell. Two tabs of the same page are harmless.
                    subprocess.run(["/usr/bin/open", found.group(0)], check=False)
                    say(">> the sign-in page was opened in the default browser on this Mac")

    reader = threading.Thread(target=relay, daemon=True)
    reader.start()
    try:
        proc.wait(timeout=LOGIN_CAP_S)
        outcome = f"codex login exited {proc.returncode}"
    except subprocess.TimeoutExpired:
        outcome = "TIMED OUT after 15 minutes: stopped"
        for signum in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.killpg(proc.pid, signum)
                proc.wait(timeout=5)
                break
            except (ProcessLookupError, subprocess.TimeoutExpired):
                continue
    reader.join(timeout=5)
    watched = watch.stop()
    left = reap(quiet=True)
    changes = guarded_changes(baseline, guarded_snapshot())
    names_after = sorted(os.listdir(CODEX_HOME))
    codex_changes = changes.get("~/.codex", [])
    summary = ([f"ALARM, stopped at once: {'; '.join(watched['alarm'])}"] if watched["alarm"] else []) + [
        outcome,
        f"codex ({codex_identity()['kind']}): {tilde(codex_identity()['runs'])}",
        "lab home signed in: " + ("yes, auth.json present (by name, never opened)" if has_auth_file() else "NO"),
        f"lab home names before {names_before}, after {names_after}",
        *say_changes(changes),
        "~/.codex/auth.json: " + ("not among the changes" if not any(c.endswith(" auth.json") for c in codex_changes) else "CHANGED"),
        f"lsof across {watched['samples']} samples: {hits_line(watched['hits'], watched['net'])}",
        "every process reaped" if not left else f"LEFT RUNNING: {left}",
    ]
    write_private(EVIDENCE / f"{tag}.md", tilde("\n".join(f"- {s}" for s in summary) + "\n"))
    for line in summary:
        say(line)
    if watched["alarm"]:
        return 5
    clean = not any(changes.values()) and not any(watched["hits"].values()) and not left
    return 0 if has_auth_file() and clean else 4


def without(text: str, name: str) -> str:
    """`text` with the line that sets `name` (by dotted name) taken out."""
    kept, table = [], ""
    for raw in text.splitlines():
        line = strip_comment(raw).strip()
        header = re.fullmatch(r"\[\[?([^\[\]]+)\]\]?", line)
        if header:
            table = header.group(1).strip()
        pair = re.fullmatch(r"([A-Za-z0-9_\-.\"' ]+?)\s*=\s*(.+)", line)
        if pair and (f"{table}.{pair.group(1).strip()}" if table else pair.group(1).strip()) == name:
            continue
        kept.append(raw)
    return "\n".join(kept) + "\n"


def cmd_selftest(args) -> int:
    """No codex runs and no lab is needed: the config check against the
    staged config and broken copies of it, and the --probe-shell match."""
    import tempfile

    failed = []

    def expect(ok: bool, what: str) -> None:
        if not ok:
            failed.append(what)

    good = config_text()
    expect(missing_settings(good) == [], f"the staged config passes: {missing_settings(good)}")
    for name, _ in REQUIRED:
        cut = without(good, name)
        expect(cut != good, f"{name}: its line was found to take out")
        missing = missing_settings(cut)
        expect(len(missing) == 1 and missing[0].startswith(f"{name} = "), f"{name} missing is named: {missing}")
    flipped = good.replace("apps = false", "apps = true")
    expect(missing_settings(flipped) == ["features.apps = false (it says True)"], f"true refused: {missing_settings(flipped)}")
    keychain = good.replace('cli_auth_credentials_store = "file"', 'cli_auth_credentials_store = "keyring"')
    expect(any(m.startswith("cli_auth_credentials_store") for m in missing_settings(keychain)), "the Keychain store refused")
    twice = good + "\n[features]\nplugins = false\n"
    expect(any("set twice" in m for m in missing_settings(twice)), "a name set twice refused")
    commented = good.replace("tool_suggest = false", "# tool_suggest = false")
    expect(any(m.startswith("features.tool_suggest") for m in missing_settings(commented)), "a commented-out line doesn't count")
    trailing = good.replace("tool_suggest = false", "tool_suggest = false # off")
    expect(missing_settings(trailing) == [], "a trailing comment is fine")
    dotted = without(good, "features.plugins").replace(
        "cli_auth_credentials_store", "features.plugins = false\ncli_auth_credentials_store", 1
    )
    expect(missing_settings(dotted) == [], f"a dotted top-level key counts: {missing_settings(dotted)}")
    inline = without(good, "features.apps").replace("[features]", "features_note = 1\n[features]")
    inline += "\n[other]\napps = false\n"
    expect(any(m.startswith("features.apps") for m in missing_settings(inline)), "the key in another table doesn't count")
    with tempfile.TemporaryDirectory() as folder:
        path = Path(folder) / "config.toml"
        path.write_text(without(good, "features.daemon_auto_start"))
        try:
            check_config(path)
            failed.append("check_config let a config without daemon_auto_start through")
        except Refused as why:
            expect("features.daemon_auto_start = false (missing)" in str(why), f"the refusal names the key: {why}")
        path.write_text(good)
        try:
            check_config(path)
        except Refused as why:
            failed.append(f"check_config refused the staged config: {why}")
    expect(is_probe_shell(f"/bin/zsh -lc '{AGENT} --probe-shell'"), "the wrapped probe-shell matches")
    expect(not is_probe_shell(f"{AGENT} --probe-shell; rm -rf x"), "anything more doesn't")

    # Listings: names, sizes and dates; a size, a date, a new and a gone name.
    with tempfile.TemporaryDirectory() as folder:
        root = Path(folder)
        (root / "a").write_text("one")
        (root / "b").write_text("two")
        (root / "sub").mkdir()
        (root / "sub" / "c").write_text("three")
        os.symlink("/nonexistent", root / "link")
        before = folder_snapshot(root)
        expect(before["entries"] == 6, f"every entry, the link not followed: {before['entries']}")
        expect(all(len(row) == 3 for row in before["top"]), "the top rows carry a size")
        expect(folder_changes(before, folder_snapshot(root)) == [], "nothing changed, nothing listed")
        stamp_b = os.lstat(root / "b").st_mtime_ns
        (root / "a").write_text("one more")  # another size
        os.utime(root / "b", ns=(stamp_b + 5_000_000_000, stamp_b + 5_000_000_000))  # another date
        (root / "sub" / "c").unlink()
        (root / "d").write_text("four")
        found = folder_changes(before, folder_snapshot(root))
        for want in ("changed a", "changed b", "added d", "removed sub/c"):
            expect(want in found, f"{want} listed: {found}")
        expect(folder_snapshot(root / "none")["exists"] is False, "a missing folder is listed as missing")
        expect(len(top_lines(before)) == 4 and "link" in top_lines(before)[2], f"top lines: {top_lines(before)}")

    # lsof: each guarded place, a near miss, sockets, the lab.
    text = "\n".join([
        "p100", "tREG", f"n{REAL_CODEX_DIR}/config.toml",
        "tREG", f"n{APP_SUPPORT_CODEX}/Local State",
        "tREG", f"n{KEYCHAINS}/login.keychain-db",
        "tREG", f"n{REAL_CODEX_DIR}-other/x",
        "p101", "tIPv4", "n10.0.0.1:50000->1.2.3.4:443", "tunix", "n->0x1234",
        "tREG", "n/private/tmp/aij-lab-c2/codex-home/state_5.sqlite",
    ])
    found = lsof_parse(text)
    expect([len(found["hits"][label]) for label, _ in GUARDED] == [1, 1, 1], f"one hit per place: {found['hits']}")
    expect(found["net"] == ["101 10.0.0.1:50000->1.2.3.4:443"], f"one internet socket, the unix one not: {found['net']}")
    expect(found["files"] == 7 and len(found["lab"]) == 1, f"counts: {found['files']}, {found['lab']}")
    quiet = lsof_parse("p1\ntREG\nn/tmp/aij-lab-c2/x\ntIPv4\nn1.2.3.4:443")
    expect(alarm_reasons(found, False) and len(alarm_reasons(found, False)) == 3, "a guarded hit stops a run")
    expect(alarm_reasons(quiet, False) == [], "a socket is fine where a model call is expected")
    expect(len(alarm_reasons(quiet, True)) == 1, "a socket stops a run that expects none")

    # The guard denies both listed places and any write inside the app.
    profile = guard_profile()
    for _, path in LISTED:
        expect(f'(subpath "{real(path)}")' in profile, f"the guard denies {path}")
    expect(f'(deny file-write* (subpath "{real(APP_BUNDLE)}")' in profile, "the guard denies writes inside the app")

    # --codex: the two known binaries, nothing else; the bundle's pathDir first on PATH.
    global CODEX
    chosen = CODEX
    try:
        CODEX = Path("/nonexistent/codex")
        try:
            codex_identity()
            failed.append("an unknown --codex was taken")
        except Refused:
            pass
        if os.access(HOMEBREW_CODEX, os.X_OK):
            CODEX = HOMEBREW_CODEX
            expect(codex_identity()["kind"] == "homebrew" and child_env()["PATH"] == PATH, "homebrew: PATH as is")
        if BUNDLED_WRAPPER.is_file():
            CODEX = BUNDLED_WRAPPER
            identity = codex_identity()  # reads the wrapper's bytes and the package file only
            expect(identity["kind"] == "bundled" and identity["runs"] == str(BUNDLED_TARGET), f"bundled: {identity}")
            expect(child_env()["PATH"].startswith(f"{identity['path_dir']}:") if identity["path_dir"] else True,
                   "the bundle's pathDir comes first on PATH")
            expect("CODEX_HOME" in child_env() and child_env()["CODEX_HOME"] == str(CODEX_HOME), "the lab home either way")
    finally:
        CODEX = chosen
        IDENTITY.clear()

    # features list: what is on.
    listing = "plugins  stable  false\nnew_fetcher  stable  true\nshell_tool  stable  true\n"
    expect(features_on(listing) == ["new_fetcher", "shell_tool"], f"features on: {features_on(listing)}")
    expect(sorted(set(features_on(listing)) - KNOWN_ON) == ["new_fetcher"], "a feature beyond 0.163's is named")

    # Stop at once kills a whole tree, children in groups of their own too
    # (a bounded `sleep`, never codex).
    shell = subprocess.Popen(
        ["/bin/sh", "-c", "/usr/bin/perl -e 'setpgrp(0,0); sleep 20' & /bin/sleep 20 & wait"],
        start_new_session=True,
    )
    time.sleep(0.5)
    tree = descendants(shell.pid, ps_table())
    kill_tree(shell.pid)
    shell.wait(timeout=10)
    time.sleep(0.3)
    alive = [pid for pid in tree if pid in ps_table()]
    expect(len(tree) >= 3 and not alive, f"kill_tree left {alive} of {tree}")

    for line in failed:
        say(f"FAIL: {line}")
    say(f"c2_probe selftest: {'OK' if not failed else f'{len(failed)} failed'}")
    return 0 if not failed else 1


def cmd_reap(args) -> int:
    return 0 if not reap() else 4


def main(argv: list) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "step",
        choices=["stage", "status", "features", "dry", "preflight", "login", "run", "reap", "compare", "selftest"],
    )
    parser.add_argument("paths", nargs="*", help="compare: two report .json files")
    parser.add_argument(
        "--codex", default=str(HOMEBREW_CODEX),
        help=f"the codex to run: {HOMEBREW_CODEX} (default) or the app's bundled {BUNDLED_WRAPPER}, in place",
    )
    parser.add_argument("--agent", default=str(AGENT_SOURCE), help="the debug juggler-agent to pin (stage)")
    parser.add_argument("--rebaseline", action="store_true", help="take the listings of ~/.codex and the app's Codex data again first")
    parser.add_argument("--no-guard", action="store_true", help="run codex without the sandbox that denies ~/.codex and the app's data")
    parser.add_argument("--no-open", action="store_true", help="login: print the page, don't open it")
    args = parser.parse_args(argv)
    os.umask(0o077)
    steps = {
        "stage": cmd_stage, "status": cmd_status, "dry": cmd_dry, "preflight": cmd_preflight,
        "login": cmd_login, "run": cmd_run, "reap": cmd_reap, "selftest": cmd_selftest,
        "features": cmd_features, "compare": cmd_compare,
    }
    global CODEX
    CODEX = Path(args.codex)
    try:
        return steps[args.step](args)
    except Refused as why:
        say(f"REFUSED: {why}")
        return 3
    except KeyboardInterrupt:
        reap()
        return 130
    except Exception:
        reap()  # nothing the lab started outlives a failure
        raise


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
