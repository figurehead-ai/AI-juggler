#!/usr/bin/env python3
"""AJ-E1612: a lab stand-in for Kimi Code (`kimi`, 2.5), for a dry run of
the sign-in sitting on AI Juggler Dev. Never the real tool, never a real
sign-in: every value it writes is `lab-not-a-token`, every page it prints
is on `example.invalid`, and it runs only on a lab home.

Dev runs it through the adapter's test override AI_JUGGLER_KIMI_BIN
(`install()` makes `<lab>/standins/bin/kimi`), with KIMI_CODE_HOME set to
the App's folder (crates/juggler-adapters/src/kimi_code.rs). What Dev runs,
and what this answers:

- `kimi --version` (discovery): prints `kimi 2.5.3`; the adapter takes the
  last word and accepts any 2.x as Kimi Code. No lab home needed.
- `kimi login` (Sign in, in a pty, KIMI_CODE_HOME=<App folder>): prints a
  fake device code and its page, waits AIJ_LAB_STANDIN_WAIT seconds as if
  the maintainer approved it, and writes the membership sign-in where 2.5
  does: `credentials/kimi-code-env-<hash>.json` (0600; `<hash>` = the
  first 16 hex of sha256 of the environment's `{oauthHost, baseUrl}`, as
  2.5 derives it, here of made-up hosts), in the OAuth token shape
  (`access_token`, `refresh_token`, `expires_at`, `scope`, `token_type`,
  `expires_in`). The adapter reads it as a subscription (sign_in.rs).
  `kimi logout` removes it.
- `kimi [-m MODEL]` (a new session, in a pty): a prompt; each line is a
  turn answered `lab`. The session is recorded as Kimi Code records it:
  a line in `session_index.jsonl` (`sessionId`, `sessionDir`, `workDir`),
  `sessions/wd_<slug>_<sha256(workDir)[:12]>/<id>/state.json` (`title`,
  `lastPrompt`, `createdAt`, `updatedAt` in epoch ms), its event log
  `agents/main/wire.jsonl` (protocol 1.5: `metadata`, then per turn
  `turn.prompt`, `message.appended`, `usage.record` with a few made-up
  tokens, `turn.ended` `completed`), and its lock
  `locks/sessions/<id>.lock` (`pid`, `startedAt`, `clientIdentity`) while
  it runs. Without a sign-in each turn says so and ends `failed`.
- `kimi --session <id>` (`-S`, `-r`, `--resume`; Dev's resume): prints the
  session's earlier turns from its event log, then appends new turns to it.
- `kimi -p PROMPT --output-format stream-json` (headless; the adapter
  refuses it for a membership account): one turn, one JSON line `lab`.
- `kimi web --no-open --port 0` (usage; the adapter's helper, a cleared
  environment with KIMI_CODE_HOME and a per-run KIMI_CODE_PASSWORD): a
  server on 127.0.0.1 only, its instance record in
  `server/instances/<id>.json` (`server_id`, `pid`, `host`, `port`,
  `started_at`, `heartbeat_at`, `host_version`; heartbeat every 15 s),
  `GET /api/v1/healthz` without a credential, `GET /api/v1/oauth/usage`
  with `Bearer <KIMI_CODE_PASSWORD or server.token>` answering
  `{code: 0, data: {kind: "ok", quota: {usages: {limit5h, limit7d,
  monthTotal, monthCode}, extraUsage: null}}}` with modest made-up ratios
  (a sign-in that isn't the membership's gets `kind: "error"`). It removes
  its record and exits on SIGTERM, or when its parent goes.
- `--selftest`: runs each of the above as a subprocess in a throwaway lab,
  and checks the names written against the adapter's Rust source.

Every run first refuses the network, logs its words to
`<lab>/standins/calls.log`, and stops (exit 3) unless KIMI_CODE_HOME is in a
lab (`--version`/`--help` excepted).

What a dry run on this stand-in cannot prove:
- the real device sign-in, its pages, and whether Kimi accepts the account;
- the real record contents beyond the names the adapter reads (`turn.prompt`
  input, `message.appended`, the state's other fields are this stand-in's
  best reading of 2.5's source), or that 2.5.x still writes protocol 1.5;
- the real `kimi web` (its routes' other answers, its own auth, its call to
  Moonshot) and the real plan's numbers;
- Dev's process recognition: the running process is `python3 …/kimi.py`,
  not `kimi`, so "a kimi process is a session" (`session_process`) isn't
  exercised; the session locks still name the stand-in's pid;
- the real TUI (keys, slash commands, its own resume picker).
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import re
import secrets
import signal
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

TOOL = "kimi"
VERSION = "2.5.3"
ENV_HOME = "KIMI_CODE_HOME"
ENV_BIN = "AI_JUGGLER_KIMI_BIN"
ENV_PASSWORD = "KIMI_CODE_PASSWORD"
CREDENTIALS = "credentials"
MEMBERSHIP_PREFIX = "kimi-code-env-"
INDEX_FILE = "session_index.jsonl"
SESSIONS_DIR = "sessions"
STATE_FILE = "state.json"
AGENTS_DIR = "agents"
MAIN_AGENT = "main"
WIRE_FILE = "wire.jsonl"
LOCKS_DIR = ("locks", "sessions")
PROTOCOL = "1.5"
TOKEN_FILE = "server.token"
INSTANCES = ("server", "instances")
HEALTH = "/api/v1/healthz"
USAGE = "/api/v1/oauth/usage"
USAGES = ("limit5h", "limit7d", "monthTotal", "monthCode")
HEARTBEAT_EVERY = 15.0
MODEL = "lab-model"
# Made-up hosts: only their hash is written (the sign-in file's name).
LAB_ENVIRONMENT = {"oauthHost": f"https://auth.{common.FAKE_HOST}", "baseUrl": f"https://api.{common.FAKE_HOST}/coding/v1"}

HELP = """Usage: kimi [OPTIONS] [COMMAND]   (lab stand-in for Kimi Code 2.5)

