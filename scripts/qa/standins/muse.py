#!/usr/bin/env python3
"""AJ-E1612: a lab stand-in for Muse Code (`muse`, 1.4.13-R6996.1), for a
dry run of the sign-in sitting on AI Juggler Dev. Never the real tool,
never a real sign-in and never the Keychain: every value it writes is
`lab-not-a-token`, every page it prints is on `example.invalid`, and it runs
only on a lab home with the file credential backend.

Dev runs it through the adapter's test override AI_JUGGLER_MUSE_BIN
(`install()` makes `<lab>/standins/bin/muse` and, beside it, the launcher's
record `.muse-version` = `1.4.13-R6996.1`: the build the lab's file-backend
sign-in, AIJ_LAB_MUSE_SIGN_INS=1, is allowed on). Each App's folder is a
home of its own: HOME, XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_CACHE_HOME and
XDG_STATE_HOME inside it, MUSE_NO_AUTO_UPDATE=1, and for a lab App
TBH_CREDENTIAL_BACKEND=file (crates/juggler-adapters/src/muse.rs). What Dev
runs, and what this answers:

- `muse --version` (discovery, only when `.muse-version` doesn't read):
  `Muse Code 1.4.13 (1.4.13-R6996.1)`. No lab home needed.
- `muse login` (Sign in, in a pty): refuses (exit 5) unless
  TBH_CREDENTIAL_BACKEND is exactly `file`, so a Keychain write can never
  be mistaken for this; else prints a fake approval code and page, waits
  AIJ_LAB_STANDIN_WAIT seconds, and writes the file backend's sign-in,
  `$XDG_CONFIG_HOME/muse/auth.json` (0600: `providers.meta.mechanism`
  `oauth`, `access_token`, `expires_at`). `muse logout` removes it.
- `muse serve` (a session Dev drives, the default runtime; stdio JSON-RPC,
  the Muse Session Protocol v1, until stdin closes): `initialize` (its
  `museHome` is `$XDG_DATA_HOME/muse`, which the adapter checks),
  `session/start` (a UUIDv7 session, a `session/started` event) and
  `session/resume`, `turn/start` (a UUIDv7 `commandId` or refused; then
  `turn/started`, the user's `item/completed`, the answer `lab` as
  `item/started` + `item/delta` + `item/completed`, `turn/completed`
  `completed` with a few made-up tokens, and `usage/changed`), and
  `model/list`, `usage/read`, `usage/refresh` (experimental), `account/read`,
  `session/list`, `session/read`, `session/setModel`, `turn/interrupt`,
  `turn/steer`. Without a sign-in a turn ends `failed`, `authRequired`.
- `muse [--model M]`, `muse [--model M] resume <id>`, `muse -- <prompt>`
  (the terminal: a new session, the terminal view after the driver lets
  go, a first message): a prompt; each line answered `lab`; a resume first
  prints the earlier turns.
- Each session, however it ran, is recorded where the adapter reads it:
  a row in `$XDG_DATA_HOME/muse/session-index.db` (WAL; `schema_meta`
  `schema_version` `1`, the `sessions` table as Muse 1.4.13 creates it:
  `session_id`, `workspace_root`, `title`, `first_user_prompt`,
  `created_at_us`, `updated_at_us`, `status`, `msp_kind` `root`, …) and its
  log `sessions/YYYY/MM/DD/<id>/session.jsonl`.
- `muse usage --json` (usage, when the server's answer fails): the same
  object `usage/refresh` gives: `{subscriptionActive, tierName, usage:
  {observedAtMs, tier, window: {usedPercent, resetsAtMs,
  windowDurationMins: 300}, weekly: {usedPercent, resetsAtMs}}}`, modest
  made-up numbers; exit 1 when not signed in.
- `--selftest`: each of the above as a subprocess in a throwaway lab, and
  the names written checked against the adapter's Rust source.

Every run first refuses the network, logs its words to
`<lab>/standins/calls.log`, and stops unless HOME and the four XDG roots
are in a lab (exit 3), MUSE_NO_AUTO_UPDATE is `1` and MUSE_AUTH_PATH unset
(exit 3), and TBH_CREDENTIAL_BACKEND is exactly `file` (exit 5)
(`--version`/`--help` excepted).

What a dry run on this stand-in cannot prove:
- the real sign-in (Meta's page, the account's subscription), and that
  1.4.13's file backend keeps away from the Keychain (that proof is the
  disassembly in docs/research/muse-code.md, and muse-gate in the sitting);
- the real `session.jsonl` records (the adapter doesn't read them; these
  are the stand-in's own), session names under ~/Library/Application
  Support/muse, and every MSP field the adapter doesn't read;
- the real plan's numbers, and whether `usage/refresh` reaches Meta;
- Dev's process recognition: the running process is `python3 …/muse.py`,
  not `muse-bin-<version>`, so `session_process` isn't exercised;
- Muse's own refusal to resume a session another client holds.
"""
from __future__ import annotations

