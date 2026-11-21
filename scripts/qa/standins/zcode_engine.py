#!/usr/bin/env python3
"""AJ-E1612 lab stand-in for ZCode's engine (`glm/zcode.cjs` 0.16.9).

`install(dest)` makes a stand-in app bundle, `<dest>/ZCode.app`, laid out
as the adapter reads one (zcode.rs `resolve()`; juggler-platform apps.rs):

    Contents/Info.plist                     made-up names, a lab bundle id
    Contents/MacOS/ZCode                    the "runtime": a sh script that
                                            refuses unless ELECTRON_RUN_AS_NODE=1,
                                            then `exec /usr/bin/python3 -I "$@"`
    Contents/Resources/glm/zcode.cjs        a copy of this file (the engine)
    Contents/Resources/config/provider/zcode-builtin.json
                                            a small made-up provider file

and returns {"AI_JUGGLER_ZCODE_ENGINE": <the engine>}. The bundle is never
opened with `open`; Dev runs it by path only, as it runs ZCode's.

What AI Juggler Dev runs, and what this answers (every run but `version`
and `--help` needs ZCODE_DATA_BASE_DIR, ZCODE_STORAGE_DIR and
ZCODE_SESSION_DB_PATH inside a lab, else it stops with exit 3):

- Discovery: `ELECTRON_RUN_AS_NODE=1 <app>/Contents/MacOS/ZCode <engine>
  version` prints `0.16.9` (no tree variables; nothing written).
- Sign in (login_spec, a pty): `<runtime> <engine> login` with the App's
  three tree variables and ZCODE_BUILTIN_PROVIDER_CONFIG_FILE. Shows a fake
  Z.AI page on example.invalid, waits AIJ_LAB_STANDIN_WAIT seconds, writes
  `<ZCODE_DATA_BASE_DIR>/.zcode/v2/credentials.json` (0600; the real keys,
  every value lab-not-a-token) and the App's provider choice
  (`v2/provider_config.json`, when missing). `logout` removes the sign-in.
- A session (driven_spec, no terminal): `<runtime> <engine> app-server
  --stdio`, one server per session, JSON-RPC lines without `jsonrpc` (the
  codec in zcode_appserver/codec.rs):
  - starts by creating the store at ZCODE_SESSION_DB_PATH (WAL, the
    `session`/`message`/`part`/`model_usage` tables of tests/zcode_fixture)
    and copying the built-in provider file into
    `<base>/.zcode/v2/runtime/provider/darwin/0.0.0-dev/endpoint-<hash>/`;
  - `session/create` -> `{session: {sessionId: "sess_<uuid>"}}` and an
    `interactive` session row in the workspace folder (with
    `importedHistory`, the caller's id and the turns imported as ZCode's
    own Claude Code import records them, no model call);
  - `session/resume` -> the stored session, or an error for an unknown id;
  - `session/subscribe` -> `{eventSeq, events: []}`; events flow after it;
  - `session/send` -> `{sessionId, accepted, stateRevision}`, the user's
    message stored, `turn.started`, then (signed in) the engine's own
    request `interaction/requestProviderRuntimeHeaders`; once Dev answers
    it, the answer is checked (`headersApplied`, no credential in
    `requestAuth`; recorded by key names only in
    `<tree>/cli/log/lab-standin.jsonl`), `part.delta` "lab", the
    assistant's message, a completed `model_usage` row (GLM-5.3, modest
    tokens) and `turn.completed` (`resultType: success`). Not signed in:
    `turn.failed` with a 401 (Dev says "couldn't use this App's sign-in");
  - a prompt holding `lab-limit-5h` or `lab-limit-week` is refused at the
    plan's limit instead: a `model_usage` error row (`rate_limited`, Z.ai's
    1308/1310 message, its reset written in UTC+8 three hours / six days
    ahead) and `turn.failed` (`ModelRateLimited`);
  - `session/stop` cancels a turn waiting for its headers
    (`turn.completed`, `resultType: cancelled`); anything else is refused;
  - ends when its stdin closes.
- Resume: the same server with `session/resume <id>`; new turns append to
  the stored session, so Dev's turns read shows the earlier turn first.
- Usage: nothing is run. Dev reads the store's `model_usage` rows (credits
  by Z.ai's formula, zcode_usage.rs) and a session's last refused call
  (a limit hit with its window and reset, zcode_limits.rs).
- `-p`, `tui`, `doctor`, a bare run: refused, as the app ships no terminal UI.

`--selftest` runs all of the above in a throwaway lab, through the
installed bundle's runtime too, and checks the names it uses against the
adapter's Rust source.

What a dry run on this stand-in cannot prove:
- that the real engine 0.16.9 takes these variables, or keeps its store,
  sign-in and provider copy where they are written here;
- the real `login` (Z.AI's OAuth page, its codes, its error words);
- that the real engine sends its headers request, or then uses the App's
  own sign-in when Dev answers it with none (UNCONFIRMED in the adapter);
- the real events' order, extra event types, tool asks and timings;
- Z.ai's limit refusals and the reset's time zone (UTC+8 here because the
  adapter reads it so: this only proves Dev shows what it reads);
- anything about the desktop app, its window or its plugins.
"""
from __future__ import annotations

import calendar
import hashlib
import importlib.util
import json
import os
import select
import shutil
import sqlite3
import subprocess
import sys
import time
import uuid
from pathlib import Path

TOOL = "zcode"
VERSION = "0.16.9"
# The engine's own build constant the provider copy is filed under.
BUILD = "0.0.0-dev"
MODEL = "GLM-5.3"
PROVIDER = "zai"

