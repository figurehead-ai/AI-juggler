#!/usr/bin/env python3
"""AJ-E1612: a lab stand-in for OpenCode 1.19.5 (`opencode`), for a dry run
of the sign-in sitting on AI Juggler Dev. Never the real tool: it reaches no
network, binds no port, writes only inside a lab (`/tmp/aij-lab…`) and every
sign-in value it writes is `lab-not-a-token`.

Dev runs it through `AI_JUGGLER_OPENCODE_BIN` (install.py), with the
adapter's variables: `XDG_DATA_HOME=<App>/data`, `XDG_CONFIG_HOME=<App>/config`,
`XDG_STATE_HOME=<App>/state`, `OPENCODE_CONFIG_DIR=<App>/config/opencode`
(opencode.rs `pinned_env`). Every run but `--version`/`--help` stops (exit 3)
unless all three XDG folders (or, unset, `$HOME/.local/share` and the rest,
as for an App used in place) lie in a lab.

What Dev runs, and what the stand-in does:

- `opencode --version` (discovery): prints `1.19.5`, a 1.x version, so the
  adapter never adds `--standalone` (that is OpenCode 2's private server;
  accepted and ignored if it comes anyway).
- `opencode auth login` (login_spec, in a pty, cwd = the App folder): the
  1.19.5 menus, clack style. `┌  Add credential`, then "Select provider" in
  the real order: OpenCode (hint "recommended"), OpenAI (hint "ChatGPT
  Plus/Pro or API key"), GitHub Copilot, Google, Anthropic, OpenRouter,
  Vercel AI Gateway, Other; `●`/`○` markers, the hint only beside the active
  row; ↑/↓ (ESC [ A/B, also j/k) move and wrap, Enter picks, Ctrl-C or Esc
  aborts (exit 130, nothing written). A pick collapses to `◇  Select
  provider` / `│  OpenAI`. With a tty on stdin it reads keys (termios);
  otherwise one line per menu: a number, a label or an id (blank = the
  active row), for selftests and scripted drives.
  - OpenAI → "Login method": ChatGPT Pro/Plus (browser), ChatGPT Pro/Plus
    (headless), Manually enter API Key. Browser prints `Go to:` a fake
    authorize address on example.invalid and "Waiting for authorization...",
    waits (AIJ_LAB_STANDIN_WAIT), then writes `data/opencode/auth.json`
    `{"openai": {"type": "oauth", "refresh", "access", "expires"}}` and
    "Login successful". Headless shows a fake device page and code instead;
    the API key method asks for a key and writes `{"type": "api"}`.
  - OpenCode (the default row) reproduces the 2026-11-16 mistake: "Create an
    api key at …" (a fake address), "Enter your API key", then
    `{"opencode": {"type": "api", "key": …}}` — the adapter reads that as an
    API-key App, not a sign-in.
  - The other providers ask for an API key the same way; Other asks for a
    provider id first. Entries merge into an existing auth.json, as the
    real one does.
  - `-p/--provider` (an id, or a label in any case) and `-m/--method` (a
    method label in any case) skip the menus; unknown ones print the real
    errors (`Unknown provider "x"`, `Unknown method "x" for openai.
    Available: …`) and exit 1 with nothing written. `auth login <url>` (a
    provider's own sign-in command) is refused: it would fetch the url.
- `opencode [--session <id>] [--prompt=<text>]` (launch_spec Interactive /
  Primed / Resume, in a pty): a banner and a `> ` prompt; every line is a
  turn answered `lab`. Like the real TUI, the session row is made at the
  first message. Each turn is written to `data/opencode/opencode.db` (WAL,
  1.x tables `project`/`session`/`message`/`part`): the user's message and
  its text part, then an assistant message (running: no completion time),
  then completed with `finish: "stop"`, tokens and cost, its text part, and
  the session's totals. `--session <id>` prints the session's earlier turns
  back from the database and appends to it; an unknown id prints "Session
  not found" and exits 1. With no sign-in in auth.json each turn ends with
  an error message ("no provider is signed in") instead of `lab`.
- `opencode run [--format json] [--model=<m>] [--session <id>] -- <text>`
  (launch_spec Headless): one turn, recorded the same way; prints `lab`, or
  JSON lines with `--format json`.
- `opencode auth list`: the credentials by provider and kind (no values).
- Usage: the adapter runs nothing for it; it reads the completed assistant
  messages' `tokens` and `cost` in opencode.db (opencode_store.rs
  `usage_events`). Each turn records modest made-up counts (about 1.2k in,
  8 out, some cache read); cost 0 on a ChatGPT sign-in (a subscription) and
  a fraction of a cent on an API key.

`--selftest` runs all of that in a throwaway lab, as subprocesses with the
adapter's variables, reads the database with the adapter's own queries, and
checks the names it writes against the Rust source.

What a dry run on this stand-in cannot prove:

- the real menus' full provider list: 1.19.5 lists every models.dev provider
  after Vercel (alphabetically, in an 8-row scrolling window) before Other;
  here only the seven named ones and Other; nor clack's exact colours,
  spinner dots or redraw bytes;
- that ChatGPT's browser sign-in works: the real one listens on
  localhost:1455 and trades a code with OpenAI; nothing is listened on here,
  and the real entry may carry fields this one lacks (an account id);
- the TUI itself (full screen, its keys, /commands, how Dev's typed text
  lands in it), model replies, tools, sub-agents, titles (the real one asks
  the model; here the first line of the first message);
- OpenCode with no sign-in: the real one may still answer through
  OpenCode's free models; here every turn fails, to rehearse "lab B";
- OpenCode 2 (`--standalone`, `session_v2`, sign-ins in the database), the
  real schema's migrations table and every column the adapter doesn't read;
- live process detection: the process is `python3 …/opencode.py`, not a
  program named `opencode`, so Dev's running-session match by name sees it
  only through the session it launched itself;
- the conversation's text in Dev: the adapter reads no OpenCode message
  text, only rows, roles, times, finish words and counts.
"""
from __future__ import annotations

import importlib.util
import json
import os
import re
import secrets
import select
import sqlite3
import string
import sys
import time
from pathlib import Path
from typing import Callable, List, Optional, Sequence, Tuple

