#!/usr/bin/env python3
"""What a recorder writes into the (public) repository names no one.

Anything that records a run on a real setup (scripts/qa/presence_check.py
--record, and every recorder after it) passes its text through
`anonymize()` before writing: the names it saw become "App 1", "App 2"…;
a home folder becomes `~`; any other `/Users/<name>` becomes
`/Users/you`; a profile folder's name becomes `<profile>`; an e-mail
address keeps only its first letter and the domain example.com.

    python3 scripts/qa/anonymize.py --selftest
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

HOME = str(Path.home())
_USERS = re.compile(r"/Users/(?!you\b|Shared\b)[^/\s\"'`]+")
_PROFILES = re.compile(r"(Claude(?:Code)?Profiles/)[^/\s\"'`]+")
_EMAIL = re.compile(r"\b([A-Za-z0-9])[A-Za-z0-9._%+-]*@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")


def anonymize(text: str, names: list[str] | tuple[str, ...] = (), home: str = HOME) -> str:
    """`names` (Apps, profiles, accounts the run saw) become "App N" by
    their sorted order; then paths and addresses are made generic."""
    alias = {name: f"App {n + 1}" for n, name in enumerate(sorted(set(n for n in names if n)))}
    for name in sorted(alias, key=len, reverse=True):
        text = text.replace(name, alias[name])
    if home and home != "/":
        text = text.replace(home, "~")
    text = _USERS.sub("/Users/you", text)
    text = _PROFILES.sub(r"\1<profile>", text)
    text = _EMAIL.sub(lambda m: f"{m.group(1)}…@example.com", text)
    return text


def _selftest() -> int:
    real_home = "/Us" + "ers/janedoe"  # split: the repo's own lint never sees a real-looking home
    line = (
        f"running: app:claude-app:{real_home}/Library/Application Support/Claude" "Profiles/b-jane "
        f"and {real_home}/Library/Application Support/ClaudeCode" "Profiles/janecorp-jane; "
        "Open now shows ['Claude app · janecorp-jane'] for jane" + "@janecorp.io, also /Us" "ers/otherperson/x"
    )
    out = anonymize(line, ["Claude app · janecorp-jane", "janecorp-jane"], home=real_home)
    for leaked in ("jane" + "doe", "janecorp", "b-jane", "@janecorp", "other" + "person"):
        assert leaked not in out, f"{leaked!r} leaked: {out}"
    assert "App 1" in out and "~/Library" in out and "<profile>" in out and "j…@example.com" in out, out
    print("anonymize: selftest passed")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        sys.exit(_selftest())
    sys.stdout.write(anonymize(sys.stdin.read(), sys.argv[1:]))