Commands:
  login                 sign in (a fake device code; writes a lab sign-in)
  logout                remove the lab sign-in
  web                   the local server (--no-open, --port N)
Options:
  -S, --session <id>    resume a session
  -m, --model <model>   the model
  -p, --prompt <text>   one headless turn (--output-format stream-json)
  -V, --version         print the version
  -h, --help            this help
"""


def dumps(value: object) -> str:
    """One record as Kimi Code writes it: compact, `type` first."""
    return json.dumps(value, separators=(",", ":"))


# ---- the home -------------------------------------------------------------

def lab_home() -> Path:
    return common.require_lab(os.environ.get(ENV_HOME), TOOL, ENV_HOME)


def membership_file(home: Path) -> Path:
    digest = hashlib.sha256(dumps(LAB_ENVIRONMENT).encode()).hexdigest()[:16]
    return home / CREDENTIALS / f"{MEMBERSHIP_PREFIX}{digest}.json"


def signed_in(home: Path) -> bool:
    folder = home / CREDENTIALS
    if not folder.is_dir():
        return False
    return any(path.is_file() and path.stat().st_size > 0 for path in folder.iterdir()
               if path.name == "kimi-code.json" or (path.name.startswith(MEMBERSHIP_PREFIX) and path.name.endswith(".json")))


def any_sign_in(home: Path) -> bool:
    folder = home / CREDENTIALS
    return folder.is_dir() and any(path.is_file() and path.stat().st_size > 0 for path in folder.iterdir())


def work_dir_key(work_dir: str) -> str:
    """`wd_<slug>_<12 hex>`, as 2.5 names a working folder."""
    normalized = work_dir.replace("\\", "/").rstrip("/")
    name = normalized.split("/")[-1] if normalized else normalized
    slug = re.sub(r"[^a-z0-9._-]+", "-", name.lower()).strip("-")[:40].strip("-")
    if slug in ("", ".", ".."):
        slug = "workspace"
    return f"wd_{slug}_{hashlib.sha256(normalized.encode()).hexdigest()[:12]}"


# ---- sign-in --------------------------------------------------------------

def login() -> int:
    home = lab_home()
    print("Kimi Code sign-in (lab stand-in)", flush=True)
    common.fake_sign_in_page(TOOL, "auth.kimi", common.device_code())
    path = membership_file(home)
    common.require_lab(path, TOOL, "the sign-in file")
    path.parent.mkdir(parents=True, exist_ok=True)
    os.chmod(path.parent, 0o700)
    expires = 3600
    common.write_json(path, {
        "access_token": common.NOT_A_TOKEN, "refresh_token": common.NOT_A_TOKEN,
        "expires_at": time.time() + expires, "scope": "kimi-code", "token_type": "Bearer",
        "expires_in": expires,
    }, TOOL)
    print("Signed in to Kimi Code (lab stand-in: a dummy sign-in, not a token).", flush=True)
    return 0


def logout() -> int:
    home = lab_home()
    path = membership_file(home)
    if path.is_file():
        common.require_lab(path, TOOL, "the sign-in file")
        path.unlink()
    print("Signed out.", flush=True)
    return 0


# ---- sessions -------------------------------------------------------------

class Session:
    """One session in the home, as Kimi Code records it."""

    def __init__(self, home: Path, session_id: str, folder: Path, work_dir: str) -> None:
        self.home, self.id, self.dir, self.work_dir = home, session_id, folder, work_dir
        self.log = folder / AGENTS_DIR / MAIN_AGENT / WIRE_FILE
        self.state_path = folder / STATE_FILE
        self.lock = home.joinpath(*LOCKS_DIR) / f"{session_id}.lock"

    @classmethod
    def new(cls, home: Path, work_dir: str) -> "Session":
        session_id = str(_uuid4())
        folder = home / SESSIONS_DIR / work_dir_key(work_dir) / session_id
        session = cls(home, session_id, folder, work_dir)
        now = common.now_ms()
        common.append_line(session.log, dumps({"type": "metadata", "protocol_version": PROTOCOL, "created_at": now}), TOOL)
        common.write_json(session.state_path, {"createdAt": now, "updatedAt": now}, TOOL)
        common.append_line(home / INDEX_FILE, dumps({"sessionId": session_id, "sessionDir": str(folder), "workDir": work_dir}), TOOL)
        return session

    @classmethod
    def find(cls, home: Path, session_id: str) -> "Session | None":
        if not re.fullmatch(r"[A-Za-z0-9_.-]+", session_id) or session_id.startswith("."):
            return None
        work_dir = ""
        index = home / INDEX_FILE
        if index.is_file():
            for line in index.read_text().splitlines():
                try:
                    entry = json.loads(line)
                except ValueError:
                    continue
                if entry.get("sessionId") == session_id and not entry.get("deleted"):
                    work_dir = entry.get("workDir") or work_dir
        root = home / SESSIONS_DIR
        for key in sorted(root.iterdir()) if root.is_dir() else []:
            folder = key / session_id
            if (folder / AGENTS_DIR).is_dir():
                return cls(home, session_id, folder, work_dir)
        return None

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
        """Each earlier turn's prompt and answer, from the event log."""
        prompts: dict[int, str] = {}
        answers: dict[int, str] = {}
        for record in self.records():
            turn = record.get("turnId")
            if record.get("type") == "turn.prompt":
                prompts[turn] = "".join(part.get("text", "") for part in record.get("input", []) if isinstance(part, dict))
            elif record.get("type") == "message.appended":
                message = record.get("message") or {}
                if message.get("role") == "assistant":
                    answers[turn] = "".join(part.get("text", "") for part in message.get("content", []) if isinstance(part, dict))
        return [(prompts[turn], answers.get(turn, "")) for turn in sorted(prompts)]

    def hold(self) -> None:
        common.write_json(self.lock, {
            "version": 1, "token": f"{os.getpid()}:{secrets.token_hex(8)}", "pid": os.getpid(),
            "hostname": "lab-host", "startedAt": common.now_ms(), "clientIdentity": "kimi_code_cli",
            "clientDisplayName": "Kimi Code CLI",
        }, TOOL)

    def release(self) -> None:
        if self.lock.is_file():
            common.require_lab(self.lock, TOOL, "the lock")
            self.lock.unlink()

    def turn(self, text: str, model: str) -> str:
        """Records one turn and returns its answer."""
        turn_id = sum(1 for record in self.records() if record.get("type") == "turn.prompt")
        start = common.now_ms()

        def record(kind: str, **fields: object) -> None:
            common.append_line(self.log, dumps({"type": kind, "agentId": MAIN_AGENT, "turnId": turn_id, **fields,
                                                "time": common.now_ms()}), TOOL)

        record("turn.prompt", input=[{"type": "text", "text": text}], origin={"kind": "user"})
        if signed_in(self.home) or any_sign_in(self.home):
            answer = common.answer_for(text)
            record("message.appended", message={"role": "assistant", "content": [{"type": "text", "text": answer}]})
            common.append_line(self.log, dumps({
                "type": "usage.record", "agentId": MAIN_AGENT, "model": model,
                "usage": {"inputOther": 120, "output": 8, "inputCacheRead": 40, "inputCacheCreation": 0},
                "usageScope": "turn", "turnId": turn_id, "time": common.now_ms()}), TOOL)
            record("turn.ended", reason="completed", durationMs=common.now_ms() - start)
            reason = "completed"
        else:
            answer = "Not signed in. Run `kimi login` first (lab stand-in)."
            record("turn.ended", reason="failed", error={"message": "not signed in"}, durationMs=common.now_ms() - start)
            reason = "failed"
        state = {}
        try:
            state = json.loads(self.state_path.read_text())
        except (OSError, ValueError):
            pass
        state.setdefault("createdAt", start)
        state.setdefault("title", text.strip().splitlines()[0][:80] if text.strip() else "")
        state.update({"lastPrompt": text, "updatedAt": common.now_ms(), "lastTurnReason": reason})
        common.write_json(self.state_path, state, TOOL)
        return answer


