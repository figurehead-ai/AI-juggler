#!/usr/bin/env python3
"""AJ-E1612 lab stand-in for Hermes Agent's `hermes` CLI (built from the
adapter, tests/hermes_fixture and the adapter's tests only: Hermes isn't
installed where this was written, and nothing of it was run or read).

`install(dest)` copies this file to `<dest>/hermes` (so a running stand-in
is `python3 -I …/hermes …`, which the adapter's process check reads as
Hermes, as it reads `python …/hermes`) and makes the launcher
`<dest>/bin/hermes`; it returns {"AI_JUGGLER_HERMES_BIN": <launcher>}.

Every run but `--version`/`--help` needs HERMES_HOME in a lab (exit 3
otherwise), each other folder variable the adapter pins in a lab when set,
and AI Juggler's guard: HERMES_MANAGED_DIR with a `config.yaml` saying
`adopt_external_logins: false` (exit 5 without it: a real Hermes there
would borrow the Codex CLI's and Claude Code's sign-ins). The stand-in
never reads CODEX_HOME or CLAUDE_CONFIG_DIR. What Dev runs (the App's
environment: HERMES_HOME=<app>, HERMES_GATEWAY_LOCK_DIR=<app>/.ai-juggler/
gateway-locks, HERMES_SHARED_AUTH_DIR=<app>/shared, HERMES_MANAGED_DIR=
<app>/.ai-juggler/hermes-policy, CODEX_HOME=<app>/.ai-juggler/no-codex,
CLAUDE_CONFIG_DIR=<app>/.ai-juggler/no-claude), and what this answers:

- Discovery: `hermes --version` -> `Hermes Agent v0.22.3 (2026.9.24)`.
- Sign in (login_spec, a pty, cwd = the App): `hermes model`. A provider
  menu (1 Nous Portal, the default; 2 OpenAI Codex; 3 Anthropic;
  4 OpenRouter), a fake page on example.invalid (a one-time code for Nous
  Portal and Codex), a wait of AIJ_LAB_STANDIN_WAIT seconds, a model menu
  (Enter takes the first); writes `$HERMES_HOME/auth.json` (0600:
  `version`, `active_provider`, `providers.<id>` with lab-not-a-token
  tokens, `credential_pool`; other providers kept), for Nous Portal also
  `$HERMES_SHARED_AUTH_DIR/nous_auth.json`, and `model:` in `config.yaml`.
  End of input cancels it (exit 130), nothing written.
- A session (launch_spec, a pty): `hermes [-m <model>]`, or primed
  `hermes [-m <model>] chat --query=<prompt>`. Not signed in: says to run
  `hermes model` and exits 1. Else a prompt (`hermes> `), each line answered
  `lab`, every turn written to `$HERMES_HOME/state.db` (WAL, schema 33, the
  tables of tests/hermes_fixture): a `sessions` row (`source` cli, `cwd`,
  REAL epoch seconds, token totals, no title, so Dev lists its first
  prompt) and a `messages` row per turn (the assistant's with
  `finish_reason` stop). `/exit`, `/quit` or end of input ends it
  (`ended_at`, `end_reason` user_exit).
- Resume: `hermes [-m <model>] --resume <id>` (an id, a title or
  `latest`): prints the earlier turns, then appends new ones to the same
  session. An unknown id exits 1.
- Usage: `hermes usage --json`, cwd = the App. Not signed in, or a
  provider without a usage reader (Nous Portal): exit 1, nothing on
  stdout, as Hermes does. OpenAI Codex: windows `Session` and `Weekly`;
  Anthropic: `Current session`, `Current week` and a model's own week;
  OpenRouter: credits only. Modest percentages that grow a little with the
  turns in the store; `fetched_at` now, resets 3 h / 5 days on (UTC).
- Gateway (Dev's OwnedGateway; not started by the app today):
  `hermes gateway run --quiet --external-supervisor` records itself in
  `gateway.pid` (`{pid, kind: hermes-gateway, argv, start_time}`),
  `gateway.lock`, `gateway_state.json` and `<locks>/host-gateway.json`,
  serves nothing, and ends on TERM/INT, when orphaned, or after 8 hours;
  `hermes gateway stop` ends the recorded one after checking its command.
- `hermes sessions list` prints the store's sessions. `-z`/`--oneshot`
  and anything else are refused (exit 2).

`--selftest` runs all of the above in a throwaway lab and checks the names
used here against the adapter's Rust source and the fixture's schema.

What a dry run on this stand-in cannot prove:
- anything about the real Hermes: its menus, words, sign-in pages, codes,
  file shapes (auth.json here is a guess the adapter only probes by
  presence), or whether it honours HERMES_HOME and the guard;
- that the real CLI writes its store as here: Hermes's gateway is the one
  writer, a CLI with no gateway starts one (a launchd job on macOS); this
  stand-in's chat writes the store itself and starts no gateway;
- its first-run `hermes setup` steps (an import offer from `~/.claude` or
  `~/.codex`), which a real first `hermes` may reach;
- `hermes usage` readings from a real provider, or a Nous Portal account's;
- titles Hermes generates, compression, tools, approvals, sub-agents.
"""
from __future__ import annotations

