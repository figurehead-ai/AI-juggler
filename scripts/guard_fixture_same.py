#!/usr/bin/env python3
"""AJ-E3806: Bot's guard tests read the harness's shared fixture from a
copy in the crate (shipped code never includes an evidence file, the
release-ships rule). This fails when the copy and the harness's file differ.

    guard_fixture_same.py            check
    guard_fixture_same.py --selftest
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HARNESS = ROOT / "docs/qa/evidence/AJ-E3709/harness/v5/fixtures/guard-phrases.json"
COPY = ROOT / "crates/juggler-app/src/features/bot/fixtures/guard-phrases.json"


def same(a: Path, b: Path) -> str | None:
    if not a.is_file() or not b.is_file():
        return f"missing: {a if not a.is_file() else b}"
    if a.read_bytes() != b.read_bytes():
        return f"{b.relative_to(ROOT) if b.is_relative_to(ROOT) else b} differs from {a.relative_to(ROOT) if a.is_relative_to(ROOT) else a}: copy the harness's file over it"
    return None


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp, "a.json"), Path(tmp, "b.json")
            a.write_text("{}")
            b.write_text("{}")
            assert same(a, b) is None
            b.write_text("{ }")
            assert same(a, b) is not None
            b.unlink()
            assert same(a, b) is not None
        print("guard-fixture-same: selftest OK")
        return 0
    problem = same(HARNESS, COPY)
    if problem:
        print(f"guard-fixture-same: {problem}")
        return 1
    print("guard-fixture-same: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
