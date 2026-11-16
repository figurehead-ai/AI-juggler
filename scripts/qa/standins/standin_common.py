"""AJ-E1612: what every lab stand-in tool shares.

The stand-ins (opencode.py, kimi.py, grok.py, zcode_engine.py, muse.py in
this folder) let an agent dry-run the sign-in sitting's whole run sheet on
AI Juggler Dev before the maintainer is invited. Each answers the commands
its adapter runs, writes its files where the real tool would, and never:

- writes outside a lab (`/tmp/aij-lab…`): every path is checked first
  (`require_lab`), and the tool's home must already be in a lab;
- reaches the network: `no_network()` refuses every connection that
  isn't this Mac's loopback, and a stand-in that serves (a local usage
  server) binds 127.0.0.1 only;
- prints a real host: its fake sign-in pages are on `example.invalid`.

`scripts/qa/standins/install.py --lab LAB` copies them into
`<lab>/standins` and prints the adapters' test overrides
(AI_JUGGLER_*_BIN, AI_JUGGLER_ZCODE_ENGINE) that point Dev at them.

Every stand-in run appends one line to `<lab>/standins/calls.log` (its
program, its words and the variables its adapter set), so the dry run can
check what Dev ran.
"""
from __future__ import annotations

import contextlib
import json
import os
import re
import secrets
import shutil
import socket
import sys
import tempfile
import time
from pathlib import Path
from typing import Callable, Iterator

LAB = re.compile(r"^(/private)?/tmp/aij-lab[^/]*(/|$)")
FAKE_HOST = "example.invalid"
# Seconds a fake sign-in "waits for the browser" (AIJ_LAB_STANDIN_WAIT, which
# lab_env.sh passes on as one of the lab's own variables).
WAIT_DEFAULT = 3.0
NOT_A_TOKEN = "lab-not-a-token"


class NotALab(SystemExit):
    """A path outside a lab: the stand-in stops (exit 3)."""


def fail(tool: str, message: str, code: int = 3) -> None:
    sys.stderr.write(f"{tool} (lab stand-in): {message}\n")
    raise SystemExit(code)


def lab_root_of(path: str | os.PathLike[str]) -> Path | None:
    """The lab (`/tmp/aij-lab…`) a path lies in, by its real path; None
    when it lies in none."""
    real = os.path.realpath(os.fspath(path))
    if ".." in Path(os.fspath(path)).parts or not LAB.match(real):
        return None
    parts = Path(real).parts  # ('/', 'private'?, 'tmp', 'aij-lab…', …)
    at = 3 if parts[1] == "private" else 2
    return Path(*parts[: at + 1])


def require_lab(path: str | os.PathLike[str] | None, tool: str, what: str) -> Path:
    """`path`, which must lie in a lab; else the stand-in stops."""
    if not path:
        fail(tool, f"{what} isn't set: a stand-in runs only in a lab (/tmp/aij-lab…)")
    if lab_root_of(path) is None:
        fail(tool, f"{what} ({path}) isn't in a lab (/tmp/aij-lab…): refusing")
    return Path(os.fspath(path))


def no_network(tool: str) -> None:
    """Every outgoing connection but the loopback is refused, in this process."""
    loopback = {"127.0.0.1", "::1", "localhost"}
    real_connect = socket.socket.connect
    real_create = socket.create_connection

    def connect(self: socket.socket, address):  # type: ignore[no-untyped-def]
        host = address[0] if isinstance(address, tuple) else None
        if self.family != getattr(socket, "AF_UNIX", None) and host not in loopback:
            fail(tool, f"refused a connection to {host}: a stand-in never reaches the network", 4)
        return real_connect(self, address)

    def create_connection(address, *args, **kwargs):  # type: ignore[no-untyped-def]
        if address[0] not in loopback:
            fail(tool, f"refused a connection to {address[0]}: a stand-in never reaches the network", 4)
        return real_create(address, *args, **kwargs)

    socket.socket.connect = connect  # type: ignore[method-assign]
    socket.create_connection = create_connection  # type: ignore[assignment]


def wait_seconds() -> float:
    try:
        return max(0.0, float(os.environ.get("AIJ_LAB_STANDIN_WAIT", WAIT_DEFAULT)))
    except ValueError:
        return WAIT_DEFAULT


def device_code() -> str:
    alphabet = "BCDFGHJKLMNPQRSTVWXZ"
    raw = "".join(secrets.choice(alphabet) for _ in range(8))
    return f"{raw[:4]}-{raw[4:]}"


def fake_sign_in_page(tool: str, page: str, code: str | None = None) -> str:
    """Prints the fake page (and code) a sign-in shows, waits as if the
    maintainer finished it, and returns the page's address."""
    url = f"https://{page}.{FAKE_HOST}/device" + (f"?user_code={code}" if code else "")
    if code:
        print(f"Your one-time code: {code}", flush=True)
    print(f"Open this page to sign in: {url}", flush=True)
    print("(lab stand-in: nothing is opened; waiting as if you signed in)", flush=True)
    time.sleep(wait_seconds())
    return url