import datetime as dt
import importlib.util
import json
import os
import re
import secrets
import shutil
import signal
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

TOOL = "hermes"
BANNER = "Hermes Agent v0.22.3 (2026.9.24)"
SCHEMA_VERSION = 33

ENV_HOME = "HERMES_HOME"
ENV_LOCKS = "HERMES_GATEWAY_LOCK_DIR"
ENV_SHARED = "HERMES_SHARED_AUTH_DIR"
ENV_POLICY = "HERMES_MANAGED_DIR"
ENV_CODEX = "CODEX_HOME"
ENV_CLAUDE = "CLAUDE_CONFIG_DIR"
ENV_BIN = "AI_JUGGLER_HERMES_BIN"
FOLDERS = (ENV_LOCKS, ENV_SHARED, ENV_POLICY, ENV_CODEX, ENV_CLAUDE)
GUARD = "adopt_external_logins: false"

AUTH = "auth.json"
SHARED_AUTH = "nous_auth.json"
STORE = "state.db"
PID_FILE = "gateway.pid"
KIND = "hermes-gateway"
GATEWAY_LIFETIME_S = 8 * 3600

# (id, menu name, fake page, shows a code, models). Every model name made up.
PROVIDERS = (
    ("nous", "Nous Portal", "portal-nous", True, ("nous/lab-large", "nous/lab-small")),
    ("openai-codex", "OpenAI Codex (ChatGPT sign-in)", "codex", True, ("openai-codex/lab-codex", "openai-codex/lab-mini")),
    ("anthropic", "Anthropic (Claude sign-in)", "anthropic", False, ("anthropic/lab-sonnet", "anthropic/lab-haiku")),
    ("openrouter", "OpenRouter", "openrouter", False, ("openrouter/lab-auto",)),
)

# tests/hermes_fixture/mod.rs: Hermes' own tables as SCHEMA_SQL creates
# them (hermes_state_common.py, version 33).
SCHEMA = r"""
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS system_prompts (
    hash TEXT PRIMARY KEY,
    prompt TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    runtime_revision INTEGER NOT NULL DEFAULT 0,
    runtime_generation INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL,
    created_source TEXT,
    user_id TEXT,
    session_key TEXT,
    chat_id TEXT,
    chat_type TEXT,
    thread_id TEXT,
    display_name TEXT,
    origin_json TEXT,
    expiry_finalized INTEGER DEFAULT 0,
    model TEXT,
    model_config TEXT,
    system_prompt TEXT,
    system_prompt_hash TEXT,
    parent_session_id TEXT,
    started_at REAL NOT NULL,
    ended_at REAL,
    end_reason TEXT,
    message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    cwd TEXT,
    git_branch TEXT,
    git_repo_root TEXT,
    git_metadata_generation INTEGER NOT NULL DEFAULT 0,
    billing_provider TEXT,
    billing_base_url TEXT,
    billing_mode TEXT,
    estimated_cost_usd REAL,
    actual_cost_usd REAL,
    cost_status TEXT,
    cost_source TEXT,
    pricing_version TEXT,
    title TEXT,
    title_source TEXT,
    last_activity_at REAL,
    last_activity_description TEXT,
    last_activity_provenance TEXT,
    api_call_count INTEGER DEFAULT 0,
    handoff_state TEXT,
    handoff_platform TEXT,
    handoff_error TEXT,
    compression_failure_cooldown_until REAL,
    compression_failure_error TEXT,
    compression_fallback_streak INTEGER NOT NULL DEFAULT 0,
    compression_ineffective_count INTEGER NOT NULL DEFAULT 0,
    compression_recovery_deadline REAL,
    compression_overload_streak INTEGER NOT NULL DEFAULT 0,
    profile_name TEXT,
    transport_profile TEXT,
    rewind_count INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    auto_archived INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    hidden INTEGER NOT NULL DEFAULT 0,
    last_read_at REAL,
    tool_names TEXT,
    transcript_revision INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_session_id) REFERENCES sessions(id),
    FOREIGN KEY (system_prompt_hash) REFERENCES system_prompts(hash)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT,
    tool_call_id TEXT,
    tool_calls TEXT,
    tool_name TEXT,
    effect_disposition TEXT,
    timestamp REAL NOT NULL,
    token_count INTEGER,
    finish_reason TEXT,
    reasoning TEXT,
    reasoning_content TEXT,
    reasoning_details TEXT,
    codex_reasoning_items TEXT,
    codex_message_items TEXT,
    platform_message_id TEXT,
    observed INTEGER DEFAULT 0,
    _compressed_summary INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    compacted INTEGER NOT NULL DEFAULT 0,
    api_content TEXT,
    display_kind TEXT,
    display_metadata TEXT,
    display_identity BLOB,
    display_order INTEGER,
    message_uid TEXT,
    absorbed_message_uids TEXT,
    tool_call_uids TEXT,
    tool_call_uid TEXT
);
"""

