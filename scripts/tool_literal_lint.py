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
code (`/tests/`, `*.test.*`, `*.spec.*`, inline `#[cfg(test)]` modules),
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
TOOL_IDS = ("claude-code", "codex", "zcode", "gemini-cli", "gemini", "opencode", "kimi-code", "grok")
LITERAL = re.compile(r"""(["'`])(%s)\1""" % "|".join(re.escape(tool) for tool in TOOL_IDS))
SCAN = (("crates", ".rs"), ("ui/src", ".ts"), ("ui/src", ".tsx"))
EXEMPT_PARTS = (
    "crates/juggler-adapters/",
    "/tests/",
    "/fixtures/",
    "ui/src/preview/",
    "/types/generated/",
    "/generated.rs",
    "/benches/",
    "/examples/",
)
EXEMPT_SUFFIXES = (".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx", "/tests.rs", "_tests.rs")


def exempt(rel: str) -> bool:
    return any(part in f"/{rel}" for part in EXEMPT_PARTS) or f"/{rel}".endswith(EXEMPT_SUFFIXES)


def shipped_text(path: Path) -> str:
    """The file without its inline `#[cfg(test)]` module (Rust)."""
    text = path.read_text(encoding="utf-8", errors="replace")
    if path.suffix != ".rs":
        return text
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]" and index + 1 < len(lines) and lines[index + 1].lstrip().startswith("mod "):
            return "\n".join(lines[:index])
    return text


def counts(root: Path) -> dict[str, int]:
    found: dict[str, int] = {}
    for folder, suffix in SCAN:
        base = root / folder
        if not base.exists():
            continue
        for path in sorted(base.rglob(f"*{suffix}")):
            rel = path.relative_to(root).as_posix()
            if exempt(rel):
                continue
            n = len(LITERAL.findall(shipped_text(path)))
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
        (root / "ui/src/screens/A.tsx").write_text('const t = provider === "claude-code" ? 1 : 2;\n')
        (root / "ui/src/screens/A.test.tsx").write_text('render("codex");\n')
        allow = root / "allow"
        cases = [
            ("", 2, "both shared sites flagged, adapters/tests/inline tests exempt"),
            ("crates/juggler-app/src/a.rs 1\nui/src/screens/A.tsx 1\n", 0, "exactly allowed passes"),
            ("crates/juggler-app/src/a.rs 2\nui/src/screens/A.tsx 1\n", 1, "a stale higher count must shrink"),
            ("crates/juggler-app/src/a.rs 1\n", 1, "an unlisted file fails"),
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