def _uuid4() -> str:
    raw = bytearray(secrets.token_bytes(16))
    raw[6] = 0x40 | (raw[6] & 0x0F)
    raw[8] = 0x80 | (raw[8] & 0x3F)
    h = raw.hex()
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"


def interactive(resume: str | None, model: str) -> int:
    home = lab_home()
    if resume is not None:
        session = Session.find(home, resume)
        if session is None:
            common.fail(TOOL, f"no session {resume} in this home", 1)
        print(f"Resumed session {session.id}", flush=True)
        for prompt, answer in session.turns():
            print(f"> {prompt}\n{answer}", flush=True)
    else:
        session = Session.new(home, os.getcwd())
    session.hold()
    banner = f"Kimi Code {VERSION} (lab stand-in) - session {session.id}"
    if not signed_in(home) and not any_sign_in(home):
        banner += "\nNot signed in: run `kimi login`."
    try:
        return common.repl(TOOL, "> ", lambda text: session.turn(text, model), banner)
    finally:
        session.release()


def headless(prompt: str, model: str) -> int:
    home = lab_home()
    session = Session.new(home, os.getcwd())
    answer = session.turn(prompt, model)
    print(dumps({"role": "assistant", "content": answer}), flush=True)
    return 0


# ---- `kimi web` -----------------------------------------------------------

