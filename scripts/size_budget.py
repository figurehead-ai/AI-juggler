#!/usr/bin/env python3
"""AJ-E2011 / ENGINEERING-GUIDE C.10: the file size budget.

- A Rust file holds at most 1,500 lines outside its inline `#[cfg(test)]`
  module (MUST for a new or grown file; 800 is the SHOULD).
- A TypeScript file in `ui/src` holds at most 400 lines (tests and
  generated types are exempt).

Today's bigger files are in `scripts/size_budget.allow` with their size
now, as a cap: such a file may shrink, never grow past its cap. The list
only shrinks: a listed file back under the budget must leave it. Generated
files are exempt.

Usage: size_budget.py [--print] [--selftest]
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "size_budget.allow"
RUST_MAX = 1500
TS_MAX = 400
EXEMPT = ("/tests/", "/types/generated/", "/generated.rs", "/target/", "/node_modules/", "/fixtures/")


def rust_lines(path: Path) -> int:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    for index, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]" and index + 1 < len(lines) and lines[index + 1].lstrip().startswith("mod "):
            return index
    return len(lines)


def sizes(root: Path) -> dict[str, tuple[int, int]]:
    """path -> (lines, budget) for every file over its budget."""
    over: dict[str, tuple[int, int]] = {}
    scans = ((root / "crates", "*.rs", RUST_MAX, rust_lines), (root / "ui" / "src", "*.ts*", TS_MAX, None))
    for base, pattern, budget, count in scans:
        if not base.exists():
            continue
        for path in sorted(base.rglob(pattern)):
            rel = path.relative_to(root).as_posix()
            if any(part in f"/{rel}" for part in EXEMPT) or ".test." in rel or ".spec." in rel:
                continue
            n = count(path) if count else len(path.read_text(encoding="utf-8", errors="replace").splitlines())
            if n > budget:
                over[rel] = (n, budget)
    return over


def caps(allow: Path) -> dict[str, int]:
    entries: dict[str, int] = {}
    if allow.exists():
        for raw in allow.read_text(encoding="utf-8").splitlines():
            line = raw.split("#", 1)[0].strip()
            if line:
                rel, cap = line.rsplit(None, 1)
                entries[rel] = int(cap)
    return entries


def check(root: Path, allow: Path) -> list[str]:
    over = sizes(root)
    listed = caps(allow)
    problems = []
    for rel, (n, budget) in sorted(over.items()):
        cap = listed.get(rel)
        if cap is None:
            problems.append(f"{rel}: {n} lines, over the {budget}-line budget — split it by feature")
        elif n > cap:
            problems.append(f"{rel}: grew to {n} lines (capped at {cap}) — new code goes in its feature's own file")
    for rel in sorted(set(listed) - set(over)):
        problems.append(f"{rel}: back within the budget — remove it from size_budget.allow (it only shrinks)")
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates" / "a" / "src").mkdir(parents=True)
        (root / "ui" / "src").mkdir(parents=True)
        (root / "crates" / "a" / "src" / "big.rs").write_text("x\n" * 1600 + "#[cfg(test)]\nmod tests {}\n" + "t\n" * 500)
        (root / "crates" / "a" / "src" / "ok.rs").write_text("x\n" * 1400 + "#[cfg(test)]\nmod tests {}\n" + "t\n" * 900)
        (root / "ui" / "src" / "Big.tsx").write_text("x\n" * 450)
        (root / "ui" / "src" / "Big.test.tsx").write_text("x\n" * 900)
        allow = root / "allow"
        cases = [
            ("crates/a/src/big.rs 1600\nui/src/Big.tsx 450\n", 0, "capped files pass; tests and test modules don't count"),
            ("crates/a/src/big.rs 1550\nui/src/Big.tsx 450\n", 1, "a capped file that grew fails"),
            ("ui/src/Big.tsx 450\n", 1, "an unlisted file over budget fails"),
            ("crates/a/src/big.rs 1600\nui/src/Big.tsx 450\ncrates/a/src/ok.rs 1400\n", 1, "a file back within budget must leave the list"),
        ]
        bad = 0
        for text, want, what in cases:
            allow.write_text(text)
            got = len(check(root, allow))
            if got != want:
                print(f"size-budget selftest FAILED: {what}: {got} problem(s), wanted {want}")
                bad += 1
    if bad:
        return 1
    print("size-budget selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--print" in sys.argv:
        for rel, (n, _) in sorted(sizes(ROOT).items()):
            print(f"{rel} {n}")
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(f"size-budget: {problem}")
    if problems:
        return 1
    print(f"size-budget: OK ({len(caps(ALLOW))} file(s) over budget, capped)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
