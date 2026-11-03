#!/usr/bin/env python3
"""Did a commit's CI already pass? (ADR-0025)

Reads a commit's check runs (JSON lines of `name`, `status`, `conclusion`,
`completed_at`, as `gh api --paginate .../commits/SHA/check-runs --jq
'.check_runs[] | {name, status, conclusion, completed_at, app: .app.slug}'`
prints them) and answers one question:

- `checks FILE NAME...`: every named check's newest run completed with
  `success`. CI's push to main uses it to skip a tree its pull request's
  head already passed with.
- `a11y FILE`: the accessibility gate really scanned and passed: `a11y`
  and every shard `a11y (i/n)` succeeded, all n of them. An `a11y` that
  passed because nothing in the interface changed (no shards ran) is not
  a scan. Promote uses it to skip a scan the same tree already passed.

Only GitHub Actions' runs count. Exit 0 when it holds; otherwise 1, with
the reason on stderr.

Usage: scripts/gate_passed.py checks FILE NAME...
       scripts/gate_passed.py a11y FILE
       scripts/gate_passed.py --selftest
"""
from __future__ import annotations

import json
import re
import sys

SHARD = re.compile(r"^a11y \((\d+)/(\d+)\)$")


def newest(lines: list[str]) -> dict[str, dict]:
    """The newest run of each check, by name (a re-run replaces a run)."""
    runs: dict[str, dict] = {}
    for line in lines:
        line = line.strip()
        if not line:
            continue
        run = json.loads(line)
        if run.get("app", "github-actions") != "github-actions":
            continue
        name = run.get("name", "")
        if name not in runs or (run.get("completed_at") or "") >= (runs[name].get("completed_at") or ""):
            runs[name] = run
    return runs


def ok(run: dict | None) -> bool:
    return bool(run) and run.get("status") == "completed" and run.get("conclusion") == "success"


def checks(lines: list[str], names: list[str]) -> list[str]:
    runs = newest(lines)
    return [f"{name}: {(runs.get(name) or {}).get('conclusion') or 'no run'}" for name in names if not ok(runs.get(name))]


def a11y(lines: list[str]) -> list[str]:
    runs = newest(lines)
    problems = []
    if not ok(runs.get("a11y")):
        problems.append(f"a11y: {(runs.get('a11y') or {}).get('conclusion') or 'no run'}")
    shards = {name: run for name, run in runs.items() if SHARD.match(name)}
    totals = {int(SHARD.match(name).group(2)) for name in shards}
    if not shards:
        problems.append("no a11y shard ran (the gate didn't scan)")
    elif len(totals) != 1:
        problems.append(f"a11y shards of different splits: {sorted(shards)}")
    else:
        total = totals.pop()
        for i in range(1, total + 1):
            name = f"a11y ({i}/{total})"
            if not ok(shards.get(name)):
                problems.append(f"{name}: {(shards.get(name) or {}).get('conclusion') or 'no run'}")
    return problems


def selftest() -> None:
    def run(name, conclusion="success", at="2026-11-02T10:00:00Z", app="github-actions", status="completed"):
        return json.dumps({"name": name, "status": status, "conclusion": conclusion, "completed_at": at, "app": app})

    green = [run(n) for n in ("verify", "ui", "visual")]
    assert checks(green, ["verify", "ui", "visual"]) == []
    assert checks(green, ["verify", "build-linux"]) == ["build-linux: no run"]
    rerun = [run("verify", "failure", "2026-11-02T09:00:00Z"), run("verify", "success", "2026-11-02T09:30:00Z")]
    assert checks(rerun, ["verify"]) == [], "the newest run decides"
    assert checks(list(reversed(rerun)), ["verify"]) == [], "whatever the order"
    assert checks([run("verify", "success", app="someone-else")], ["verify"]) == ["verify: no run"], "only Actions' runs"
    assert checks([run("verify", None, status="in_progress")], ["verify"]) == ["verify: no run"]
    assert checks([run("verify", "skipped")], ["verify"]) == ["verify: skipped"]

    scanned = [run("a11y")] + [run(f"a11y ({i}/3)") for i in (1, 2, 3)]
    assert a11y(scanned) == []
    assert a11y([run("a11y")] + [run("a11y (1/3)", "skipped")] + [run(f"a11y ({i}/3)") for i in (2, 3)]) == ["a11y (1/3): skipped"]
    assert a11y([run("a11y")]) == ["no a11y shard ran (the gate didn't scan)"], "a pass with no UI change isn't a scan"
    assert a11y([run("a11y")] + [run(f"a11y ({i}/3)") for i in (1, 2)]) == ["a11y (3/3): no run"]
    assert a11y([run("a11y", "failure")] + [run(f"a11y ({i}/2)") for i in (1, 2)]) == ["a11y: failure"]
    assert a11y([run("a11y"), run("a11y (1/2)"), run("a11y (2/2)"), run("a11y (1/4)")])[0].startswith("a11y shards of different splits")
    print("gate-passed selftest: OK")


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        selftest()
        return 0
    if len(args) >= 3 and args[0] == "checks":
        problems = checks(open(args[1], encoding="utf-8").read().splitlines(), args[2:])
    elif len(args) == 2 and args[0] == "a11y":
        problems = a11y(open(args[1], encoding="utf-8").read().splitlines())
    else:
        print(__doc__.strip().splitlines()[-3:], file=sys.stderr)
        return 2
    for problem in problems:
        print(f"gate-passed: {problem}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