ENV_STORAGE = "ZCODE_STORAGE_DIR"
ENV_BASE = "ZCODE_DATA_BASE_DIR"
ENV_DB = "ZCODE_SESSION_DB_PATH"
ENV_BUILTIN = "ZCODE_BUILTIN_PROVIDER_CONFIG_FILE"
ENV_RUN_AS_NODE = "ELECTRON_RUN_AS_NODE"
ENV_ENGINE = "AI_JUGGLER_ZCODE_ENGINE"

ENGINE_IN_APP = "glm/zcode.cjs"
BUILTIN_IN_APP = "config/provider/zcode-builtin.json"
RUNTIME_NAME = "ZCode"
SIGN_IN = "v2/credentials.json"
LIMIT_5H = "lab-limit-5h"
LIMIT_WEEK = "lab-limit-week"
UTC8_S = 8 * 3600

# tests/zcode_fixture/mod.rs: the columns engine 0.16.9's own schema has
# for the tables the adapter reads.
SCHEMA = """
CREATE TABLE IF NOT EXISTS session (
    id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, directory TEXT NOT NULL,
    path TEXT, title TEXT, version TEXT, time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL, time_compacting INTEGER, time_archived INTEGER,
    task_type TEXT, trace_id TEXT);
CREATE TABLE IF NOT EXISTS model_usage (
    id TEXT PRIMARY KEY, session_id TEXT, provider_id TEXT, model_id TEXT NOT NULL,
    status TEXT, started_at INTEGER NOT NULL, input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0, reasoning_tokens INTEGER NOT NULL DEFAULT 0,
    cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_input_tokens INTEGER NOT NULL DEFAULT 0,
    computed_total_tokens INTEGER, duration_ms INTEGER, error_type TEXT,
    error_code TEXT, error_message TEXT);
CREATE TABLE IF NOT EXISTS message (
    id TEXT PRIMARY KEY, session_id TEXT NOT NULL, time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL, data TEXT NOT NULL, sequence INTEGER);
CREATE TABLE IF NOT EXISTS part (
    id TEXT PRIMARY KEY, message_id TEXT NOT NULL, session_id TEXT NOT NULL,
    time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL,
    sequence INTEGER);
"""

HELP = f"""zcode {VERSION} (AI Juggler lab stand-in)

Usage: zcode [command] [options]

Commands:
  app-server --stdio   the desktop app's server (JSON-RPC over stdio)
  login                sign in to Z.AI (a fake page in the lab)
  logout               sign out
  version              print the version
"""


def _load_common():
    """standin_common.py beside this file, else beside the app bundle it
    lies in (the installed engine is `<standins>/ZCode.app/…/zcode.cjs`)."""
    me = Path(__file__).resolve()
    candidates = [me.parent / "standin_common.py"]
    candidates += [up.parent / "standin_common.py" for up in me.parents if up.suffix == ".app"][:1]
    for path in candidates:
        if path.is_file():
            spec = importlib.util.spec_from_file_location("standin_common", path)
            module = importlib.util.module_from_spec(spec)
            assert spec.loader is not None
            spec.loader.exec_module(module)
            return module
    sys.stderr.write("zcode (lab stand-in): standin_common.py isn't beside it\n")
    raise SystemExit(3)


common = _load_common()


# --------------------------------------------------------------------------
# install
# --------------------------------------------------------------------------

INFO_PLIST = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>ZCode lab stand-in</string>
  <key>CFBundleIdentifier</key>
  <string>invalid.example.aij-lab.zcode-standin</string>
  <key>CFBundleShortVersionString</key>
  <string>3.15.3</string>
  <key>CFBundleVersion</key>
  <string>3.15.3</string>
</dict>
</plist>
"""

RUNTIME_SCRIPT = """#!/bin/sh
# AJ-E1612 lab stand-in for ZCode's runtime: runs the engine file it is
# given with the system Python, as Node.js mode would. It has no window.
if [ "$ELECTRON_RUN_AS_NODE" != "1" ]; then
  echo "ZCode (lab stand-in): runs only as the engine's runtime (ELECTRON_RUN_AS_NODE=1); it has no window" >&2
  exit 2
fi
exec /usr/bin/python3 -I "$@"
"""

BUILTIN = {
    "schemaVersion": 1,
    "revision": 1,
    "labStandIn": True,
    "config": {
        "providerConfigRules": {
            "templateRules": [{"templateId": "lab-api", "providerId": PROVIDER,
                               "baseUrl": f"https://api.{common.FAKE_HOST}/anthropic"}],
            "providerRules": [],
        },
        "modelConfigRules": {
            "modelRules": [{"modelId": MODEL, "providerId": PROVIDER}],
            "modelApiRules": [], "providerSiteRules": [], "templateModelRules": [],
            "builtinProviderModelRules": [],
        },
    },
}


def install(dest: Path) -> dict[str, str]:
    """The stand-in app bundle in `dest` (a lab's stand-ins folder, which
    holds standin_common.py); the adapter's override."""
    common.require_lab(dest, TOOL, "the stand-ins' folder")
    contents = Path(dest) / "ZCode.app" / "Contents"
    (contents / "MacOS").mkdir(parents=True, exist_ok=True)
    (contents / "Info.plist").write_text(INFO_PLIST)
    runtime = contents / "MacOS" / RUNTIME_NAME
    runtime.write_text(RUNTIME_SCRIPT)
    runtime.chmod(0o755)
    engine = contents / "Resources" / ENGINE_IN_APP
    engine.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(Path(__file__).resolve(), engine)
    engine.chmod(0o755)
    builtin = contents / "Resources" / BUILTIN_IN_APP
    builtin.parent.mkdir(parents=True, exist_ok=True)
    builtin.write_text(json.dumps(BUILTIN, indent=2) + "\n")
    return {ENV_ENGINE: str(engine)}


# --------------------------------------------------------------------------
# the engine's files
# --------------------------------------------------------------------------