import importlib.util
import json
import os
import re
import secrets
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load_common():
    spec = importlib.util.spec_from_file_location("standin_common", HERE / "standin_common.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


common = _load_common()

TOOL = "muse"
VERSION = "1.4.13"
BUILD = "1.4.13-R6996.1"
VERSION_FILE = ".muse-version"
ENV_BIN = "AI_JUGGLER_MUSE_BIN"
ENV_BACKEND = "TBH_CREDENTIAL_BACKEND"
FILE_BACKEND = "file"
ENV_NO_UPDATE = "MUSE_NO_AUTO_UPDATE"
ENV_LAUNCHER_AUTH = "MUSE_AUTH_PATH"
ENV_API_KEY = "META_API_KEY"
ROOTS = ("XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME")
MUSE_DIR = "muse"
AUTH_FILE = "auth.json"
INDEX_FILE = "session-index.db"
SCHEMA_VERSION = "1"
COLUMNS = ("session_id", "workspace_root", "title", "first_user_prompt", "created_at_us", "updated_at_us",
           "status", "msp_kind", "msp_parent_session_id")
FIVE_HOURS_MINS = 300
MODEL = "muse-lab-1"
TIER_NAME = "Lab"
CLIENT_TUI = "muse_tui"
UUID7 = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")

# Muse 1.4.13's own `sessions` table (as the adapter's fixture,
# crates/juggler-adapters/tests/muse_fixture/mod.rs, records it).
SESSIONS_TABLE = """CREATE TABLE IF NOT EXISTS sessions (  session_id          TEXT PRIMARY KEY,
  session_stream_id   TEXT NOT NULL,
  session_dir         TEXT NOT NULL,
  session_log_path    TEXT NOT NULL UNIQUE,
  layout              TEXT NOT NULL,
  workspace_root      TEXT,
  workspace_key       TEXT,
  provider_id         TEXT,
  model_id            TEXT,
  git_branch          TEXT,
  title               TEXT NOT NULL,
  effective_long_title TEXT,
  title_provenance TEXT CHECK(title_provenance IN ('generated','manual')),
  title_name_fingerprint TEXT,
  title_source_fingerprint TEXT,
  first_user_prompt   TEXT,
  search_text         TEXT NOT NULL,
  created_at_us       INTEGER,
  updated_at_us       INTEGER,
  prompt_count        INTEGER NOT NULL DEFAULT 0,
  status              TEXT NOT NULL,
  status_rank         INTEGER NOT NULL,
  source_fingerprint  TEXT,
  indexed_at_us       INTEGER NOT NULL,
  latest_segment_terminated INTEGER NOT NULL DEFAULT 0,
  session_name        TEXT,
  session_name_revision INTEGER CHECK(session_name_revision >= 0),
  session_name_description_dependency TEXT,
  logged_session_name TEXT,
  logged_session_name_revision INTEGER CHECK(logged_session_name_revision >= 0),
  logged_session_name_seeded INTEGER,
  msp_created_at_us   INTEGER,
  msp_updated_at_us   INTEGER,
  msp_last_activity_at_us INTEGER,
  msp_order_key_us    INTEGER,
  msp_turn_count      INTEGER,
  msp_fork_source_session_id TEXT,
  msp_fork_cut_cursor TEXT,
  msp_fork_cut_explicit INTEGER,
  msp_fork_command_id TEXT,
  msp_provider_id     TEXT,
  msp_model_id        TEXT,
  msp_title           TEXT,
  msp_first_user_prompt TEXT,
  msp_branch          TEXT,
  msp_source_fingerprint TEXT,
  msp_kind            TEXT,
  msp_parent_session_id TEXT,
  msp_created_by_client_name TEXT,
  msp_created_by_client_version TEXT,
  CHECK ((effective_long_title IS NULL) = (title_provenance IS NULL)),
  CHECK (effective_long_title IS NULL OR title_provenance='generated' OR title_name_fingerprint IS NOT NULL),
  CHECK ((session_name IS NULL) = (session_name_revision IS NULL)),
  CHECK ((logged_session_name IS NULL) = (logged_session_name_revision IS NULL)));"""

HELP = """Usage: muse [OPTIONS] [PROMPT] | muse <COMMAND>   (lab stand-in for Muse Code 1.4.13)

Commands:
  resume <id>     resume a session in the terminal
  serve           the session server (MSP over stdio)
  login, logout   the lab sign-in (file backend only)
  usage --json    the plan's windows
Options:
  --model <m>     the model
  -V, --version   print the version
  -h, --help      this help
"""


def dumps(value: object) -> str:
    return json.dumps(value, separators=(",", ":"))


def now_us() -> int:
    return time.time_ns() // 1000


def iso(ms: int) -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(ms / 1000)) + f".{ms % 1000:03d}Z"


def uuid7() -> str:
    raw = bytearray(common.now_ms().to_bytes(6, "big") + secrets.token_bytes(10))
    raw[6] = 0x70 | (raw[6] & 0x0F)
    raw[8] = 0x80 | (raw[8] & 0x3F)
    h = raw.hex()
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"


# ---- the lab home ---------------------------------------------------------

class Home:
    """The App's folder as the adapter sets it, checked to be a lab's."""

    def __init__(self) -> None:
        env = os.environ
        self.home = common.require_lab(env.get("HOME"), TOOL, "HOME")
        for root in ROOTS:
            common.require_lab(env.get(root), TOOL, root)
        if env.get(ENV_NO_UPDATE) != "1":
            common.fail(TOOL, f"{ENV_NO_UPDATE} isn't 1: the real launcher would update itself on this run")
        if env.get(ENV_LAUNCHER_AUTH) is not None:
            common.fail(TOOL, f"{ENV_LAUNCHER_AUTH} is set: the real launcher would read a sign-in outside this folder")
        if env.get(ENV_BACKEND) != FILE_BACKEND:
            common.fail(TOOL, f"{ENV_BACKEND} is {env.get(ENV_BACKEND)!r}, not exactly 'file': the real Muse would "
                              "use this Mac's Keychain sign-in (one per Mac) here; refusing", 5)
        self.config = Path(env["XDG_CONFIG_HOME"]) / MUSE_DIR
        self.data = Path(env["XDG_DATA_HOME"]) / MUSE_DIR
        self.auth = self.config / AUTH_FILE
        self.index = self.data / INDEX_FILE

    def subscription(self) -> bool:
        return self.auth.is_file() and self.auth.stat().st_size > 0

    def signed_in(self) -> bool:
        return self.subscription() or bool(os.environ.get(ENV_API_KEY))

    def connect(self) -> sqlite3.Connection:
        common.require_lab(self.index, TOOL, "the session index")
        self.data.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(self.index, timeout=5)
        conn.execute("PRAGMA journal_mode = WAL")
        conn.executescript("CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
                           + SESSIONS_TABLE)
        conn.execute("INSERT OR IGNORE INTO schema_meta VALUES ('schema_version', ?)", (SCHEMA_VERSION,))
        conn.commit()
        return conn


