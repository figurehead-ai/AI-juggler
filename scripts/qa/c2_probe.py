#!/usr/bin/env python3
"""AJ-E3013 lab probes C2 and C3, headless, at one shared `codex app-server`.

The question (docs/research/desktop-pool-members.md §4.4, §7.4): when several
threads share one `codex app-server`, can the MCP server it spawns tell which
thread is calling? This answers it at its source, with the `codex` CLI only
(the ChatGPT desktop app is never launched), on a lab home that is never a
real profile. Runbook: docs/qa/evidence/AJ-E3013/c2-runbook.md.

Run it with the standard library only: `python3 -I scripts/qa/c2_probe.py STEP`

    stage      make the lab (0700), pin the probe binary, write config.toml,
               record ~/.codex's names and dates as the baseline
    status     what is staged; signed in or not (by file name only)
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

Every step that runs codex refuses when CODEX_HOME names another folder, when
~/.codex differs from the staged baseline (names and dates only; nothing in
it is ever opened), or when another Codex is running; it runs codex with a
small environment and inside a macOS sandbox that denies every read and
write under ~/.codex (`--no-guard` drops only that sandbox), checks the
server's own `codexHome`, samples ps and lsof while it runs, compares
~/.codex afterwards, and reaps every process it started. It never reads
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
BASELINE = EVIDENCE / "codex-dir-baseline.json"
GROUPS = EVIDENCE / "process-groups.txt"
CODEX = Path("/opt/homebrew/bin/codex")
HOME = Path.home()
REAL_CODEX_DIR = HOME / ".codex"
AGENT_SOURCE = HOME / "Dev/AIJuggler-shared-target/feed/debug/juggler-agent"
# Read for its version only; nothing in the ChatGPT app is ever run.
APP_CODEX_PACKAGE = Path(
    "/Applications/ChatGPT.app/Contents/Resources/codex-cli/codex-package.json"
)
MARKER = b"AIJ_PROBE_V1"
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

# No downloads, nothing shared: the first dry run (2026-11-16) found the
# app-server cloning github.com/openai/plugins (93 MB) into this home at
# start. These flags (names from `codex features list`) turn off that sync,
# remote plugins and Apps connectors, MCP dependency installs, tool
# suggestions, in-app updates and the shared local app-server daemon.
[features]
plugins = false
remote_plugin = false
apps = false
skill_mcp_dependency_install = false
tool_suggest = false
in_app_updates = false
daemon_auto_start = false

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


def codex_version() -> str:
    out = subprocess.run(
        guarded([str(CODEX), "--version"], True), env=child_env(), cwd=WORK,
        capture_output=True, text=True, timeout=30, check=False,
    )
    return out.stdout.strip() or out.stderr.strip()


def app_codex_version() -> str:
    try:
        return json.loads(APP_CODEX_PACKAGE.read_text()).get("version", "?")
    except (OSError, ValueError):
        return "not installed"


# ----------------------------------------------------------- ~/.codex, by names


def codex_dir_snapshot() -> dict:
    """~/.codex's names and modification dates (never a file's content)."""
    top = []
    try:
        entries = sorted(os.scandir(REAL_CODEX_DIR), key=lambda e: e.name)
    except FileNotFoundError:
        return {"exists": False, "top": [], "deep": {}, "deep_sha256": ""}
    for entry in entries:
        meta = entry.stat(follow_symlinks=False)
        top.append([entry.name, meta.st_mtime_ns])
    deep = {".": os.lstat(REAL_CODEX_DIR).st_mtime_ns}
    for folder, dirs, files in os.walk(REAL_CODEX_DIR, followlinks=False):
        for name in dirs + files:
            path = os.path.join(folder, name)
            try:
                deep[os.path.relpath(path, REAL_CODEX_DIR)] = os.lstat(path).st_mtime_ns
            except FileNotFoundError:
                continue
    blob = json.dumps(sorted(deep.items())).encode()
    return {
        "exists": True,
        "top": top,
        "deep": deep,
        "deep_sha256": hashlib.sha256(blob).hexdigest(),
        "entries": len(deep),
    }


def top_lines(snapshot: dict) -> list:
    """`ls -la`, names and dates only."""
    return [
        f"{dt.datetime.fromtimestamp(ns / 1e9):%Y-%m-%d %H:%M:%S}  {name}"
        for name, ns in snapshot["top"]
    ]


def codex_dir_changes(before: dict, after: dict) -> list:
    """Names (relative to ~/.codex) added, removed or with another date."""
    old, new = before.get("deep", {}), after.get("deep", {})
    changed = [name for name in new if name in old and new[name] != old[name]]
    added = [name for name in new if name not in old]
    removed = [name for name in old if name not in new]
    return [f"changed {n}" for n in sorted(changed)] + [
        f"added {n}" for n in sorted(added)
    ] + [f"removed {n}" for n in sorted(removed)]


def load_baseline() -> dict:
    if not BASELINE.exists():
        raise Refused("no baseline of ~/.codex: run `stage` first")
    return json.loads(BASELINE.read_text())


def save_baseline() -> dict:
    snapshot = codex_dir_snapshot()
    snapshot["taken"] = dt.datetime.now().isoformat(timespec="seconds")
    write_private(BASELINE, json.dumps(snapshot))
    return snapshot


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


def lsof(pids: list) -> dict:
    """Open files of `pids`: anything under ~/.codex or the Keychains, the
    count of network sockets, and the lab paths, never contents."""
    if not pids:
        return {"codex_dir": [], "keychains": [], "net": [], "files": 0, "lab": []}
    out = subprocess.run(
        ["/usr/sbin/lsof", "-nP", "-w", "-p", ",".join(map(str, pids)), "-F", "ptn"],
        capture_output=True, text=True, check=False,
    ).stdout
    codex_dir = real(REAL_CODEX_DIR)
    keychains = str(HOME / "Library/Keychains")
    pid, kind, hits, keys, net, files, lab = 0, "", [], [], [], 0, set()
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
            if name == codex_dir or name.startswith(codex_dir + "/") or name.startswith(str(REAL_CODEX_DIR) + "/"):
                hits.append(f"{pid} {name}")
            if name.startswith(keychains):
                keys.append(f"{pid} {name}")
            if "aij-lab-c2" in name:
                lab.add(name)
    return {"codex_dir": hits, "keychains": keys, "net": net, "files": files, "lab": sorted(lab)}


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
        "PATH": PATH,
        "CODEX_HOME": str(CODEX_HOME),
        "CODEX_SQLITE_HOME": str(CODEX_HOME),
    }


def guard_profile() -> str:
    """A sandbox for the whole codex tree: everything allowed but any read
    or write under ~/.codex (an attempt fails instead of happening)."""
    target = real(REAL_CODEX_DIR)
    if '"' in target or "\\" in target:
        raise Refused("~/.codex's path can't be quoted for the sandbox")
    return f'(version 1)(allow default)(deny file-read* file-write* (subpath "{target}"))'


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
    if not os.access(AGENT, os.X_OK) or MARKER not in AGENT.read_bytes():
        raise Refused(f"{AGENT} is missing or isn't a probe build: run `stage`")
    if not (CODEX_HOME / "config.toml").exists():
        raise Refused("the lab config.toml is missing: run `stage`")
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
    baseline = save_baseline() if rebaseline else load_baseline()
    now = codex_dir_snapshot()
    changes = codex_dir_changes(baseline, now)
    if changes:
        raise Refused(
            "~/.codex differs from the staged baseline (names and dates only). If that was the "
            "maintainer's own Codex, look at the names and run again with --rebaseline:\n  "
            + "\n  ".join(changes[:40])
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
    and every open name under ~/.codex or the Keychains, and every internet
    socket, across all samples (short-lived children such as the probe's
    first start are caught too)."""

    def __init__(self, root: int, every: float = 0.5):
        self.root, self.every = root, every
        self.seen: dict = {}
        self.codex_dir: set = set()
        self.keychains: set = set()
        self.net: set = set()
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
                self.codex_dir.update(found["codex_dir"])
                self.keychains.update(found["keychains"])
                self.net.update(found["net"])
                self.samples += 1
            self.halt.wait(self.every)

    def stop(self) -> dict:
        self.halt.set()
        self.thread.join(timeout=10)
        return {
            "samples": self.samples,
            "processes": sorted(self.seen.values(), key=lambda row: row["pid"]),
            "codex_dir": sorted(self.codex_dir),
            "keychains": sorted(self.keychains),
            "net": sorted(self.net),
        }