class Paths:
    """The App's tree as the adapter pins it."""

    def __init__(self) -> None:
        self.storage = common.require_lab(os.environ.get(ENV_STORAGE), TOOL, ENV_STORAGE)
        self.base = common.require_lab(os.environ.get(ENV_BASE), TOOL, ENV_BASE)
        self.db = common.require_lab(os.environ.get(ENV_DB), TOOL, ENV_DB)
        self.sign_in = self.base / ".zcode" / SIGN_IN
        self.personal = self.base / ".zcode" / "v2" / "provider_config.json"
        self.log = self.storage / "cli" / "log" / "lab-standin.jsonl"

    def signed_in(self) -> bool:
        try:
            data = json.loads(self.sign_in.read_text())
        except (OSError, ValueError):
            return False
        return bool(data.get("oauth:zai:access_token"))

    def model(self) -> str:
        try:
            chosen = json.loads(self.personal.read_text()).get("modelSelection", {}).get("modelId")
        except (OSError, ValueError, AttributeError):
            chosen = None
        return chosen if isinstance(chosen, str) and chosen else MODEL

    def note(self, what: str, **fields) -> None:
        line = json.dumps(dict({"at": common.now_ms(), "what": what}, **fields), sort_keys=True)
        common.append_line(self.log, line, TOOL)


def provider_runtime(paths: Paths) -> Path | None:
    """`prepareCliProviderRuntimeEnv`: the built-in provider file, copied
    into the App's tree; None when it isn't given or isn't one."""
    given = os.environ.get(ENV_BUILTIN)
    if not given:
        return None
    try:
        builtin = json.loads(Path(given).read_text())
    except (OSError, ValueError):
        return None
    if not isinstance(builtin, dict) or "schemaVersion" not in builtin:
        return None
    endpoint = hashlib.sha256(given.encode()).hexdigest()[:12]
    copy = paths.base / ".zcode" / "v2" / "runtime" / "provider" / "darwin" / BUILD / f"endpoint-{endpoint}" / "zcode-builtin.json"
    common.write_json(copy, builtin, TOOL, 0o644)
    return copy


class Store:
    """The session store, held open while the engine runs (WAL, as the
    engine keeps it: Dev joins the log while it runs)."""

    def __init__(self, path: Path) -> None:
        common.require_lab(path, TOOL, "the session store")
        path.parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(str(path), isolation_level=None)
        self.conn.execute("PRAGMA journal_mode = WAL")
        self.conn.executescript(SCHEMA)

    def close(self) -> None:
        self.conn.close()

    def session(self, session_id: str):
        return self.conn.execute("SELECT id, title, directory FROM session WHERE id = ?",
                                 (session_id,)).fetchone()

    def add_session(self, session_id: str, directory: str, title: str | None) -> None:
        now = common.now_ms()
        self.conn.execute(
            "INSERT INTO session (id, project_id, parent_id, directory, title, version, time_created,"
            " time_updated, task_type) VALUES (?, ?, NULL, ?, ?, ?, ?, ?, 'interactive')",
            (session_id, "prj_" + hashlib.sha256(directory.encode()).hexdigest()[:12], directory, title,
             VERSION, now, now))

    def touch(self, session_id: str, title: str | None = None) -> None:
        self.conn.execute("UPDATE session SET time_updated = ?, title = COALESCE(title, ?) WHERE id = ?",
                          (common.now_ms(), title, session_id))

    def message(self, session_id: str, role: str, text: str, extra: dict | None = None) -> str:
        at = common.now_ms()
        message_id = "msg_" + uuid.uuid4().hex
        sequence = self.conn.execute("SELECT COUNT(*) FROM message WHERE session_id = ?",
                                     (session_id,)).fetchone()[0]
        data = dict({"id": message_id, "sessionID": session_id, "role": role, "time": {"created": at}},
                    **(extra or {}))
        part = {"id": "prt_" + uuid.uuid4().hex, "messageID": message_id, "sessionID": session_id,
                "type": "text", "text": text}
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO message (id, session_id, time_created, time_updated, data, sequence)"
                          " VALUES (?, ?, ?, ?, ?, ?)", (message_id, session_id, at, at, json.dumps(data), sequence))
        self.conn.execute("INSERT INTO part (id, message_id, session_id, time_created, time_updated, data, sequence)"
                          " VALUES (?, ?, ?, ?, ?, ?, 0)", (part["id"], message_id, session_id, at, at, json.dumps(part)))
        self.conn.execute("COMMIT")
        return message_id

    def usage(self, session_id: str, model: str, started: int, *, tokens: tuple[int, int, int] = (0, 0, 0),
              error: tuple[str, str] | None = None) -> None:
        input_tokens, output_tokens, cache_read = tokens
        self.conn.execute(
            "INSERT INTO model_usage (id, session_id, provider_id, model_id, status, started_at, input_tokens,"
            " output_tokens, cache_read_input_tokens, computed_total_tokens, duration_ms, error_type,"
            " error_code, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ("mu_" + uuid.uuid4().hex, session_id, PROVIDER, model, "error" if error else "completed", started,
             input_tokens, output_tokens, cache_read, input_tokens + output_tokens, common.now_ms() - started,
             error[0] if error else None, "" if error else None, error[1] if error else None))


def limit_message(window: str, now_s: float) -> str:
    """Z.ai's refusal at the plan's limit (zcode_limits.rs MESSAGES), its
    reset written in UTC+8 as Z.ai writes it; the request id made up."""
    code, words, ahead = (("1308", "Usage limit reached for 5 hour.", 3 * 3600) if window == "5h"
                          else ("1310", "Weekly/Monthly Limit Exhausted.", 6 * 86400))
    reset = time.strftime("%Y-%m-%d %H:%M:%S", time.gmtime(int(now_s) + ahead + UTC8_S))
    return f"[{code}][{words} Your limit will reset at {reset}][{os.urandom(15).hex()}]"


