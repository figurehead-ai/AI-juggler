#!/usr/bin/env python3
"""AJ-E1525 (ADR-0019 §5, ENGINEERING-GUIDE A2): no tool-id literal outside
the adapters.

A tool id written in shared code ("claude-code", "codex", ...) is a branch
that every new tool has to find. The UI and the commands read the
adapter's ToolSpec through `tools_catalog` instead. This lint counts tool-id
string literals per file in shared code and compares the counts with the
allowlist `scripts/tool_literals.allow`, which **only shrinks**:

- a file over its allowed count, or not listed at all, fails (a new
  literal);
- a file under its allowed count fails too, until its line in the
  allowlist is lowered or removed (so the allowlist can't hide new ones
  later).

Exempt: `crates/juggler-adapters/` (the adapters own their ids), test
code (`/tests/`, `*.test.*`, `*.spec.*`, inline `#[cfg(test)]` modules, wherever they sit and however they are
declared, and files that are a `#[cfg(test)] mod name;`),
generated code, fixtures and the UI's preview data (`ui/src/preview/`).

Usage: tool_literal_lint.py [--print] [--selftest]
  --print     print today's counts in the allowlist's format
  --selftest  check the lint against a seeded tree
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "tool_literals.allow"
# The registered tools' ids, plus the ones E16 adds: a literal of any of
# them in shared code is a per-tool branch.
TOOL_IDS = ("claude-code", "codex", "zcode", "gemini-cli", "gemini", "opencode", "kimi-code", "grok-build")
LITERAL = re.compile(r"""(["'`])(%s)\1""" % "|".join(re.escape(tool) for tool in TOOL_IDS))
# Rust has no backtick strings: `codex` there is a doc comment's prose.
RUST_LITERAL = re.compile(r"""(["'])(%s)\1""" % "|".join(re.escape(tool) for tool in TOOL_IDS))
SCAN = (("crates", ".rs"), ("ui/src", ".ts"), ("ui/src", ".tsx"))
EXEMPT_PARTS = (
    "crates/juggler-adapters/",
    "/tests/",
    "/fixtures/",
    "ui/src/test/",
    "ui/src/preview/",
    "/types/generated/",
    "/generated.rs",
    "/benches/",
    "/examples/",
)
EXEMPT_SUFFIXES = (".test.ts", ".test.tsx", ".test-kit.ts", ".spec.ts", ".spec.tsx", "/tests.rs", "_tests.rs")


def exempt(rel: str) -> bool:
    return any(part in f"/{rel}" for part in EXEMPT_PARTS) or f"/{rel}".endswith(EXEMPT_SUFFIXES)


# `#[cfg(test)]` (or `#[cfg(all(test, …))]`), then any other attributes,
# then `mod name {` or `mod name;`, with or without `pub`/`pub(crate)`.
CFG_TEST = re.compile(r"#\[cfg\((?:all\()?\s*test\b[^\]]*\]")
TEST_MOD = re.compile(r"\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*([{;])")


def _test_mods(text: str) -> tuple[str, list[str]]:
    """`text` without its inline test modules, and the names of the test
    modules it declares in other files (`#[cfg(test)] mod name;`)."""
    out: list[str] = []
    files: list[str] = []
    pos = 0
    for found in CFG_TEST.finditer(text):
        if found.start() < pos:
            continue
        decl = TEST_MOD.match(text, found.end())
        if not decl:
            continue
        out.append(text[pos : found.start()])
        if decl.group(2) == ";":
            files.append(decl.group(1))
            pos = decl.end()
            continue
        depth, index = 1, decl.end()
        while index < len(text) and depth:
            depth += {"{": 1, "}": -1}.get(text[index], 0)
            index += 1
        pos = index
    out.append(text[pos:])
    return "".join(out), files


def test_files(root: Path) -> set[str]:
    """Rust files that are test modules of another file (`#[cfg(test)] mod
    name;`), whatever they are called: `name.rs` or `name/mod.rs` beside a
    `lib.rs`/`main.rs`/`mod.rs`, under `parent/` beside any other file."""
    found: set[str] = set()
    for path in (root / "crates").rglob("*.rs") if (root / "crates").exists() else ():
        _, names = _test_mods(path.read_text(encoding="utf-8", errors="replace"))
        base = path.parent if path.name in ("lib.rs", "main.rs", "mod.rs") else path.parent / path.stem
        for name in names:
            for candidate in (base / f"{name}.rs", base / name / "mod.rs"):
                found.add(candidate.relative_to(root).as_posix())
    return found


def shipped_text(path: Path) -> str:
    """The file without its inline `#[cfg(test)]` modules (Rust)."""
    text = path.read_text(encoding="utf-8", errors="replace")
    if path.suffix != ".rs":
        return text
    return _test_mods(text)[0]


def counts(root: Path) -> dict[str, int]:
    found: dict[str, int] = {}
    tests = test_files(root)
    for folder, suffix in SCAN:
        base = root / folder
        if not base.exists():
            continue
        for path in sorted(base.rglob(f"*{suffix}")):
            rel = path.relative_to(root).as_posix()
            if exempt(rel) or rel in tests:
                continue
            n = len((RUST_LITERAL if suffix == ".rs" else LITERAL).findall(shipped_text(path)))
            if n:
                found[rel] = n
    return found


def allowed(allow: Path) -> dict[str, int]:
    entries: dict[str, int] = {}
    if not allow.exists():
        return entries
    for raw in allow.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        rel, count = line.rsplit(None, 1)
        entries[rel] = int(count)
    return entries


def check(root: Path, allow: Path) -> list[str]:
    now = counts(root)
    limits = allowed(allow)
    problems = []
    for rel, n in sorted(now.items()):
        limit = limits.get(rel, 0)
        if n > limit:
            problems.append(
                f"{rel}: {n} tool-id literal(s), {limit} allowed — read the ToolSpec from tools_catalog (ADR-0019)"
            )
    for rel, limit in sorted(limits.items()):
        n = now.get(rel, 0)
        if n < limit:
            problems.append(
                f"{rel}: {n} left, the allowlist says {limit} — lower its line in {allow.relative_to(root).as_posix() if allow.is_relative_to(root) else allow} (it only shrinks)"
            )
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates/juggler-app/src").mkdir(parents=True)
        (root / "crates/juggler-adapters/src").mkdir(parents=True)
        (root / "ui/src/screens").mkdir(parents=True)
        (root / "crates/juggler-app/src/a.rs").write_text(
            'fn f(id: &str) -> bool { id == "codex" }\n#[cfg(test)]\nmod tests { const X: &str = "claude-code"; }\n'
        )
        (root / "crates/juggler-adapters/src/codex.rs").write_text('pub const CODEX_ID: &str = "codex";\n')
        (root / "crates/juggler-app/src/b.rs").write_text(
            'fn g() {}\n#[cfg(test)]\npub(crate) mod tests { const X: &str = "codex"; }\n'
            '#[cfg(test)] #[path = "kit.rs"]\npub mod kit_inline { fn k() { let _ = "zcode"; } }\n'
            '#[cfg(all(test, unix))]\nmod fakes;\n'
        )
        (root / "crates/juggler-app/src/b").mkdir()
        (root / "crates/juggler-app/src/b/fakes.rs").write_text('const F: &str = "codex";\n')
        (root / "crates/juggler-app/src/c.rs").write_text(
            '#[cfg(test)]\nmod tests { }\n/// Reads `codex` prose.\nfn after() -> &\'static str { "gemini" }\n'
        )
        (root / "ui/src/screens/A.tsx").write_text('const t = provider === "claude-code" ? 1 : 2;\n')
        (root / "ui/src/screens/A.test.tsx").write_text('render("codex");\n')
        allow = root / "allow"
        cases = [
            ("", 3, "shared sites flagged (code after a test module too); every test-module form exempt"),
            ("crates/juggler-app/src/a.rs 1\nui/src/screens/A.tsx 1\ncrates/juggler-app/src/c.rs 1\n", 0, "exactly allowed passes"),
            ("crates/juggler-app/src/a.rs 2\nui/src/screens/A.tsx 1\ncrates/juggler-app/src/c.rs 1\n", 1, "a stale higher count must shrink"),
            ("crates/juggler-app/src/a.rs 1\ncrates/juggler-app/src/c.rs 1\n", 1, "an unlisted file fails"),
        ]
        bad = 0
        for text, want, what in cases:
            allow.write_text(text)
            got = len(check(root, allow))
            if got != want:
                print(f"tool-literal-lint selftest FAILED: {what}: {got} problem(s), wanted {want}")
                bad += 1
        if bad:
            return 1
    print("tool-literal-lint selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--print" in sys.argv:
        for rel, n in sorted(counts(ROOT).items()):
            print(f"{rel} {n}")
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(f"tool-literal-lint: {problem}")
    if problems:
        return 1
    total = sum(counts(ROOT).values())
    print(f"tool-literal-lint: OK ({total} allowlisted site(s) left, in {len(allowed(ALLOW))} file(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