class AppServer:
    """One `codex app-server` on the lab home, over stdio (newline JSON-RPC),
    as AI Juggler's adapter drives it (crates/juggler-adapters/src/
    codex_appserver)."""

    def __init__(self, guard: bool, tag: str):
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
        self.watch = Watcher(self.proc.pid)
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
        out.append(
            f"Across all samples: open under ~/.codex {len(watched['codex_dir'])} {watched['codex_dir'] or ''};"
            f" under ~/Library/Keychains {len(watched['keychains'])}; internet sockets {len(watched['net'])}"
            f" {watched['net'] or ''}"
        )
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
            f"lsof: {found['files']} open names; under ~/.codex: {len(found['codex_dir'])}"
            f" {found['codex_dir'] or ''}; Keychains: {len(found['keychains'])}; internet sockets: {len(found['net'])} {found['net'] or ''};"
            f" lab paths: {', '.join(found.get('lab', [])) or 'none'}"
        )
        out.append("")
    write_private(path, tilde("\n".join(out) + "\n"))
    return path


def finish(server: AppServer, baseline: dict, body: dict, summary: list, tag: str) -> int:
    server.stop()
    left = reap(quiet=True)
    after = codex_dir_snapshot()
    changes = codex_dir_changes(baseline, after)
    body["codex_dir_after"] = {"top": top_lines(after), "deep_sha256": after["deep_sha256"], "changes": changes}
    body["left_running"] = [f"{p} {c[:100]}" for p, _, c in left]
    body["watch"] = server.watched
    touched = [hit for entry in body.get("samples", []) for hit in entry["lsof"]["codex_dir"]]
    touched += [hit for hit in server.watched.get("codex_dir", []) if hit not in touched]
    homes = {row["env"]["CODEX_HOME"] for row in server.watched.get("processes", [])}
    samples = len(body.get("samples", [])) + server.watched.get("samples", 0)
    summary.insert(0, f"~/.codex: {'UNCHANGED' if not changes else 'CHANGED: ' + '; '.join(changes[:10])}"
                   f" (names and dates, {after.get('entries', 0)} entries, digest {after['deep_sha256'][:16]})")
    summary.insert(1, f"lsof under ~/.codex across {samples} samples: {len(touched)} {touched or ''}")
    summary.insert(2, f"processes started: {len(server.watched.get('processes', []))}; their CODEX_HOME values: {sorted(h or '(none)' for h in homes)}")
    summary.insert(3, "every process reaped" if not left else f"LEFT RUNNING: {body['left_running']}")
    path = write_report(tag, body, summary)
    for line in summary:
        say(line)
    say(f"report: {path}")
    return 0 if not changes and not touched and not left else 4