HERE = Path(__file__).resolve().parent
TOOL = "opencode"
VERSION = "1.19.5"


def _load_common():
    spec = importlib.util.spec_from_file_location("standin_common", HERE / "standin_common.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


C = _load_common()

# The real 1.19.5 order (`m={opencode:0,openai:1,"github-copilot":2,google:3,
# anthropic:4,openrouter:5,vercel:6}`), names from its models.dev snapshot.
PROVIDERS: Tuple[Tuple[str, str, Optional[str]], ...] = (
    ("opencode", "OpenCode", "recommended"),
    ("openai", "OpenAI", "ChatGPT Plus/Pro or API key"),
    ("github-copilot", "GitHub Copilot", None),
    ("google", "Google", None),
    ("anthropic", "Anthropic", None),
    ("openrouter", "OpenRouter", None),
    ("vercel", "Vercel AI Gateway", None),
    ("other", "Other", None),
)
OPENAI_METHODS = ("ChatGPT Pro/Plus (browser)", "ChatGPT Pro/Plus (headless)", "Manually enter API Key")

# The names the adapter reads (checked against the Rust source in --selftest).
DB_FILE = "opencode.db"
AUTH_FILE = "auth.json"
SESSION_COLUMNS = """
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, workspace_id TEXT,
    parent_id TEXT, slug TEXT NOT NULL, directory TEXT NOT NULL, path TEXT,
    title TEXT, version TEXT NOT NULL, share_url TEXT,
    summary_additions INTEGER, summary_deletions INTEGER, summary_files INTEGER,
    summary_diffs TEXT, metadata TEXT,
    cost REAL NOT NULL DEFAULT 0,
    tokens_input INTEGER NOT NULL DEFAULT 0, tokens_output INTEGER NOT NULL DEFAULT 0,
    tokens_reasoning INTEGER NOT NULL DEFAULT 0, tokens_cache_read INTEGER NOT NULL DEFAULT 0,
    tokens_cache_write INTEGER NOT NULL DEFAULT 0,
    revert TEXT, permission TEXT, agent TEXT, model TEXT,
    time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL,
    time_compacting INTEGER, time_archived INTEGER"""
SCHEMA = f"""
CREATE TABLE IF NOT EXISTS project (id TEXT PRIMARY KEY, worktree TEXT NOT NULL, vcs TEXT, name TEXT,
    icon_url TEXT, icon_color TEXT, time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL,
    time_initialized INTEGER, sandboxes TEXT NOT NULL DEFAULT '[]');
CREATE TABLE IF NOT EXISTS session ({SESSION_COLUMNS});
CREATE TABLE IF NOT EXISTS message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL,
    time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, session_id TEXT NOT NULL,
    time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS message_session_idx ON message (session_id);
CREATE INDEX IF NOT EXISTS part_message_idx ON part (message_id);
CREATE INDEX IF NOT EXISTS part_session_idx ON part (session_id);
"""
SLUG_WORDS = ("calm", "brisk", "quiet", "amber", "lucky", "otter", "heron", "maple", "cedar", "comet")


class Cancelled(Exception):
    """Ctrl-C or Esc at a prompt."""


# --------------------------------------------------------------- the lab


def folders() -> Tuple[Path, Path, Path]:
    """The XDG data, config and state folders, as OpenCode resolves them;
    each must lie in a lab."""
    home = os.environ.get("HOME", "")
    pairs = (("XDG_DATA_HOME", ".local/share"), ("XDG_CONFIG_HOME", ".config"), ("XDG_STATE_HOME", ".local/state"))
    out = []
    for name, fallback in pairs:
        value = os.environ.get(name) or (os.path.join(home, fallback) if home else "")
        out.append(C.require_lab(value, TOOL, name))
    return out[0], out[1], out[2]


def data_dir() -> Path:
    return folders()[0] / "opencode"


# --------------------------------------------------------------- clack-style terminal


class Term:
    """Clack's look: colours and keys only on a terminal, as picocolors and
    clack decide; plain lines otherwise."""

    def __init__(self) -> None:
        self.tty_in = sys.stdin.isatty()
        self.color = sys.stdout.isatty() and "NO_COLOR" not in os.environ

    def paint(self, code: str, text: str) -> str:
        return f"\x1b[{code}m{text}\x1b[0m" if self.color else text

    def gray(self, text: str) -> str:
        return self.paint("90", text)

    def dim(self, text: str) -> str:
        return self.paint("2", text)

    def cyan(self, text: str) -> str:
        return self.paint("36", text)

    def green(self, text: str) -> str:
        return self.paint("32", text)

    def out(self, text: str) -> None:
        sys.stdout.write(text)
        sys.stdout.flush()

    def intro(self, title: str) -> None:
        self.out(f"\n{self.gray('┌')}  {title}\n")

    def info(self, message: str) -> None:
        self.out(f"{self.gray('│')}\n{self.paint('34', '●')}  {message}\n")

    def warn(self, message: str) -> None:
        self.out(f"{self.gray('│')}\n{self.paint('33', '▲')}  {message}\n")

    def error(self, message: str) -> None:
        self.out(f"{self.gray('│')}\n{self.paint('31', '■')}  {message}\n")

    def outro(self, message: str) -> None:
        self.out(f"{self.gray('│')}\n{self.gray('└')}  {message}\n\n")

    def spin(self, message: str, seconds: float, done: str) -> None:
        """A spinner for `seconds`, then its stop line."""
        self.out(f"{self.gray('│')}\n")
        frames = "◒◐◓◑"
        if not sys.stdout.isatty():
            self.out(f"{frames[0]}  {message}\n")
            time.sleep(seconds)
        else:
            end = time.monotonic() + seconds
            at = 0
            while True:
                self.out(f"\r\x1b[K{self.paint('35', frames[at % 4])}  {message}")
                if time.monotonic() >= end:
                    break
                time.sleep(min(0.08, max(0.0, end - time.monotonic())))
                at += 1
            self.out("\r\x1b[K")
        self.out(f"{self.green('◇')}  {done}\n")

    # -- keys

    def _keys(self) -> "KeyReader":
        return KeyReader()

    def select(self, message: str, options: Sequence[Tuple[str, str, Optional[str]]], initial: int = 0) -> int:
        """A clack select; the chosen index. Raises Cancelled."""
        cursor = initial

        def block(state: str) -> str:
            symbol = {"active": self.cyan("◆"), "submit": self.green("◇"), "cancel": self.paint("31", "■")}[state]
            title = f"{self.gray('│')}\n{symbol}  {message}\n"
            label = options[cursor][1]
            if state == "submit":
                return f"{title}{self.gray('│')}  {self.dim(label)}\n"
            if state == "cancel":
                return f"{title}{self.gray('│')}  {self.paint('9;2', label)}\n{self.gray('│')}\n"
            rows = []
            for at, (_, name, hint) in enumerate(options):
                if at == cursor:
                    rows.append(f"{self.green('●')} {name}" + (f" {self.dim('(' + hint + ')')}" if hint else ""))
                else:
                    rows.append(f"{self.dim('○')} {self.dim(name)}")
            bar = self.cyan("│")
            return f"{title}{bar}  " + f"\n{bar}  ".join(rows) + f"\n{self.cyan('└')}\n"

        if not self.tty_in:
            self.out(block("active"))
            while True:
                line = sys.stdin.readline()
                if not line:
                    self.out(block("cancel"))
                    raise Cancelled()
                picked = pick(line.strip(), options, cursor)
                if picked is not None:
                    cursor = picked
                    self.out(block("submit"))
                    return cursor
                self.out(f"(lab stand-in: no option {line.strip()!r}; type 1-{len(options)}, a label or an id)\n")
        drawn = block("active")
        self.out("\x1b[?25l" + drawn)
        try:
            with self._keys() as keys:
                while True:
                    key = keys.read()
                    if key in ("up", "down"):
                        cursor = (cursor + (1 if key == "down" else -1)) % len(options)
                    elif key == "enter":
                        self._redraw(drawn, block("submit"))
                        return cursor
                    elif key == "cancel":
                        self._redraw(drawn, block("cancel"))
                        raise Cancelled()
                    else:
                        continue
                    new = block("active")
                    self._redraw(drawn, new)
                    drawn = new
        finally:
            self.out("\x1b[?25h")

    def _redraw(self, old: str, new: str) -> None:
        lines = old.count("\n")
        self.out(f"\x1b[{lines}A\r\x1b[J" + new)

    def password(self, message: str) -> str:
        """A clack password prompt; what was typed (never stored). Raises
        Cancelled."""
        title = f"{self.gray('│')}\n{self.cyan('◆')}  {message}\n"
        if not self.tty_in:
            self.out(f"{title}{self.cyan('│')}  \n{self.cyan('└')}\n")
            while True:
                line = sys.stdin.readline()
                if not line:
                    raise Cancelled()
                if line.strip():
                    self.out(f"{self.gray('│')}\n{self.green('◇')}  {message}\n{self.gray('│')}  {'▪' * len(line.strip())}\n")
                    return line.strip()
                self.out(f"{self.paint('33', '▲')}  {message}\n{self.paint('33', '└')}  Required\n")
        typed = ""
        drawn = f"{title}{self.cyan('│')}  \n{self.cyan('└')}\n"
        self.out(drawn)
        with self._keys() as keys:
            while True:
                key = keys.read(typing=True)
                if key == "cancel":
                    self._redraw(drawn, f"{self.gray('│')}\n{self.paint('31', '■')}  {message}\n{self.gray('│')}\n")
                    raise Cancelled()
                if key == "enter":
                    if typed:
                        self._redraw(drawn, f"{self.gray('│')}\n{self.green('◇')}  {message}\n{self.gray('│')}  {self.dim('▪' * len(typed))}\n")
                        return typed
                    new = f"{self.gray('│')}\n{self.paint('33', '▲')}  {message}\n{self.paint('33', '│')}  \n{self.paint('33', '└')}  Required\n"
                elif key == "backspace":
                    typed = typed[:-1]
                    new = f"{title}{self.cyan('│')}  {'▪' * len(typed)}\n{self.cyan('└')}\n"
                elif key.startswith("char:"):
                    typed += key[5:]
                    new = f"{title}{self.cyan('│')}  {'▪' * len(typed)}\n{self.cyan('└')}\n"
                else:
                    continue
                self._redraw(drawn, new)
                drawn = new

    def text(self, message: str, validate: Callable[[str], Optional[str]]) -> str:
        """A clack text prompt (Other's provider id)."""
        title = f"{self.gray('│')}\n{self.cyan('◆')}  {message}\n"
        self.out(f"{title}{self.cyan('│')}  \n{self.cyan('└')}\n")
        while True:
            if self.tty_in:
                termios_line = KeyReader.read_line()
                if termios_line is None:
                    raise Cancelled()
                line = termios_line
            else:
                read = sys.stdin.readline()
                if not read:
                    raise Cancelled()
                line = read.strip()
            problem = validate(line)
            if problem is None:
                self.out(f"{self.gray('│')}\n{self.green('◇')}  {message}\n{self.gray('│')}  {self.dim(line)}\n")
                return line
            self.out(f"{self.paint('33', '▲')}  {message}\n{self.paint('33', '└')}  {problem}\n")


class KeyReader:
    """Keys from a terminal in non-canonical, no-echo, no-signal mode (Ctrl-C
    arrives as a byte, as in clack)."""

    def __enter__(self) -> "KeyReader":
        import termios

        self.fd = sys.stdin.fileno()
        self.saved = termios.tcgetattr(self.fd)
        mode = termios.tcgetattr(self.fd)
        mode[3] &= ~(termios.ICANON | termios.ECHO | termios.ISIG)
        mode[6][termios.VMIN] = 1
        mode[6][termios.VTIME] = 0
        termios.tcsetattr(self.fd, termios.TCSADRAIN, mode)
        return self

    def __exit__(self, *exc) -> None:  # type: ignore[no-untyped-def]
        import termios

        termios.tcsetattr(self.fd, termios.TCSADRAIN, self.saved)

    def _byte(self, timeout: Optional[float] = None) -> Optional[bytes]:
        if timeout is not None:
            ready, _, _ = select.select([self.fd], [], [], timeout)
            if not ready:
                return None
        got = os.read(self.fd, 1)
        return got or None

    def read(self, typing: bool = False) -> str:
        """up, down, enter, cancel, backspace, char:<c> or other (j/k move
        unless `typing`)."""
        byte = self._byte()
        if byte is None or byte == b"\x03" or byte == b"\x04":
            return "cancel"
        if byte in (b"\r", b"\n"):
            return "enter"
        if byte in (b"\x7f", b"\x08"):
            return "backspace"
        if byte == b"\x1b":
            nxt = self._byte(0.05)
            if nxt is None:
                return "cancel"  # a lone Esc
            if nxt in (b"[", b"O"):
                last = self._byte(0.05)
                return {b"A": "up", b"B": "down"}.get(last or b"", "other")
            return "other"
        try:
            char = byte.decode()
        except UnicodeDecodeError:
            return "other"
        if not typing and char in ("k", "j"):
            return "up" if char == "k" else "down"
        return f"char:{char}" if char.isprintable() else "other"

    @staticmethod
    def read_line() -> Optional[str]:
        line = sys.stdin.readline()
        return line.strip() if line else None


def pick(answer: str, options: Sequence[Tuple[str, str, Optional[str]]], default: int) -> Optional[int]:
    """A line's choice: blank = the active row, a 1-based number, a label
    or an id (any case)."""
    if not answer:
        return default
    if answer.isdigit() and 1 <= int(answer) <= len(options):
        return int(answer) - 1
    for at, (value, label, _) in enumerate(options):
        if answer.lower() in (value.lower(), label.lower()):
            return at
    return None


# --------------------------------------------------------------- sign-in


def read_auth(path: Path) -> dict:
    try:
        value = json.loads(path.read_text())
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def save_auth(provider: str, entry: dict) -> None:
    path = data_dir() / AUTH_FILE
    auth = read_auth(path)
    auth[provider] = entry
    C.write_json(path, auth, TOOL)


def api_key_entry() -> dict:
    return {"type": "api", "key": C.NOT_A_TOKEN}


def oauth_entry() -> dict:
    return {"type": "oauth", "refresh": C.NOT_A_TOKEN, "access": C.NOT_A_TOKEN,
            "expires": C.now_ms() + 10 * 24 * 3600 * 1000}


def option_value(args: List[str], names: Sequence[str]) -> Tuple[Optional[str], List[str]]:
    """`--name v`, `--name=v`, `-n v`; the value and the rest."""
    rest: List[str] = []
    value = None
    at = 0
    while at < len(args):
        arg = args[at]
        hit = next((name for name in names if arg == name or arg.startswith(name + "=")), None)
        if hit is not None:
            if arg.startswith(hit + "="):
                value = arg[len(hit) + 1:]
            elif at + 1 < len(args):
                value = args[at + 1]
                at += 1
            else:
                value = ""
        else:
            rest.append(arg)
        at += 1
    return value, rest


def auth_login(args: List[str]) -> int:
    folders()
    term = Term()
    provider_arg, args = option_value(args, ("--provider", "-p"))
    method_arg, args = option_value(args, ("--method", "-m"))
    args = [arg for arg in args if arg != "--standalone"]
    print()
    term.intro("Add credential")
    if args:
        term.error(f"lab stand-in: `auth login {args[0]}` would fetch {args[0]}/.well-known/opencode; not emulated")
        return 1
    try:
        if provider_arg is not None:
            found = next((value for value, label, _ in PROVIDERS[:-1]
                          if provider_arg == value or provider_arg.lower() == label.lower()), None)
            if found is None:
                term.error(f'Unknown provider "{provider_arg}"')
                return 1
            provider = found
        else:
            provider = PROVIDERS[term.select("Select provider", PROVIDERS)][0]
        if provider == "openai":
            return openai_login(term, method_arg)
        if provider == "other":
            provider = term.text("Enter provider id",
                                 lambda text: None if re.fullmatch(r"[0-9a-z-]+", text or "") else "a-z, 0-9 and hyphens only")
            provider = re.sub(r"^@ai-sdk/", "", provider)
            term.warn(f"This only stores a credential for {provider} - you will need configure it in opencode.json, check the docs for examples.")
        if provider == "opencode":
            term.info(f"Create an api key at https://opencode.{C.FAKE_HOST}/auth")
        if provider == "vercel":
            term.info(f"You can create an api key at https://vercel.{C.FAKE_HOST}/ai-gateway-token")
        term.password("Enter your API key")
    except (Cancelled, KeyboardInterrupt):
        return 130
    save_auth(provider, api_key_entry())
    term.outro("Done")
    return 0


def openai_login(term: Term, method_arg: Optional[str]) -> int:
    if method_arg is not None:
        index = next((at for at, label in enumerate(OPENAI_METHODS) if label.lower() == method_arg.lower()), -1)
        if index == -1:
            term.error(f'Unknown method "{method_arg}" for openai. Available: {", ".join(OPENAI_METHODS)}')
            return 1
    else:
        index = term.select("Login method", [(str(at), label, None) for at, label in enumerate(OPENAI_METHODS)])
    if index == 2:
        term.password("Enter your API key")
        save_auth("openai", api_key_entry())
        term.outro("Done")
        return 0
    state = secrets.token_hex(16)
    if index == 0:
        term.info(f"Go to: https://auth.openai.{C.FAKE_HOST}/oauth/authorize?response_type=code&client_id=lab"
                  f"&redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback&state={state}")
        term.info("Complete authorization in your browser. This window will close automatically.")
    else:
        code = C.device_code()
        term.info(f"Go to: https://auth.openai.{C.FAKE_HOST}/codex/device")
        term.info(f"Enter code: {code}")
    term.out("(lab stand-in: nothing is opened or listened on; waiting as if you signed in)\n")
    term.spin("Waiting for authorization...", C.wait_seconds(), "Login successful")
    save_auth("openai", oauth_entry())
    term.outro("Done")
    return 0


def auth_list() -> int:
    folders()
    path = data_dir() / AUTH_FILE
    entries = read_auth(path)
    names = {value: label for value, label, _ in PROVIDERS}
    print(f"\n┌  Credentials {path}")
    for provider, entry in entries.items():
        kind = entry.get("type", "?") if isinstance(entry, dict) else "?"
        print(f"│\n●  {names.get(provider, provider)} {kind}")
    print(f"│\n└  {len(entries)} credentials\n")
    return 0


# --------------------------------------------------------------- the store


def ident(prefix: str, descending: bool = False) -> str:
    """OpenCode's id form: a prefix, 12 hex of time (inverted for sessions,
    so newer sorts first) and 14 random base62."""
    stamp = C.now_ms() * 0x1000 + secrets.randbelow(0x1000)
    if descending:
        stamp = (1 << 48) - 1 - (stamp & ((1 << 48) - 1))
    alphabet = string.digits + string.ascii_letters
    tail = "".join(secrets.choice(alphabet) for _ in range(14))
    return f"{prefix}_{stamp & ((1 << 48) - 1):012x}{tail}"


def open_store() -> sqlite3.Connection:
    db = data_dir() / DB_FILE
    C.require_lab(db, TOOL, "the database")
    db.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(db), timeout=5)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.executescript(SCHEMA)
    return conn