HELP = f"""{BANNER} (AI Juggler lab stand-in)

usage: hermes [-m MODEL] [--resume ID] [command]

commands:
  chat [--query=TEXT]     an interactive session (the default)
  model                   choose a provider, sign in, choose a model
  usage [--json]          the provider's account limits
  gateway run|stop        the home's gateway
  sessions list           the home's sessions
"""


def _load_common():
    path = Path(__file__).resolve().parent / "standin_common.py"
    if not path.is_file():
        sys.stderr.write("hermes (lab stand-in): standin_common.py isn't beside it\n")
        raise SystemExit(3)
    spec = importlib.util.spec_from_file_location("standin_common", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


common = _load_common()


def install(dest: Path) -> dict[str, str]:
    """`<dest>/hermes` (this file) and its launcher `<dest>/bin/hermes`."""
    common.require_lab(dest, TOOL, "the stand-ins' folder")
    shutil.copy2(Path(__file__).resolve(), Path(dest) / "hermes")
    wrapper = common.install_wrapper(Path(dest), "hermes", "hermes")
    return {ENV_BIN: str(wrapper)}


# --------------------------------------------------------------------------
# the home
# --------------------------------------------------------------------------

def iso(seconds: float) -> str:
    return dt.datetime.fromtimestamp(int(seconds), dt.timezone.utc).isoformat()


class Home:
    def __init__(self) -> None:
        self.home = common.require_lab(os.environ.get(ENV_HOME), TOOL, ENV_HOME)
        for name in FOLDERS:
            if os.environ.get(name):
                common.require_lab(os.environ[name], TOOL, name)
        policy = os.environ.get(ENV_POLICY)
        try:
            guarded = bool(policy) and GUARD in (Path(policy) / "config.yaml").read_text()
        except OSError:
            guarded = False
        if not guarded:
            common.fail(TOOL, f"no AI Juggler guard ({ENV_POLICY}/config.yaml with `{GUARD}`): "
                              "a real Hermes here would borrow the Codex CLI's and Claude Code's sign-ins", 5)
        self.auth = self.home / AUTH
        self.shared = Path(os.environ.get(ENV_SHARED) or self.home / "shared") / SHARED_AUTH
        self.locks = Path(os.environ[ENV_LOCKS]) if os.environ.get(ENV_LOCKS) else None
        self.store = self.home / STORE

    def auth_data(self) -> dict:
        try:
            data = json.loads(self.auth.read_text())
        except (OSError, ValueError):
            return {}
        return data if isinstance(data, dict) else {}

    def signed_in(self) -> bool:
        """As the adapter probes it (sign_in.rs `hermes_signed_in`)."""
        filled = lambda path: path.is_file() and path.stat().st_size > 0  # noqa: E731
        return filled(self.auth) or filled(self.shared)

    def provider(self) -> str | None:
        active = self.auth_data().get("active_provider")
        if isinstance(active, str):
            return active
        return "nous" if self.shared.is_file() else None

    def model(self) -> str:
        try:
            text = (self.home / "config.yaml").read_text()
        except OSError:
            text = ""
        found = re.search(r"^model:\n(?:  .*\n)*?  default: (\S+)", text, re.M)
        if found:
            return found.group(1)
        provider = self.provider()
        return next((p[4][0] for p in PROVIDERS if p[0] == provider), "nous/lab-large")


class Store:
    """state.db, held open (WAL) while a session runs."""

    def __init__(self, path: Path) -> None:
        common.require_lab(path, TOOL, "the session store")
        path.parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(str(path), isolation_level=None)
        self.conn.execute("PRAGMA journal_mode = WAL")
        self.conn.executescript(SCHEMA)
        if self.conn.execute("SELECT COUNT(*) FROM schema_version").fetchone()[0] == 0:
            self.conn.execute("INSERT INTO schema_version (version) VALUES (?)", (SCHEMA_VERSION,))

    def close(self) -> None:
        self.conn.close()

    def find(self, wanted: str) -> str | None:
        """An id, a title, or `latest` (the newest root session)."""
        if wanted == "latest":
            row = self.conn.execute("SELECT id FROM sessions WHERE parent_session_id IS NULL"
                                    " ORDER BY COALESCE(last_activity_at, started_at) DESC LIMIT 1").fetchone()
        else:
            row = self.conn.execute("SELECT id FROM sessions WHERE id = ? OR title = ?"
                                    " ORDER BY id = ? DESC LIMIT 1", (wanted, wanted, wanted)).fetchone()
        return row[0] if row else None

    def start(self, cwd: str, model: str, provider: str | None) -> str:
        session_id = f"{time.strftime('%Y%m%d_%H%M%S')}_{secrets.token_hex(3)}"
        now = time.time()
        self.conn.execute(
            "INSERT INTO sessions (id, source, created_source, model, started_at, last_activity_at, cwd,"
            " billing_provider, billing_mode) VALUES (?, 'cli', 'cli', ?, ?, ?, ?, ?, 'subscription')",
            (session_id, model, now, now, cwd, provider))
        return session_id

    def reopen(self, session_id: str) -> None:
        self.conn.execute("UPDATE sessions SET ended_at = NULL, end_reason = NULL, last_activity_at = ?"
                          " WHERE id = ?", (time.time(), session_id))

    def end(self, session_id: str) -> None:
        now = time.time()
        self.conn.execute("UPDATE sessions SET ended_at = ?, end_reason = 'user_exit', last_activity_at = ?"
                          " WHERE id = ?", (now, now, session_id))

    def turns(self, session_id: str) -> list[tuple[str, str]]:
        return self.conn.execute("SELECT role, content FROM messages WHERE session_id = ? AND active = 1"
                                 " ORDER BY id", (session_id,)).fetchall()

    def turn(self, session_id: str, text: str, answer: str) -> None:
        tokens_in, tokens_out, cached = 900, 12, 600
        self.conn.execute("BEGIN")
        now = time.time()
        self.conn.execute("INSERT INTO messages (session_id, role, content, timestamp, token_count, message_uid)"
                          " VALUES (?, 'user', ?, ?, ?, ?)", (session_id, text, now, tokens_in, secrets.token_hex(8)))
        done = time.time()
        self.conn.execute("INSERT INTO messages (session_id, role, content, timestamp, token_count, finish_reason,"
                          " message_uid) VALUES (?, 'assistant', ?, ?, ?, 'stop', ?)",
                          (session_id, answer, done, tokens_out, secrets.token_hex(8)))
        self.conn.execute(
            "UPDATE sessions SET message_count = message_count + 2, api_call_count = api_call_count + 1,"
            " input_tokens = input_tokens + ?, output_tokens = output_tokens + ?,"
            " cache_read_tokens = cache_read_tokens + ?, last_activity_at = ?,"
            " transcript_revision = transcript_revision + 1 WHERE id = ?",
            (tokens_in, tokens_out, cached, done, session_id))
        self.conn.execute("COMMIT")


# --------------------------------------------------------------------------
# commands
# --------------------------------------------------------------------------

def ask(prompt: str, count: int) -> int | None:
    """A menu choice, 1..count (Enter: 1); None at end of input."""
    for _ in range(3):
        sys.stdout.write(prompt)
        sys.stdout.flush()
        line = sys.stdin.readline()
        if not line:
            return None
        text = line.strip() or "1"
        if text.isdigit() and 1 <= int(text) <= count:
            return int(text)
        print(f"Choose 1-{count}.", flush=True)
    return None


def model_command(home: Home) -> int:
    print("Select a provider:", flush=True)
    for n, provider in enumerate(PROVIDERS, 1):
        print(f"  {n}. {provider[1]}" + ("  (default)" if n == 1 else ""), flush=True)
    chosen = ask("Provider [1]: ", len(PROVIDERS))
    if chosen is None:
        print("\nCancelled.", flush=True)
        return 130
    pid, name, page, coded, models = PROVIDERS[chosen - 1]
    print(f"Signing in to {name}…", flush=True)
    common.fake_sign_in_page(TOOL, page, common.device_code() if coded else None)
    print("Select a model:", flush=True)
    for n, model in enumerate(models, 1):
        print(f"  {n}. {model}", flush=True)
    picked = ask("Model [1]: ", len(models))
    if picked is None:
        print("\nCancelled.", flush=True)
        return 130
    model = models[picked - 1]
    now = time.time()
    data = home.auth_data()
    providers = data.get("providers") if isinstance(data.get("providers"), dict) else {}
    providers[pid] = {"auth_type": "oauth", "access_token": common.NOT_A_TOKEN,
                      "refresh_token": common.NOT_A_TOKEN, "obtained_at": iso(now), "expires_at": iso(now + 3600)}
    pool = data.get("credential_pool") if isinstance(data.get("credential_pool"), dict) else {}
    pool.setdefault(pid, [])
    common.write_json(home.auth, {"version": 1, "active_provider": pid, "providers": providers,
                                  "credential_pool": pool}, TOOL)
    if pid == "nous":
        common.write_json(home.shared, {"access_token": common.NOT_A_TOKEN, "refresh_token": common.NOT_A_TOKEN,
                                        "obtained_at": iso(now)}, TOOL)
    config = home.home / "config.yaml"
    try:
        text = config.read_text()
    except OSError:
        text = ""
    text = re.sub(r"^model:\n(?:[ \t]+.*\n?)*", "", text, flags=re.M)
    common.require_lab(config, TOOL, "config.yaml")
    config.write_text(text + f"model:\n  provider: {pid}\n  default: {model}\n")
    print(f"Signed in to {name}; the model is {model}.", flush=True)
    return 0


def chat(home: Home, model: str | None, resume: str | None, query: str | None) -> int:
    if not home.signed_in():
        print("No inference provider is set up. Run `hermes model` to choose one and sign in.", flush=True)
        return 1
    model = model or home.model()
    store = Store(home.store)
    try:
        if resume:
            session_id = store.find(resume)
            if session_id is None:
                print(f"No session matches '{resume}'.", flush=True)
                return 1
            store.reopen(session_id)
            earlier = store.turns(session_id)
            print(f"{BANNER} · {model} · lab stand-in", flush=True)
            print(f"Resumed session {session_id} ({len(earlier)} messages):", flush=True)
            for role, content in earlier:
                print(f"  {'you' if role == 'user' else 'hermes'}: {content}", flush=True)
        else:
            session_id = store.start(os.getcwd(), model, home.provider())
            print(f"{BANNER} · {model} · lab stand-in", flush=True)
            print(f"Session {session_id}. /exit to leave.", flush=True)

        def on_turn(text: str) -> str:
            if text.startswith("/"):
                return f"(lab stand-in: {text.split()[0]} isn't served)"
            answer = common.answer_for(text)
            store.turn(session_id, text, answer)
            return answer

        if query:
            print(f"hermes> {query}", flush=True)
            print(on_turn(query), flush=True)
        code = common.repl(TOOL, "hermes> ", on_turn)
        store.end(session_id)
        return code
    finally:
        store.close()


def turns_in_store(home: Home) -> int:
    if not home.store.is_file():
        return 0
    conn = sqlite3.connect(f"file:{home.store}?mode=ro", uri=True)
    try:
        return conn.execute("SELECT COUNT(*) FROM messages WHERE role = 'assistant'").fetchone()[0]
    except sqlite3.Error:
        return 0
    finally:
        conn.close()


def usage_document(home: Home, provider: str) -> dict | None:
    """`hermes usage --json` for `provider`; None when Hermes has no
    reader for it (Nous Portal) or no sign-in."""
    turns = turns_in_store(home)
    now = time.time()
    session = round(min(90.0, 6.0 + 1.5 * turns), 1)
    week = round(min(60.0, 2.0 + 0.5 * turns), 1)

    def window(label: str, used: float | None, ahead: float | None) -> dict:
        return {"label": label, "used_percent": used, "resets_at": iso(now + ahead) if ahead else None, "detail": None}

    base = {"provider": provider, "source": "usage_api", "title": "Account limits", "fetched_at": iso(now),
            "details": [], "unavailable_reason": None}
    if provider == "openai-codex":
        return dict(base, plan="Plus", windows=[window("Session", session, 3 * 3600), window("Weekly", week, 5 * 86400)])
    if provider == "anthropic":
        return dict(base, plan="Max", windows=[window("Current session", session, 3 * 3600),
                                                window("Current week", week, 5 * 86400),
                                                window("Current week (lab-sonnet)", week / 2, 5 * 86400)])
    if provider == "openrouter":
        return dict(base, source="credits_api", title="OpenRouter credits", plan=None, windows=[],
                    details=["Credits: $4.20 left of $5.00 (lab stand-in)"])
    return None


def usage_command(home: Home, words: list[str]) -> int:
    provider = words[words.index("--provider") + 1] if "--provider" in words[:-1] else home.provider()
    document = usage_document(home, provider) if home.signed_in() and provider else None
    if document is None:
        sys.stderr.write(f"No account usage available for {provider or 'this home'}.\n")
        return 1
    if "--json" in words:
        print(json.dumps(document), flush=True)
    else:
        print(document["title"], flush=True)
        for window in document["windows"]:
            print(f"  {window['label']}: {window['used_percent']}% used", flush=True)
    return 0


def command_line(pid: int) -> str:
    done = subprocess.run(["/bin/ps", "-o", "command=", "-p", str(pid)], capture_output=True, text=True)
    return done.stdout.strip()


def gateway_run(home: Home, words: list[str]) -> int:
    if home.locks is None:
        # Hermes' default is under the real home (~/.local/state/hermes).
        common.fail(TOOL, f"{ENV_LOCKS} isn't set: the gateway's lock folder would be the real home's")
    record = home.home / PID_FILE
    stopping = []
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda *_: stopping.append(True))
    common.write_json(record, {"pid": os.getpid(), "kind": KIND, "argv": ["hermes"] + words,
                               "start_time": time.time()}, TOOL, 0o644)
    common.write_json(home.home / "gateway_state.json", {"gateway_state": "running", "pid": os.getpid()}, TOOL, 0o644)
    (home.home / "gateway.lock").write_text(f"{os.getpid()}\n")
    common.write_json(home.locks / "host-gateway.json", {"pid": os.getpid(), "home": str(home.home),
                                                         "host": "127.0.0.1", "port": None,
                                                         "token": common.NOT_A_TOKEN}, TOOL)
    started = time.time()
    try:
        while not stopping and os.getppid() != 1 and time.time() - started < GATEWAY_LIFETIME_S:
            time.sleep(0.2)
    finally:
        for path in (record, home.home / "gateway.lock", home.locks / "host-gateway.json"):
            try:
                path.unlink()
            except OSError:
                pass
        common.write_json(home.home / "gateway_state.json", {"gateway_state": "stopped"}, TOOL, 0o644)
    return 0