def usage_answer(home: Path) -> dict:
    if not signed_in(home):
        return {"code": 0, "data": {"kind": "error", "status": 401}}
    now = time.time()

    def reset(seconds: float) -> str:
        return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now + seconds))

    return {"code": 0, "data": {"kind": "ok", "quota": {"usages": {
        "limit5h": {"usedRatio": 0.12, "resetAt": reset(3 * 3600)},
        "limit7d": {"usedRatio": 0.31, "resetAt": reset(4 * 86400)},
        "monthTotal": {"usedRatio": 0.08, "resetAt": reset(20 * 86400)},
        "monthCode": {"usedRatio": 0.05},
    }, "extraUsage": None}}}


def web(args: list[str]) -> int:
    import http.server

    home = lab_home()
    port = 0
    if "--port" in args:
        at = args.index("--port")
        try:
            port = int(args[at + 1])
        except (IndexError, ValueError):
            common.fail(TOOL, "--port takes a number", 2)
    token_path = home / TOKEN_FILE
    if not token_path.is_file():
        common.require_lab(token_path, TOOL, "the server token")
        token_path.write_text(f"{common.NOT_A_TOKEN}-server-{secrets.token_hex(4)}\n")
        os.chmod(token_path, 0o600)
    accepted = {token_path.read_text().strip()}
    if os.environ.get(ENV_PASSWORD):
        accepted.add(os.environ[ENV_PASSWORD])

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_: object) -> None:  # quiet
            pass

        def answer(self, status: int, body: dict) -> None:
            data = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:  # noqa: N802
            path = self.path.split("?", 1)[0]
            if path == HEALTH:
                return self.answer(200, {"code": 0, "data": {"ok": True}})
            header = self.headers.get("Authorization", "")
            if not header.startswith("Bearer ") or header[7:] not in accepted:
                return self.answer(401, {"code": 401, "message": "unauthorized"})
            if path == USAGE:
                return self.answer(200, usage_answer(home))
            return self.answer(404, {"code": 404, "message": "not in the lab stand-in"})

    server = http.server.HTTPServer(("127.0.0.1", port), Handler)
    server.timeout = 0.5
    bound = server.server_address[1]
    server_id = "01LAB" + secrets.token_hex(10).upper()
    record = home.joinpath(*INSTANCES) / f"{server_id}.json"
    started = common.now_ms()

    def beat() -> None:
        common.write_json(record, {"server_id": server_id, "pid": os.getpid(), "host": "127.0.0.1", "port": bound,
                                   "started_at": started, "heartbeat_at": common.now_ms(), "host_version": VERSION}, TOOL)

    def stop(*_: object) -> None:
        raise SystemExit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGHUP, stop)
    parent = os.getppid()
    beat()
    print(f"Kimi Code web (lab stand-in) on http://127.0.0.1:{bound}", flush=True)
    last = time.monotonic()
    try:
        while os.getppid() == parent:
            server.handle_request()
            if time.monotonic() - last >= HEARTBEAT_EVERY:
                beat()
                last = time.monotonic()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        if record.is_file():
            record.unlink()
    return 0