def close_store(conn: sqlite3.Connection) -> None:
    """Closes the store as the real one leaves it closed: no `-wal` beside
    it (macOS's own SQLite keeps an empty one, which would make the
    adapter's reader open the closed file as a live one). Only the last
    connection may switch the mode; any other leaves it as it is."""
    try:
        conn.commit()
        conn.execute("PRAGMA journal_mode=DELETE")
    except sqlite3.OperationalError:
        pass
    conn.close()


def signed_in_provider() -> Optional[str]:
    auth = read_auth(data_dir() / AUTH_FILE)
    if "openai" in auth:
        return "openai"
    return next(iter(auth), None)


def project_of(conn: sqlite3.Connection, cwd: str) -> str:
    root = Path(cwd)
    while root != root.parent and not (root / ".git").exists():
        root = root.parent
    if (root / ".git").exists():
        import hashlib

        project = hashlib.sha1(str(root).encode()).hexdigest()  # stands in for the root commit
        worktree = str(root)
    else:
        project, worktree = "global", "/"
    now = C.now_ms()
    conn.execute("INSERT OR IGNORE INTO project (id, worktree, vcs, time_created, time_updated) VALUES (?, ?, ?, ?, ?)",
                 (project, worktree, "git" if project != "global" else None, now, now))
    return project


