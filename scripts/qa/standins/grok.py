#!/usr/bin/env python3
"""AJ-E1612: a lab stand-in for Grok Build 1.0.77 (`grok`), for a dry run of
the sign-in sitting on AI Juggler Dev. Never the real tool: it reaches no
network, binds no port, writes only inside a lab (`/tmp/aij-lab…`) and every
sign-in value it writes is `lab-not-a-token`.

Dev runs it through `AI_JUGGLER_GROK_BIN` (install.py; the launcher is a
`#!/bin/sh` script, so the adapter's `is_node_script` doesn't turn it
away), with `GROK_HOME=<App folder>` and the `GROK_CLAUDE_*`/`GROK_CURSOR_*`
switches at 0 (grok_build.rs). Every run but `--version`/`--help` stops
(exit 3) unless the home (`GROK_HOME`, or `$HOME/.grok` when unset, as for
an App used in place) lies in a lab.

What Dev runs, and what the stand-in does:

- `grok --version` (discovery): `grok 1.0.77 (lab-standin)`.
- `grok login` (login_spec, in a pty, cwd = the App folder): prints a fake
  sign-in address on example.invalid and "Waiting for authorization...",
  waits (AIJ_LAB_STANDIN_WAIT), writes `<GROK_HOME>/auth.json` as one entry
  keyed by the (fake) sign-in address, `{"key", "auth_mode": "web_login",
  "refresh_token", "email"}` (the shape of the adapter's fixture; only
  `auth_mode` is read), and says it signed in. `--device-auth`
  (`--device-code`) shows a fake approval page and code instead; `--oauth`
  is the default here.
- `grok [--model <m>] [--allow …|--deny …|--rules …] [-- <message>]`
  (launch_spec Interactive / Primed, in a pty): a banner and a `> ` prompt;
  each line is a turn answered `lab`. The session is made at start (a
  UUIDv7, or `-s/--session-id`) in `<GROK_HOME>/sessions/<url-encoded
  cwd>/<id>/`: `summary.json` (title = the first message's first line,
  `created_at`/`updated_at` ISO times, `current_model`, `num_messages`,
  `cwd`; rewritten in place, never replaced, so its birth time stays the
  session's start), `updates.jsonl` (the ACP stream as 1.0.77 writes it:
  `session/update` lines with `user_message_chunk` and
  `agent_message_chunk`, then an `_x.ai/session/update` `turn_completed`
  with `stop_reason: "end_turn"`, the line the adapter reads as the turn's
  end) and `signals.json`. Without a sign-in it says to run `grok login`
  first and exits 1 (lab B).
- `grok --resume <id> [-- <message>]` (launch_spec Resume; `-r`, an id or a
  title; `-c/--continue` for the newest here): prints the session's earlier
  turns back from `updates.jsonl`, then appends new turns to the same
  files; an unknown one exits 1.
- `grok -p <text> --output-format json` (launch_spec Headless; also plain,
  streaming-json): one turn in a new session (or `-r`'s), printed as a
  result object / JSON lines / the answer.
- `grok usage <id> [turn]`: the session's per-turn and total tokens and
  cost; `grok usage [--json]` alone: a made-up plan line (the real one asks
  Grok's background service). `grok sessions list`: this folder's sessions.
- Usage, as Dev reads it: nothing. The adapter (`impl ToolQuota for
  GrokBuildAdapter {}`, grok_sessions.rs "signals.json isn't read") reads no
  Grok usage, so the App's Usage should name no Grok source; the run
  sheet's question "whether its token reading comes from signals.json" is
  answered no. The stand-in still writes `signals.json` with the research's
  counters (`turn_count`, `tool_call_count`, `error_count`, `cancel_count`,
  `context_tokens_used`, `context_window_tokens`) and running token totals
  (`input_tokens`, `cached_input_tokens`, `output_tokens`,
  `total_cost_usd`: names seen in the 1.0.77 binary, their place a guess),
  modest made-up numbers (about 1.2k in, 8 out per turn); each
  `turn_completed` carries that turn's `usage`, which `grok usage` sums.

`--selftest` runs all of that in a throwaway lab, as subprocesses with the
adapter's variables, reads the files the way grok_sessions.rs does, and
checks the names it writes against the Rust source.

What a dry run on this stand-in cannot prove:

- xAI's real sign-in (its browser loopback or device code, the welcome
  screen's in-TUI sign-in) and the real auth.json key (the real one keys by
  xAI's own sign-in address; here an example.invalid one);
- the full-screen TUI, its keys and /commands, how Dev's typed text lands
  in it; model replies, tools, plans, sub-agents, titles (the real one may
  name a session otherwise);
- the real `summary.json`/`signals.json` field names beyond the ones the
  adapter reads (the docs give no schema); `chat_history.jsonl`,
  `active_sessions.json` and the other session files aren't written;
  long folder names aren't shortened (slug + hash + `.cwd`);
- `grok usage`'s and the headless result's real layout;
- live process detection: the process is `python3 …/grok.py`, not a
  program named `grok`, so Dev's running-session match by name sees it
  only through the session it launched itself.
"""
from __future__ import annotations