# ---- the command line -----------------------------------------------------

def main(argv: list[str]) -> int:
    common.no_network(TOOL)
    common.log_call(TOOL, argv)
    if argv[:1] == ["--selftest"]:
        return selftest()
    if any(word in ("-V", "--version") for word in argv):
        print(f"kimi {VERSION}")
        return 0
    if any(word in ("-h", "--help") for word in argv):
        print(HELP)
        return 0
    model, resume, prompt = MODEL, None, None
    words = list(argv)
    rest: list[str] = []
    while words:
        word = words.pop(0)
        if word in ("-m", "--model") and words:
            model = words.pop(0)
        elif word in ("-S", "--session", "-r", "--resume") and words:
            resume = words.pop(0)
        elif word in ("-p", "--prompt") and words:
            prompt = words.pop(0)
        elif word == "--output-format" and words:
            words.pop(0)
        else:
            rest.append(word)
    if rest[:1] == ["login"]:
        return login()
    if rest[:1] == ["logout"]:
        return logout()
    if rest[:1] == ["web"]:
        return web(rest[1:])
    if rest and not rest[0].startswith("-"):
        common.fail(TOOL, f"`kimi {rest[0]}` isn't part of the lab stand-in", 2)
    if prompt is not None:
        return headless(prompt, model)
    return interactive(resume, model)


# ---- selftest -------------------------------------------------------------

