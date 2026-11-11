#!/usr/bin/env python3
"""AJ-E2011 R5: a lint is silenced only with its reason.

Every `#[allow(…)]` / `#![allow(…)]` / `#[expect(…)]` in the Rust sources
must say why (`reason = "…"`). Rust's `#[expect]` is preferred for a single
site: it fails once the lint no longer fires, so a stale exception can't
linger.

Today's files with reasonless attributes are in
`scripts/allow_reasons.allow` with their count, as a cap. A file may only
go down, never up. The list only shrinks: a listed file that went down
must be updated (`--update` writes today's counts), and one with none
left must leave the list.

Usage: allow_reasons.py [--update] [--selftest]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "allow_reasons.allow"
START = re.compile(r"#!?\[(allow|expect)\(")


def reasonless(text: str) -> int:
    """How many lint attributes in `text` give no reason."""
    count = 0
    for match in START.finditer(text):
        depth, i = 1, match.end()
        while i < len(text) and depth:
            depth += {"(": 1, ")": -1}.get(text[i], 0)
            i += 1
        if "reason" not in text[match.end():i]:
            count += 1
    return count


def counts(root: Path) -> dict[str, int]:
    out: dict[str, int] = {}
    for path in sorted((root / "crates").rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if "/target/" in rel:
            continue
        n = reasonless(path.read_text(encoding="utf-8", errors="replace"))
        if n:
            out[rel] = n
    return out


def load(path: Path) -> dict[str, int]:
    caps: dict[str, int] = {}
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                n, rel = line.split(maxsplit=1)
                caps[rel] = int(n)
    return caps


def problems(now: dict[str, int], caps: dict[str, int]) -> list[str]:
    out = []
    for rel, n in sorted(now.items()):
        cap = caps.get(rel)
        if cap is None:
            out.append(f"{rel}: {n} lint exception(s) without `reason = \"…\"`")
        elif n > cap:
            out.append(f"{rel}: {n} without a reason, the cap is {cap}")
        elif n < cap:
            out.append(f"{rel}: down to {n} (cap {cap}): run allow_reasons.py --update to keep it there")
    for rel in sorted(set(caps) - set(now)):
        out.append(f"{rel}: none left, run allow_reasons.py --update to drop it from the list")
    return out


def write(path: Path, now: dict[str, int]) -> None:
    lines = ["# AJ-E2011 R5: reasonless #[allow]/#[expect] per file, as caps that only shrink.",
             "# Written by scripts/allow_reasons.py --update."]
    lines += [f"{n} {rel}" for rel, n in sorted(now.items())]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def selftest() -> None:
    assert reasonless("#[allow(dead_code)]\nfn a() {}") == 1
    assert reasonless('#[allow(dead_code, reason = "kept for the API")]') == 0
    assert reasonless('#![allow(\n    clippy::unwrap_used,\n    reason = "tests"\n)]') == 0
    assert reasonless("#[expect(clippy::x)] #![allow(a, b)]") == 2
    assert reasonless("#[cfg(test)] #[derive(Debug)]") == 0
    assert not problems({"a.rs": 2}, {"a.rs": 2})
    assert problems({"a.rs": 3}, {"a.rs": 2}), "grew"
    assert problems({"b.rs": 1}, {}), "new file"
    assert problems({"a.rs": 1}, {"a.rs": 2}), "shrank: the cap must follow"
    assert problems({}, {"a.rs": 1}), "gone: off the list"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "list"
        write(path, {"a.rs": 2})
        assert load(path) == {"a.rs": 2}
    print("allow_reasons: selftest ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    now = counts(ROOT)
    if "--update" in sys.argv:
        caps = load(ALLOW)
        grew = [rel for rel, n in now.items() if n > caps.get(rel, 0)] if caps else []
        if grew:
            for rel in grew:
                print(f"allow_reasons: {rel} grew; give the new exceptions a reason instead", file=sys.stderr)
            return 1
        write(ALLOW, now)
        print(f"allow_reasons: list written ({sum(now.values())} in {len(now)} files)")
        return 0
    found = problems(now, load(ALLOW))
    for line in found:
        print(f"allow_reasons: {line}", file=sys.stderr)
    if not found:
        print(f"allow_reasons: OK ({sum(now.values())} reasonless exceptions left in {len(now)} files)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
