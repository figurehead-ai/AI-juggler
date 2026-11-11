#!/usr/bin/env python3
"""No system autocorrect in AI Juggler's text fields (the maintainer's
"go" sitting, 2026-11-07: typing "aij-test" in the rail's search became
"Air-test", "Nightly" became "e that Nightlye").

Every `<input>` in shipped UI code that takes typed text (no `type`, or
`text`, `search`, `email`, `url`, `password`, `number`, `tel`) SHALL carry
`autoCorrect="off"`, `autoCapitalize="off"` and `spellCheck={false}`:
searches, names, folders, ids and keys are never words to correct.
Prose (a `<textarea>`, a message to an agent) keeps the system's help.

Exempt: tests (`*.test.*`, `*.spec.*`, `/tests/`), generated types and
the preview's data.

Usage: text_input_lint.py [--fix] [--selftest]
  --fix       add the three attributes where they're missing
  --selftest  check the lint against seeded text
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "ui" / "src"
EXEMPT = ("/tests/", ".test.", ".spec.", "/types/generated/", "/preview/", "/test/")
INPUT = re.compile(r"<input\b")
TYPE = re.compile(r"""\btype\s*=\s*(?:\{\s*)?["'`]([a-z-]+)["'`]""")
TEXT_TYPES = {"text", "search", "email", "url", "password", "number", "tel"}
NEEDS = ('autoCorrect="off"', 'autoCapitalize="off"', "spellCheck={false}")


def tag_end(text: str, start: int) -> int:
    """Where the JSX tag opened at `start` ends: its `>` outside braces
    and quotes (an attribute's `(e) => …` doesn't end it)."""
    depth, quote, i = 0, "", start
    while i < len(text):
        c = text[i]
        if quote:
            if c == "\\":
                i += 1
            elif c == quote:
                quote = ""
        elif c in "\"'`":
            quote = c
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        elif c == ">" and depth == 0:
            return i
        i += 1
    return len(text)


def missing(text: str) -> list[tuple[int, int]]:
    """(the tag's start, its end) of every text input without the three."""
    out = []
    for match in INPUT.finditer(text):
        end = tag_end(text, match.start())
        tag = text[match.start() : end]
        kind = TYPE.search(tag)
        if kind and kind.group(1) not in TEXT_TYPES:
            continue
        if not all(need in tag for need in NEEDS):
            out.append((match.start(), end))
    return out


def fix(text: str) -> str:
    """Adds whatever of the three a text input lacks, right after `<input`."""
    for start, end in reversed(missing(text)):
        tag = text[start:end]
        add = " ".join(need for need in NEEDS if need not in tag)
        text = text[: start + len("<input")] + " " + add + text[start + len("<input") :]
    return text


def files() -> list[Path]:
    return [p for p in sorted(UI.rglob("*.tsx")) if not any(part in str(p) for part in EXEMPT)]


def selftest() -> int:
    bad = '<input value={v} onChange={(e) => set(e.target.value)} />'
    good = '<input autoCorrect="off" autoCapitalize="off" spellCheck={false} type="search" />'
    box = '<input type="checkbox" checked />'
    assert len(missing(bad)) == 1, "a bare input is caught"
    assert missing(good) == [], "a search with the three passes"
    assert missing(box) == [], "a checkbox isn't a text field"
    assert missing(fix(bad)) == [], "--fix adds them"
    assert fix(good) == good, "--fix leaves a good one alone"
    print("text-input-lint selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if argv[1:] == ["--selftest"]:
        return selftest()
    if argv[1:] == ["--fix"]:
        changed = 0
        for path in files():
            text = path.read_text()
            fixed = fix(text)
            if fixed != text:
                path.write_text(fixed)
                changed += 1
        print(f"text-input-lint: fixed {changed} file(s)")
        return 0
    found = []
    for path in files():
        text = path.read_text()
        for start, _ in missing(text):
            line = text.count("\n", 0, start) + 1
            found.append(f"{path.relative_to(ROOT)}:{line}")
    if found:
        print("text-input-lint: text fields without autocorrect off (add autoCorrect=\"off\" autoCapitalize=\"off\" spellCheck={false}, or run --fix):", file=sys.stderr)
        for one in found:
            print(f"  {one}", file=sys.stderr)
        return 1
    print("text-input-lint: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