def gateway_stop(home: Home) -> int:
    record = home.home / PID_FILE
    try:
        data = json.loads(record.read_text())
        pid = int(data["pid"]) if data.get("kind") == KIND else 0
    except (OSError, ValueError, KeyError, TypeError):
        pid = 0
    line = command_line(pid) if pid > 1 else ""
    if not (line and "gateway" in line and "run" in line):
        print("No gateway is running for this home.", flush=True)
        return 0
    os.kill(pid, signal.SIGTERM)
    for _ in range(50):
        if not record.exists() or not command_line(pid):
            break
        time.sleep(0.1)
    else:
        os.kill(pid, signal.SIGKILL)
    print("Gateway stopped.", flush=True)
    return 0


def sessions_list(home: Home) -> int:
    if not home.store.is_file():
        print("No sessions.", flush=True)
        return 0
    conn = sqlite3.connect(f"file:{home.store}?mode=ro", uri=True)
    try:
        for row in conn.execute("SELECT id, COALESCE(title, ''), message_count, cwd FROM sessions"
                                " WHERE parent_session_id IS NULL ORDER BY started_at DESC"):
            print("  ".join(str(value) for value in row), flush=True)
    finally:
        conn.close()
    return 0


VALUE_OPTIONS = {"-m": "model", "--model": "model", "-r": "resume", "--resume": "resume", "-q": "query",
                 "--query": "query", "-p": "profile", "--profile": "profile", "--provider": "provider"}