import importlib.util
import json
import os
import re
import secrets
import sys
import time
import uuid
from pathlib import Path
from typing import Dict, List, Optional, Tuple

HERE = Path(__file__).resolve().parent
TOOL = "grok"
VERSION = "1.0.77"


def _load_common():
    spec = importlib.util.spec_from_file_location("standin_common", HERE / "standin_common.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


C = _load_common()

# The names the adapter reads (checked against the Rust source in --selftest).
AUTH_FILE = "auth.json"
SESSIONS_DIR = "sessions"
SUMMARY_FILE = "summary.json"
UPDATES_FILE = "updates.jsonl"
SIGNALS_FILE = "signals.json"
DEFAULT_MODEL = "grok-build-0.1"
CONTEXT_WINDOW = 256000

SUBCOMMANDS = ("agent", "clone", "completions", "dashboard", "doctor", "du", "disk-usage", "export", "help",
               "inspect", "leader", "login", "logout", "mcp", "memory", "models", "plugin", "sessions", "setup",
               "trace", "update", "usage", "version", "v", "worker", "worktree", "wrap")
VALUE_OPTIONS = {"--add-dir", "--agent", "--agents", "--allow", "--allowedTools", "--allowed-tools", "--cwd",
                 "--debug-file", "--deny", "--disallowedTools", "--disallowed-tools", "--json-schema",
                 "--leader-socket", "-m", "--model", "--max-turns", "--output-format", "-p", "--single",
                 "--permission-mode", "--prompt-file", "--prompt-json", "--reasoning-effort", "--effort", "--rules",
                 "-s", "--session-id", "--sandbox", "--system-prompt-override", "--system-prompt", "--tools",
                 "--worktree-ref", "--ref"}
OPTIONAL_VALUE = {"-r", "--resume", "-w", "--worktree"}


# --------------------------------------------------------------- the lab


def grok_home() -> Path:
    value = os.environ.get("GROK_HOME")
    if not value:
        home = os.environ.get("HOME", "")
        value = os.path.join(home, ".grok") if home else ""
    return C.require_lab(value, TOOL, "GROK_HOME")


def write_in_place(path: Path, value: object) -> None:
    """Writes `value` over `path` in place (same file, so its birth time is
    kept), in a lab only."""
    C.require_lab(path, TOOL, "the file")
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(value, indent=2)
    with open(path, "r+" if path.exists() else "w") as handle:
        handle.seek(0)
        handle.write(text)
        handle.truncate()


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text())
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def iso_now() -> str:
    now = time.time()
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(now)) + f".{int(now * 1000) % 1000:03d}Z"


def uuid7() -> str:
    ms = C.now_ms() & ((1 << 48) - 1)
    rand = secrets.randbits(74)
    value = (ms << 80) | (0x7 << 76) | ((rand >> 62) << 64) | (0b10 << 62) | (rand & ((1 << 62) - 1))
    return str(uuid.UUID(int=value))


def folder_name(cwd: str) -> str:
    """The cwd, URL-encoded as grok_sessions.rs `folder_name` writes it."""
    out = []
    for byte in cwd.encode():
        char = chr(byte)
        out.append(char if (char.isascii() and char.isalnum()) or char in "-._~" else f"%{byte:02X}")
    return "".join(out)