class Session:
    """One OpenCode session in the store."""

    def __init__(self, conn: sqlite3.Connection, session_id: Optional[str], cwd: str, model: Optional[str]) -> None:
        self.conn = conn
        self.id = session_id
        self.cwd = cwd
        self.model = model

    def exists(self) -> bool:
        return self.id is not None and self.conn.execute("SELECT 1 FROM session WHERE id = ?", (self.id,)).fetchone() is not None

    def create(self, title: str) -> None:
        now = C.now_ms()
        self.id = ident("ses", descending=True)
        project = project_of(self.conn, self.cwd)
        slug = f"{secrets.choice(SLUG_WORDS)}-{secrets.choice(SLUG_WORDS)}"
        self.conn.execute(
            "INSERT INTO session (id, project_id, slug, directory, title, version, time_created, time_updated) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (self.id, project, slug, self.cwd, title, VERSION, now, now))
        self.conn.commit()

    def earlier(self) -> List[Tuple[str, str]]:
        """(role, text) of every message so far, oldest first."""
        out = []
        for message_id, data in self.conn.execute(
                "SELECT id, data FROM message WHERE session_id = ? ORDER BY time_created, id", (self.id,)):
            role = json.loads(data).get("role", "?")
            texts = [json.loads(part)["text"] for (part,) in self.conn.execute(
                "SELECT data FROM part WHERE message_id = ? ORDER BY id", (message_id,))
                     if json.loads(part).get("type") == "text"]
            error = json.loads(data).get("error")
            out.append((role, " ".join(texts) if texts else (f"[error] {error.get('data', {}).get('message', '')}" if error else "")))
        return out

    def _part(self, message_id: str, text: str, at: int) -> None:
        part_id = ident("prt")
        data = {"id": part_id, "sessionID": self.id, "messageID": message_id, "type": "text", "text": text,
                "time": {"start": at, "end": at}}
        self.conn.execute("INSERT INTO part (id, message_id, session_id, time_created, time_updated, data) VALUES (?, ?, ?, ?, ?, ?)",
                          (part_id, message_id, self.id, at, at, json.dumps(data)))

    def turn(self, text: str) -> str:
        """Records one turn (user, then the assistant running, then done);
        returns the answer shown."""
        if self.id is None:
            first = text.strip().splitlines()[0][:100] if text.strip() else "New session"
            self.create(first)
        provider = signed_in_provider()
        model = self.model.split("/", 1)[1] if self.model and "/" in self.model else (self.model or "lab-model")
        provider_id = (self.model.split("/", 1)[0] if self.model and "/" in self.model else None) or provider or "opencode"
        now = C.now_ms()
        user_id = ident("msg")
        user = {"id": user_id, "sessionID": self.id, "role": "user", "time": {"created": now}, "agent": "build",
                "model": {"providerID": provider_id, "modelID": model}}
        self.conn.execute("INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?, ?, ?, ?, ?)",
                          (user_id, self.id, now, now, json.dumps(user)))
        self._part(user_id, text, now)
        self.conn.execute("UPDATE session SET time_updated = ? WHERE id = ?", (now, self.id))
        self.conn.commit()
        started = now + 1
        reply_id = ident("msg")
        reply = {"id": reply_id, "sessionID": self.id, "role": "assistant", "parentID": user_id, "mode": "build",
                 "agent": "build", "modelID": model, "providerID": provider_id,
                 "path": {"cwd": self.cwd, "root": self.cwd}, "cost": 0,
                 "tokens": {"input": 0, "output": 0, "reasoning": 0, "cache": {"read": 0, "write": 0}},
                 "time": {"created": started}}
        self.conn.execute("INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?, ?, ?, ?, ?)",
                          (reply_id, self.id, started, started, json.dumps(reply)))
        self.conn.commit()
        if C.wait_seconds() > 0:
            time.sleep(0.4)
        done = C.now_ms()
        if provider is None:
            message = "no provider is signed in: run `opencode auth login` (lab stand-in)"
            reply["error"] = {"name": "ProviderAuthError", "data": {"providerID": provider_id, "message": message}}
            reply["time"]["completed"] = done
            answer = f"Error: {message}"
        else:
            tokens = {"input": 1180 + secrets.randbelow(80), "output": 8, "reasoning": 0,
                      "cache": {"read": 640, "write": 0}}
            cost = 0.0 if read_auth(data_dir() / AUTH_FILE).get(provider_id, {}).get("type") == "oauth" else 0.0021
            reply.update({"finish": "stop", "cost": cost, "tokens": tokens})
            reply["time"]["completed"] = done
            self._part(reply_id, C.answer_for(text), done)
            self.conn.execute(
                "UPDATE session SET cost = cost + ?, tokens_input = tokens_input + ?, tokens_output = tokens_output + ?, "
                "tokens_cache_read = tokens_cache_read + ? WHERE id = ?",
                (cost, tokens["input"], tokens["output"], tokens["cache"]["read"], self.id))
            answer = C.answer_for(text)
        self.conn.execute("UPDATE message SET data = ?, time_updated = ? WHERE id = ?", (json.dumps(reply), done, reply_id))
        self.conn.execute("UPDATE session SET time_updated = ? WHERE id = ?", (done, self.id))
        self.conn.commit()
        return answer


# --------------------------------------------------------------- sessions


def parse_session_args(args: List[str]) -> Tuple[Optional[str], Optional[str], Optional[str], bool, List[str]]:
    session, args = option_value(args, ("--session", "-s"))
    prompt, args = option_value(args, ("--prompt",))
    model, args = option_value(args, ("--model", "-m"))
    cont = any(arg in ("--continue", "-c") for arg in args)
    rest = [arg for arg in args if arg not in ("--standalone", "--continue", "-c")]
    return session, prompt, model, cont, rest


def latest_here(conn: sqlite3.Connection, cwd: str) -> Optional[str]:
    row = conn.execute("SELECT id FROM session WHERE directory = ? AND parent_id IS NULL ORDER BY time_updated DESC LIMIT 1",
                       (cwd,)).fetchone()
    return row[0] if row else None


def exit_on_signals() -> None:
    """Ending the session from AI Juggler (SIGTERM, or SIGHUP when its
    terminal closes) closes the store as the real one does on its way out."""
    import signal

    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, lambda got, _frame: sys.exit(128 + got))