def parse(words: list[str]) -> tuple[dict[str, str], list[str]]:
    """The options (anywhere before or after `chat`) and the command words."""
    options: dict[str, str] = {}
    rest: list[str] = []
    at = 0
    while at < len(words):
        word = words[at]
        name, _, value = word.partition("=")
        if name in VALUE_OPTIONS and (value or at + 1 < len(words)) and (not rest or rest[0] == "chat"):
            options[VALUE_OPTIONS[name]] = value if "=" in word else words[at + 1]
            at += 1 if "=" in word else 2
            continue
        if word in ("-c", "--continue") and (not rest or rest[0] == "chat"):
            options["resume"] = "latest"
        else:
            rest.append(word)
        at += 1
    return options, rest


def main(argv: list[str]) -> int:
    common.no_network(TOOL)
    common.log_call(TOOL, argv)
    words = argv[1:]
    if words[:1] in (["--version"], ["-V"]):
        print(BANNER, flush=True)
        return 0
    if words[:1] in (["--help"], ["-h"]):
        print(HELP, end="", flush=True)
        return 0
    if "-z" in words or "--oneshot" in words or any(w.startswith("--oneshot=") for w in words):
        sys.stderr.write("hermes (lab stand-in): one-shot runs skip every approval; AI Juggler never runs them\n")
        return 2
    options, rest = parse(words)
    home = Home()
    command = rest[0] if rest else "chat"
    if command == "chat" and len(rest) <= 1:
        return chat(home, options.get("model"), options.get("resume"), options.get("query"))
    if rest == ["model"]:
        return model_command(home)
    if command == "usage":
        return usage_command(home, words)
    if rest[:2] == ["gateway", "run"]:
        return gateway_run(home, words)
    if rest[:2] == ["gateway", "stop"]:
        return gateway_stop(home)
    if rest[:2] == ["sessions", "list"]:
        return sessions_list(home)
    sys.stderr.write(f"hermes (lab stand-in): `{' '.join(words)}` isn't something AI Juggler runs\n")
    return 2