def signed_in(home: Path) -> bool:
    modes = [entry.get("auth_mode") for entry in read_json(home / AUTH_FILE).values() if isinstance(entry, dict)]
    return any(mode in ("web_login", "oidc", "external", "api_key") for mode in modes)


# --------------------------------------------------------------- sign-in


def login(args: List[str]) -> int:
    home = grok_home()
    device = any(arg in ("--device-auth", "--device-code") for arg in args)
    state = secrets.token_hex(12)
    if device:
        code = C.device_code()
        print(f"Open this URL in your browser to approve: https://accounts.{C.FAKE_HOST}/device", flush=True)
        print(f"Code: {code}", flush=True)
        waiting = "Waiting for approval"
    else:
        print(f"Open this URL to sign in to Grok: https://accounts.{C.FAKE_HOST}/sign-in?state={state}", flush=True)
        waiting = "Waiting for authorization..."
    print("(lab stand-in: nothing is opened or listened on; waiting as if you signed in)", flush=True)
    print(waiting, flush=True)
    try:
        time.sleep(C.wait_seconds())
    except KeyboardInterrupt:
        print("\nSign-in cancelled.", flush=True)
        return 130
    auth = read_json(home / AUTH_FILE)
    auth[f"https://accounts.{C.FAKE_HOST}/sign-in"] = {
        "key": C.NOT_A_TOKEN, "auth_mode": "web_login", "refresh_token": C.NOT_A_TOKEN,
        "email": f"lab@{C.FAKE_HOST}"}
    C.write_json(home / AUTH_FILE, auth, TOOL)
    print("Signed in to Grok.", flush=True)
    return 0


# --------------------------------------------------------------- sessions


