#!/usr/bin/env python3
"""No new direct read of this machine's disk, memory or system state
outside the seams a test can replace.

A test of code that reads the real machine passes or fails with the
machine: the Local cache's tests failed on CI's nearly full runner
because the cache read the disk itself (#198), and were fixed by giving
the cache a replaceable reader (`Manager::read_disk_with`). New code that
needs free disk, memory or a system snapshot takes it from a seam: the
platform crate (`juggler_platform::room`), the supervisor's sampler, or
a reader the caller can swap in tests.

The places reading directly today are listed with their counts in
scripts/machine_state.allow; a count only goes down. Tests are exempt
(they may read the machine to check a reader).

Usage: scripts/machine_state_lint.py          (fails naming each file over its count)
       scripts/machine_state_lint.py --selftest
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts/machine_state.allow"
READS = re.compile(
    r"room::(?:disk_space|total_memory)\("
    r"|\.(?:total_memory|used_memory|available_memory|free_memory)\(\)"
    r"|Disks::new[A-Za-z_]*\("
    r"|System::new[A-Za-z_]*\("
)


def exempt(rel: str) -> bool:
    return (
        rel.startswith("crates/juggler-platform/")
        or "/tests/" in rel
        or "/benches/" in rel
        or rel.endswith("/tests.rs")
    )


def counts(files: dict[str, str]) -> dict[str, int]:
    out: dict[str, int] = {}
    for rel, text in files.items():
        if exempt(rel):
            continue
        n = len(READS.findall(text))
        if n:
            out[rel] = n
    return out


def allowed(text: str) -> dict[str, int]:
    out: dict[str, int] = {}
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            path, n = line.rsplit(None, 1)
            out[path] = int(n)
    return out


def problems(found: dict[str, int], allow: dict[str, int]) -> list[str]:
    out = []
    for rel, n in sorted(found.items()):
        limit = allow.get(rel, 0)
        if n > limit:
            out.append(f"{rel}: {n} direct machine read(s), {limit} allowed — take it from a seam a test can replace")
    for rel, limit in sorted(allow.items()):
        n = found.get(rel, 0)
        if n < limit:
            out.append(f"{rel}: {n} left, the allowlist says {limit} — lower its line (it only shrinks)")
    return out


def selftest() -> None:
    files = {
        "crates/juggler-app/src/a.rs": "let d = juggler_platform::room::disk_space(&p);\nlet m = system.total_memory();",
        "crates/juggler-app/tests/b.rs": "let d = room::disk_space(&p);",
        "crates/juggler-platform/src/room.rs": "Disks::new_with_refreshed_list()",
        "crates/juggler-app/src/c.rs": "let s = sysinfo::System::new();",
    }
    found = counts(files)
    assert found == {"crates/juggler-app/src/a.rs": 2, "crates/juggler-app/src/c.rs": 1}, found
    assert problems(found, {"crates/juggler-app/src/a.rs": 2, "crates/juggler-app/src/c.rs": 1}) == []
    assert len(problems(found, {"crates/juggler-app/src/a.rs": 2})) == 1, "a new read fails"
    assert "lower its line" in problems(found, {"crates/juggler-app/src/a.rs": 3, "crates/juggler-app/src/c.rs": 1})[0]
    assert allowed("x.rs 2  # why\n\n# only a comment\n") == {"x.rs": 2}
    print("machine-state-lint selftest: OK")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    files = {
        str(path.relative_to(ROOT)): path.read_text(encoding="utf-8", errors="replace")
        for path in (ROOT / "crates").rglob("*.rs")
    }
    found = counts(files)
    issues = problems(found, allowed(ALLOW.read_text(encoding="utf-8")))
    for issue in issues:
        print(f"machine-state-lint: {issue}")
    if issues:
        return 1
    print(f"machine-state-lint: OK ({sum(found.values())} allowlisted read(s), in {len(found)} file(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
