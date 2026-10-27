#!/usr/bin/env python3
"""AJ-E1535: plain words on screen — no developer terms in what users read.

The 0.1 walks found "COMP", "tok", "F1", "transplant", "switch-back ↺",
"re-adopted", "dogfood cell" and "CI row" on screen: words from our specs
and tests, not the user's. This lint reads the text that can reach the
screen — string literals and JSX text in shipped UI code, and string
literals in shipped Rust code of the crates whose strings the UI shows —
and counts each banned term per file. Comments are skipped, and so is a
one-word literal that reads as an identifier (`"transplant"` is a wire
value, `"switch-back-card"` a test id), unless it is a banned tag on its
own ("COMP", "tok").

The allowlist `scripts/plain_words.allow` (`path term count`) **only
shrinks**: a file over its count, or not listed, fails; a file under its
count fails until its line is lowered or removed.

Exempt: tests (`/tests/`, `*.test.*`, `*.spec.*`, inline `#[cfg(test)]`
modules), generated code, examples (maintainer reports), fixtures and
the UI's preview data.

Usage: plain_words_lint.py [--print] [--selftest]
  --print     print today's counts in the allowlist's format
  --selftest  check the lint against a seeded tree
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "plain_words.allow"
# term -> (pattern, the plain words to use instead)
BANNED: dict[str, tuple[re.Pattern[str], str]] = {
    "tier-tag": (re.compile(r"\b(?:COMP|MEAS|EST|Comp|Meas|Est)\b"), "measured / worked out / estimate"),
    "tok": (re.compile(r"\btok\b"), "tokens"),
    "fidelity-code": (re.compile(r"(?<!S-)\bF[0-4]\b"), "what carries (\"Full story, fresh start\")"),
    "failure-code": (re.compile(r"\bS-F[0-9]\b"), "say what went wrong, without the spec's code"),
    "transplant": (re.compile(r"\b[Tt]ransplant"), "\"Same conversation, new account\""),
    "switch-tier": (re.compile(r"\bT[1-4]\b"), "what carries (\"Same conversation\", \"Full story, fresh start\")"),
    "switch-back": (re.compile(r"[Ss]witch-back"), "\"Switched back\""),
    "re-adopted": (re.compile(r"[Rr]e-adopted"), "\"picked up after restart\""),
    "dogfood": (re.compile(r"[Dd]ogfood"), "\"tried by hand\""),
    "ci-row": (re.compile(r"\bCI (?:row|fixtures)\b"), "\"automatic tests\""),
    "live-cell": (re.compile(r"\b[Ll]ive cell"), "\"hand check\""),
    "beta-debt": (re.compile(r"\bbeta debt\b"), "\"still in beta\""),
    "tool-pairs": (re.compile(r"\btool pairs\b"), "\"switches\""),
    "unvalidated": (re.compile(r"\b[Uu]nvalidated\b"), "\"not proven yet\""),
    "imported-sessions": (re.compile(r"\bimported sessions\b"), "say what they are"),
    "no-folder-picked": (re.compile(r"No folder picked"), "show the footer only where it applies"),
    # The review of 0.1.4 (the readability pass): developer words and tags
    # still on screen.
    "orphan-helper": (re.compile(r"\b(?:[Aa]n?|\d+|[Mm]anaged) (?:orphan|helper)s?\b"), "\"left running after its session closed\", \"a process it started\""),
    "reset-colon": (re.compile(r"\b[Rr]eset:(?=\s*$|\s*\d)"), "\"resets in 2 h\""),
    "model-api-prices": (re.compile(r"'s API prices"), "\"at list prices for <model>\""),
    "latest-session": (re.compile(r"From the latest session"), "\"Last read in a session\""),
    "calendar-emoji": (re.compile("\U0001F4C5"), "the words alone (\"Renews 23 Oct\")"),
}
# A one-word literal that is a banned tag on its own still counts.
TAGS = {"COMP", "MEAS", "EST", "Comp", "Meas", "Est", "tok"}
IDENTIFIER = re.compile(r"^[A-Za-z0-9_:./#@-]*$")
SCAN = (
    ("ui/src", (".ts", ".tsx")),
    ("crates/juggler-app/src", (".rs",)),
    ("crates/juggler-handover/src", (".rs",)),
    ("crates/juggler-supervisor/src", (".rs",)),
)
EXEMPT_PARTS = ("/tests/", "/fixtures/", "ui/src/preview/", "/types/generated/", "/generated.rs", "/benches/", "/examples/")
EXEMPT_SUFFIXES = (".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx", "/tests.rs", "_tests.rs")
# Strings and comments, in source order (so a quote inside a comment, or
# `//` inside a string, is read as what it is).
TOKENS = re.compile(
    r"""//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|`(?:\\.|[^`\\])*`|>([^<>{}\n]+)<""",
    re.S,
)
# Rust: raw strings, and a char literal is one character (a lifetime
# like `'a` is not a quote).
RUST_TOKENS = re.compile(
    r"""//[^\n]*|/\*.*?\*/|r(#*)".*?"\1|"(?:\\.|[^"\\])*"|'(?:\\u\{[0-9a-fA-F]+\}|\\.|[^'\\\n])'""",
    re.S,
)
# A template's `${…}` is code, not words.
INTERPOLATION = re.compile(r"\$\{[^}]*\}")


def exempt(rel: str) -> bool:
    return any(part in f"/{rel}" for part in EXEMPT_PARTS) or f"/{rel}".endswith(EXEMPT_SUFFIXES)