class Session:
    """One Grok Build session folder."""

    def __init__(self, folder: Path, session_id: str) -> None:
        self.folder = folder
        self.id = session_id

    @classmethod
    def create(cls, home: Path, cwd: str, model: str, session_id: Optional[str]) -> "Session":
        group = home / SESSIONS_DIR / folder_name(cwd)
        sid = session_id or uuid7()
        session = cls(group / sid, sid)
        C.require_lab(session.folder, TOOL, "the session folder")
        session.folder.mkdir(parents=True, exist_ok=True)
        now = iso_now()
        write_in_place(session.folder / SUMMARY_FILE, {
            "session_id": sid, "title": None, "cwd": cwd, "created_at": now, "updated_at": now,
            "current_model": model, "num_messages": 0})
        write_in_place(session.folder / SIGNALS_FILE, {
            "turn_count": 0, "tool_call_count": 0, "error_count": 0, "cancel_count": 0,
            "context_tokens_used": 0, "context_window_tokens": CONTEXT_WINDOW,
            "input_tokens": 0, "cached_input_tokens": 0, "output_tokens": 0, "total_cost_usd": 0.0})
        (session.folder / UPDATES_FILE).touch()
        return session

    @staticmethod
    def all(home: Path) -> List["Session"]:
        out = []
        root = home / SESSIONS_DIR
        if root.is_dir():
            for group in root.iterdir():
                if group.is_dir():
                    out.extend(Session(folder, folder.name) for folder in group.iterdir()
                               if (folder / SUMMARY_FILE).is_file())
        return out

    @classmethod
    def find(cls, home: Path, wanted: str) -> Optional["Session"]:
        sessions = cls.all(home)
        by_id = [session for session in sessions if session.id == wanted]
        if by_id:
            return by_id[0]
        by_title = [session for session in sessions if session.summary().get("title") == wanted]
        return max(by_title, key=lambda s: s.summary().get("updated_at", "")) if by_title else None

    def summary(self) -> dict:
        return read_json(self.folder / SUMMARY_FILE)

    def updates(self) -> List[dict]:
        out = []
        try:
            lines = (self.folder / UPDATES_FILE).read_text().splitlines()
        except OSError:
            return out
        for line in lines:
            try:
                out.append(json.loads(line))
            except ValueError:
                continue
        return out

    def earlier(self) -> List[Tuple[str, str]]:
        out = []
        for line in self.updates():
            update = line.get("params", {}).get("update", {})
            kind = update.get("sessionUpdate")
            if kind in ("user_message_chunk", "agent_message_chunk"):
                out.append(("user" if kind == "user_message_chunk" else "agent", update.get("content", {}).get("text", "")))
        return out

    def _append(self, method: str, update: dict) -> None:
        path = self.folder / UPDATES_FILE
        count = len(self.updates())
        line = {"timestamp": int(time.time()), "method": method,
                "params": {"sessionId": self.id, "update": update, "_meta": {"eventId": f"{self.id}-{count + 1}"}}}
        C.append_line(path, json.dumps(line, separators=(",", ":")), TOOL)

    def turn(self, text: str, model: Optional[str] = None) -> str:
        self._append("session/update", {"sessionUpdate": "user_message_chunk", "content": {"type": "text", "text": text}})
        summary = self.summary()
        if not summary.get("title"):
            summary["title"] = text.strip().splitlines()[0][:80] if text.strip() else None
        summary["updated_at"] = iso_now()
        summary["num_messages"] = int(summary.get("num_messages") or 0) + 1
        write_in_place(self.folder / SUMMARY_FILE, summary)
        if C.wait_seconds() > 0:
            time.sleep(0.4)
        answer = C.answer_for(text)
        self._append("session/update", {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": answer}})
        usage = {"input_tokens": 1180 + secrets.randbelow(80), "cached_input_tokens": 640, "output_tokens": 8,
                 "cost_usd": 0.0012}
        self._append("_x.ai/session/update", {"sessionUpdate": "turn_completed", "stop_reason": "end_turn", "usage": usage})
        signals = read_json(self.folder / SIGNALS_FILE)
        signals["turn_count"] = int(signals.get("turn_count") or 0) + 1
        for key in ("input_tokens", "cached_input_tokens", "output_tokens"):
            signals[key] = int(signals.get(key) or 0) + usage[key]
        signals["total_cost_usd"] = round(float(signals.get("total_cost_usd") or 0) + usage["cost_usd"], 6)
        signals["context_tokens_used"] = usage["input_tokens"] + usage["output_tokens"]
        signals.setdefault("context_window_tokens", CONTEXT_WINDOW)
        write_in_place(self.folder / SIGNALS_FILE, signals)
        summary = self.summary()
        summary["updated_at"] = iso_now()
        summary["num_messages"] = int(summary.get("num_messages") or 0) + 1
        if model:
            summary["current_model"] = model
        write_in_place(self.folder / SUMMARY_FILE, summary)
        return answer

    def turn_usage(self) -> List[dict]:
        return [line["params"]["update"].get("usage", {}) for line in self.updates()
                if line.get("params", {}).get("update", {}).get("sessionUpdate") == "turn_completed"]


# --------------------------------------------------------------- the command line


def parse(argv: List[str]) -> Tuple[Dict[str, List[Optional[str]]], List[str], Optional[str], List[str]]:
    """Options (name → values), flags-and-positionals before a subcommand,
    the subcommand (or None) and its words / the message words."""
    options: Dict[str, List[Optional[str]]] = {}
    positional: List[str] = []
    at = 0
    while at < len(argv):
        arg = argv[at]
        if arg == "--":
            return options, positional, None, argv[at + 1:]
        name, eq, inline = arg.partition("=") if arg.startswith("--") else (arg, "", "")
        if name in VALUE_OPTIONS:
            value = inline if eq else (argv[at + 1] if at + 1 < len(argv) else "")
            at += 0 if eq else 1
            options.setdefault(name, []).append(value)
        elif name in OPTIONAL_VALUE:
            if eq:
                value: Optional[str] = inline
            elif at + 1 < len(argv) and not argv[at + 1].startswith("-"):
                value = argv[at + 1]
                at += 1
            else:
                value = None
            options.setdefault(name, []).append(value)
        elif arg.startswith("-") and arg != "-":
            options.setdefault(arg, []).append(None)
        elif not positional and arg in SUBCOMMANDS:
            return options, positional, arg, argv[at + 1:]
        else:
            positional.append(arg)
        at += 1
    return options, positional, None, []


def first(options: Dict[str, List[Optional[str]]], *names: str) -> Tuple[bool, Optional[str]]:
    for name in names:
        if name in options:
            return True, options[name][-1]
    return False, None


def newest_here(home: Path, cwd: str) -> Optional[Session]:
    here = [session for session in Session.all(home) if os.path.realpath(session.summary().get("cwd", "")) == os.path.realpath(cwd)]
    return max(here, key=lambda s: s.summary().get("updated_at", "")) if here else None


def session_main(options: Dict[str, List[Optional[str]]], positional: List[str], message_words: List[str]) -> int:
    home = grok_home()
    _, cwd_option = first(options, "--cwd")
    if cwd_option:
        os.chdir(cwd_option)
    cwd = os.getcwd()
    _, model = first(options, "--model", "-m")
    model = model or DEFAULT_MODEL
    message = " ".join(positional + message_words).strip() or None
    headless, single = first(options, "-p", "--single")
    _, fmt = first(options, "--output-format")
    if not signed_in(home):
        if headless:
            print("Error: not signed in. Run `grok login` first.", file=sys.stderr)
        else:
            print("Welcome to Grok Build (lab stand-in). You're not signed in: run `grok login`, then start Grok again.",
                  flush=True)
        return 1
    resuming, wanted = first(options, "--resume", "-r")
    continuing, _ = first(options, "--continue", "-c")
    session: Optional[Session] = None
    if resuming or continuing:
        session = Session.find(home, wanted) if wanted else newest_here(home, cwd)
        if session is None:
            print(f"Error: no session found matching '{wanted or cwd}'", file=sys.stderr)
            return 1
    if headless:
        prompt = (single or "") + (" " + message if message else "")
        if not prompt.strip():
            print("Error: -p needs a prompt", file=sys.stderr)
            return 1
        _, chosen = first(options, "--session-id", "-s")
        session = session or Session.create(home, cwd, model, chosen)
        answer = session.turn(prompt.strip(), model)
        usage = session.turn_usage()[-1]
        if fmt == "json":
            print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "result": answer,
                              "session_id": session.id, "num_turns": 1, "usage": usage,
                              "total_cost_usd": usage["cost_usd"]}))
        elif fmt in ("streaming-json", "streaming-messages-json"):
            print(json.dumps({"type": "message", "role": "assistant", "text": answer, "session_id": session.id}))
            print(json.dumps({"type": "result", "result": answer, "session_id": session.id, "usage": usage}))
        else:
            print(answer)
        return 0
    if session is None:
        _, chosen = first(options, "--session-id", "-s")
        session = Session.create(home, cwd, model, chosen)
    print(f"Grok Build {VERSION} (lab stand-in: answers every message with `lab`; /exit to quit)", flush=True)
    print(f"Session {session.id}", flush=True)
    for role, text in session.earlier():
        print(f"{'>' if role == 'user' else ' '} {text}", flush=True)
    if message:
        print(f"> {message}", flush=True)
        print(session.turn(message, model), flush=True)
    return C.repl(TOOL, "> ", lambda text: session.turn(text, model))


