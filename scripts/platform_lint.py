#!/usr/bin/env python3
"""Cross-platform readiness (docs/research/cross-platform-readiness.md §5.1):
no new macOS- or Unix-only assumption outside the platform seams.

Windows and Linux builds are coming. Each assumption written straight into
feature code is a place a port has to find; behind a seam it is one
implementation per platform. This lint counts, per file and kind, what a
seam should hold, in shipped code outside the seams:

- `unix-api`      `std::os::unix` in code (modes, owners, sockets, signals)
- `unix-socket`   `UnixListener` / `UnixStream` / `UnixDatagram` in code
- `usr-bin`       a string literal naming `/usr/bin/…`
- `applications`  a string literal naming `/Applications`
- `app-bundle`    a string literal matching `.app` (a macOS bundle)
- `slash-split`   `rsplit('/')` and kin on a path (Windows uses `\\` too)
- `mac-words`     "macOS" or "Mac" in text that can reach the screen
                  (UI and Rust copy; identifiers and comments don't count)

The seams are exempt: `crates/juggler-platform/` (the OS seams), the
audited per-OS shims (`juggler-login-item`, `juggler-scap-init`,
`juggler-vm-stats`) and `juggler-adapter/src/system_tools.rs` (the one place
system tools are named by path). So are tests (`/tests/`, `*.test.*`,
`*.spec.*`, inline `#[cfg(test)]` / `#[cfg(all(test, …))]` modules), test
fixture programs (`src/bin/fake_*`), generated code and the UI's preview
data.

The allowlist `scripts/platform.allow` (`path kind count`) **only
shrinks**, like `tool_literals.allow` and `plain_words.allow`: a file over
its count, or not listed, fails (a new assumption: put it behind a seam);
a file under its count fails until its line is lowered or removed.

Usage: platform_lint.py [--print] [--selftest]
  --print     print today's counts in the allowlist's format
  --selftest  check the lint against a seeded tree
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plain_words_lint as words  # noqa: E402  (shared tokenizer, same rules for "on screen")

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "platform.allow"

SEAMS = (
    "crates/juggler-platform/",
    "crates/juggler-login-item/",
    "crates/juggler-scap-init/",
    "crates/juggler-vm-stats/",
    "crates/juggler-adapter/src/system_tools.rs",
)
SCAN = (("crates", (".rs",)), ("ui/src", (".ts", ".tsx")))
EXEMPT_PARTS = words.EXEMPT_PARTS + ("/src/bin/fake_", "/build.rs")

# What each kind is, and the seam to use instead.
KINDS: dict[str, str] = {
    "unix-api": "a juggler-platform seam (private_fs, local_ipc, file_trust, user)",
    "unix-socket": "juggler_platform::local_ipc",
    "usr-bin": "juggler_adapter::system_tools (one place, per OS)",
    "applications": "a per-OS location behind a seam",
    "app-bundle": "a per-OS executable identity behind a seam",
    "slash-split": "Path::file_name / components (both separators)",
    "mac-words": "words for any computer (\"this computer\", \"the system\")",
}
CODE: dict[str, re.Pattern[str]] = {
    "unix-api": re.compile(r"\bstd::os::unix\b"),
    "unix-socket": re.compile(r"\bUnix(?:Listener|Stream|Datagram)\b"),
    "slash-split": re.compile(r"""\brsplit(?:_once|_terminator)?\(\s*['"]/['"]|\brsplitn\(\s*\d+\s*,\s*['"]/['"]"""),
}
LITERAL: dict[str, re.Pattern[str]] = {
    "usr-bin": re.compile(r"/usr/bin/"),
    "applications": re.compile(r"/Applications\b"),
    "app-bundle": re.compile(r"\.app(?![A-Za-z0-9_])"),
}
MAC_WORDS = re.compile(r"\bmacOS\b|\bMac\b")
# A test module that also names a platform (`#[cfg(all(test, unix))]`).
TEST_CFG = re.compile(r"^(\s*)#\[cfg\(all\(test\b[^\]]*\)\)\]\s*$", re.M)


def exempt(rel: str) -> bool:
    return (
        any(rel.startswith(seam) for seam in SEAMS)
        or any(part in f"/{rel}" for part in EXEMPT_PARTS)
        or f"/{rel}".endswith(words.EXEMPT_SUFFIXES)
    )


def shipped(text: str, rust: bool) -> str:
    if not rust:
        return text
    return words.without_test_modules(TEST_CFG.sub(r"\1#[cfg(test)]", text))


def without_comments(source: str, rust: bool) -> str:
    """Code with its comments blanked; strings stay (`'/'` is code here)."""
    tokens = words.RUST_TOKENS if rust else words.TOKENS
    return tokens.sub(lambda m: " " if m.group(0).startswith(("//", "/*")) else m.group(0), source)


def literals(source: str, rust: bool) -> list[str]:
    """Every string literal's body (identifiers included: `"/usr/bin/open"`)."""
    out = []
    for match in (words.RUST_TOKENS if rust else words.TOKENS).finditer(source):
        token = match.group(0)
        if token.startswith(("//", "/*")) or (not rust and match.group(1) is not None):
            continue
        if rust:
            if token.startswith("'"):
                continue
            out.append(token[token.index('"') + 1 : token.rindex('"')])
        else:
            body = token[1:-1]
            out.append(words.INTERPOLATION.sub("", body) if token.startswith("`") else body)
    return out


def file_counts(text: str, rust: bool) -> dict[str, int]:
    source = shipped(text, rust)
    found: dict[str, int] = {}

    def add(kind: str, n: int) -> None:
        if n:
            found[kind] = found.get(kind, 0) + n

    if rust:
        code = without_comments(source, rust)
        for kind, pattern in CODE.items():
            add(kind, len(pattern.findall(code)))
    for body in literals(source, rust):
        for kind, pattern in LITERAL.items():
            add(kind, len(pattern.findall(body)))
    for body in words.texts(source, rust):
        add("mac-words", len(MAC_WORDS.findall(body)))
    return found


def counts(root: Path) -> dict[tuple[str, str], int]:
    found: dict[tuple[str, str], int] = {}
    for folder, suffixes in SCAN:
        base = root / folder
        if not base.exists():
            continue
        for path in sorted(p for p in base.rglob("*") if p.suffix in suffixes):
            rel = path.relative_to(root).as_posix()
            if exempt(rel) or (folder == "crates" and "/src/" not in rel):
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            for kind, n in file_counts(text, path.suffix == ".rs").items():
                found[(rel, kind)] = n
    return found


def check(root: Path, allow: Path) -> list[str]:
    now = counts(root)
    limits = words.allowed(allow)
    problems = []
    for (rel, kind), n in sorted(now.items()):
        limit = limits.get((rel, kind), 0)
        if n > limit:
            problems.append(f"{rel}: {n} × {kind}, {limit} allowed — put it behind {KINDS[kind]}")
    for (rel, kind), limit in sorted(limits.items()):
        if kind not in KINDS:
            problems.append(f"{rel}: unknown kind {kind} in the allowlist")
            continue
        n = now.get((rel, kind), 0)
        if n < limit:
            problems.append(f"{rel}: {n} × {kind} left, the allowlist says {limit} — lower its line (it only shrinks)")
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        app = root / "crates/juggler-app/src"
        app.mkdir(parents=True)
        (root / "crates/juggler-platform/src").mkdir(parents=True)
        (root / "crates/juggler-app/tests").mkdir(parents=True)
        (root / "ui/src/components").mkdir(parents=True)
        (app / "a.rs").write_text(
            "// std::os::unix in a comment, and /usr/bin/open, are fine\n"
            "use std::os::unix::fs::PermissionsExt;\n"
            'fn f(p: &str) -> Option<&str> { p.rsplit(\'/\').next() }\n'
            'const OPEN: &str = "/usr/bin/open";\n'
            'fn g(p: &str) -> bool { p.ends_with(".app") }\n'
            'fn h() -> String { "Open it on your Mac".into() }\n'
            "const ID: &str = \"macos\";\n"
            "#[cfg(all(test, unix))]\nmod tests { fn t() { let _ = std::os::unix::net::UnixStream::pair(); } }\n"
        )
        (root / "crates/juggler-platform/src/ipc.rs").write_text("use std::os::unix::net::UnixListener;\n")
        (root / "crates/juggler-app/tests/t.rs").write_text("use std::os::unix::fs::PermissionsExt;\n")
        (root / "ui/src/components/A.tsx").write_text(
            'export const A = () => <p>Quit it in macOS Settings</p>;\nconst x = "/Applications/Foo.app";\n'
        )
        (root / "ui/src/components/A.test.tsx").write_text('render("This Mac");\n')
        allow = root / "allow"
        everything = (
            "crates/juggler-app/src/a.rs unix-api 1\ncrates/juggler-app/src/a.rs slash-split 1\n"
            "crates/juggler-app/src/a.rs usr-bin 1\ncrates/juggler-app/src/a.rs app-bundle 1\n"
            "crates/juggler-app/src/a.rs mac-words 1\nui/src/components/A.tsx mac-words 1\n"
            "ui/src/components/A.tsx applications 1\nui/src/components/A.tsx app-bundle 1\n"
        )
        cases = [
            ("", 8, "every kind flagged; comments, ids, seams, tests and test modules exempt"),
            (everything, 0, "exactly allowed passes"),
            (everything.replace("a.rs unix-api 1", "a.rs unix-api 2"), 1, "a stale higher count must shrink"),
            (everything + "crates/juggler-app/src/a.rs unix-socket 1\n", 1, "a gone kind must leave the list"),
        ]
        bad = 0
        for text, want, what in cases:
            allow.write_text(text)
            got = check(root, allow)
            if len(got) != want:
                print(f"platform-lint selftest FAILED: {what}: {len(got)} problem(s), wanted {want}: {got}")
                bad += 1
        if bad:
            return 1
    print("platform-lint selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--print" in sys.argv:
        for (rel, kind), n in sorted(counts(ROOT).items()):
            print(f"{rel} {kind} {n}")
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(f"platform-lint: {problem}")
    if problems:
        print(f"platform-lint: {len(problems)} problem(s)")
        return 1
    limits = words.allowed(ALLOW)
    print(f"platform-lint: OK ({sum(limits.values())} allowlisted site(s) left, in {len({rel for rel, _ in limits})} file(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
