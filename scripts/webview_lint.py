#!/usr/bin/env python3
"""AJ-E2003: what the web views may do stays narrow.

Fails when
- a capability file (crates/juggler-app/capabilities/*.json) grants a
  shell, fs, http or opener permission, any plugin's whole `default` set
  or `core:default`, a remote URL, or a window wildcard of "*";
- tauri.conf.json's CSP loses one of the required directives, or the
  config allows remote IPC, or a window is created by the config
  (windows are built guarded in code: src/webview_guard.rs);
- a browser is opened (`.open_url(`) outside webview_guard.rs and the
  system-settings seam (scripts/webview_lint.allow);
- a command the page invokes (`invoke("name"` under ui/src) isn't one the
  main window's set grants (the registry in src/lib.rs, with each
  `#[tauri::command(rename = …)]` applied, as build.rs generates it), or
  the terminal window's set names a command that doesn't exist;
- a registered command takes a path-like argument (a string named path,
  dest, cwd, folder, file, root, source…) and its body never asks
  `picked::vouch` (R6: the page passes ids, or a path the user picked,
  dropped or AI Juggler already knows), unless scripts/path_params.allow
  names it with the reason;
- a web view window is built (`WebviewWindowBuilder::new` /
  `from_config`) without going through `webview_guard`.

Usage: webview_lint.py [--selftest]
"""
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
APP = ROOT / "crates/juggler-app"
ALLOW = ROOT / "scripts/webview_lint.allow"

FORBIDDEN = re.compile(r"^(shell|fs|http|opener):|^core:default$|:default$")
CSP_NEEDS = {
    "script-src": "'self'",
    "object-src": "'none'",
    "base-uri": "'none'",
    "frame-ancestors": "'none'",
    "form-action": "'none'",
}


def capability_problems(name: str, cap: dict) -> list[str]:
    out = []
    for perm in cap.get("permissions", []):
        ident = perm if isinstance(perm, str) else perm.get("identifier", "")
        if FORBIDDEN.search(ident):
            out.append(f"{name}: grants {ident}")
    if cap.get("remote"):
        out.append(f"{name}: allows remote URLs")
    if "*" in cap.get("windows", []) or "*" in cap.get("webviews", []):
        out.append(f"{name}: applies to every window")
    return out


def csp_problems(csp: str) -> list[str]:
    directives = {
        part.split()[0]: part.split()[1:] for part in (p.strip() for p in csp.split(";")) if part
    }
    return [
        f"CSP: {key} must be {want}"
        for key, want in CSP_NEEDS.items()
        if directives.get(key) != [want]
    ]


def registered() -> set[str]:
    """The invoked names of every registered command, as build.rs grants them."""
    src = (APP / "src/lib.rs").read_text()
    body = src[src.index("macro_rules! app_commands"):]
    body = body[: body.index("\n}\n")]
    names = {
        words[1][:-1].split("::")[-1]
        for words in (line.split() for line in body.splitlines())
        if len(words) == 2 and words[0] in ("Read", "Write") and words[1].endswith(",")
    }
    renames = {}
    for path in (APP / "src").rglob("*.rs"):
        for m in re.finditer(
            r'#\[tauri::command\([^)]*rename\s*=\s*"([^"]+)"[^)]*\)\](?:\s*(?://[^\n]*|#\[[^\n]*\]))*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)',
            path.read_text(),
        ):
            renames[m.group(2)] = m.group(1)
    return {renames.get(name, name) for name in names}


PATHLIKE = re.compile(r"(^|_)(path|paths|dest|cwd|folder|folders|file|dir|root|source)$")