# ---- sign-in --------------------------------------------------------------

def login() -> int:
    home = Home()
    print("Muse Code sign-in (lab stand-in, file backend)", flush=True)
    common.fake_sign_in_page(TOOL, "muse-login", common.device_code())
    home.config.mkdir(parents=True, exist_ok=True)
    os.chmod(home.config, 0o700)
    common.write_json(home.auth, {"version": 1, "credential_backend": FILE_BACKEND, "providers": {"meta": {
        "mechanism": "oauth", "access_token": common.NOT_A_TOKEN, "refresh_token": common.NOT_A_TOKEN,
        "expires_at": common.now_ms() + 3_600_000}}}, TOOL)
    print("Signed in to Muse Code (lab stand-in: a dummy sign-in in this folder's auth.json).", flush=True)
    return 0


def logout() -> int:
    home = Home()
    if home.auth.is_file():
        common.require_lab(home.auth, TOOL, "the sign-in file")
        home.auth.unlink()
    print("Signed out.", flush=True)
    return 0


# ---- usage ----------------------------------------------------------------

def usage_result() -> dict:
    now = common.now_ms()
    return {"subscriptionActive": True, "tierName": TIER_NAME, "usage": {
        "observedAtMs": now, "tier": "tier_lab",
        "window": {"usedPercent": 12, "resetsAtMs": now + 3 * 3_600_000, "windowDurationMins": FIVE_HOURS_MINS},
        "weekly": {"usedPercent": 31, "resetsAtMs": now + 4 * 86_400_000}}}


def usage_command(args: list[str]) -> int:
    home = Home()
    if not home.subscription():
        common.fail(TOOL, "not signed in with a subscription (run `muse login`)", 1)
    result = usage_result()
    if "--json" in args:
        print(json.dumps(result))
    else:
        usage = result["usage"]
        print(f"{TIER_NAME}: 5-hour {usage['window']['usedPercent']}%, weekly {usage['weekly']['usedPercent']}%")
    return 0


# ---- sessions -------------------------------------------------------------

