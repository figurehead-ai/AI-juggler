#!/usr/bin/env python3
"""AJ-E2015: ARCHITECTURE.md stays true.

Fails when
- its "Revisited for: X.Y" line is behind the workspace's version (a new
  minor means the map is read and brought up to date at its gate);
- a crate's `src/lib.rs` doc has no "Must not know:" line (each crate says
  what it must stay ignorant of, so a boundary can't erode silently).

Usage: architecture_fresh.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def minor(version: str) -> tuple[int, int]:
    major, minor_, *_ = version.split(".")
    return int(major), int(minor_)


def problems(root: pathlib.Path) -> list[str]:
    out = []
    arch = (root / "ARCHITECTURE.md").read_text()
    m = re.search(r"^Revisited for: (\d+\.\d+)", arch, re.M)
    cargo = (root / "Cargo.toml").read_text()
    v = re.search(r'^version = "([^"]+)"', cargo, re.M)
    if not m:
        out.append("ARCHITECTURE.md has no 'Revisited for: X.Y' line")
    elif v and minor(m.group(1)) < minor(v.group(1)):
        out.append(f"ARCHITECTURE.md was revisited for {m.group(1)}; the workspace is {v.group(1)}: read it, bring it up to date, move the line")
    for lib in sorted(root.glob("crates/*/src/lib.rs")):
        head = lib.read_text()[:6000]
        if "Must not know:" not in head:
            out.append(f"{lib.relative_to(root)}: its crate doc has no 'Must not know:' line")
    return out


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        assert minor("0.4.1") == (0, 4) and minor("1.10.0") > minor("1.9.3")
        print("architecture-fresh: selftest ok")
        return 0
    found = problems(ROOT)
    for problem in found:
        print(f"architecture-fresh: {problem}")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