def begin(args, tag: str, need_auth: bool = False):
    baseline = preconditions(args.rebaseline, need_auth)
    body = {
        "tag": tag,
        "codex": codex_version(),
        "app_bundled_codex": app_codex_version(),
        "guard": not args.no_guard,
        "agent_sha256": sha256(AGENT),
        "config_sha256": sha256(CODEX_HOME / "config.toml"),
        "codex_dir_before": {"top": top_lines(baseline), "deep_sha256": baseline["deep_sha256"]},
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
    if MARKER not in source.read_bytes():
        raise Refused(f"{source} isn't a debug build with the probe ({MARKER.decode()} missing)")
    if os.path.lexists(AGENT):
        os.unlink(AGENT)
    shutil.copy2(source, AGENT)
    os.chmod(AGENT, 0o700)
    write_private(CODEX_HOME / "config.toml", config_text())
    if not BASELINE.exists() or args.rebaseline:
        baseline = save_baseline()
    else:
        baseline = load_baseline()
    say(f"lab: {LAB} (0700); home {CODEX_HOME}; names: {sorted(os.listdir(CODEX_HOME))}")
    say(f"probe: {AGENT} sha256 {sha256(AGENT)} (from {source})")
    say(f"config.toml sha256 {sha256(CODEX_HOME / 'config.toml')}")
    say(f"~/.codex baseline taken {baseline.get('taken')}: {baseline.get('entries')} names, digest {baseline['deep_sha256'][:16]}")
    for line in top_lines(baseline):
        say(f"    {line}")
    say(f"codex: {codex_version()} (the ChatGPT app bundles {app_codex_version()})")
    say("signed in: " + ("yes (auth.json present, by name)" if has_auth_file() else "no"))
    return 0


def cmd_status(args) -> int:
    say(f"lab home: {CODEX_HOME} names {sorted(os.listdir(CODEX_HOME)) if CODEX_HOME.exists() else 'missing'}")
    say("signed in: " + ("yes (auth.json present, by name)" if has_auth_file() else "no"))
    if AGENT.exists():
        say(f"probe: sha256 {sha256(AGENT)}, marker {'present' if MARKER in AGENT.read_bytes() else 'MISSING'}")
    try:
        changes = codex_dir_changes(load_baseline(), codex_dir_snapshot())
        say("~/.codex vs baseline: " + ("unchanged" if not changes else f"{len(changes)} changes: {changes[:10]}"))
    except Refused as why:
        say(str(why))
    say(f"lab processes: {lab_processes() or 'none'}; other Codex: {other_codex(set()) or 'none'}")
    return 0


def cmd_dry(args) -> int:
    tag = f"dry-{stamp()}"
    baseline, body = begin(args, tag)
    rotate_probe_file(tag)
    since = now_ms()
    server = AppServer(not args.no_guard, tag)
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
    server = AppServer(not args.no_guard, tag)
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
    left = reap(quiet=True)
    after = codex_dir_snapshot()
    changes = codex_dir_changes(baseline, after)
    names_after = sorted(os.listdir(CODEX_HOME))
    summary = [
        outcome,
        "lab home signed in: " + ("yes, auth.json present (by name, never opened)" if has_auth_file() else "NO"),
        f"lab home names before {names_before}, after {names_after}",
        "~/.codex: " + ("UNCHANGED (names and dates)" if not changes else "CHANGED: " + "; ".join(changes[:20])),
        "~/.codex/auth.json: " + ("not among the changes" if not any(c.endswith(" auth.json") for c in changes) else "CHANGED"),
        "every process reaped" if not left else f"LEFT RUNNING: {left}",
    ]
    write_private(EVIDENCE / f"{tag}.md", tilde("\n".join(f"- {s}" for s in summary) + "\n"))
    for line in summary:
        say(line)
    return 0 if has_auth_file() and not changes and not left else 4


def cmd_reap(args) -> int:
    return 0 if not reap() else 4


def main(argv: list) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("step", choices=["stage", "status", "dry", "preflight", "login", "run", "reap"])
    parser.add_argument("--agent", default=str(AGENT_SOURCE), help="the debug juggler-agent to pin (stage)")
    parser.add_argument("--rebaseline", action="store_true", help="take ~/.codex's names and dates again first")
    parser.add_argument("--no-guard", action="store_true", help="run codex without the ~/.codex-denying sandbox")
    parser.add_argument("--no-open", action="store_true", help="login: print the page, don't open it")
    args = parser.parse_args(argv)
    os.umask(0o077)
    steps = {
        "stage": cmd_stage, "status": cmd_status, "dry": cmd_dry, "preflight": cmd_preflight,
        "login": cmd_login, "run": cmd_run, "reap": cmd_reap,
    }
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