def write_json(path: Path, value: object, tool: str, mode: int = 0o600) -> None:
    """Writes `value` to `path` (in a lab only), atomically, owner-only."""
    require_lab(path, tool, "the file")
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(fd, "w") as handle:
            json.dump(value, handle, indent=2)
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp)
        raise


def append_line(path: Path, line: str, tool: str) -> None:
    require_lab(path, tool, "the file")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as handle:
        handle.write(line.rstrip("\n") + "\n")


def now_ms() -> int:
    return int(time.time() * 1000)


def new_id(prefix: str = "") -> str:
    return prefix + secrets.token_hex(8)


# The variables a stand-in records about how it was started (never a value
# that could be a key: none of these is one).
SEEN_VARS = (
    "HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME",
    "KIMI_CODE_HOME", "GROK_HOME", "ZCODE_DATA_BASE_DIR", "ZCODE_STORAGE_DIR", "ZCODE_SESSION_DB_PATH",
    "TBH_CREDENTIAL_BACKEND", "MUSE_NO_AUTO_UPDATE", "ELECTRON_RUN_AS_NODE", "ZCODE_BUILTIN_PROVIDER_CONFIG_FILE",
    "HERMES_HOME", "HERMES_GATEWAY_LOCK_DIR", "HERMES_SHARED_AUTH_DIR", "HERMES_MANAGED_DIR", "CODEX_HOME", "CLAUDE_CONFIG_DIR",
)


def log_call(tool: str, argv: list[str]) -> None:
    """One line per run in `<lab>/standins/calls.log`, when the stand-in
    lies in a lab's `standins` folder (a selftest's copy logs nowhere)."""
    here = Path(__file__).resolve().parent
    if here.name != "standins" or lab_root_of(here) is None or not (here / "INSTALLED").is_file():
        return
    seen = {name: os.environ[name] for name in SEEN_VARS if name in os.environ}
    line = json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%S"), "tool": tool, "argv": argv,
                       "cwd": os.getcwd(), "env": seen}, sort_keys=True)
    with contextlib.suppress(OSError):
        append_line(here / "calls.log", line, tool)


def repl(tool: str, prompt: str, on_turn: Callable[[str], str], banner: str = "") -> int:
    """A minimal interactive session: a prompt, one line in, its answer out,
    until end of input, `/exit` or `/quit`. `on_turn` records the turn in the
    tool's own store and returns the answer."""
    if banner:
        print(banner, flush=True)
    while True:
        try:
            sys.stdout.write(prompt)
            sys.stdout.flush()
            line = sys.stdin.readline()
        except KeyboardInterrupt:
            print()
            return 130
        if not line:
            print()
            return 0
        text = line.strip()
        if not text:
            continue
        if text in ("/exit", "/quit", "exit", "quit"):
            return 0
        print(on_turn(text), flush=True)


def answer_for(text: str) -> str:
    """Every stand-in answers the sitting's one prompt the same way."""
    return "lab"


@contextlib.contextmanager
def selftest_lab(tool: str) -> Iterator[Path]:
    """A throwaway lab for a stand-in's selftest, removed afterwards."""
    root = Path(tempfile.mkdtemp(prefix=f"aij-lab-standin-{tool}-", dir="/tmp"))
    try:
        yield root
    finally:
        shutil.rmtree(root, ignore_errors=True)


def install_wrapper(dest: Path, name: str, script: str) -> Path:
    """`<dest>/bin/<name>`: a small launcher that runs this folder's
    `script` with the system Python, isolated (-I)."""
    require_lab(dest, name, "the stand-ins' folder")
    bin_dir = dest / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    wrapper = bin_dir / name
    wrapper.write_text(f'#!/bin/sh\nexec /usr/bin/python3 -I "{dest / script}" "$@"\n')
    wrapper.chmod(0o755)
    return wrapper


def selftest_common() -> None:
    assert lab_root_of("/tmp/aij-lab-t16/a/b") == Path("/private/tmp/aij-lab-t16") or \
        lab_root_of("/tmp/aij-lab-t16/a/b") == Path("/tmp/aij-lab-t16")
    assert lab_root_of("/Users/you/.kimi-code") is None
    assert lab_root_of("/tmp/aij-lab-t16/../x") is None
    assert lab_root_of("/tmp/other") is None
    try:
        require_lab("/Users/you", "x", "HOME")
        raise AssertionError("a real home was accepted")
    except SystemExit as stop:
        assert stop.code == 3
    no_network("x")
    try:
        socket.create_connection(("192.0.2.1", 443), timeout=0.1)
        raise AssertionError("a connection left the Mac")
    except SystemExit as stop:
        assert stop.code == 4
    url = f"https://kimi.{FAKE_HOST}/device"
    assert url.endswith(".invalid/device")