def unvouched() -> list[str]:
    """Registered commands with a path-like string argument whose body
    never calls vouch (directly or through a `*_vouched` helper)."""
    src = (APP / "src/lib.rs").read_text()
    body = src[src.index("macro_rules! app_commands"):]
    body = body[: body.index("\n}\n")]
    names = {
        words[1][:-1].split("::")[-1]
        for words in (line.split() for line in body.splitlines())
        if len(words) == 2 and words[0] in ("Read", "Write") and words[1].endswith(",")
    }
    found = []
    for path in sorted((APP / "src").rglob("*.rs")):
        text = path.read_text()
        for m in re.finditer(r"#\[tauri::command[^\]]*\][^{]*?\bfn (\w+)\s*(?:<[^>]*>)?\((.*?)\)\s*(?:->[^{]*)?\{", text, re.S):
            if m.group(1) not in names:
                continue
            args = [
                (n, t) for n, t in re.findall(r"(\w+)\s*:\s*([^,]+)", m.group(2))
                if PATHLIKE.search(n) and re.search(r"String|PathBuf|&str", t)
            ]
            if not args:
                continue
            end = text.find("\n}\n", m.end())
            if "vouch" not in text[m.end(): end]:
                found.append(f"{m.group(1)}({', '.join(n for n, _ in args)})")
    return found


def invoked() -> dict[str, str]:
    """Every command name the page invokes literally → the first file."""
    found: dict[str, str] = {}
    for path in sorted((ROOT / "ui/src").rglob("*.ts*")):
        if ".test." in path.name or "/preview/" in str(path):
            continue
        for name in re.findall(r'invoke(?:<[^>()]*(?:<[^>]*>)?[^>()]*>)?\(\s*"([a-z_0-9]+)"', path.read_text()):
            found.setdefault(name, str(path.relative_to(ROOT)))
    return found


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        assert capability_problems("x", {"permissions": ["fs:allow-read"]})
        assert capability_problems("x", {"permissions": ["core:default"]})
        assert capability_problems("x", {"permissions": ["dialog:default"]})
        assert capability_problems("x", {"permissions": [{"identifier": "shell:allow-open"}]})
        assert capability_problems("x", {"remote": {"urls": ["https://x"]}})
        assert not capability_problems("x", {"permissions": ["core:event:allow-listen", "main-window"]})
        assert csp_problems("default-src 'self'")
        assert not csp_problems(
            "default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; "
            "frame-ancestors 'none'; form-action 'none'"
        )
        print("webview_lint: selftest ok")
        return 0
    problems: list[str] = []
    for path in sorted((APP / "capabilities").glob("*.json")):
        problems += capability_problems(path.name, json.loads(path.read_text()))
    conf = json.loads((APP / "tauri.conf.json").read_text())
    security = conf.get("app", {}).get("security", {})
    problems += csp_problems(security.get("csp", ""))
    if security.get("dangerousRemoteDomainIpcAccess"):
        problems.append("tauri.conf.json: remote IPC access")
    for window in conf.get("app", {}).get("windows", []):
        if window.get("create", True):
            problems.append(f"tauri.conf.json: window {window.get('label', 'main')} is created unguarded")
    allowed = {
        line.split("#")[0].strip()
        for line in ALLOW.read_text().splitlines()
        if line.split("#")[0].strip()
    }
    for path in sorted((APP / "src").rglob("*.rs")):
        rel = str(path.relative_to(ROOT))
        text = path.read_text()
        if ".open_url(" in text and rel not in allowed and not rel.endswith("webview_guard.rs"):
            problems.append(f"{rel}: opens the browser itself (use webview_guard::open_web)")
        if rel.endswith("webview_guard.rs"):
            continue
        for m in re.finditer(r"WebviewWindowBuilder::(new|from_config)\(", text):
            before = text[max(0, m.start() - 80): m.start()]
            if "guarded(" not in before:
                line = text.count("\n", 0, m.start()) + 1
                problems.append(f"{rel}:{line}: a window built without webview_guard::guarded")
    granted = registered()
    for name, where in sorted(invoked().items()):
        if name not in granted:
            problems.append(f"{where}: invokes {name}, which no window is granted")
    terminal = (APP / "permissions/terminal-window.toml").read_text()
    for ident in re.findall(r'"allow-([a-z0-9-]+)"', terminal):
        if ident.replace("-", "_") not in granted:
            problems.append(f"permissions/terminal-window.toml: allow-{ident} names no registered command")
    reasons = {
        line.split("#")[0].strip()
        for line in (ROOT / "scripts/path_params.allow").read_text().splitlines()
        if line.split("#")[0].strip()
    }
    for command in unvouched():
        if command.split("(")[0] not in reasons:
            problems.append(f"command {command} takes a path the page could make up: vouch for it (picked.rs)")
    for problem in problems:
        print(f"webview_lint: {problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