def tui(args: List[str]) -> int:
    folders()
    exit_on_signals()
    session_id, prompt, model, cont, rest = parse_session_args(args)
    if rest:
        target = rest[0]
        if not os.path.isdir(target):
            print(f"Error: {target} isn't a folder", file=sys.stderr)
            return 1
        os.chdir(target)
    cwd = os.getcwd()
    conn = open_store()
    if cont and session_id is None:
        session_id = latest_here(conn, cwd)
    session = Session(conn, session_id, cwd, model)
    print(f"opencode {VERSION} (lab stand-in: answers every message with `lab`; /exit to quit)", flush=True)
    if session_id is not None:
        if not session.exists():
            close_store(conn)
            print(f"Error: Session not found: {session_id}", file=sys.stderr)
            return 1
        print(f"Session {session_id}", flush=True)
        for role, text in session.earlier():
            print(f"{'>' if role == 'user' else ' '} {text}", flush=True)
    if prompt:
        print(f"> {prompt}", flush=True)
        print(session.turn(prompt), flush=True)
    try:
        return C.repl(TOOL, "> ", session.turn)
    finally:
        close_store(conn)


def run(args: List[str]) -> int:
    folders()
    if "--" in args:
        at = args.index("--")
        args, words = args[:at], args[at + 1:]
    else:
        words = []
    fmt, args = option_value(args, ("--format",))
    session_id, _, model, cont, rest = parse_session_args(args)
    message = " ".join(words + [arg for arg in rest if not arg.startswith("-")]).strip()
    if not message:
        print("Error: You must provide a message or a command", file=sys.stderr)
        return 1
    cwd = os.getcwd()
    conn = open_store()
    try:
        if cont and session_id is None:
            session_id = latest_here(conn, cwd)
        session = Session(conn, session_id, cwd, model)
        if session_id is not None and not session.exists():
            print(f"Error: Session not found: {session_id}", file=sys.stderr)
            return 1
        answer = session.turn(message)
        if fmt == "json":
            now = C.now_ms()
            print(json.dumps({"type": "step_start", "timestamp": now, "sessionID": session.id}))
            print(json.dumps({"type": "text", "timestamp": now, "sessionID": session.id,
                              "part": {"type": "text", "text": answer, "sessionID": session.id}}))
            print(json.dumps({"type": "step_finish", "timestamp": now, "sessionID": session.id}))
        else:
            print(answer)
        return 0 if not answer.startswith("Error:") else 1
    finally:
        close_store(conn)