# --------------------------------------------------------------------------
# selftest
# --------------------------------------------------------------------------

def columns(sql: str) -> dict[str, list[str]]:
    out = {}
    for name, body in re.findall(r"CREATE TABLE IF NOT EXISTS (\w+) \((.*?)\);", sql, re.S):
        out[name] = [item.split()[0] for item in re.split(r",\s*(?![^()]*\))", body.strip()) if item.strip()]
    return out


def static_check() -> None:
    root = Path(__file__).resolve().parent.parent.parent.parent
    src = root / "crates/juggler-adapters/src"
    assert src.is_dir(), "the selftest runs from the repository"
    expect = {
        "hermes.rs": [f'ENV_HOME: &str = "{ENV_HOME}"', f'ENV_LOCKS: &str = "{ENV_LOCKS}"',
                      f'ENV_SHARED_AUTH: &str = "{ENV_SHARED}"', f'ENV_POLICY: &str = "{ENV_POLICY}"',
                      f'ENV_CODEX: &str = "{ENV_CODEX}"', f'ENV_CLAUDE: &str = "{ENV_CLAUDE}"',
                      f'ENV_BIN_OVERRIDE: &str = "{ENV_BIN}"', 'SHARED_DIR: &str = "shared"',
                      'args: vec!["model".into()]', 'args.push("--resume".to_owned())',
                      'args.push("chat".to_owned())', 'format!("--query={prompt}")', '"-m".to_owned()',
                      '"Hermes Agent v0.22.3 (2026.9.24)'],
        "sign_in.rs": [f'HERMES_AUTH: &str = "{AUTH}"', 'HERMES_SHARED_AUTH: &str = "shared"', f'"{SHARED_AUTH}"'],
        "hermes_sessions.rs": [f'STORE_FILE: &str = "{STORE}"', f"SCHEMA_READ: i64 = {SCHEMA_VERSION}",
                               "SELECT max(version) FROM schema_version", "m.role = 'user'"],
        "hermes_usage.rs": ['args(["usage", "--json"])', '"Session" | "Current session"', '"Weekly" | "Current week"',
                            'get("unavailable_reason")', 'get("fetched_at")', 'get("windows")', 'get("label")',
                            'get("used_percent")', 'get("resets_at")', 'get("plan")'],
        "hermes_gateway.rs": ['RUN_ARGS: &[&str] = &["gateway", "run", "--quiet", "--external-supervisor"]',
                              'STOP_ARGS: &[&str] = &["gateway", "stop"]', f'PID_FILE: &str = "{PID_FILE}"',
                              f'KIND: &str = "{KIND}"'],
        "hermes_guard.rs": ['POLICY_DIR: &str = ".ai-juggler/hermes-policy"', "adopt_external_logins: false",
                            'LOCKS_DIR: &str = ".ai-juggler/gateway-locks"', 'NO_CODEX: &str = ".ai-juggler/no-codex"',
                            'NO_CLAUDE: &str = ".ai-juggler/no-claude"'],
    }
    for name, snippets in expect.items():
        text = (src / name).read_text()
        for snippet in snippets:
            assert snippet in text, f"{name} no longer has {snippet!r}"
    read = re.search(r"const COLUMNS: &\[&str\] = &\[(.*?)\];", (src / "hermes_sessions.rs").read_text(), re.S)
    assert read, "hermes_sessions.rs COLUMNS"
    mine = columns(SCHEMA)
    for column in re.findall(r'"(\w+)"', read.group(1)):
        assert column in mine["sessions"], f"the store lacks {column}, which the adapter reads"
    fixture = (root / "crates/juggler-adapters/tests/hermes_fixture/mod.rs").read_text()
    assert columns(fixture) == mine, "the store's tables differ from tests/hermes_fixture"


