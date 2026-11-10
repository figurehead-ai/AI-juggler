#!/usr/bin/env python3
"""An Account's record is never printed (CodeQL rust/cleartext-logging).

CodeQL flags any Account value that reaches a log or a panic message,
tests included: an `assert!(…, "{accounts:?}")` is a high alert that
blocks the merge (PR #218). Log or assert with counts, ids or reasons
instead. This catches the common shape before CI does: an Account
variable (`account`, `accounts`, `subscription`, `subscriptions`)
Debug-formatted inline (`{accounts:?}`) or as an argument
(`"{:?}", account`). Usage: cleartext_lint.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
NAMES = r"(?:accounts?|subscriptions?)"
INLINE = re.compile(r"\{" + NAMES + r":#?\?\}")
ARG = re.compile(r"\{:#?\?\}[^\"]*\"\s*,\s*&?" + NAMES + r"\b")


def findings(text: str) -> list[int]:
    lines = []
    for i, line in enumerate(text.splitlines(), 1):
        if INLINE.search(line) or ARG.search(line):
            lines.append(i)
    return lines


def scan(root: pathlib.Path) -> list[str]:
    out = []
    for path in sorted(root.glob("crates/**/*.rs")):
        if "/target/" in path.as_posix():
            continue
        for line in findings(path.read_text()):
            out.append(f"{path.relative_to(root)}:{line}")
    return out


def selftest() -> None:
    assert findings('assert_eq!(accounts.len(), 3, "{accounts:?}");') == [1]
    assert findings('tracing::warn!("{account:#?}");') == [1]
    assert findings('panic!("got {:?}", accounts);') == [1]
    assert findings('panic!("got {:?}", &subscription);') == [1]
    assert findings('assert!(n == 1, "exactly one Account unknown");') == []
    assert findings('assert_eq!(ids(&accounts[0]), ["a"], "{listed:?}");') == []
    assert findings('tracing::warn!(%error, kept = kept.is_some(), "x");') == []
    print("cleartext_lint selftest: ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    found = scan(ROOT)
    for site in found:
        print(f"{site}: an Account is Debug-formatted; print a count, id or reason (CodeQL rust/cleartext-logging)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