def usage_main(words: List[str]) -> int:
    home = grok_home()
    positional = [word for word in words if not word.startswith("-")]
    if not positional:
        plan = {"plan": "lab", "used_percent": 3, "window": "weekly", "resets_at": iso_now()}
        if "--json" in words:
            print(json.dumps(plan))
        else:
            print("Plan usage (lab stand-in, made up): 3% of the weekly limit used")
        return 0
    session = Session.find(home, positional[0])
    if session is None:
        print(f"Error: no session found matching '{positional[0]}'", file=sys.stderr)
        return 1
    turns = session.turn_usage()
    picked = range(len(turns))
    if len(positional) > 1:
        if not positional[1].isdigit() or not 1 <= int(positional[1]) <= len(turns):
            print(f"Error: no turn {positional[1]} in session {session.id}", file=sys.stderr)
            return 1
        picked = range(int(positional[1]) - 1, int(positional[1]))
    print(f"Session {session.id}")
    for at in picked:
        turn = turns[at]
        print(f"  Turn {at + 1}: input {turn['input_tokens']:,} (cached {turn['cached_input_tokens']:,}) · "
              f"output {turn['output_tokens']:,} · ${turn['cost_usd']:.4f}")
    total = {key: sum(turns[at][key] for at in picked) for key in ("input_tokens", "cached_input_tokens", "output_tokens", "cost_usd")}
    print(f"Total: input {total['input_tokens']:,} (cached {total['cached_input_tokens']:,}) · "
          f"output {total['output_tokens']:,} · ${total['cost_usd']:.4f}")
    return 0