def adapter_sessions(store: Path) -> list[tuple]:
    """hermes_sessions.rs `sessions()`'s query."""
    conn = sqlite3.connect(f"file:{store}?mode=ro", uri=True)
    try:
        assert conn.execute("SELECT max(version) FROM schema_version").fetchone()[0] >= SCHEMA_VERSION
        return conn.execute(
            "SELECT s.id, COALESCE(NULLIF(s.title, ''), (SELECT m.content FROM messages m WHERE m.session_id = s.id"
            " AND m.role = 'user' ORDER BY m.id LIMIT 1), ''), COALESCE(s.cwd, ''), s.input_tokens, s.output_tokens"
            " FROM sessions s WHERE (s.parent_session_id IS NULL OR s.parent_session_id = '')"
            " AND COALESCE(s.hidden, 0) = 0 ORDER BY COALESCE(s.last_activity_at, s.ended_at, s.started_at) DESC"
        ).fetchall()
    finally:
        conn.close()


def selftest() -> int:
    common.selftest_common()
    static_check()
    me = str(Path(__file__).resolve())
    py = ["/usr/bin/python3", "-I"]
    with common.selftest_lab(TOOL) as lab:
        work = lab / "work" / "lab"
        work.mkdir(parents=True)

        def app(name: str) -> Path:
            folder = lab / "data" / "accounts" / "hermes-agent" / name
            policy = folder / ".ai-juggler" / "hermes-policy"
            policy.mkdir(parents=True)
            (policy / "config.yaml").write_text("auth:\n  adopt_external_logins: false\n")
            return folder

        def env_of(folder: Path, **more: str) -> dict[str, str]:
            env = {"PATH": "/usr/bin:/bin", "HOME": str(lab), "AIJ_LAB_STANDIN_WAIT": "0",
                   ENV_HOME: str(folder), ENV_LOCKS: str(folder / ".ai-juggler/gateway-locks"),
                   ENV_SHARED: str(folder / "shared"), ENV_POLICY: str(folder / ".ai-juggler/hermes-policy"),
                   ENV_CODEX: str(folder / ".ai-juggler/no-codex"), ENV_CLAUDE: str(folder / ".ai-juggler/no-claude")}
            env.update(more)
            return env

        def run(args: list[str], env: dict[str, str], stdin: str = "", cwd: Path = work,
                prog: list[str] | None = None) -> subprocess.CompletedProcess:
            return subprocess.run((prog or py + [me]) + args, env=env, cwd=str(cwd), input=stdin,
                                  capture_output=True, text=True, timeout=60)

        a, b = app("hermes-a"), app("hermes-b")
        done = run(["--version"], {"PATH": "/usr/bin:/bin"})
        assert done.returncode == 0 and done.stdout.startswith(BANNER), done
        done = run([], env_of(a, **{ENV_HOME: "/Users/someone/.hermes"}))
        assert done.returncode == 3, done
        done = run([], env_of(a, **{ENV_POLICY: str(lab / "nowhere")}))
        assert done.returncode == 5 and "guard" in done.stderr, done
        # Not signed in: a session is refused, usage has nothing.
        done = run([], env_of(a), "hi\n")
        assert done.returncode == 1 and "hermes model" in done.stdout, done
        done = run(["usage", "--json"], env_of(a), cwd=a)
        assert done.returncode == 1 and done.stdout == "", done
        # Sign in: OpenAI Codex, its first model.
        done = run(["model"], env_of(a), "2\n\n", cwd=a)
        assert done.returncode == 0 and f".{common.FAKE_HOST}/" in done.stdout and "one-time code" in done.stdout, done
        auth = json.loads((a / AUTH).read_text())
        assert auth["active_provider"] == "openai-codex" and auth["providers"]["openai-codex"]["access_token"] == common.NOT_A_TOKEN
        assert oct((a / AUTH).stat().st_mode & 0o777) == "0o600"
        assert "default: openai-codex/lab-codex" in (a / "config.yaml").read_text()
        assert run(["model"], env_of(b), "").returncode == 130 and not (b / AUTH).exists()
        # A session, then its resume with the earlier turn shown.
        done = run([], env_of(a), "reply with the word lab\n/exit\n")
        assert done.returncode == 0 and "hermes> lab\n" in done.stdout, done
        listed = adapter_sessions(a / STORE)
        assert len(listed) == 1 and listed[0][1:] == ("reply with the word lab", os.path.realpath(work), 900, 12), listed
        session = listed[0][0]
        done = run(["--resume", session], env_of(a), "and again\n")
        assert done.returncode == 0 and "you: reply with the word lab" in done.stdout and "hermes: lab" in done.stdout, done
        conn = sqlite3.connect(f"file:{a / STORE}?mode=ro", uri=True)
        rows = conn.execute("SELECT role, content FROM messages WHERE session_id = ? ORDER BY id", (session,)).fetchall()
        ended = conn.execute("SELECT end_reason, message_count FROM sessions WHERE id = ?", (session,)).fetchone()
        conn.close()
        assert rows == [("user", "reply with the word lab"), ("assistant", "lab"), ("user", "and again"),
                        ("assistant", "lab")], rows
        assert ended == ("user_exit", 4), ended
        assert run(["--resume", "nope"], env_of(a), "").returncode == 1
        # Primed: `chat --query=…`, a session of its own.
        done = run(["-m", "openai-codex/lab-mini", "chat", "--query=hello there"], env_of(a), "")
        assert done.returncode == 0 and "hello there" in done.stdout, done
        assert len(adapter_sessions(a / STORE)) == 2
        # Usage as hermes_usage.rs reads it.
        done = run(["usage", "--json"], env_of(a), cwd=a)
        document = json.loads(done.stdout)
        assert done.returncode == 0 and document["unavailable_reason"] is None and document["plan"] == "Plus"
        assert [w["label"] for w in document["windows"]] == ["Session", "Weekly"]
        assert all(isinstance(w["used_percent"], float) and 0 <= w["used_percent"] < 100 for w in document["windows"])
        dt.datetime.fromisoformat(document["fetched_at"])
        dt.datetime.fromisoformat(document["windows"][0]["resets_at"])
        # Nous Portal: the shared sign-in; no usage reader, as in Hermes.
        assert run(["model"], env_of(b), "1\n2\n", cwd=b).returncode == 0
        assert (b / "shared" / SHARED_AUTH).is_file() and json.loads((b / AUTH).read_text())["active_provider"] == "nous"
        done = run(["usage", "--json"], env_of(b), cwd=b)
        assert done.returncode == 1 and done.stdout == "", done
        # The gateway: recorded, then ended by Hermes' own stop.
        gateway = subprocess.Popen(py + [me, "gateway", "run", "--quiet", "--external-supervisor"], env=env_of(a),
                                   cwd=str(a), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            for _ in range(50):
                if (a / PID_FILE).is_file():
                    break
                time.sleep(0.1)
            record = json.loads((a / PID_FILE).read_text())
            assert record["kind"] == KIND and record["pid"] == gateway.pid
            assert (a / ".ai-juggler/gateway-locks/host-gateway.json").is_file()
            assert run(["gateway", "stop"], env_of(a), cwd=a).returncode == 0
            assert gateway.wait(timeout=5) == 0 and not (a / PID_FILE).exists()
        finally:
            if gateway.poll() is None:
                gateway.kill()
                gateway.wait()
            if gateway.stderr:
                gateway.stderr.close()
        # Installed: the launcher runs `…/hermes`, Hermes by its name.
        dest = lab / "standins"
        dest.mkdir()
        shutil.copy2(Path(__file__).resolve().parent / "standin_common.py", dest / "standin_common.py")
        made = install(dest)
        wrapper = Path(made[ENV_BIN])
        assert f'"{dest / "hermes"}"' in wrapper.read_text()
        done = run(["--version"], {"PATH": "/usr/bin:/bin"}, prog=[str(wrapper)])
        assert done.returncode == 0 and done.stdout.startswith(BANNER), done
        done = run(["usage", "--json"], env_of(a), cwd=a, prog=[str(wrapper)])
        assert done.returncode == 0 and json.loads(done.stdout)["provider"] == "openai-codex", done
    print("hermes stand-in: selftest passed")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        sys.exit(selftest())
    sys.exit(main(sys.argv))