HELP = f"""opencode {VERSION} (lab stand-in for AJ-E1612)

Commands:
  opencode [project]         start the TUI (--session <id>, --prompt=<text>, --model <p/m>, --continue)
  opencode run [message..]   one turn (--format json, --session <id>, --model <p/m>)
  opencode auth login        add a credential (-p/--provider, -m/--method)
  opencode auth list         list credentials

Options:
  -h, --help     show help
  -v, --version  show version number
"""


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    C.no_network(TOOL)
    C.log_call(TOOL, argv)
    if argv == ["--selftest"]:
        return selftest()
    if argv and argv[0] in ("--version", "-v"):
        print(VERSION)
        return 0
    if argv and argv[0] in ("--help", "-h", "help"):
        print(HELP)
        return 0
    folders()
    words = [arg for arg in argv if arg != "--standalone"]
    if words[:1] in (["auth"], ["providers"]):
        sub = words[1:2]
        if sub == ["login"]:
            return auth_login(words[2:])
        if sub in (["list"], ["ls"]):
            return auth_list()
        print(f"lab stand-in: `opencode {' '.join(words)}` isn't emulated", file=sys.stderr)
        return 2
    if words[:1] == ["run"]:
        return run(words[1:])
    if words[:1] and words[0] in ("serve", "web", "acp", "mcp", "upgrade", "uninstall", "stats", "export", "import",
                                  "session", "models", "debug", "github", "console", "completion", "attach", "pr", "agent"):
        print(f"lab stand-in: `opencode {words[0]}` isn't emulated (the adapter doesn't run it)", file=sys.stderr)
        return 2
    return tui(argv)