def sessions_main(words: List[str]) -> int:
    home = grok_home()
    if words[:1] not in (["list"], ["search"]):
        print(f"lab stand-in: `grok sessions {' '.join(words)}` isn't emulated", file=sys.stderr)
        return 2
    for session in sorted(Session.all(home), key=lambda s: s.summary().get("updated_at", ""), reverse=True):
        summary = session.summary()
        print(f"{session.id}  {summary.get('updated_at', '')}  {summary.get('title') or '(untitled)'}  {summary.get('cwd', '')}")
    return 0


HELP = f"""Grok Build TUI (lab stand-in {VERSION} for AJ-E1612)

Usage: grok [OPTIONS] [PROMPT] [COMMAND]

Options: -m/--model, -p/--single, --output-format, -r/--resume [ID], -c/--continue,
         -s/--session-id, --cwd, --allow, --deny, --rules (accepted), -h/--help, -v/--version
Commands: login, usage [SESSION_ID] [TURN], sessions list
"""


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    C.no_network(TOOL)
    C.log_call(TOOL, argv)
    if argv == ["--selftest"]:
        return selftest()
    if argv[:1] in (["--version"], ["-v"], ["-V"], ["version"], ["v"]):
        print(f"grok {VERSION} (lab-standin)")
        return 0
    if argv[:1] in (["--help"], ["-h"], ["help"]):
        print(HELP)
        return 0
    grok_home()
    options, positional, command, words = parse(argv)
    if command is None:
        return session_main(options, positional, words)
    if command == "login":
        return login(words)
    if command == "usage":
        return usage_main(words)
    if command == "sessions":
        return sessions_main(words)
    print(f"lab stand-in: `grok {command}` isn't emulated (the adapter doesn't run it)", file=sys.stderr)
    return 2


def install(dest: Path) -> dict:
    wrapper = C.install_wrapper(dest, "grok", "grok.py")
    return {"AI_JUGGLER_GROK_BIN": str(wrapper)}


# --------------------------------------------------------------- selftest

REPO = HERE.parent.parent.parent


def static_check() -> List[str]:
    """The names written here, as the adapter's Rust source spells them."""
    src = REPO / "crates/juggler-adapters/src"
    if not src.is_dir():
        print("grok stand-in: the Rust source isn't beside this copy; static check skipped")
        return []
    text = {name: (src / name).read_text() for name in ("grok_build.rs", "grok_sessions.rs", "sign_in.rs")}
    wanted = [
        ("sign_in.rs", f'GROK_AUTH: &str = "{AUTH_FILE}";'),
        ("sign_in.rs", "auth_mode: String"),
        ("grok_build.rs", 'pub const ENV_HOME: &str = "GROK_HOME";'),
        ("grok_build.rs", 'const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_GROK_BIN";'),
        ("grok_build.rs", '"web_login" | "oidc" | "external"'),
        ("grok_build.rs", 'args: vec!["login".into()]'),
        ("grok_build.rs", 'args.extend(["--resume".into(), session_id.clone()]);'),
        ("grok_build.rs", '"-p".into(),'),
        ("grok_build.rs", '"--output-format".into(),'),
        ("grok_build.rs", 'args.extend(["--model".into(), model.clone()]);'),
        ("grok_build.rs", 'args.extend(["--".into(), first.clone()]);'),
        ("grok_build.rs", "first.contains(\"node\") || first.contains(\"bun\")"),
        # No Grok usage is read: if that changes, this stand-in's usage
        # shape needs another look.
        ("grok_build.rs", "impl ToolQuota for GrokBuildAdapter {}"),
        ("grok_sessions.rs", f'pub const SESSIONS_DIR: &str = "{SESSIONS_DIR}";'),
        ("grok_sessions.rs", f'pub const SUMMARY_FILE: &str = "{SUMMARY_FILE}";'),
        ("grok_sessions.rs", f'pub const UPDATES_FILE: &str = "{UPDATES_FILE}";'),
        ("grok_sessions.rs", 'record.get("title")'),
        ("grok_sessions.rs", '"created_at"'),
        ("grok_sessions.rs", '"updated_at"'),
        ("grok_sessions.rs", 'value.pointer("/params/update/sessionUpdate")'),
        ("grok_sessions.rs", '"turn_completed" => Some(TailTurn::Ended)'),
        ("grok_sessions.rs", '"user_message_chunk"'),
        ("grok_sessions.rs", '"agent_message_chunk"'),
        ("grok_sessions.rs", 'b"-._~".contains(&byte)'),
        ("grok_sessions.rs", '"parentSessionId"'),
    ]
    return [f"{file}: no `{needle}`" for file, needle in wanted if needle not in text[file]]