# --------------------------------------------------------------------------
# the app-server
# --------------------------------------------------------------------------

class AppServer:
    def __init__(self, paths: Paths) -> None:
        self.paths = paths
        self.store = Store(paths.db)
        self.builtin = provider_runtime(paths)
        self.session: str | None = None
        self.workspace = ""
        self.subscribed = False
        self.seq = 0
        self.revision = 0
        self.server_ids = 0
        self.turn: dict | None = None

    # -- the wire ----------------------------------------------------------
    @staticmethod
    def write(message: dict) -> None:
        sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
        sys.stdout.flush()

    def reply(self, rid, result) -> None:
        self.write({"id": rid, "result": result})

    def refuse(self, rid, message: str, code: int = -32000) -> None:
        self.write({"id": rid, "error": {"code": code, "message": message}})

    def event(self, kind: str, payload: dict) -> None:
        if not (self.subscribed and self.session):
            return
        self.seq += 1
        self.write({"method": "session/event", "params": {
            "eventId": common.new_id("evt_"), "sessionId": self.session,
            "turnId": self.turn["id"] if self.turn else None, "seq": self.seq,
            "timestamp": common.now_ms(), "type": kind, "payload": payload}})

    def serve(self) -> int:
        try:
            while True:
                line = sys.stdin.readline()
                if not line:
                    return 0
                line = line.strip()
                if not line:
                    continue
                try:
                    message = json.loads(line)
                except ValueError:
                    continue
                if isinstance(message, dict):
                    self.take(message)
        finally:
            self.store.close()

    def take(self, message: dict) -> None:
        method = message.get("method")
        if method is None:
            if self.turn and message.get("id") == self.turn.get("headers"):
                self.headers_answered(message)
            return
        if "id" not in message:
            return  # a notification: none is expected
        rid, params = message["id"], message.get("params") or {}
        handler = {
            "session/create": self.create, "session/resume": self.resume,
            "session/subscribe": self.subscribe, "session/send": self.send, "session/stop": self.stop,
        }.get(method)
        if handler is None:
            self.refuse(rid, f"Method not found: {method}", -32601)
        else:
            handler(rid, params)

    # -- the session -------------------------------------------------------
    def opened(self, rid, session_id: str) -> None:
        self.session, self.subscribed, self.seq = session_id, False, 0
        row = self.store.session(session_id)
        self.reply(rid, {"session": {"sessionId": session_id, "title": row[1] if row else None,
                                     "workspace": {"workspacePath": self.workspace, "workspaceKey": self.workspace}}})

    def create(self, rid, params: dict) -> None:
        if self.builtin is None:
            self.refuse(rid, "Built-in provider config not found: set ZCODE_BUILTIN_PROVIDER_CONFIG_FILE")
            return
        self.workspace = str((params.get("workspace") or {}).get("workspacePath") or os.getcwd())
        history = params.get("importedHistory")
        session_id = params.get("sessionId") if history else None
        session_id = session_id or f"sess_{uuid.uuid4()}"
        if self.store.session(session_id):
            self.refuse(rid, f"Session already exists: {session_id}")
            return
        title = history.get("title") if isinstance(history, dict) else None
        self.store.add_session(session_id, self.workspace, title)
        if isinstance(history, dict):
            for turn in history.get("messages") or []:
                role, text = turn.get("role"), turn.get("content")
                if role == "user":
                    self.store.message(session_id, "user", text, {"metadata": {"migrationSource": "claudeCode"}})
                elif role == "assistant":
                    self.store.message(session_id, "assistant", text, {
                        "cost": 0, "tokens": {"input": 0, "output": 0, "reasoning": 0, "cache": {"read": 0, "write": 0}}})
        self.paths.note("session/create", session=session_id, imported=isinstance(history, dict))
        self.opened(rid, session_id)

    def resume(self, rid, params: dict) -> None:
        session_id = params.get("sessionId")
        row = self.store.session(session_id) if isinstance(session_id, str) else None
        if row is None:
            self.refuse(rid, f"Session not found: {session_id}")
            return
        self.workspace = str((params.get("workspace") or {}).get("workspacePath") or row[2])
        self.paths.note("session/resume", session=session_id)
        self.opened(rid, session_id)

    def subscribe(self, rid, params: dict) -> None:
        if not self.session or params.get("sessionId") != self.session:
            self.refuse(rid, "No such session to subscribe to")
            return
        self.subscribed = True
        self.reply(rid, {"eventSeq": self.seq, "events": []})

    def send(self, rid, params: dict) -> None:
        text = params.get("content")
        if not self.session or params.get("sessionId") != self.session:
            self.refuse(rid, "No such session")
            return
        if self.turn:
            self.refuse(rid, "A turn is already running")
            return
        if not isinstance(text, str) or not text.strip():
            self.refuse(rid, "Nothing to send", -32602)
            return
        self.revision += 1
        number = 1 + self.store.conn.execute(
            "SELECT COUNT(*) FROM message WHERE session_id = ? AND json_extract(data, '$.role') = 'user'",
            (self.session,)).fetchone()[0]
        self.reply(rid, {"sessionId": self.session, "accepted": True, "stateRevision": self.revision})
        self.store.message(self.session, "user", text)
        self.store.touch(self.session, text.strip()[:60])
        self.turn = {"id": common.new_id("turn_"), "text": text, "started": common.now_ms(), "headers": None}
        self.event("turn.started", {"turnNumber": number, "input": text})
        if not self.paths.signed_in():
            self.fail("AuthFailed", "401 Unauthorized: not signed in to Z.AI (run `zcode login`)")
            return
        self.server_ids += 1
        request = f"server-{self.server_ids}"
        self.turn["headers"] = request
        model = self.paths.model()
        self.write({"id": request, "method": "interaction/requestProviderRuntimeHeaders", "params": {
            "requestId": common.new_id("hdr_"), "sessionId": self.session, "providerId": PROVIDER,
            "modelSelection": {"providerId": PROVIDER, "modelId": model}, "reason": "model_request",
            "workspace": {"workspacePath": self.workspace, "workspaceKey": self.workspace}}})

    def headers_answered(self, message: dict) -> None:
        result = message.get("result")
        auth = result.get("requestAuth") if isinstance(result, dict) else None
        applied = isinstance(result, dict) and result.get("headersApplied") is True
        handed = sorted(auth) if isinstance(auth, dict) else []
        # Names only: a value is never written down.
        self.paths.note("headers answered", session=self.session, headersApplied=applied,
                        requestAuthKeys=handed, error="error" in message)
        if handed:
            sys.stderr.write("zcode (lab stand-in): AI Juggler handed credential fields in requestAuth: "
                             + ", ".join(handed) + "\n")
        if not applied:
            self.fail("ProviderHeadersFailed", "The client didn't apply the provider's runtime headers")
            return
        self.model_call()

    def model_call(self) -> None:
        assert self.turn and self.session
        text, started, model = self.turn["text"], self.turn["started"], self.paths.model()
        window = "5h" if LIMIT_5H in text else "week" if LIMIT_WEEK in text else None
        if window:
            refusal = limit_message(window, time.time())
            self.store.usage(self.session, model, started, error=("rate_limited", refusal))
            self.store.touch(self.session)
            self.fail("ModelRateLimited", refusal)
            return
        answer = common.answer_for(text)
        message_id = common.new_id("msg_")
        part_id = common.new_id("prt_")
        self.event("part.delta", {"messageId": message_id, "partId": part_id, "field": "text", "delta": answer})
        tokens = (1200, 12, 800)
        self.store.usage(self.session, model, started, tokens=tokens)
        self.store.message(self.session, "assistant", answer, {
            "modelID": model, "providerID": PROVIDER, "cost": 0,
            "tokens": {"input": tokens[0], "output": tokens[1], "reasoning": 0, "cache": {"read": tokens[2], "write": 0}},
            "time": {"created": started, "completed": common.now_ms()}})
        self.store.touch(self.session)
        self.end({"response": answer, "tokenCount": tokens[0] + tokens[1], "toolCallCount": 0,
                  "duration": common.now_ms() - started, "resultType": "success"})

    def fail(self, kind: str, message: str) -> None:
        self.event("turn.failed", {"turnPhase": "model", "error": {"type": kind, "message": message}})
        self.turn = None

    def end(self, payload: dict) -> None:
        self.event("turn.completed", payload)
        self.turn = None

    def stop(self, rid, params: dict) -> None:
        if not self.session or params.get("sessionId") != self.session:
            self.refuse(rid, "No such session")
            return
        self.reply(rid, {"sessionId": self.session, "stopped": self.turn is not None})
        if self.turn:
            self.end({"response": "", "tokenCount": 0, "toolCallCount": 0,
                      "duration": common.now_ms() - self.turn["started"], "resultType": "cancelled"})