def static_check() -> list[str]:
    """The names this writes still match the adapter's Rust source."""
    root = HERE.parents[2]
    src = root / "crates/juggler-adapters/src"
    if not src.is_dir():
        print("kimi selftest: no Rust source beside this copy; the static check is skipped")
        return []
    needles = {
        "kimi_code.rs": [f'ENV_HOME: &str = "{ENV_HOME}"', f'"{ENV_BIN}"', 'args: vec!["login".into()]',
                         'args.extend(["--session".into(), session_id.clone()])', '"-p".into()',
                         '"stream-json".into()', '"-m".into()', "major >= 2", '"--session", "-S", "--resume", "-r"'],
        "sign_in.rs": [f'KIMI_CREDENTIALS: &str = "{CREDENTIALS}"', f'KIMI_MEMBERSHIP_ENV_PREFIX: &str = "{MEMBERSHIP_PREFIX}"'],
        "kimi_sessions.rs": [f'INDEX_FILE: &str = "{INDEX_FILE}"', f'SESSIONS_DIR: &str = "{SESSIONS_DIR}"',
                             f'STATE_FILE: &str = "{STATE_FILE}"', 'rename = "sessionId"', 'rename = "sessionDir"',
                             'rename = "workDir"', '"title"', '"lastPrompt"', '"createdAt"', '"updatedAt"'],
        "kimi_sessions/wire.rs": [f'AGENTS_DIR: &str = "{AGENTS_DIR}"', f'MAIN_AGENT: &str = "{MAIN_AGENT}"',
                                  f'WIRE_FILE: &str = "{WIRE_FILE}"',
                                  f'LOCKS_DIR: [&str; 2] = ["{LOCKS_DIR[0]}", "{LOCKS_DIR[1]}"]',
                                  f'KNOWN_PROTOCOLS: &[&str] = &["{PROTOCOL}"]', '"metadata"', '"turn.prompt"',
                                  '"turn.ended"', '"usage.record"', 'Some("completed")', 'b"{\\"type\\":\\""',
                                  "input_other", "input_cache_read", "input_cache_creation", "started_at",
                                  "client_identity", 'rename_all = "camelCase"', "protocol_version", "created_at"],
        "kimi_usage.rs": [f'INSTANCES: [&str; 2] = ["{INSTANCES[0]}", "{INSTANCES[1]}"]', f'TOKEN_FILE: &str = "{TOKEN_FILE}"',
                          f'HEALTH: &str = "{HEALTH}"', f'USAGE: &str = "{USAGE}"',
                          "&[" + ", ".join(f'"{key}"' for key in USAGES) + "]",
                          '.args(["web", "--no-open", "--port", "0"])', f'ENV_PASSWORD: &str = "{ENV_PASSWORD}"',
                          '"usedRatio"', '"resetAt"', '"extraUsage"', 'rename = "server_id"', "heartbeat_at",
                          "host_version", "HEARTBEAT_FRESH_MS: i64 = 60_000", 'Some("ok")', '"code"', '"quota"', '"usages"'],
    }
    missing = []
    for name, wanted in needles.items():
        text = (src / name).read_text()
        missing += [f"{name}: {needle}" for needle in wanted if needle not in text]
    return missing