def install(dest: Path) -> dict:
    wrapper = C.install_wrapper(dest, "opencode", "opencode.py")
    return {"AI_JUGGLER_OPENCODE_BIN": str(wrapper)}


# --------------------------------------------------------------- selftest

REPO = HERE.parent.parent.parent


def static_check() -> List[str]:
    """The names written here, as the adapter's Rust source spells them."""
    src = REPO / "crates/juggler-adapters/src"
    if not src.is_dir():
        print("opencode stand-in: the Rust source isn't beside this copy; static check skipped")
        return []
    text = {name: (src / name).read_text() for name in ("opencode.rs", "opencode_store.rs", "sign_in.rs", "opencode_environment.rs")}
    wanted = [
        ("opencode_store.rs", f'pub const DB_FILE: &str = "{DB_FILE}";'),
        ("sign_in.rs", f'OPENCODE_AUTH: &str = "{AUTH_FILE}";'),
        ("opencode_environment.rs", 'account_dir.join("data").join("opencode")'),
        ("opencode.rs", 'pub const ENV_DATA_HOME: &str = "XDG_DATA_HOME";'),
        ("opencode.rs", 'pub const ENV_CONFIG_HOME: &str = "XDG_CONFIG_HOME";'),
        ("opencode.rs", 'pub const ENV_STATE_HOME: &str = "XDG_STATE_HOME";'),
        ("opencode.rs", '(ENV_DATA_HOME, text(state_dir.join("data")))'),
        ("opencode.rs", 'const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_OPENCODE_BIN";'),
        ("opencode.rs", 'vec!["auth".into(), "login".into()]'),
        ("opencode.rs", 'args.extend(["--session".into(), session_id.clone()]);'),
        ("opencode.rs", 'args.push(format!("--prompt={prompt}"));'),
        ("opencode.rs", 'args.push("run".into());'),
        ("opencode.rs", 'args.extend(["--format".into(), "json".into()]);'),
        ("opencode.rs", '"1" => Some(Line::V1),'),
        ("opencode.rs", '"oauth" | "wellknown"'),
        ("opencode.rs", 'kind == "api"'),
        ("opencode_store.rs", "json_extract(data, '$.role')"),
        ("opencode_store.rs", '.pointer("/time/completed")'),
        ("opencode_store.rs", 'value.get("finish")'),
        ("opencode_store.rs", 'text(value.get("modelID"))'),
        ("opencode_store.rs", 'text(value.get("providerID"))'),
        ("opencode_store.rs", '"session_v2", "session"'),
    ]
    for column in ("id", "directory", "title", "parent_id", "cost", "tokens_input", "tokens_output", "tokens_reasoning",
                   "tokens_cache_read", "tokens_cache_write", "time_created", "time_updated", "time_archived"):
        wanted.append(("opencode_store.rs", column))
    for key in ("input", "output", "reasoning", "read", "write", "cache", "tokens", "time", "created", "cost"):
        wanted.append(("opencode_store.rs", f'"{key}"'))
    return [f"{file}: no `{needle}`" for file, needle in wanted if needle not in text[file]]


