#!/usr/bin/env python3
"""AJ-E2012: the UI linter, with today's findings as a baseline that may
only shrink.

Runs oxlint (type-aware, on TypeScript 7's own checker; `ui/.oxlintrc.json`:
typescript strict rules, react and its hooks, jsx-a11y, `max-lines`,
complexity, and only `lib/ipc` calling the backend) and compares each
file's count per rule with `ui/lint-baseline.json`. A count above its
baseline fails (new code adds no debt); a count below it fails too until
the baseline is lowered with `--update` (so the debt paid stays paid).

Usage (from the repo root or ui/):
    ui_lint.py            # check
    ui_lint.py --update   # lower the baseline to today's counts (never raises it)
    ui_lint.py --selftest
"""
from __future__ import annotations

import collections
import json
import pathlib
import subprocess
import sys

UI = pathlib.Path(__file__).resolve().parent.parent / "ui"
BASELINE = UI / "lint-baseline.json"


def counts() -> dict[str, dict[str, int]]:
    out = subprocess.run(
        ["pnpm", "exec", "oxlint", "--type-aware", "-f", "json", "src"],
        cwd=UI, capture_output=True, text=True, check=False,
    )
    found: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    for item in json.loads(out.stdout).get("diagnostics", []):
        found[item.get("filename", "?")][item.get("code", "?")] += 1
    return {f: dict(c) for f, c in found.items()}


def compare(now: dict, base: dict) -> list[str]:
    problems = []
    for file in sorted(set(now) | set(base)):
        for rule in sorted(set(now.get(file, {})) | set(base.get(file, {}))):
            n, b = now.get(file, {}).get(rule, 0), base.get(file, {}).get(rule, 0)
            if n > b:
                problems.append(f"{file}: {rule} {n} (baseline {b}) — new findings: fix them")
            elif n < b:
                problems.append(f"{file}: {rule} {n} (baseline {b}) — fewer now: run ui_lint.py --update to keep it that way")
    return problems


def main() -> int:
    if sys.argv[1:] == ["--selftest"]:
        assert compare({"a": {"r": 1}}, {"a": {"r": 1}}) == []
        assert "new findings" in compare({"a": {"r": 2}}, {"a": {"r": 1}})[0]
        assert "fewer now" in compare({}, {"a": {"r": 1}})[0]
        print("ui-lint: selftest ok")
        return 0
    now = counts()
    base = json.loads(BASELINE.read_text()) if BASELINE.exists() else {}
    if sys.argv[1:] == ["--update"]:
        if BASELINE.exists() and any("new findings" in p for p in compare(now, base)):
            print("ui-lint: there are new findings; fix them before lowering the baseline")
            return 1
        BASELINE.write_text(json.dumps(now, indent=1, sort_keys=True) + "\n")
        print(f"ui-lint: baseline written ({sum(sum(r.values()) for r in now.values())} findings)")
        return 0
    problems = compare(now, base)
    for problem in problems:
        print(f"ui-lint: {problem}")
    if not problems:
        print(f"ui-lint: OK ({sum(sum(r.values()) for r in base.values())} baselined findings left)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