def selftest() -> int:
    import subprocess

    C.selftest_common()
    failures = static_check()

    def check(ok: bool, what: str) -> None:
        if not ok:
            failures.append(what)

    def answers(out: str) -> int:
        return len(re.findall(r"(?m)^(?:> )?lab$", out))

    check(folder_name("/w/my app") == "%2Fw%2Fmy%20app", folder_name("/w/my app"))
    with C.selftest_lab(TOOL) as lab:
        def app(name: str) -> Tuple[Path, dict]:
            folder = lab / "data" / "accounts" / "grok-build" / name
            folder.mkdir(parents=True)
            env = {"HOME": str(lab), "PATH": "/usr/bin:/bin", "AIJ_LAB_STANDIN_WAIT": "0", "GROK_HOME": str(folder)}
            env.update({f"GROK_{who}_{what}_ENABLED": "0" for who in ("CLAUDE", "CURSOR")
                        for what in ("AGENTS", "HOOKS", "MCPS", "RULES", "SKILLS")})
            return folder, env

        def go(args: List[str], env: dict, stdin: str = "", cwd: Optional[Path] = None) -> "subprocess.CompletedProcess[str]":
            return subprocess.run(["/usr/bin/python3", "-I", str(Path(__file__).resolve()), *args], input=stdin,
                                  capture_output=True, text=True, env=env, cwd=str(cwd) if cwd else None, timeout=60)

        a, env_a = app("lab-a")
        b, env_b = app("lab-b")
        done = go(["--version"], {"PATH": "/usr/bin:/bin"})
        check(re.match(r"grok 1\.0\.77 ", done.stdout) is not None and done.returncode == 0, f"--version: {done.stdout!r}")
        done = go(["login"], dict(env_a, GROK_HOME="/tmp/not-a-lab-grok"))
        check(done.returncode == 3 and not Path("/tmp/not-a-lab-grok").exists(), f"outside a lab: {done.returncode}")
        done = go(["login"], env_a, cwd=a)
        auth = read_json(a / AUTH_FILE)
        check(done.returncode == 0 and f".{C.FAKE_HOST}/" in done.stdout and "Waiting for authorization..." in done.stdout,
              f"login: {done.stdout}")
        check(len(auth) == 1 and all(entry["auth_mode"] == "web_login" and entry["key"] == C.NOT_A_TOKEN
                                     and entry["refresh_token"] == C.NOT_A_TOKEN for entry in auth.values()), f"login wrote {auth}")
        check(all(C.FAKE_HOST in key for key in auth), "a real host in auth.json")
        check(oct((a / AUTH_FILE).stat().st_mode & 0o777) == "0o600", "auth.json isn't 0600")
        done = go(["login", "--device-auth"], env_b, cwd=b)
        check(done.returncode == 0 and "Code: " in done.stdout, f"device login: {done.stdout}")
        (b / AUTH_FILE).unlink()
        # A session in the project, read as grok_sessions.rs reads it.
        project = lab / "repos" / "lab"
        project.mkdir(parents=True)
        done = go([], env_a, "reply with the word lab\n", cwd=project)
        check(done.returncode == 0 and answers(done.stdout) == 1, f"session: {done.stdout} {done.stderr}")
        records = [record for group in (a / SESSIONS_DIR).iterdir() for record in group.glob(f"*/{SUMMARY_FILE}")]
        check(len(records) == 1, f"records: {records}")
        record = records[0]
        group = record.parent.parent
        decoded = re.sub(r"%([0-9A-F]{2})", lambda m: chr(int(m.group(1), 16)), group.name)
        check(os.path.realpath(decoded) == os.path.realpath(project), f"folder {group.name} isn't the project")
        summary = read_json(record)
        session_id = record.parent.name
        check(summary.get("title") == "reply with the word lab" and summary.get("created_at", "").endswith("Z")
              and summary.get("updated_at", "") >= summary.get("created_at", ""), f"summary: {summary}")
        check(uuid.UUID(session_id).version == 7, f"session id {session_id}")

        def tail_turn() -> Optional[str]:
            for line in reversed((record.parent / UPDATES_FILE).read_text().splitlines()):
                kind = json.loads(line).get("params", {}).get("update", {}).get("sessionUpdate")
                if kind == "turn_completed":
                    return "ended"
                if kind in ("user_message_chunk", "agent_message_chunk", "agent_thought_chunk", "tool_call",
                            "tool_call_update", "plan"):
                    return "running"
            return None

        check(tail_turn() == "ended", f"turn end: {tail_turn()}")
        born = record.stat().st_birthtime
        # Resume: the earlier turn comes back, a new one is appended.
        done = go(["--resume", session_id, "--model", "grok-build-0.1", "--", "reply with the word lab"], env_a, "", cwd=project)
        earlier = done.stdout.find("> reply with the word lab\n  lab\n")
        check(done.returncode == 0 and earlier != -1 and answers(done.stdout[earlier:].split("  lab\n", 1)[1]) == 1,
              f"resume: {done.stdout} {done.stderr}")
        kinds = [json.loads(line)["params"]["update"]["sessionUpdate"]
                 for line in (record.parent / UPDATES_FILE).read_text().splitlines()]
        check(kinds == ["user_message_chunk", "agent_message_chunk", "turn_completed"] * 2, f"updates: {kinds}")
        check(record.stat().st_birthtime == born, "summary.json was replaced, not rewritten")
        check(tail_turn() == "ended", "turn end after resume")
        done = go(["--resume", "no-such-session"], env_a, "", cwd=project)
        check(done.returncode == 1, "an unknown session was resumed")
        # Usage: signals.json's counters, and `grok usage <id>`.
        signals = read_json(record.parent / SIGNALS_FILE)
        check(signals.get("turn_count") == 2 and signals.get("input_tokens", 0) > 2000
              and signals.get("context_window_tokens") == CONTEXT_WINDOW, f"signals: {signals}")
        done = go(["usage", session_id], env_a, cwd=project)
        check(done.returncode == 0 and "Turn 2:" in done.stdout and "Total: input" in done.stdout, f"usage: {done.stdout}")
        done = go(["-p", "reply with the word lab", "--output-format", "json"], env_a, cwd=project)
        result = json.loads(done.stdout) if done.returncode == 0 else {}
        check(result.get("result") == "lab" and result.get("session_id"), f"headless: {done.stdout} {done.stderr}")
        # lab B (no sign-in) says so; the launcher isn't a Node script.
        done = go([], env_b, "reply with the word lab\n", cwd=project)
        check(done.returncode == 1 and "grok login" in done.stdout and not (b / SESSIONS_DIR).exists(), f"lab B: {done.stdout}")
        dest = lab / "standins"
        dest.mkdir()
        wrapper = Path(install(dest)["AI_JUGGLER_GROK_BIN"])
        head = wrapper.read_text().splitlines()[0]
        check(head.startswith("#!") and "node" not in head and "bun" not in head, f"launcher: {head}")
    for line in failures:
        print(f"FAIL {line}", file=sys.stderr)
    print("grok stand-in: selftest " + ("FAILED" if failures else "passed"))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