def selftest() -> int:
    import urllib.error
    import urllib.request

    failures: list[str] = []

    def check(ok: bool, what: str) -> None:
        if not ok:
            failures.append(what)

    common.selftest_common()
    failures += [f"the Rust source changed: {line}" for line in static_check()]
    me = str(Path(__file__).resolve())
    with common.selftest_lab(TOOL) as lab:
        home = lab / "apps" / "kimi-a"
        project = lab / "projects" / "shop"
        home.mkdir(parents=True)
        project.mkdir(parents=True)
        base = {"PATH": "/usr/bin:/bin", "AIJ_LAB_STANDIN_WAIT": "0", ENV_HOME: str(home), "HOME": str(lab)}

        def run(args: list[str], env: dict | None = None, text: str = "", cwd: Path = project) -> subprocess.CompletedProcess:
            return subprocess.run(["/usr/bin/python3", "-I", me, *args], input=text, capture_output=True, text=True,
                                  env=base if env is None else env, cwd=cwd, timeout=60)

        # Version, as discovery reads it (the last word; 2.x and later).
        done = run(["--version"], env={"PATH": "/usr/bin:/bin"})
        word = done.stdout.splitlines()[0].split()[-1] if done.stdout.strip() else ""
        check(done.returncode == 0 and int(word.lstrip("v").split(".")[0]) >= 2, f"--version: {done.stdout!r}")
        # Never outside a lab.
        check(run([], env={**base, ENV_HOME: "/Users/someone/.kimi-code"}).returncode == 3, "a real home was accepted")
        check(run([], env={"PATH": "/usr/bin:/bin"}).returncode == 3, "no KIMI_CODE_HOME was accepted")
        # Before a sign-in: a turn says so and ends failed.
        done = run([], text="hi\n/exit\n")
        check("Not signed in" in done.stdout, f"unsigned turn: {done.stdout!r}")
        # Sign-in: the membership's file, where 2.5 writes it.
        done = run(["login"])
        files = list((home / CREDENTIALS).glob(f"{MEMBERSHIP_PREFIX}*.json"))
        check(done.returncode == 0 and len(files) == 1 and common.FAKE_HOST in done.stdout, f"login: {done.stdout!r} {files}")
        if files:
            name = files[0].name
            stem = name[len(MEMBERSHIP_PREFIX):-len(".json")]
            check(re.fullmatch(r"[0-9a-f]{16}", stem) is not None, f"the sign-in's name: {name}")
            body = json.loads(files[0].read_text())
            check(body.get("access_token") == common.NOT_A_TOKEN and oct(files[0].stat().st_mode & 0o777) == "0o600",
                  "the sign-in's contents or mode")
        check(not (home / CREDENTIALS / "kimi-code.json").exists(), "an older-name sign-in was written")
        # A session: listed, its turn answered and ended, its lock gone after.
        index_before = (home / INDEX_FILE).read_text().splitlines()
        done = run([], text="hello\n/exit\n")
        check(done.returncode == 0 and answers(done.stdout) == 1, f"session: {done.stdout!r} {done.stderr!r}")
        lines = [json.loads(line) for line in (home / INDEX_FILE).read_text().splitlines()]
        check(len(lines) == len(index_before) + 1, "the index line")
        entry = lines[-1]
        session_id = entry["sessionId"]
        folder = Path(entry["sessionDir"])
        check(entry["workDir"] == os.path.realpath(project), f"workDir: {entry}")
        check(folder.parent.name == work_dir_key(entry["workDir"]) and folder.parent.parent == home / SESSIONS_DIR,
              f"the session's folder: {folder}")
        state = json.loads((folder / STATE_FILE).read_text())
        check(state.get("title") == "hello" and isinstance(state.get("createdAt"), int), f"state.json: {state}")
        log = folder / AGENTS_DIR / MAIN_AGENT / WIRE_FILE
        raw = log.read_text().splitlines()
        records = [json.loads(line) for line in raw]
        check(records[0] == {"type": "metadata", "protocol_version": PROTOCOL, "created_at": records[0].get("created_at")},
              "the log's version line")
        check(all(line.startswith('{"type":"') for line in raw), "a record doesn't open with its type")
        check(wire_status(records) == "idle", "the turn's end wasn't read")
        check(any(r["type"] == "usage.record" and r["usage"]["inputOther"] > 0 for r in records), "no usage record")
        check(not home.joinpath(*LOCKS_DIR, f"{session_id}.lock").exists(), "the lock stayed")
        # Resume: the earlier turn shown, a new one appended to the same log.
        done = run(["--session", session_id], text="again\n/exit\n")
        check("> hello" in done.stdout and answers(done.stdout) == 2, f"resume: {done.stdout!r}")
        prompts = [r for r in map(json.loads, log.read_text().splitlines()) if r["type"] == "turn.prompt"]
        check([p["turnId"] for p in prompts] == [0, 1] and len((home / INDEX_FILE).read_text().splitlines()) == len(lines),
              "the resumed turn's record")
        check(run(["--session", "no-such-session"], text="").returncode == 1, "an unknown session resumed")
        # Usage: `kimi web` as the adapter starts it (a cleared environment).
        secret = secrets.token_hex(16)
        server = subprocess.Popen(["/usr/bin/python3", "-I", me, "web", "--no-open", "--port", "0"],
                                  env={"PATH": "/usr/bin:/bin", ENV_HOME: str(home), ENV_PASSWORD: secret},
                                  cwd=home, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            port = 0
            for _ in range(200):
                for path in home.joinpath(*INSTANCES).glob("*.json") if home.joinpath(*INSTANCES).is_dir() else []:
                    info = json.loads(path.read_text())
                    if info.get("pid") == server.pid and info.get("port"):
                        port = info["port"]
                        check(info["host"] == "127.0.0.1" and abs(info["heartbeat_at"] - common.now_ms()) < 60_000,
                              f"the instance record: {info}")
                if port or server.poll() is not None:
                    break
                time.sleep(0.05)
            check(port != 0, "kimi web wrote no instance record")
            if port:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}{HEALTH}", timeout=5) as answer:
                    check(answer.status == 200, "healthz")
                ask = urllib.request.Request(f"http://127.0.0.1:{port}{USAGE}", headers={"Authorization": f"Bearer {secret}"})
                with urllib.request.urlopen(ask, timeout=5) as answer:
                    body = json.loads(answer.read())
                check(usage_reads(body) == {"limit5h", "limit7d", "monthTotal", "monthCode"}, f"usage: {body}")
                try:
                    urllib.request.urlopen(f"http://127.0.0.1:{port}{USAGE}", timeout=5)
                    check(False, "usage without a credential")
                except urllib.error.HTTPError as refused:
                    check(refused.code == 401, "usage without a credential")
        finally:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
        check(not list(home.joinpath(*INSTANCES).glob("*.json")), "kimi web left its record")
        # Sign-out.
        check(run(["logout"]).returncode == 0 and not list((home / CREDENTIALS).glob(f"{MEMBERSHIP_PREFIX}*")), "logout")
    for line in failures:
        print(f"FAILED: {line}", file=sys.stderr)
    print("kimi stand-in: selftest " + ("FAILED" if failures else "passed"))
    return 1 if failures else 0


