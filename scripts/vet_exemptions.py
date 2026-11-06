#!/usr/bin/env python3
"""AJ-E2007 R1: cargo vet's exemption list only shrinks.

Compares supply-chain/config.toml with the base revision (BASE, else
origin/main) and fails when
- there are more exemptions than the base had, or
- a crate the base didn't exempt now is (a new dependency needs an audit,
  or an imported one, never a fresh exemption).
A newer version of a crate that was already exempt may replace its entry
(a Dependabot bump); `cargo vet prune` removes entries audits now cover.

Usage: vet_exemptions.py [--selftest]
"""
from __future__ import annotations

import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONFIG = "supply-chain/config.toml"
ENTRY = re.compile(r"^\[\[exemptions\.([^\]]+)\]\]", re.M)


def exempted(text: str) -> list[str]:
    return [name.strip('"') for name in ENTRY.findall(text)]


def problems(base: list[str], now: list[str]) -> list[str]:
    out = []
    if len(now) > len(base):
        out.append(f"{len(now)} exemptions, the base had {len(base)}: the list only shrinks")
    for crate in sorted(set(now) - set(base)):
        out.append(f"{crate} is newly exempted: audit it (cargo vet certify) or import an audit")
    return out


def selftest() -> None:
    assert not problems(["a", "b"], ["a"])
    assert not problems(["a", "b"], ["a", "b"]), "a bump keeps the count"
    assert problems(["a"], ["a", "b"])
    assert problems(["a", "b"], ["a", "c"]), "a swapped-in crate"
    text = '[[exemptions.serde]]\nversion = "1"\n\n[[exemptions."a-b"]]\n'
    assert exempted(text) == ["serde", "a-b"], exempted(text)
    print("vet_exemptions: selftest ok")


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return 0
    now = exempted((ROOT / CONFIG).read_text(encoding="utf-8"))
    base_ref = os.environ.get("BASE") or "origin/main"
    shown = subprocess.run(
        ["git", "show", f"{base_ref}:{CONFIG}"], cwd=ROOT, capture_output=True, text=True
    )
    if shown.returncode != 0:
        print(f"vet_exemptions: {len(now)} exemptions; no list at {base_ref} yet, nothing to compare")
        return 0
    found = problems(exempted(shown.stdout), now)
    for line in found:
        print(f"vet_exemptions: {line}", file=sys.stderr)
    if not found:
        print(f"vet_exemptions: OK ({len(now)} exemptions, base {base_ref})")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
