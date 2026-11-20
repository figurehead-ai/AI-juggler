#!/usr/bin/env python3
"""Runs every line of a just recipe and reports every failure at once.

`just` stops a recipe at its first failing line, so a gate with several
problems shows them one run at a time (0.4.7's static gate took four).
This runs each line in its own shell, as `just` does, carries on past a
failure, expands a line that is itself a bare `just <recipe>` into that
recipe's own lines (so a nested gate keeps going too), and at the end
lists every line that failed. It exits 1 if any failed (D-137: find
everything, fix together).

  keep_going.py RECIPE        e.g. keep_going.py verify-static
  keep_going.py --selftest
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NESTED = re.compile(r"^just\s+([A-Za-z0-9_-]+)\s*$")


def lines_of(recipe: str, run=subprocess.run) -> list[str]:
    """The recipe's command lines, as `just --dry-run` prints them
    (interpolated, dependencies first)."""
    shown = run(
        ["just", "--dry-run", recipe], cwd=ROOT, capture_output=True, text=True, check=False
    )
    if shown.returncode != 0:
        raise SystemExit(f"keep-going: just can't show {recipe}: {shown.stderr.strip()}")
    # just prints the plan on stderr (stdout is empty on a dry run)
    text = shown.stderr or shown.stdout
    return [line for line in text.splitlines() if line.strip() and not line.startswith("#")]


def expand(recipe: str, seen: set[str] | None = None, run=subprocess.run) -> list[str]:
    """Every leaf line of `recipe`, nested `just X` lines expanded."""
    seen = set() if seen is None else seen
    if recipe in seen:
        return [f"just {recipe}"]
    seen.add(recipe)
    out: list[str] = []
    for line in lines_of(recipe, run):
        nested = NESTED.match(line.strip())
        out.extend(expand(nested.group(1), seen, run) if nested else [line])
    return out


def main(argv: list[str]) -> int:
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) != 2:
        print(__doc__.strip().splitlines()[-2].strip(), file=sys.stderr)
        return 2
    lines = expand(argv[1])
    failed: list[tuple[str, int]] = []
    for number, line in enumerate(lines, 1):
        print(f"\n== [{number}/{len(lines)}] {line}", flush=True)
        code = subprocess.run(["bash", "-c", line], cwd=ROOT, check=False).returncode
        if code != 0:
            failed.append((line, code))
            print(f"== FAILED (exit {code}): {line}", flush=True)
    print(f"\nkeep-going {argv[1]}: {len(lines) - len(failed)} of {len(lines)} passed")
    for line, code in failed:
        print(f"  FAILED (exit {code}): {line}")
    return 1 if failed else 0


def selftest() -> int:
    plans = {"outer": "echo a\njust inner\necho c\n", "inner": "echo b1\necho b2\n"}

    class Shown:
        def __init__(self, text: str) -> None:
            self.returncode, self.stderr, self.stdout = 0, text, ""

    def fake(cmd, **_kwargs):
        return Shown(plans[cmd[-1]])

    got = expand("outer", run=fake)
    assert got == ["echo a", "echo b1", "echo b2", "echo c"], got
    plans["loop"] = "just loop\n"
    assert expand("loop", run=fake) == ["just loop"], "a recipe calling itself isn't expanded forever"
    print("keep-going selftest: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