def answers(output: str) -> int:
    """How many answers (`lab`) a terminal printed."""
    return sum(1 for line in output.splitlines() if line.removeprefix("> ").strip() == common.answer_for(""))


def wire_status(records: list[dict]) -> str:
    """What the adapter's reader says of a log (protocol, then turn open)."""
    if not records or records[0].get("type") != "metadata" or records[0].get("protocol_version") != PROTOCOL:
        return "unknown"
    turn = None
    for record in records[1:]:
        if record.get("type") == "turn.prompt":
            turn = record.get("time")
        elif record.get("type") == "turn.ended":
            turn = None
    return "busy" if turn is not None else "idle"


def usage_reads(body: dict) -> set[str]:
    """The windows the adapter's `parse_answer` would take (empty when it
    would refuse the answer)."""
    if body.get("code") != 0 or (body.get("data") or {}).get("kind") != "ok":
        return set()
    quota = body["data"].get("quota") or {}
    usages = quota.get("usages") or {}
    if any(key not in USAGES for key in usages) or quota.get("extraUsage") is not None:
        return set()
    for entry in usages.values():
        ratio = entry.get("usedRatio")
        if not isinstance(ratio, (int, float)) or not 0 <= ratio <= 1.05:
            return set()
        reset = entry.get("resetAt")
        if reset is not None and not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", reset):
            return set()
    return set(usages)


def install(dest: Path) -> dict[str, str]:
    wrapper = common.install_wrapper(dest, "kimi", Path(__file__).name)
    return {ENV_BIN: str(wrapper)}


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