# --------------------------------------------------------------------------
# the other commands
# --------------------------------------------------------------------------

def login(paths: Paths) -> int:
    provider_runtime(paths)
    print("Signing in to Z.AI…", flush=True)
    page = common.fake_sign_in_page(TOOL, "zai")
    common.write_json(paths.sign_in, {
        "oauth:zai:access_token": common.NOT_A_TOKEN,
        "zcodejwttoken": common.NOT_A_TOKEN,
        "oauth:zai:user_info": json.dumps({"id": "lab-user", "name": "Lab user"}),
        "oauth:active_provider": PROVIDER,
    }, TOOL)
    if not paths.personal.exists():
        common.write_json(paths.personal, {"modelSelection": {"providerId": PROVIDER, "modelId": MODEL}}, TOOL)
    paths.note("login", page=page)
    print("Logged in to Z.AI.", flush=True)
    return 0


def logout(paths: Paths) -> int:
    if paths.sign_in.exists():
        common.require_lab(paths.sign_in, TOOL, "the sign-in")
        paths.sign_in.unlink()
    print("Logged out.", flush=True)
    return 0


def main(argv: list[str]) -> int:
    common.no_network(TOOL)
    common.log_call(TOOL, argv)
    words = argv[1:]
    if words in (["version"], ["--version"], ["-v"]):
        print(VERSION, flush=True)
        return 0
    if not words or words[0] in ("-h", "--help", "help"):
        if not words:
            sys.stderr.write("Error: Cannot find package '@zcode/tui' (lab stand-in: ZCode's app ships no terminal UI)\n")
            return 1
        print(HELP, end="", flush=True)
        return 0
    paths = Paths()
    command = words[0]
    if command == "app-server":
        if "--stdio" not in words[1:]:
            sys.stderr.write("zcode (lab stand-in): only `app-server --stdio` is served\n")
            return 2
        return AppServer(paths).serve()
    if command == "login":
        return login(paths)
    if command == "logout":
        return logout(paths)
    sys.stderr.write(f"zcode (lab stand-in): `{' '.join(words)}` isn't something AI Juggler runs; "
                     "the app ships no terminal UI\n")
    return 2


# --------------------------------------------------------------------------
# selftest
# --------------------------------------------------------------------------

def repo() -> Path:
    """The repository this file lies in (the selftest runs from it)."""
    root = Path(__file__).resolve().parent.parent.parent.parent
    assert (root / "crates/juggler-adapters/src").is_dir(), "the selftest runs from the repository"
    return root