class Session:
    """One session: its index row and its log."""

    def __init__(self, home: Home, row: dict) -> None:
        self.home, self.row = home, row
        self.id = row["session_id"]
        self.log = Path(row["session_log_path"])

    @classmethod
    def start(cls, home: Home, workspace: str, model: str, client: str) -> "Session":
        session_id = uuid7()
        at = now_us()
        day = time.strftime("%Y/%m/%d", time.gmtime(at / 1e6))
        folder = home.data / "sessions" / day / session_id
        log = folder / "session.jsonl"
        common.require_lab(log, TOOL, "the session log")
        conn = home.connect()
        try:
            conn.execute(
                "INSERT INTO sessions (session_id, session_stream_id, session_dir, session_log_path, layout,"
                " workspace_root, provider_id, model_id, title, search_text, created_at_us, updated_at_us,"
                " prompt_count, status, status_rank, indexed_at_us, msp_created_at_us, msp_updated_at_us,"
                " msp_turn_count, msp_provider_id, msp_model_id, msp_kind, msp_created_by_client_name,"
                " msp_created_by_client_version)"
                " VALUES (?1, ?1, ?2, ?3, 'session_jsonl', ?4, 'muse', ?5, '', '', ?6, ?6, 0, 'valid', 0, ?6, ?6, ?6,"
                " 0, 'muse', ?5, 'root', ?7, ?8)",
                (session_id, str(folder), str(log), workspace, model, at, client, VERSION))
            conn.commit()
        finally:
            conn.close()
        common.append_line(log, dumps({"type": "session_start", "sessionId": session_id, "workspaceRoot": workspace,
                                       "modelId": model, "atMs": at // 1000}), TOOL)
        return cls.find(home, session_id)  # type: ignore[return-value]

    @classmethod
    def find(cls, home: Home, session_id: str) -> "Session | None":
        if not home.index.is_file():
            return None
        conn = home.connect()
        conn.row_factory = sqlite3.Row
        try:
            row = conn.execute("SELECT * FROM sessions WHERE session_id = ?", (session_id,)).fetchone()
        finally:
            conn.close()
        return cls(home, dict(row)) if row else None

    def records(self) -> list[dict]:
        out = []
        if self.log.is_file():
            for line in self.log.read_text().splitlines():
                try:
                    out.append(json.loads(line))
                except ValueError:
                    pass
        return out

    def turns(self) -> list[tuple[str, str]]:
        said: dict[str, list[str]] = {}
        for record in self.records():
            if record.get("type") in ("user_message", "agent_message"):
                said.setdefault(record["turnId"], ["", ""])[record["type"] == "agent_message"] = record.get("text", "")
        return [(pair[0], pair[1]) for pair in said.values()]

    def view(self) -> dict:
        row = Session.find(self.home, self.id).row  # type: ignore[union-attr]
        return {"sessionId": self.id, "status": "idle", "turnCount": row["prompt_count"], "path": str(self.log),
                "createdAt": iso(row["created_at_us"] // 1000), "updatedAt": iso(row["updated_at_us"] // 1000),
                "workspaceRoot": row["workspace_root"], "providerId": "muse", "modelId": row["model_id"],
                "approvalMode": {"mode": "promptUnmatched", "source": "startup", "lastCommandId": None},
                "forkedFrom": None, "activeTurnId": None}

    def record_turn(self, turn_id: str, text: str, answer: str | None, terminal: str) -> None:
        at = common.now_ms()
        common.append_line(self.log, dumps({"type": "user_message", "turnId": turn_id, "text": text, "atMs": at}), TOOL)
        if answer is not None:
            common.append_line(self.log, dumps({"type": "agent_message", "turnId": turn_id, "text": answer,
                                                "atMs": common.now_ms()}), TOOL)
        common.append_line(self.log, dumps({"type": "turn_end", "turnId": turn_id, "terminal": terminal,
                                            "atMs": common.now_ms()}), TOOL)
        conn = self.home.connect()
        try:
            stamp = now_us()
            first = text.strip().splitlines()[0][:200] if text.strip() else ""
            conn.execute(
                "UPDATE sessions SET prompt_count = prompt_count + 1, msp_turn_count = COALESCE(msp_turn_count, 0) + 1,"
                " first_user_prompt = COALESCE(first_user_prompt, ?2), msp_first_user_prompt = COALESCE(msp_first_user_prompt, ?2),"
                " title = CASE WHEN title = '' THEN ?3 ELSE title END, search_text = search_text || ' ' || ?2,"
                " updated_at_us = ?4, msp_updated_at_us = ?4, msp_last_activity_at_us = ?4, indexed_at_us = ?4,"
                " latest_segment_terminated = 1 WHERE session_id = ?1",
                (self.id, first, first[:80], stamp))
            conn.commit()
        finally:
            conn.close()

    def set_model(self, model: str) -> None:
        conn = self.home.connect()
        try:
            conn.execute("UPDATE sessions SET model_id = ?2, msp_model_id = ?2 WHERE session_id = ?1", (self.id, model))
            conn.commit()
        finally:
            conn.close()


# ---- the terminal ---------------------------------------------------------

def terminal(resume: str | None, prompt: str | None, model: str) -> int:
    home = Home()
    if not home.signed_in():
        common.fail(TOOL, "not signed in: run `muse login`, or set META_API_KEY", 1)
    if resume is not None:
        session = Session.find(home, resume)
        if session is None:
            common.fail(TOOL, f"no session {resume} in this folder", 1)
        print(f"Resumed session {session.id}", flush=True)
        for said, answer in session.turns():
            print(f"> {said}\n{answer}", flush=True)
    else:
        session = Session.start(home, os.getcwd(), model, CLIENT_TUI)

    def on_turn(text: str) -> str:
        answer = common.answer_for(text)
        session.record_turn(uuid7(), text, answer, "completed")
        return answer

    banner = f"Muse Code {VERSION} (lab stand-in) - session {session.id}"
    if prompt is not None:
        print(banner, flush=True)
        print(f"> {prompt}\n{on_turn(prompt)}", flush=True)
        banner = ""
    return common.repl(TOOL, "> ", on_turn, banner)


# ---- `muse serve` (MSP v1 over stdio) -------------------------------------

class Server:
    def __init__(self, home: Home) -> None:
        self.home = home
        self.experimental = False
        self.initialized = False
        self.sessions: dict[str, Session] = {}
        self.cursor = 0

    def send(self, frame: dict) -> None:
        frame = {"jsonrpc": "2.0", **frame}
        sys.stdout.write(dumps(frame) + "\n")
        sys.stdout.flush()

    def event(self, method: str, params: dict) -> None:
        self.cursor += 1
        session = params.get("sessionId", "")
        self.send({"method": method, "params": {"sessionId": session, "viewCursor": f"v9:{session}:{self.cursor}", **params},
                   "emittedAtMs": common.now_ms()})

    def refuse(self, request_id: object, code: int, message: str) -> None:
        self.send({"id": request_id, "error": {"code": code, "message": message}})

    def serve(self) -> int:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                frame = json.loads(line)
            except ValueError:
                self.send({"id": None, "error": {"code": -32700, "message": "parse error"}})
                continue
            if not isinstance(frame, dict) or "method" not in frame:
                continue  # an answer to one of ours (we ask nothing) or noise
            request_id = frame.get("id")
            params = frame.get("params") or {}
            try:
                result = self.handle(frame["method"], params, request_id)
            except LookupError as missing:
                self.refuse(request_id, -32602, str(missing))
                continue
            if request_id is not None and result is not None:
                self.send({"id": request_id, "result": result})
        return 0

    def command_id(self, params: dict) -> str:
        command = params.get("commandId", "")
        if not isinstance(command, str) or not UUID7.match(command):
            raise LookupError("commandId: expected UUIDv7")
        return command

    def session(self, params: dict) -> Session:
        session = self.sessions.get(params.get("sessionId", ""))
        if session is None:
            raise LookupError(f"no open session {params.get('sessionId')!r}")
        return session

    def handle(self, method: str, params: dict, request_id: object) -> dict | None:
        if method == "initialize":
            self.experimental = bool((params.get("capabilities") or {}).get("experimentalApi"))
            return {"serverInfo": {"name": "muse-session-server", "version": VERSION},
                    "userAgent": f"muse-session-server/{VERSION} (macos; lab-stand-in)",
                    "museHome": str(self.home.data), "platformFamily": "unix", "platformOs": "macos",
                    "schema": {"version": 1, "fingerprint": "sha256:lab-stand-in"}, "grantedCapabilities": [],
                    "experimentalApi": self.experimental, "sessionDurability": "durable"}
        if method == "initialized":
            self.initialized = True
            return None
        if method in ("usage/refresh", "account/read") and not self.experimental:
            raise LookupError(f"{method} needs capabilities.experimentalApi")
        if method == "session/start":
            self.command_id(params)
            workspace = params.get("workspaceRoot") or os.getcwd()
            session = Session.start(self.home, workspace, params.get("modelId") or MODEL, "ai_juggler")
            self.sessions[session.id] = session
            view = session.view()
            self.event("session/started", {"session": view})
            return {"session": view, "viewCursor": f"v9:{session.id}:{self.cursor}"}
        if method == "session/resume":
            self.command_id(params)
            session = Session.find(self.home, params.get("sessionId", ""))
            if session is None:
                raise LookupError(f"no session {params.get('sessionId')!r}")
            self.sessions[session.id] = session
            result = {"session": session.view(), "viewCursor": f"v9:{session.id}:{self.cursor}"}
            if not params.get("excludeItems"):
                result["items"] = self.items(session)
            return result
        if method == "turn/start":
            command = self.command_id(params)
            session = self.session(params)
            self.send({"id": request_id, "result": {"commandId": command, "status": "accepted", "turnId": command,
                                                    "startedNewTurn": True, "disposition": "started"}})
            self.turn(session, command, "".join(part.get("text", "") for part in params.get("input") or []
                                                if isinstance(part, dict)))
            return None
        if method in ("turn/steer", "turn/interrupt"):
            command = self.command_id(params)
            self.session(params)
            return {"commandId": command, "status": "accepted"}
        if method == "session/setModel":
            command = self.command_id(params)
            model = (params.get("model") or {}).get("modelId") or MODEL
            self.session(params).set_model(model)
            return {"commandId": command, "status": "accepted"}
        if method == "model/list":
            return {"providerId": "muse", "profileId": None, "source": "bundledCatalog", "models": [
                {"modelId": MODEL, "displayLabel": "Muse Lab 1", "providerId": "muse", "profileId": None,
                 "releaseDate": "2026-11-01", "description": None, "contextLimit": 200000, "outputLimit": 32000,
                 "cost": None, "isActive": True, "isDefault": True}]}
        if method == "usage/read":
            return usage_result() if self.home.subscription() else {}
        if method == "usage/refresh":
            if not self.home.subscription():
                raise LookupError("not signed in with a subscription")
            return usage_result()
        if method == "account/read":
            lane = "subscription" if self.home.subscription() else ("apiKey" if self.home.signed_in() else None)
            return {"state": "loggedIn" if lane else "loggedOut", "lane": lane}
        if method == "session/list":
            conn = self.home.connect()
            try:
                ids = [row[0] for row in conn.execute("SELECT session_id FROM sessions ORDER BY updated_at_us DESC")]
            finally:
                conn.close()
            return {"sessions": [Session.find(self.home, one).view() for one in ids]}  # type: ignore[union-attr]
        if method == "session/read":
            session = Session.find(self.home, params.get("sessionId", ""))
            if session is None:
                raise LookupError(f"no session {params.get('sessionId')!r}")
            return {"session": session.view(), "items": self.items(session)}
        if request_id is not None:
            self.refuse(request_id, -32601, f"{method} isn't part of the lab stand-in")
        return None

    def items(self, session: Session) -> list[dict]:
        out = []
        for record in session.records():
            kind = {"user_message": "userMessage", "agent_message": "agentMessage"}.get(record.get("type"))
            if kind:
                out.append({"itemId": f"{record['turnId']}-{kind}", "kind": kind, "turnId": record["turnId"],
                            "status": "completed", "text": record.get("text", "")})
        return out

    def turn(self, session: Session, turn_id: str, text: str) -> None:
        sid = session.id
        self.event("turn/started", {"sessionId": sid, "turnId": turn_id, "commandId": turn_id})
        self.event("item/completed", {"sessionId": sid, "item": {
            "itemId": uuid7(), "kind": "userMessage", "turnId": turn_id, "revision": 1, "status": "completed",
            "recordedAt": iso(common.now_ms()), "text": text, "commandId": turn_id}})
        if not self.home.signed_in():
            session.record_turn(turn_id, text, None, "failed")
            self.event("turn/completed", {"sessionId": sid, "turnId": turn_id, "terminal": "failed", "error": {
                "kind": "authRequired", "message": "Not signed in: run `muse login` (lab stand-in)"}})
            return
        answer = common.answer_for(text)
        item = uuid7()
        self.event("item/started", {"sessionId": sid, "item": {"itemId": item, "kind": "agentMessage", "turnId": turn_id,
                                                               "revision": 1, "status": "inProgress", "text": ""}})
        self.event("item/delta", {"sessionId": sid, "itemId": item, "field": "text", "delta": answer})
        self.event("item/completed", {"sessionId": sid, "item": {
            "itemId": item, "kind": "agentMessage", "turnId": turn_id, "revision": 2, "status": "completed",
            "recordedAt": iso(common.now_ms()), "text": answer}})
        session.record_turn(turn_id, text, answer, "completed")
        self.event("turn/completed", {"sessionId": sid, "turnId": turn_id, "terminal": "completed", "durationMs": 40,
                                      "timeToFirstTokenMs": 10, "usage": {"inputTokens": 120, "outputTokens": 8,
                                                                          "cachedTokens": 40, "reasoningTokens": 0}})
        if self.home.subscription():
            self.send({"method": "usage/changed", "params": usage_result(), "emittedAtMs": common.now_ms()})


# ---- the command line -----------------------------------------------------

VALUE_OPTIONS = {"--agents", "--provider", "--preset", "--model", "--contributor-mode", "-c", "--config",
                 "--reasoning-effort", "--effort", "--base-url", "--image", "--workspace", "--worktree-base",
                 "--worktree-existing", "--approval-mode", "--permission-profile", "--approval-judge"}


def main(argv: list[str]) -> int:
    common.no_network(TOOL)
    common.log_call(TOOL, argv)
    if argv[:1] == ["--selftest"]:
        return selftest()
    words = list(argv)
    model, positional = MODEL, []
    while words:
        word = words.pop(0)
        if word == "--":
            positional.append(("prompt", " ".join(words)))
            break
        if word in ("-V", "--version"):
            print(f"Muse Code {VERSION} ({BUILD})")
            return 0
        if word in ("-h", "--help"):
            print(HELP)
            return 0
        if word in VALUE_OPTIONS and words:
            value = words.pop(0)
            if word == "--model":
                model = value
        elif word.startswith("-"):
            continue
        else:
            positional.append(("word", word))
            positional += [("word", rest) for rest in words]
            break
    first = positional[0] if positional else None
    command = first[1] if first and first[0] == "word" else None
    rest = [value for _, value in positional[1:]]
    if command == "login":
        return login()
    if command == "logout":
        return logout()
    if command == "serve":
        return Server(Home()).serve()
    if command == "usage":
        return usage_command(rest)
    if command == "resume":
        if not rest:
            common.fail(TOOL, "resume takes a session id", 2)
        return terminal(rest[0], None, model)
    if command is not None and command in {"config", "export", "trace", "model-profile", "skills", "voice", "plugins",
                                           "sandbox", "schema", "session-message", "mcp", "auth", "init"}:
        common.fail(TOOL, f"`muse {command}` isn't part of the lab stand-in", 2)
    prompt = " ".join(value for _, value in positional) if positional else None
    return terminal(None, prompt, model)


# ---- selftest -------------------------------------------------------------

def static_check() -> list[str]:
    root = HERE.parents[2]
    src = root / "crates/juggler-adapters/src"
    if not src.is_dir():
        print("muse selftest: no Rust source beside this copy; the static check is skipped")
        return []
    needles = {
        "muse.rs": [f'VERSION_FILE: &str = "{VERSION_FILE}"', f'FILE_BACKEND_PROVEN: &[&str] = &["{BUILD}"]',
                    f'ENV_CREDENTIAL_BACKEND: &str = "{ENV_BACKEND}"', f'FILE_BACKEND: &str = "{FILE_BACKEND}"',
                    f'ENV_NO_UPDATE: &str = "{ENV_NO_UPDATE}"', "(ENV_NO_UPDATE, \"1\")",
                    f'ENV_LAUNCHER_AUTH: &str = "{ENV_LAUNCHER_AUTH}"', f'"{ENV_BIN}"',
                    'CONFIG_ROOT: &str = ".config"', 'DATA_ROOT: &str = ".local/share"', 'CACHE_ROOT: &str = ".cache"',
                    'STATE_ROOT: &str = ".local/state"', f'MUSE_DIR: &str = "{MUSE_DIR}"',
                    'args: vec!["login".into()]', 'args.extend(["resume".into(), session_id.clone()])',
                    'args.extend(["--model".into(), model.clone()])', 'args.extend(["--".into(), prompt.clone()])']
                   + [f'"{root_name}"' for root_name in ROOTS],
        "sign_in.rs": [f'MUSE_AUTH: &str = "{AUTH_FILE}"'],
        "muse_sessions.rs": [f'INDEX_FILE: &str = "{INDEX_FILE}"', f'SCHEMA_VERSION: &str = "{SCHEMA_VERSION}"',
                             "schema_meta WHERE key = 'schema_version'", "msp_kind = 'root'"]
                            + [f'"{column}"' for column in COLUMNS],
        "muse_serve/mod.rs": ['SERVER_ARGS: &[&str] = &["serve"]', "data_of(&account.state_dir)"],
        "muse_serve/codec.rs": ['"initialize"', '"initialized"', '"session/start"', '"session/resume"', '"turn/start"',
                                '"usage/read"', '"usage/refresh"', '"museHome"', '"sessionId"', '"turn/started"',
                                '"turn/completed"', '"item/delta"', '"item/completed"', '"agentMessage"', '"terminal"',
                                'Some("completed")', '"activeTurnId"', '"workspaceRoot"', '"commandId"', '"excludeItems"',
                                '"turnId"', '"inputTokens"', '"usage/changed"', '"experimentalApi"'],
        "muse_usage.rs": ['&["usage", "--json"]', '"observedAtMs"', '"usedPercent"', '"resetsAtMs"', '"window"',
                          '"weekly"', '"tierName"', '"subscriptionActive"', '"windowDurationMins"',
                          f"FIVE_HOURS_MINS: u64 = {FIVE_HOURS_MINS}"],
    }
    missing = []
    for name, wanted in needles.items():
        text = (src / name).read_text()
        missing += [f"{name}: {needle}" for needle in wanted if needle not in text]
    return missing


class Client:
    """A `muse serve` driven as the adapter drives it."""

    def __init__(self, me: str, env: dict, cwd: Path) -> None:
        self.proc = subprocess.Popen(["/usr/bin/python3", "-I", me, "serve"], env=env, cwd=cwd, text=True,
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.next_id = 0
        self.seen: list[dict] = []

    def send(self, method: str, params: dict | None = None, notify: bool = False) -> int:
        frame: dict = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            frame["params"] = params
        if not notify:
            self.next_id += 1
            frame["id"] = self.next_id
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(frame) + "\n")
        self.proc.stdin.flush()
        return self.next_id

    def until(self, test) -> dict:  # type: ignore[no-untyped-def]
        assert self.proc.stdout is not None
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            line = self.proc.stdout.readline()
            if not line:
                raise AssertionError(f"the server ended: {self.proc.stderr.read() if self.proc.stderr else ''}")
            frame = json.loads(line)
            assert frame.get("jsonrpc") == "2.0", frame
            self.seen.append(frame)
            if test(frame):
                return frame
        raise AssertionError("no answer in time")

    def ask(self, method: str, params: dict | None = None) -> dict:
        sent = self.send(method, params)
        return self.until(lambda frame: frame.get("id") == sent and "method" not in frame)

    def close(self) -> int:
        assert self.proc.stdin is not None
        self.proc.stdin.close()
        try:
            return self.proc.wait(timeout=10)
        finally:
            for stream in (self.proc.stdout, self.proc.stderr):
                if stream:
                    stream.close()


def reading_of(result: dict) -> list[tuple[str, float]]:
    """The windows the adapter's `muse_usage::reading_of` would take."""
    assert result.get("subscriptionActive") is not False
    usage = result.get("usage")
    if usage is None:
        return []
    assert isinstance(usage.get("observedAtMs"), int)
    out = []
    window = usage.get("window")
    if window and window.get("windowDurationMins") == FIVE_HOURS_MINS:
        out.append(("five_hour", float(window["usedPercent"])))
    if usage.get("weekly"):
        out.append(("weekly", float(usage["weekly"]["usedPercent"])))
    return out


def read_only(index: Path) -> sqlite3.Connection:
    """The index opened as the adapter opens it: joining the write-ahead
    log while one is there, else `immutable` (nothing written beside it)."""
    flag = "mode=ro" if Path(f"{index}-wal").exists() else "immutable=1"
    return sqlite3.connect(f"file:{index}?{flag}", uri=True)


def listed(index: Path) -> list[tuple]:
    """The sessions the adapter's `muse_sessions::sessions` lists."""
    conn = read_only(index)
    try:
        version = conn.execute("SELECT value FROM schema_meta WHERE key = 'schema_version'").fetchone()
        assert version == (SCHEMA_VERSION,), version
        have = {row[0] for row in conn.execute("SELECT name FROM pragma_table_info('sessions')")}
        assert set(COLUMNS) <= have, set(COLUMNS) - have
        return conn.execute(
            "SELECT session_id, COALESCE(NULLIF(title, ''), first_user_prompt, ''), COALESCE(workspace_root, ''),"
            " COALESCE(created_at_us, 0), COALESCE(updated_at_us, created_at_us, 0) FROM sessions"
            " WHERE (msp_kind IS NULL OR msp_kind = 'root') AND (msp_parent_session_id IS NULL OR msp_parent_session_id = '')"
            " ORDER BY updated_at_us DESC").fetchall()
    finally:
        conn.close()


def selftest() -> int:
    failures: list[str] = []

    def check(ok: bool, what: str) -> None:
        if not ok:
            failures.append(what)

    common.selftest_common()
    failures += [f"the Rust source changed: {line}" for line in static_check()]
    with common.selftest_lab(TOOL) as lab:
        # The launcher and its record, as install() lays them out.
        dest = lab / "standins"
        dest.mkdir()
        for name in (Path(__file__).name, "standin_common.py"):
            (dest / name).write_bytes((HERE / name).read_bytes())
        wrapper = Path(install(dest)[ENV_BIN])
        record = (wrapper.parent / VERSION_FILE).read_text().splitlines()[0].strip()
        version, _, build = record.partition("-R")
        check(record == BUILD and re.fullmatch(r"\d+\.\d+\.\d+", version) is not None and build != "",
              f"{VERSION_FILE}: {record!r}")
        done = subprocess.run([str(wrapper), "--version"], capture_output=True, text=True, timeout=30,
                              env={"PATH": "/usr/bin:/bin", ENV_NO_UPDATE: "1"})
        found = re.search(r"\((\d+\.\d+\.\d+-R[0-9.]+)\)", done.stdout)
        check(done.returncode == 0 and found is not None and found.group(1) == BUILD
              and re.search(r"\d+\.\d+\.\d+", done.stdout).group(0) == VERSION, f"--version: {done.stdout!r}")

        app = lab / "apps" / "muse-a"
        project = lab / "projects" / "shop"
        project.mkdir(parents=True)
        env = {"PATH": "/usr/bin:/bin", "AIJ_LAB_STANDIN_WAIT": "0", "HOME": str(app),
               "XDG_CONFIG_HOME": str(app / ".config"), "XDG_DATA_HOME": str(app / ".local/share"),
               "XDG_CACHE_HOME": str(app / ".cache"), "XDG_STATE_HOME": str(app / ".local/state"),
               ENV_NO_UPDATE: "1", ENV_BACKEND: FILE_BACKEND}
        auth = app / ".config" / MUSE_DIR / AUTH_FILE
        index = app / ".local/share" / MUSE_DIR / INDEX_FILE
        me = str(Path(__file__).resolve())

        def run(args: list[str], extra: dict | None = None, text: str = "", drop: tuple = ()) -> subprocess.CompletedProcess:
            use = {k: v for k, v in {**env, **(extra or {})}.items() if k not in drop}
            return subprocess.run(["/usr/bin/python3", "-I", me, *args], input=text, capture_output=True, text=True,
                                  env=use, cwd=project, timeout=60)

        # Never outside a lab, never without the file backend.
        check(run(["login"], {"HOME": "/Users/someone"}).returncode == 3, "a real HOME was accepted")
        check(run(["login"], {"XDG_CONFIG_HOME": "/Users/someone/.config"}).returncode == 3, "a real config root")
        for backend in (None, "FILE", "keychain", "file "):
            done = run(["login"], {ENV_BACKEND: backend} if backend is not None else None,
                       drop=(ENV_BACKEND,) if backend is None else ())
            check(done.returncode == 5 and "Keychain" in done.stderr and not auth.exists(),
                  f"a {backend!r} backend wasn't refused: {done.returncode} {done.stderr!r}")
        check(run(["serve"], drop=(ENV_BACKEND,)).returncode == 5, "serve without the file backend")
        check(run(["usage", "--json"], drop=(ENV_BACKEND,)).returncode == 5, "usage without the file backend")
        check(run(["login"], {ENV_LAUNCHER_AUTH: "/Users/someone/auth"}).returncode == 3, "MUSE_AUTH_PATH accepted")
        check(run(["login"], drop=(ENV_NO_UPDATE,)).returncode == 3, "a run that would update was accepted")
        # Not signed in: the terminal refuses; a driven turn fails as authRequired.
        check(run([], text="hi\n").returncode == 1, "an unsigned terminal session started")
        client = Client(me, env, project)
        try:
            client.ask("initialize", {"clientInfo": {"name": "ai_juggler", "title": "AI Juggler", "version": "0"}})
            client.send("initialized", notify=True)
            opened = client.ask("session/start", {"commandId": uuid7(), "workspaceRoot": str(project)})
            unsigned = opened["result"]["session"]["sessionId"]
            client.ask("turn/start", {"sessionId": unsigned, "commandId": uuid7(), "input": [{"type": "text", "text": "hi"}]})
            end = client.until(lambda f: f.get("method") == "turn/completed")
            check(end["params"]["terminal"] == "failed" and end["params"]["error"]["kind"] == "authRequired",
                  f"an unsigned turn: {end}")
        finally:
            client.close()
        # Sign-in: the file backend's auth.json, in this folder.
        done = run(["login"])
        check(done.returncode == 0 and common.FAKE_HOST in done.stdout and auth.is_file(), f"login: {done.stdout!r}")
        if auth.is_file():
            body = json.loads(auth.read_text())
            check(body["providers"]["meta"]["access_token"] == common.NOT_A_TOKEN
                  and oct(auth.stat().st_mode & 0o777) == "0o600", "auth.json's contents or mode")
        # A driven session: initialize names this folder, a turn is answered and ends.
        client = Client(me, env, project)
        try:
            init = client.ask("initialize", {"clientInfo": {"name": "ai_juggler", "title": "AI Juggler", "version": "0"}})
            check(os.path.realpath(init["result"]["museHome"]) == os.path.realpath(app / ".local/share/muse"),
                  f"museHome: {init}")
            client.send("initialized", notify=True)
            bad = client.ask("session/start", {"commandId": "not-a-uuid", "workspaceRoot": str(project)})
            check("error" in bad, "a commandId that isn't a UUIDv7 was taken")
            opened = client.ask("session/start", {"commandId": uuid7(), "workspaceRoot": str(project)})
            session = opened["result"]["session"]
            sid = session["sessionId"]
            check(UUID7.match(sid) is not None and session["activeTurnId"] is None
                  and any(f.get("method") == "session/started" for f in client.seen), f"session/start: {opened}")
            sent = client.ask("turn/start", {"sessionId": sid, "commandId": uuid7(),
                                             "input": [{"type": "text", "text": "hello"}]})
            check(sent["result"]["status"] == "accepted", f"turn/start: {sent}")
            end = client.until(lambda f: f.get("method") == "turn/completed")
            text = "".join(f["params"]["delta"] for f in client.seen if f.get("method") == "item/delta"
                           and f["params"]["field"] == "text" and f["params"]["sessionId"] == sid)
            check(end["params"]["terminal"] == "completed" and text == "lab" and end["params"]["usage"]["inputTokens"] > 0,
                  f"the turn: {end} {text!r}")
            check(all(f["params"].get("sessionId") == sid for f in client.seen
                      if f.get("method", "").startswith(("turn/", "item/"))), "an event for another session")
            models = client.ask("model/list", {"sessionId": sid})
            check(models["result"]["models"][0]["isDefault"] is True, "model/list")
            seen = client.ask("usage/read", {})
            check(reading_of(seen["result"]) == [("five_hour", 12.0), ("weekly", 31.0)], f"usage/read: {seen}")
            check("error" in client.ask("usage/refresh", {}), "usage/refresh without the experimental API")
            client.ask("turn/start", {"sessionId": sid, "commandId": uuid7(), "input": [{"type": "text", "text": "x"}]})
            client.until(lambda f: f.get("method") == "turn/completed")
        finally:
            check(client.close() == 0, "serve didn't end when its stdin closed")
        rows = listed(index)
        mine = [row for row in rows if row[0] == sid]
        check(len(mine) == 1 and mine[0][1] == "hello" and os.path.realpath(mine[0][2]) == os.path.realpath(project)
              and mine[0][3] > 0 and mine[0][4] >= mine[0][3], f"the index: {rows}")
        check(not Path(f"{index}-wal").exists() or Path(f"{index}-wal").stat().st_size == 0,
              "a write-ahead log left behind (the adapter reads a closed index immutable)")
        # The terminal view after the driver lets go: earlier turns, then a new one.
        done = run(["resume", sid], text="again\n/exit\n")
        answers = [line for line in done.stdout.splitlines() if line.removeprefix("> ").strip() == "lab"]
        check(done.returncode == 0 and "> hello\nlab" in done.stdout and "> x\nlab" in done.stdout and len(answers) == 3,
              f"resume: {done.stdout!r}")
        # A driven resume continues the same session.
        client = Client(me, env, project)
        try:
            client.ask("initialize", {"clientInfo": {"name": "ai_juggler", "title": "AI Juggler", "version": "0"},
                                      "capabilities": {"experimentalApi": True}})
            client.send("initialized", notify=True)
            resumed = client.ask("session/resume", {"commandId": uuid7(), "sessionId": sid, "excludeItems": True})
            check(resumed["result"]["session"]["turnCount"] == 3 and "items" not in resumed["result"], f"resume: {resumed}")
            read = client.ask("session/read", {"sessionId": sid})
            check([item["text"] for item in read["result"]["items"]][:2] == ["hello", "lab"], f"session/read: {read}")
            client.ask("turn/start", {"sessionId": sid, "commandId": uuid7(), "input": [{"type": "text", "text": "more"}]})
            client.until(lambda f: f.get("method") == "turn/completed")
            fresh = client.ask("usage/refresh", {})
            check(fresh["result"]["tierName"] == TIER_NAME and len(reading_of(fresh["result"])) == 2, f"refresh: {fresh}")
        finally:
            client.close()
        rows = [row for row in listed(index) if row[0] == sid]
        conn = read_only(index)
        count = conn.execute("SELECT prompt_count FROM sessions WHERE session_id = ?", (sid,)).fetchone()[0]
        conn.close()
        check(len(rows) == 1 and count == 4, f"the resumed session's row: {rows} {count}")
        # A new terminal session and a first message.
        before = len(listed(index))
        done = run(["--model", "muse-lab-1", "--", "first words"], text="/exit\n")
        check(done.returncode == 0 and "> first words\nlab" in done.stdout and len(listed(index)) == before + 1,
              f"a primed session: {done.stdout!r}")
        # Usage, Muse's own command (the adapter's fallback).
        done = run(["usage", "--json"])
        check(done.returncode == 0 and reading_of(json.loads(done.stdout)) == [("five_hour", 12.0), ("weekly", 31.0)],
              f"usage --json: {done.stdout!r}")
        check(run(["logout"]).returncode == 0 and not auth.exists(), "logout")
        check(run(["usage", "--json"]).returncode == 1, "usage without a sign-in")
    for line in failures:
        print(f"FAILED: {line}", file=sys.stderr)
    print("muse stand-in: selftest " + ("FAILED" if failures else "passed"))
    return 1 if failures else 0


def install(dest: Path) -> dict[str, str]:
    wrapper = common.install_wrapper(dest, "muse", Path(__file__).name)
    record = wrapper.parent / VERSION_FILE
    common.require_lab(record, TOOL, "the launcher's record")
    record.write_text(BUILD + "\n")
    return {ENV_BIN: str(wrapper)}


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