def shipped_text(path: Path) -> str:
    """The file without its inline `#[cfg(test)]` module (Rust)."""
    text = path.read_text(encoding="utf-8", errors="replace")
    if path.suffix != ".rs":
        return text
    return without_test_modules(text)


def without_test_modules(text: str) -> str:
    """Rust source without its `#[cfg(test)]` modules."""
    # Every `#[cfg(test)] mod …` goes, wherever it sits: a `mod x;` line,
    # or a `mod x { … }` block to its closing brace (counted by line).
    lines = text.splitlines()
    kept: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        following = lines[index + 1].lstrip() if index + 1 < len(lines) else ""
        if line.strip() == "#[cfg(test)]" and following.startswith(("mod ", "pub mod ", "pub(crate) mod ")):
            index += 1
            if following.rstrip().endswith(";"):
                index += 1
                continue
            depth = 0
            while index < len(lines):
                depth += lines[index].count("{") - lines[index].count("}")
                index += 1
                if depth <= 0 and "{" in "".join(lines[: index]):
                    break
            continue
        kept.append(line)
        index += 1
    return "\n".join(kept)


def texts(source: str, rust: bool) -> list[str]:
    """What can reach the screen: string literals and JSX text."""
    out = []
    for match in (RUST_TOKENS if rust else TOKENS).finditer(source):
        token = match.group(0)
        if token.startswith(("//", "/*")):
            continue
        if not rust and match.group(1) is not None:
            out.append(match.group(1))  # JSX text
            continue
        if rust and token.startswith("'"):
            continue  # a char
        body = token[token.index('"') + 1 : token.rindex('"')] if rust else token[1:-1]
        if token.startswith("`"):
            body = INTERPOLATION.sub("", body)
        if IDENTIFIER.match(body.strip()) and body.strip() not in TAGS:
            continue
        out.append(body)
    return out


def counts(root: Path) -> dict[tuple[str, str], int]:
    found: dict[tuple[str, str], int] = {}
    for folder, suffixes in SCAN:
        base = root / folder
        if not base.exists():
            continue
        for path in sorted(p for p in base.rglob("*") if p.suffix in suffixes):
            rel = path.relative_to(root).as_posix()
            if exempt(rel):
                continue
            for text in texts(shipped_text(path), path.suffix == ".rs"):
                for term, (pattern, _) in BANNED.items():
                    n = len(pattern.findall(text))
                    if n:
                        found[(rel, term)] = found.get((rel, term), 0) + n
    return found


def allowed(allow: Path) -> dict[tuple[str, str], int]:
    entries: dict[tuple[str, str], int] = {}
    if not allow.exists():
        return entries
    for raw in allow.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        rel, term, count = line.split()
        entries[(rel, term)] = int(count)
    return entries


def check(root: Path, allow: Path) -> list[str]:
    now = counts(root)
    limits = allowed(allow)
    problems = []
    for (rel, term), n in sorted(now.items()):
        limit = limits.get((rel, term), 0)
        if n > limit:
            problems.append(f"{rel}: {n} × {term}, {limit} allowed — say it in plain words: {BANNED[term][1]}")
    for (rel, term), limit in sorted(limits.items()):
        n = now.get((rel, term), 0)
        if n < limit:
            problems.append(f"{rel}: {n} × {term} left, the allowlist says {limit} — lower its line (it only shrinks)")
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "ui/src/components").mkdir(parents=True)
        (root / "crates/juggler-app/src").mkdir(parents=True)
        (root / "ui/src/components/A.tsx").write_text(
            "// a transplant F1 comment is fine\n"
            'const id = "switch-back-card"; const wire = "transplant";\n'
            "const tag = \"COMP\";\n"
            "const t = `${n} tok`;\n"
            "export const A = () => <p>Re-adopted after restart</p>;\n"
        )
        (root / "ui/src/components/A.test.tsx").write_text('expect(x).toBe("F1 · Beta");\n')
        (root / "crates/juggler-app/src/a.rs").write_text(
            'fn f() -> String { format!("transplant {}", "F1") }\n'
            "fn g<'a>(x: &'a str) -> char { 'F' }\n"
            "#[cfg(test)]\nmod tests { const X: &str = \"F1 · Beta\"; }\n"
        )
        allow = root / "allow"
        cases = [
            ("", 4, "JSX text, a tag alone, a template and a Rust format string flagged; ids, comments and tests exempt"),
            (
                "ui/src/components/A.tsx tier-tag 1\nui/src/components/A.tsx tok 1\n"
                "ui/src/components/A.tsx re-adopted 1\ncrates/juggler-app/src/a.rs transplant 1\n",
                0,
                "exactly allowed passes",
            ),
            (
                "ui/src/components/A.tsx tier-tag 2\nui/src/components/A.tsx tok 1\n"
                "ui/src/components/A.tsx re-adopted 1\ncrates/juggler-app/src/a.rs transplant 1\n",
                1,
                "a stale higher count must shrink",
            ),
        ]
        bad = 0
        for text, want, what in cases:
            allow.write_text(text)
            got = check(root, allow)
            if len(got) != want:
                print(f"plain-words-lint selftest FAILED: {what}: {len(got)} problem(s), wanted {want}: {got}")
                bad += 1
        if bad:
            return 1
    print("plain-words-lint selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--print" in sys.argv:
        for (rel, term), n in sorted(counts(ROOT).items()):
            print(f"{rel} {term} {n}")
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(problem)
    if problems:
        print(f"plain-words-lint: {len(problems)} problem(s)")
        return 1
    print("plain-words-lint: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