def static_check() -> None:
    """The names used here, still the adapter's (its Rust source)."""
    REPO = repo()
    src = REPO / "crates/juggler-adapters/src"
    expect = {
        "zcode.rs": [f'ENGINE_IN_APP: &str = "{ENGINE_IN_APP}"', f'RUNTIME_NAME: &str = "{RUNTIME_NAME}"',
                     f'BUILTIN_IN_APP: &str = "{BUILTIN_IN_APP}"', f'ENV_STORAGE_DIR: &str = "{ENV_STORAGE}"',
                     f'ENV_DATA_BASE_DIR: &str = "{ENV_BASE}"', f'ENV_SESSION_DB_PATH: &str = "{ENV_DB}"',
                     f'ENV_BUILTIN_PROVIDER_CONFIG: &str = "{ENV_BUILTIN}"', f'ENV_RUN_AS_NODE: &str = "{ENV_RUN_AS_NODE}"',
                     f'ENV_ENGINE_OVERRIDE: &str = "{ENV_ENGINE}"', 'TREE_NAME: &str = ".zcode"',
                     '[engine.as_os_str(), "version".as_ref()]', '&["login"]'],
        "sign_in.rs": [f'ZCODE_SIGN_IN_PATH: &str = "{SIGN_IN}"'],
        "zcode_sessions.rs": ['DB_FILE: &str = "db.sqlite"', 'tree.join("cli").join("db")',
                              "task_type = 'interactive'", "time_archived IS NULL",
                              "FROM message m LEFT JOIN part p ON p.message_id = m.id", 'get("role")', 'Some("text")'],
        "zcode_usage.rs": ["FROM model_usage WHERE status = 'completed' AND started_at >= ?1",
                           "cache_creation_input_tokens, cache_read_input_tokens, started_at", f'"{MODEL}" =>'],
        "zcode_limits.rs": ['"Usage limit reached for 5 hour."', '"Weekly/Monthly Limit Exhausted."',
                            '" Your limit will reset at "', "RESET_OFFSET_MS: i64 = 8 * 3_600_000",
                            "error_type, error_message, started_at", 'Some("rate_limited")'],
        "zcode_appserver/mod.rs": ['SERVER_ARGS: &[&str] = &["app-server", "--stdio"]'],
        "zcode_appserver/codec.rs": [
            'CREATE: &str = "session/create"', 'RESUME: &str = "session/resume"',
            'SUBSCRIBE: &str = "session/subscribe"', 'SEND: &str = "session/send"', 'STOP: &str = "session/stop"',
            'EVENT: &str = "session/event"', 'HEADERS: &str = "interaction/requestProviderRuntimeHeaders"',
            'pointer("/session/sessionId")', '"turn.started" =>', '"turn.completed" =>', '"turn.failed" =>',
            '"part.delta" =>', 'Some("success") => TurnEnd::Completed', 'Some("cancelled") => TurnEnd::Interrupted',
            '"headersApplied": true, "requestAuth": {}', '"importedHistory"', '"source": "claudeCode"'],
    }
    for name, snippets in expect.items():
        text = (src / name).read_text()
        for snippet in snippets:
            assert snippet in text, f"{name} no longer has {snippet!r}"
    apps = (REPO / "crates/juggler-platform/src/apps.rs").read_text()
    for snippet in ('app.join("Contents").join("Resources")', 'app.join("Contents").join("MacOS").join(name)',
                    'dir.extension().is_some_and(|ext| ext == "app")'):
        assert snippet in apps, f"apps.rs no longer has {snippet!r}"
    # The fixture's tables are the ones made here, column for column.
    fixture = (REPO / "crates/juggler-adapters/tests/zcode_fixture/mod.rs").read_text()
    assert columns(fixture) == columns(SCHEMA), "the store's tables differ from tests/zcode_fixture"


def columns(sql: str) -> dict[str, list[str]]:
    """Each CREATE TABLE's column names, in order."""
    import re
    out = {}
    for name, body in re.findall(r"CREATE TABLE IF NOT EXISTS (\w+) \((.*?)\);", sql, re.S):
        out[name] = [item.split()[0] for item in re.split(r",\s*(?![^()]*\))", body.strip()) if item.strip()]
    return out