def selftest() -> int:
    import subprocess

    C.selftest_common()
    failures = static_check()

    def check(ok: bool, what: str) -> None:
        if not ok:
            failures.append(what)

    def answers(out: str) -> int:
        """Lines that are the answer `lab` (after the `> ` prompt, as no
        echo comes back without a terminal)."""
        return len(re.findall(r"(?m)^(?:> )?lab$", out))

    with C.selftest_lab(TOOL) as lab:
        def app(name: str) -> Tuple[Path, dict]:
            folder = lab / "data" / "accounts" / "opencode" / name
            for sub in ("data/opencode", "config/opencode", "state/opencode"):
                (folder / sub).mkdir(parents=True)
            env = {"HOME": str(lab), "PATH": "/usr/bin:/bin", "AIJ_LAB_STANDIN_WAIT": "0",
                   "XDG_DATA_HOME": str(folder / "data"), "XDG_CONFIG_HOME": str(folder / "config"),
                   "XDG_STATE_HOME": str(folder / "state"), "OPENCODE_CONFIG_DIR": str(folder / "config/opencode")}
            return folder, env

        def go(args: List[str], env: dict, stdin: str = "", cwd: Optional[Path] = None) -> "subprocess.CompletedProcess[str]":
            return subprocess.run(["/usr/bin/python3", "-I", str(Path(__file__).resolve()), *args], input=stdin,
                                  capture_output=True, text=True, env=env, cwd=str(cwd) if cwd else None, timeout=60)

        a, env_a = app("lab-a")
        done = go(["--version"], {"PATH": "/usr/bin:/bin"})
        check(done.stdout.strip() == VERSION and done.returncode == 0, f"--version: {done.stdout!r}")
        # Outside a lab: refused, nothing written.
        outside = dict(env_a, XDG_DATA_HOME="/tmp/not-a-lab-data")
        done = go(["auth", "login"], outside, "2\n1\n")
        check(done.returncode == 3 and not Path("/tmp/not-a-lab-data").exists(), f"outside a lab: {done.returncode}")
        # The planned path: OpenAI (row 2), then the browser (row 1).
        done = go(["auth", "login"], env_a, "2\n1\n", cwd=a)
        auth = read_auth(a / "data/opencode/auth.json")
        check(done.returncode == 0, f"login OpenAI: exit {done.returncode} {done.stderr}")
        check("◇  Select provider\n│  OpenAI" in done.stdout, "login OpenAI: no collapsed answer")
        check("● OpenCode (recommended)" in done.stdout and "○ OpenAI\n" in done.stdout,
              "login OpenAI: the hint isn't only beside the active row")
        check("ChatGPT Pro/Plus (browser)" in done.stdout and "Waiting for authorization..." in done.stdout
              and f".{C.FAKE_HOST}/" in done.stdout, "login OpenAI: no browser step")
        check(set(auth) == {"openai"} and auth["openai"]["type"] == "oauth"
              and auth["openai"]["refresh"] == auth["openai"]["access"] == C.NOT_A_TOKEN
              and isinstance(auth["openai"]["expires"], int), f"login OpenAI wrote {auth}")
        check(oct((a / "data/opencode/auth.json").stat().st_mode & 0o777) == "0o600", "auth.json isn't 0600")
        # The default row: the 2026-11-16 mistake, an API key.
        b, env_b = app("lab-b")
        done = go(["auth", "login"], env_b, "\nsomething typed\n", cwd=b)
        auth = read_auth(b / "data/opencode/auth.json")
        check(done.returncode == 0 and "Enter your API key" in done.stdout, f"login default: {done.stdout[-300:]}")
        check(auth == {"opencode": {"type": "api", "key": C.NOT_A_TOKEN}}, f"login default wrote {auth}")
        # Flags, an unknown method, and an abort.
        c, env_c = app("lab-c")
        done = go(["auth", "login", "-p", "OPENAI", "-m", "nope"], env_c)
        check(done.returncode == 1 and 'Unknown method "nope" for openai. Available: ChatGPT Pro/Plus (browser), '
              'ChatGPT Pro/Plus (headless), Manually enter API Key' in done.stdout, f"-m nope: {done.stdout}")
        done = go(["auth", "login", "-p", "nowhere"], env_c)
        check(done.returncode == 1 and 'Unknown provider "nowhere"' in done.stdout, f"-p nowhere: {done.stdout}")
        done = go(["auth", "login"], env_c, "")
        check(done.returncode == 130, f"abort: exit {done.returncode}")
        check(not (c / "data/opencode/auth.json").exists(), "an abort or an error wrote auth.json")
        done = go(["auth", "login", "--provider", "openai", "--method", "chatgpt pro/plus (HEADLESS)"], env_c)
        check(done.returncode == 0 and "Enter code: " in done.stdout
              and read_auth(c / "data/opencode/auth.json")["openai"]["type"] == "oauth", f"headless: {done.stdout}")
        # A session in the project, its turn end and usage, read as the adapter reads them.
        project = lab / "repos" / "lab"
        project.mkdir(parents=True)
        done = go([], env_a, "reply with the word lab\n", cwd=project)
        check(done.returncode == 0 and answers(done.stdout) == 1, f"session: {done.stdout} {done.stderr}")
        db = a / "data/opencode" / DB_FILE
        check(not Path(str(db) + "-wal").exists(), "the WAL log stayed after the session ended")
        conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        rows = conn.execute("SELECT id, directory, COALESCE(title, ''), parent_id, cost, tokens_input, tokens_output, "
                            "tokens_reasoning, tokens_cache_read, tokens_cache_write, time_created, time_updated, "
                            "time_archived FROM session").fetchall()
        check(len(rows) == 1 and os.path.realpath(rows[0][1]) == os.path.realpath(project) and rows[0][3] is None
              and rows[0][5] > 0, f"session rows: {rows}")
        session_id = rows[0][0] if rows else ""
        check(session_id.startswith("ses_"), f"session id {session_id}")

        def last_turn() -> Optional[str]:
            row = conn.execute("SELECT time_created, json_extract(data, '$.role'), data FROM message WHERE session_id = ?1 "
                               "ORDER BY time_created DESC, id DESC LIMIT 1", (session_id,)).fetchone()
            if row is None:
                return None
            data = json.loads(row[2])
            if row[1] == "user":
                return "running"
            if data.get("error") or data.get("finish") not in (None, "tool-calls", "unknown"):
                return "ended"
            return "ended" if data.get("time", {}).get("completed") and data.get("finish") is None else "running"

        check(last_turn() == "ended", f"turn end: {last_turn()}")
        conn.close()
        # Resume: the earlier turn comes back, a new one is appended.
        done = go(["--session", session_id], env_a, "reply with the word lab\n", cwd=project)
        earlier = done.stdout.find("> reply with the word lab\n  lab\n")
        check(done.returncode == 0 and earlier != -1 and answers(done.stdout[earlier:].split("  lab\n", 1)[1]) == 1,
              f"resume: {done.stdout}")
        done = go(["--session", "ses_nope"], env_a, "", cwd=project)
        check(done.returncode == 1 and "Session not found" in done.stderr, "an unknown session was opened")
        done = go(["--prompt=reply with the word lab"], env_a, "", cwd=project)
        check(done.returncode == 0 and answers(done.stdout) == 1, f"--prompt: {done.stdout}")
        done = go(["run", "--format", "json", "--", "reply with the word lab"], env_a, cwd=project)
        check(done.returncode == 0 and '"text": "lab"' in done.stdout, f"run: {done.stdout} {done.stderr}")
        conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        count = conn.execute("SELECT COUNT(*) FROM message WHERE session_id = ?", (session_id,)).fetchone()[0]
        check(count == 4, f"resume appended {count - 2} messages, not 2")
        usage = conn.execute("SELECT id, session_id, time_created, data FROM message WHERE time_updated >= ?1 AND "
                             "json_extract(data, '$.role') = 'assistant'", (0,)).fetchall()
        tokens = [json.loads(data)["tokens"] for (_, _, _, data) in usage if json.loads(data)["time"].get("completed")]
        check(len(tokens) == 4 and all(t["input"] > 0 and t["output"] > 0 for t in tokens), f"usage: {tokens}")
        check(all(json.loads(data)["cost"] == 0.0 for (_, _, _, data) in usage), "a ChatGPT sign-in was priced")
        sessions = conn.execute("SELECT COUNT(*) FROM session").fetchone()[0]
        check(sessions == 3, f"{sessions} sessions (one, its resume, --prompt's and run's: 3)")
        conn.close()
        # lab B (an API key) answers; an App with no sign-in fails its turn.
        done = go([], env_b, "reply with the word lab\n", cwd=project)
        check(answers(done.stdout) == 1, "an API-key App didn't answer")
        d, env_d = app("lab-d")
        done = go([], env_d, "reply with the word lab\n", cwd=project)
        check("no provider is signed in" in done.stdout, f"no sign-in: {done.stdout}")
        # A copy installed in a lab gets its launcher.
        dest = lab / "standins"
        dest.mkdir()
        made = install(dest)
        check(Path(made["AI_JUGGLER_OPENCODE_BIN"]).is_file(), "install made no launcher")
    for line in failures:
        print(f"FAIL {line}", file=sys.stderr)
    print("opencode stand-in: selftest " + ("FAILED" if failures else "passed"))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
