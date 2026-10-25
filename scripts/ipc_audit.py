#!/usr/bin/env python3
"""AJ-E2011 / ENGINEERING-GUIDE A3, T1, I3, I4: the IPC audit.

Reads the commands `juggler-app` registers (the `app_commands!` registry
in `lib.rs`, each line `Read|Write module::command,`; or a plain
`generate_handler![...]`), their definitions (`#[tauri::command]` functions under
`crates/juggler-app/src`) and every `invoke("…")` in the UI (non-test
code), then checks:

- every command the UI invokes is registered (no allowlist: a typo is a
  runtime failure);
- every registered command has a caller (I3);
- a command is `async` (T1): Tauri runs a sync command on the main
  thread, so any read there (storage, files, processes, a lock another
  thread holds) freezes the window (0.1.5 review: opening a page showed
  the spinning wheel). The only way past is `// ipc: main-thread (why)`
  just above it, with its reason, and a `main-thread` line in the
  allowlist (today: none);
- a command body is at most 40 lines (A3: thin commands);
- a command in `features/<feature>/` is named `<feature>_<verb>`, and every
  command has a verb after its feature (I4).

The last five start from `scripts/ipc_audit.allow`, which only shrinks: a
listed command that now passes (or is gone) must leave the list.

Usage: ipc_audit.py [--print] [--selftest]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "ipc_audit.allow"
MAX_BODY = 40
RULES = ("uncalled", "sync", "main-thread", "long", "name")
MARKER = re.compile(r"ipc: main-thread\s*\(([^)]+)\)")
COMMAND = re.compile(r"#\[tauri::command(?:\(([^)]*)\))?\][^\n]*\n((?:\s*(?://[^\n]*|#\[[^\n]*\])\n)*)\s*pub(?:\(crate\))?\s+(async\s+)?fn\s+(\w+)")
RENAME = re.compile(r'rename\s*=\s*"([a-z0-9_]+)"')
# A type argument may nest one level (`invoke<Page<Row>>("…")`).
INVOKE = re.compile(r"""\binvoke\s*(?:<(?:[^<>]|<[^<>]*>)*>)?\s*\(\s*["'`]([a-z0-9_]+)["'`]""")
HANDLER = re.compile(r"generate_handler!\[(.*?)\]", re.S)
# The registry (lib.rs `app_commands!`): one `Read|Write path::to::command,` a line.
REGISTRY = re.compile(r"^\s*(?:Read|Write)\s+((?:\w+::)*\w+),", re.M)


def registered(lib: str, renames: dict[str, str] | None = None) -> set[str]:
    if "macro_rules! app_commands" in lib:
        return {(renames or {}).get(path.split("::")[-1], path.split("::")[-1]) for path in REGISTRY.findall(lib)}
    match = HANDLER.search(lib)
    if not match:
        return set()
    names = set()
    for item in match.group(1).split(","):
        item = re.sub(r"//[^\n]*", "", item).strip()
        if item:
            fn = item.split("::")[-1]
            names.add((renames or {}).get(fn, fn))
    return names


def body_lines(text: str, start: int) -> int:
    """Lines from the fn's `{` to its closing `}` (brace counting)."""
    open_at = text.index("{", start)
    depth = 0
    for index in range(open_at, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text.count("\n", open_at, index) - 1
    return 0


def definitions(src: Path) -> dict[str, dict]:
    found = {}
    for path in sorted(src.rglob("*.rs")):
        rel = path.relative_to(src).as_posix()
        if "/tests/" in f"/{rel}":
            continue
        text = path.read_text(encoding="utf-8")
        for match in COMMAND.finditer(text):
            rename = RENAME.search(match.group(1) or "")
            fn = match.group(4)
            above = text[max(0, match.start() - 400): match.start()].splitlines()[-3:]
            marks = [line for line in above + match.group(2).splitlines() if "ipc: main-thread" in line]
            found[rename.group(1) if rename else fn] = {
                "fn": fn,
                "file": rel,
                "async": bool(match.group(3)),
                "marker": bool(marks),
                "reason": next((m.group(1).strip() for line in marks if (m := MARKER.search(line))), ""),
                "lines": body_lines(text, match.end()),
            }
    return found


def ui_invokes(ui: Path) -> set[str]:
    names = set()
    for path in sorted(list(ui.rglob("*.ts")) + list(ui.rglob("*.tsx"))):
        rel = path.as_posix()
        if ".test." in rel or ".spec." in rel or "/types/generated/" in rel:
            continue
        names.update(INVOKE.findall(path.read_text(encoding="utf-8")))
    return names


def findings(root: Path) -> tuple[list[str], dict[str, set[str]]]:
    """Hard failures, and the per-rule sets the allowlist governs."""
    src = root / "crates" / "juggler-app" / "src"
    lib = (src / "lib.rs").read_text(encoding="utf-8") if (src / "lib.rs").exists() else ""
    defs = definitions(src)
    commands = registered(lib, {info["fn"]: name for name, info in defs.items()})
    called = ui_invokes(root / "ui" / "src")
    hard = [f"the UI invokes `{name}`, which isn't a registered command" for name in sorted(called - commands)]
    rules: dict[str, set[str]] = {rule: set() for rule in RULES}
    for name in sorted(commands):
        info = defs.get(name)
        if name not in called:
            rules["uncalled"].add(name)
        if info is None:
            continue
        if not info["async"] and not info["marker"]:
            rules["sync"].add(name)
        if not info["async"] and info["marker"]:
            rules["main-thread"].add(name)
            if not info["reason"]:
                hard.append(f"`{name}` is marked `// ipc: main-thread` without its reason — `// ipc: main-thread (why)`")
        if info["lines"] > MAX_BODY:
            rules["long"].add(name)
        feature = re.match(r"features/([a-z0-9_]+)/", info["file"])
        if "_" not in name or (feature and not name.startswith(f"{feature.group(1)}_")):
            rules["name"].add(name)
    return hard, rules


WORDS = {
    "uncalled": "is registered but nothing in the UI calls it (I3) — call it, remove it, or list it with a date",
    "sync": "is a sync command, so Tauri runs it on the main thread — make it `async` with its work in `crate::ipc::blocking` (T1)",
    "main-thread": "runs on the main thread — make it `async`; only a command that must run there may stay, marked `// ipc: main-thread (why)` and listed with its reason",
    "long": f"has a body over {MAX_BODY} lines — move the logic into the feature's service (A3)",
    "name": "isn't named <feature>_<verb> (I4)",
}


def allowed(allow: Path) -> dict[str, set[str]]:
    entries: dict[str, set[str]] = {rule: set() for rule in RULES}
    if allow.exists():
        for raw in allow.read_text(encoding="utf-8").splitlines():
            line = raw.split("#", 1)[0].strip()
            if line:
                rule, name = line.split(None, 1)
                entries.setdefault(rule, set()).add(name.split()[0])
    return entries


def check(root: Path, allow: Path) -> list[str]:
    hard, rules = findings(root)
    listed = allowed(allow)
    problems = list(hard)
    for rule in RULES:
        for name in sorted(rules[rule] - listed.get(rule, set())):
            problems.append(f"`{name}` {WORDS[rule]}")
        for name in sorted(listed.get(rule, set()) - rules[rule]):
            problems.append(f"`{name}` now passes `{rule}` (or is gone) — remove its line from the allowlist (it only shrinks)")
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "crates" / "juggler-app" / "src"
        (src / "features" / "tools").mkdir(parents=True)
        (root / "ui" / "src").mkdir(parents=True)
        (src / "lib.rs").write_text("tauri::generate_handler![features::tools::commands::tools_catalog, commands::old_thing, commands::app_pure]")
        (src / "features" / "tools" / "commands.rs").write_text(
            "#[tauri::command]\npub async fn tools_catalog() -> u8 {\n    1\n}\n"
        )
        (src / "commands.rs").write_text(
            "#[tauri::command]\npub fn old_thing() -> u8 {\n" + "    let _x = 1;\n" * 45 + "    1\n}\n"
            "// ipc: main-thread (pure)\n#[tauri::command]\npub fn app_pure() -> u8 {\n    1\n}\n"
        )
        (root / "ui" / "src" / "a.ts").write_text('invoke("tools_catalog"); invoke<number>("old_thing"); invoke("app_pure");\n')
        allow = root / "allow"
        cases = [
            ("sync old_thing\nlong old_thing\nmain-thread app_pure\n", 0, "a thin async command, a listed marked one and a listed old one pass"),
            ("sync old_thing\nlong old_thing\n", 1, "a marked command that isn't listed fails"),
            ("long old_thing\nmain-thread app_pure\n", 1, "an unlisted sync command fails"),
            ("sync old_thing\nmain-thread app_pure\n", 1, "an unlisted long body fails"),
            ("sync old_thing\nlong old_thing\nmain-thread app_pure\nname tools_catalog\n", 1, "a listed command that passes must leave the list"),
        ]
        bad = 0
        for text, want, what in cases:
            allow.write_text(text)
            got = len(check(root, allow))
            if got != want:
                print(f"ipc-audit selftest FAILED: {what}: {got} problem(s), wanted {want}: {check(root, allow)}")
                bad += 1
        # A marker with no reason is refused, listed or not.
        (src / "quiet.rs").write_text("// ipc: main-thread\n#[tauri::command]\npub fn app_quiet() -> u8 {\n    1\n}\n")
        (src / "lib.rs").write_text("tauri::generate_handler![features::tools::commands::tools_catalog, commands::old_thing, commands::app_pure, quiet::app_quiet]")
        (root / "ui" / "src" / "c.ts").write_text('invoke("app_quiet");\n')
        allow.write_text("sync old_thing\nlong old_thing\nmain-thread app_pure\nmain-thread app_quiet\n")
        if not any("without its reason" in problem for problem in check(root, allow)):
            print("ipc-audit selftest FAILED: a main-thread marker needs its reason")
            bad += 1
        (src / "quiet.rs").unlink()
        (root / "ui" / "src" / "c.ts").unlink()
        (src / "lib.rs").write_text("tauri::generate_handler![features::tools::commands::tools_catalog, commands::old_thing, commands::app_pure]")
        (root / "ui" / "src" / "b.ts").write_text('invoke("tools_catalgo");\n')
        allow.write_text("sync old_thing\nlong old_thing\nmain-thread app_pure\n")
        if not any("tools_catalgo" in problem for problem in check(root, allow)):
            print("ipc-audit selftest FAILED: a typo'd invoke must fail")
            bad += 1
        # The registry form reads the same commands.
        lib = "macro_rules! app_commands {\n    Read  features::tools::commands::tools_catalog,\n    Write commands::old_thing,\n    Read  commands::app_pure,\n}"
        if registered(lib) != {"tools_catalog", "old_thing", "app_pure"}:
            print("ipc-audit selftest FAILED: the app_commands! registry must read like generate_handler!")
            bad += 1
    if bad:
        return 1
    print("ipc-audit selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--print" in sys.argv:
        hard, rules = findings(ROOT)
        for problem in hard:
            print(f"# HARD: {problem}")
        for rule in RULES:
            for name in sorted(rules[rule]):
                print(f"{rule} {name}")
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(f"ipc-audit: {problem}")
    if problems:
        return 1
    _, rules = findings(ROOT)
    print("ipc-audit: OK (" + ", ".join(f"{len(rules[rule])} {rule}" for rule in RULES) + " allowlisted)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