class Client:
    """Dev's side of one app-server, over the child's stdio, every read
    bounded in time."""

    def __init__(self, argv: list[str], env: dict[str, str], cwd: Path) -> None:
        self.proc = subprocess.Popen(argv, env=env, cwd=str(cwd), stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.buffer = b""
        self.ids = 0

    def send(self, message: dict) -> None:
        assert self.proc.stdin is not None
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()

    def request(self, method: str, params: dict) -> int:
        self.ids += 1
        self.send({"id": self.ids, "method": method, "params": params})
        return self.ids

    def read(self, timeout: float = 10.0) -> dict:
        assert self.proc.stdout is not None
        fd = self.proc.stdout.fileno()
        deadline = time.time() + timeout
        while b"\n" not in self.buffer:
            left = deadline - time.time()
            ready, _, _ = select.select([fd], [], [], max(0.0, left))
            if not ready:
                raise AssertionError("the engine said nothing in time")
            chunk = os.read(fd, 65536)
            if not chunk:
                raise AssertionError("the engine ended: " + (self.proc.stderr.read().decode() if self.proc.stderr else ""))
            self.buffer += chunk
        line, self.buffer = self.buffer.split(b"\n", 1)
        return json.loads(line)

    def until(self, done) -> list[dict]:
        seen = []
        for _ in range(50):
            message = self.read()
            seen.append(message)
            if done(message):
                return seen
        raise AssertionError(f"never came: {seen}")

    def answer_of(self, rid: int) -> dict:
        return self.until(lambda m: m.get("id") == rid and "method" not in m)[-1]

    def close(self) -> int:
        assert self.proc.stdin is not None
        self.proc.stdin.close()
        try:
            return self.proc.wait(timeout=10)
        finally:
            if self.proc.poll() is None:
                self.proc.kill()
            for stream in (self.proc.stdout, self.proc.stderr):
                if stream:
                    stream.close()


def event_type(message: dict) -> str | None:
    return message.get("params", {}).get("type") if message.get("method") == "session/event" else None


def run_turn(client: Client, session: str, text: str, answer_headers: bool = True) -> list[dict]:
    rid = client.request("session/send", {"sessionId": session, "content": text})
    seen = client.until(lambda m: m.get("id") == rid)
    assert seen[-1]["result"]["accepted"] is True, seen
    seen += client.until(lambda m: event_type(m) in ("turn.failed", "turn.completed")
                         or m.get("method") == "interaction/requestProviderRuntimeHeaders")
    if seen[-1].get("method") == "interaction/requestProviderRuntimeHeaders":
        assert answer_headers
        client.send({"id": seen[-1]["id"], "result": {"headersApplied": True, "requestAuth": {}}})
        seen += client.until(lambda m: event_type(m) in ("turn.failed", "turn.completed"))
    return seen


def read_turns(db: Path, session: str) -> list[tuple[str, str]]:
    """The turns as the adapter's `turns()` reads them."""
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        rows = conn.execute("SELECT m.id, m.data, p.data FROM message m LEFT JOIN part p ON p.message_id = m.id"
                            " WHERE m.session_id = ? ORDER BY m.time_created, m.rowid, p.time_created, p.rowid",
                            (session,)).fetchall()
    finally:
        conn.close()
    return [(json.loads(data)["role"], json.loads(part)["text"]) for _, data, part in rows
            if part and json.loads(part).get("type") == "text"]


def selftest() -> int:
    common.selftest_common()
    static_check()
    me = str(Path(__file__).resolve())
    py = ["/usr/bin/python3", "-I"]
    with common.selftest_lab(TOOL) as lab:
        app_a, app_b = lab / "data" / "accounts" / TOOL / "zcode-a", lab / "data" / "accounts" / TOOL / "zcode-b"
        work = lab / "work" / "lab"
        for folder in (app_a / ".zcode", app_b / ".zcode", work):
            folder.mkdir(parents=True)
        builtin = lab / "builtin.json"
        builtin.write_text(json.dumps(BUILTIN))

        def env_of(app: Path, **more: str) -> dict[str, str]:
            env = {"PATH": "/usr/bin:/bin", "HOME": str(lab), "AIJ_LAB_STANDIN_WAIT": "0",
                   ENV_RUN_AS_NODE: "1", ENV_BUILTIN: str(builtin), ENV_STORAGE: str(app / ".zcode"),
                   ENV_BASE: str(app), ENV_DB: str(app / ".zcode/cli/db/db.sqlite")}
            env.update(more)
            return env

        def run(argv: list[str], env: dict[str, str], stdin: str = "") -> subprocess.CompletedProcess:
            return subprocess.run(argv, env=env, cwd=str(work), input=stdin, capture_output=True, text=True, timeout=60)

        # Discovery: no tree variables.
        done = run(py + [me, "version"], {"PATH": "/usr/bin:/bin", ENV_RUN_AS_NODE: "1"})
        assert done.returncode == 0 and done.stdout.splitlines()[0] == VERSION, done
        # Outside a lab: refused, nothing written.
        done = run(py + [me, "app-server", "--stdio"], env_of(app_a, **{ENV_STORAGE: "/Users/someone/.zcode"}))
        assert done.returncode == 3 and "isn't in a lab" in done.stderr, done
        # Sign in.
        done = run(py + [me, "login"], env_of(app_a))
        assert done.returncode == 0 and f".{common.FAKE_HOST}/" in done.stdout, done
        creds = json.loads((app_a / ".zcode" / SIGN_IN).read_text())
        assert creds["oauth:zai:access_token"] == common.NOT_A_TOKEN and set(creds) == {
            "oauth:zai:access_token", "zcodejwttoken", "oauth:zai:user_info", "oauth:active_provider"}
        assert oct((app_a / ".zcode" / SIGN_IN).stat().st_mode & 0o777) == "0o600"
        assert list((app_a / ".zcode/v2/runtime/provider/darwin" / BUILD).glob("endpoint-*/zcode-builtin.json"))
        # A session: create, subscribe, a turn with the headers reply.
        client = Client(py + [me, "app-server", "--stdio"], env_of(app_a), work)
        db = app_a / ".zcode/cli/db/db.sqlite"
        rid = client.request("session/create", {"workspace": {"workspacePath": str(work), "workspaceKey": str(work)},
                                                "persistence": "immediate"})
        session = client.answer_of(rid)["result"]["session"]["sessionId"]
        assert session.startswith("sess_") and db.is_file()
        rid = client.request("session/subscribe", {"sessionId": session, "deliveryKind": "desktop-continuous"})
        assert client.answer_of(rid)["result"] == {"eventSeq": 0, "events": []}
        seen = run_turn(client, session, "reply with the word lab")
        kinds = [event_type(m) for m in seen if event_type(m)]
        assert kinds == ["turn.started", "part.delta", "turn.completed"], kinds
        assert seen[-1]["params"]["payload"]["resultType"] == "success"
        assert any(m.get("method") == "interaction/requestProviderRuntimeHeaders" for m in seen)
        assert client.close() == 0
        notes = [json.loads(line) for line in (app_a / ".zcode/cli/log/lab-standin.jsonl").read_text().splitlines()]
        assert any(n["what"] == "headers answered" and n["headersApplied"] and n["requestAuthKeys"] == [] for n in notes)
        # Resume: the same session, the earlier turn first; a limit hit.
        client = Client(py + [me, "app-server", "--stdio"], env_of(app_a), work)
        rid = client.request("session/resume", {"sessionId": session,
                                                "workspace": {"workspacePath": str(work), "workspaceKey": str(work)}})
        assert client.answer_of(rid)["result"]["session"]["sessionId"] == session
        rid = client.request("session/resume", {"sessionId": "sess_unknown", "workspace": {}})
        assert "error" in client.answer_of(rid)
        rid = client.request("session/resume", {"sessionId": session, "workspace": {"workspacePath": str(work)}})
        client.answer_of(rid)
        rid = client.request("session/subscribe", {"sessionId": session, "deliveryKind": "desktop-continuous"})
        client.answer_of(rid)
        seen = run_turn(client, session, "and again")
        assert seen[-1]["params"]["type"] == "turn.completed"
        seen = run_turn(client, session, f"{LIMIT_5H} please")
        failed = seen[-1]["params"]["payload"]["error"]
        assert seen[-1]["params"]["type"] == "turn.failed" and failed["type"] == "ModelRateLimited"
        assert client.close() == 0
        turns = read_turns(db, session)
        assert turns[:4] == [("user", "reply with the word lab"), ("assistant", "lab"),
                             ("user", "and again"), ("assistant", "lab")], turns
        # Usage: the store's rows as the adapter reads them.
        conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        completed = conn.execute("SELECT COUNT(*), SUM(input_tokens) FROM model_usage WHERE status = 'completed'").fetchone()
        last = conn.execute("SELECT status, error_type, error_message, started_at FROM model_usage WHERE session_id = ?"
                            " ORDER BY started_at DESC, rowid DESC LIMIT 1", (session,)).fetchone()
        listed = conn.execute("SELECT id FROM session WHERE task_type = 'interactive' AND time_archived IS NULL"
                              " AND (parent_id IS NULL OR parent_id = '')").fetchall()
        conn.close()
        assert completed == (2, 2400) and listed == [(session,)], (completed, listed)
        assert last[0] == "error" and last[1] == "rate_limited"
        import re
        hit = re.fullmatch(r"\[1308\]\[Usage limit reached for 5 hour\. Your limit will reset at "
                           r"(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\]\[([0-9a-f]{30})\]", last[2])
        assert hit, last[2]
        reset_utc = calendar.timegm(time.strptime(hit.group(1), "%Y-%m-%d %H:%M:%S")) - UTC8_S
        assert 0 < reset_utc * 1000 - last[3] <= 3 * 3600 * 1000 + 5000, "the reset, read in UTC+8, is ~3 h on"
        # An import: the caller's id, no model call.
        client = Client(py + [me, "app-server", "--stdio"], env_of(app_a), work)
        imported = "sess_0b1c2d3e-0000-4000-8000-0000000000aa"
        rid = client.request("session/create", {"sessionId": imported, "workspace": {"workspacePath": str(work)},
                                                "persistence": "immediate", "titleGenerationEnabled": False,
                                                "importedHistory": {"source": "claudeCode", "title": "Handover",
                                                                    "messages": [{"role": "user", "content": "Fix it"},
                                                                                 {"role": "assistant", "content": "Done"}]}})
        assert client.answer_of(rid)["result"]["session"]["sessionId"] == imported
        assert client.close() == 0
        assert read_turns(db, imported) == [("user", "Fix it"), ("assistant", "Done")]
        # Lab B, not signed in: its turn fails as a sign-in.
        client = Client(py + [me, "app-server", "--stdio"], env_of(app_b), work)
        rid = client.request("session/create", {"workspace": {"workspacePath": str(work)}, "persistence": "immediate"})
        other = client.answer_of(rid)["result"]["session"]["sessionId"]
        client.answer_of(client.request("session/subscribe", {"sessionId": other, "deliveryKind": "desktop-continuous"}))
        seen = run_turn(client, other, "reply with the word lab", answer_headers=False)
        assert seen[-1]["params"]["type"] == "turn.failed" and "401" in seen[-1]["params"]["payload"]["error"]["message"]
        assert client.close() == 0
        assert not (app_b / ".zcode" / SIGN_IN).exists()
        # Installed: the bundle as the adapter resolves it, run by its runtime.
        dest = lab / "standins"
        dest.mkdir()
        shutil.copy2(Path(__file__).resolve().parent / "standin_common.py", dest / "standin_common.py")
        made = install(dest)
        engine = Path(made[ENV_ENGINE])
        app = next(up for up in engine.parents if up.suffix == ".app")
        runtime = app / "Contents" / "MacOS" / RUNTIME_NAME
        assert (app / "Contents/Resources" / BUILTIN_IN_APP).is_file() and os.access(runtime, os.X_OK)
        assert "dev.zcode.app" not in (app / "Contents/Info.plist").read_text()
        done = run([str(runtime), str(engine), "version"], {"PATH": "/usr/bin:/bin", ENV_RUN_AS_NODE: "1"})
        assert done.returncode == 0 and done.stdout.strip() == VERSION, done
        done = run([str(runtime), str(engine), "version"], {"PATH": "/usr/bin:/bin"})
        assert done.returncode == 2 and "no window" in done.stderr, done
        bundled_env = env_of(app_a, **{ENV_BUILTIN: str(app / "Contents/Resources" / BUILTIN_IN_APP)})
        client = Client([str(runtime), str(engine), "app-server", "--stdio"], bundled_env, work)
        rid = client.request("session/resume", {"sessionId": session, "workspace": {"workspacePath": str(work)}})
        assert client.answer_of(rid)["result"]["session"]["sessionId"] == session
        client.answer_of(client.request("session/subscribe", {"sessionId": session, "deliveryKind": "desktop-continuous"}))
        assert run_turn(client, session, "through the bundle")[-1]["params"]["type"] == "turn.completed"
        assert client.close() == 0
    print("zcode stand-in: selftest passed")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        sys.exit(selftest())
    sys.exit(main(sys.argv))
