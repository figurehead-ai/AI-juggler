#!/usr/bin/env python3
"""A release ships its whole roadmap scope, or says what it defers and why.

docs/plan/ROADMAP.md gives each minor version a section (`## 0.4 — …`)
whose table names its tasks: single ids (`AJ-E3207`), ranges
(`AJ-E2001…AJ-E2019`) and whole epics (`E29`, every task in
docs/tasks/E29-*.md). Each named task must be DONE (its header's
`Status: DONE…`) AND proven: docs/qa/evidence/<id>/manifest.md has, for
every numbered requirement N of the task, a line naming `R<N>` with PASS
(or `DEFERRED (D-NN)` for a requirement the maintainer moved). A status
word alone proves nothing. Otherwise the task must be listed in the
section's deferral line, which names the maintainer's decision:

    **Deferred (D-90):** AJ-E2008, AJ-E2010 — to 0.5, scheduled fuzzing ...

A table row that names no task at all is reported too, so a promise can't
hide in a row nothing checks.

Usage:
    release_scope.py report 0.4      # the list, exit 0
    release_scope.py check 0.4.1     # exit 1 when anything is open
The nightly runs `check` before it builds a new minor (x.y.0) and the
promotion runs it before any x.y.z reaches a channel; preflight prints the
report on release/* branches. (The cause, 2026-11-06: 0.4.0 shipped to
beta with all of E20 and part of E33 not built, because nothing compared
the roadmap with the task statuses.)
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ROADMAP = ROOT / "docs/plan/ROADMAP.md"
TASKS = ROOT / "docs/tasks"

HEADER = re.compile(r"^### (AJ-E\d+) (.*?) \| Status: ([^|]*)", re.M)
RANGE = re.compile(r"AJ-E(\d+)\s*(?:…|\.\.\.?)\s*AJ-E(\d+)")
SINGLE = re.compile(r"AJ-E\d+")
EPIC = re.compile(r"(?<![\w-])E(\d{1,2})(?![\d\w])")
DEFERRED = re.compile(r"^\*\*Deferred \((D-\d+)\):\*\*(.*)$", re.M)


EVIDENCE = ROOT / "docs/qa/evidence"
REQS: dict[str, int] = {}


def tasks() -> dict[str, tuple[str, str, str]]:
    """Every task: id → (title, status, epic number); REQS gets its count
    of numbered requirements."""
    found = {}
    for path in sorted(TASKS.glob("E*-*.md")):
        epic = re.match(r"E(\d+)-", path.name).group(1)
        text = path.read_text(encoding="utf-8")
        heads = list(HEADER.finditer(text))
        for i, m in enumerate(heads):
            found[m.group(1)] = (m.group(2).strip(), m.group(3).strip(), epic)
            body = text[m.end(): heads[i + 1].start() if i + 1 < len(heads) else len(text)]
            req = re.search(r"\*\*Requirements\*\*:?(.*?)(?=\n\*\*|\Z)", body, re.S)
            REQS[m.group(1)] = len(re.findall(r"^\d+\.", req.group(1), re.M)) if req else 0
    return found


def unproven(task: str) -> str | None:
    """Why a DONE task isn't proven, or None when every requirement is."""
    manifest = EVIDENCE / task / "manifest.md"
    if not manifest.exists():
        return "no evidence manifest"
    lines = manifest.read_text(encoding="utf-8").splitlines()
    gaps = [
        f"R{n}" for n in range(1, REQS.get(task, 0) + 1)
        if not any(re.search(rf"\bR{n}\b", l) and re.search(r"PASS|DEFERRED \(D-\d+\)", l) for l in lines)
    ]
    return f"manifest has no PASS line for {', '.join(gaps)}" if gaps else None


def section(minor: str) -> str:
    text = ROADMAP.read_text(encoding="utf-8")
    m = re.search(rf"^## {re.escape(minor)}\b.*?$(.*?)(?=^## )", text, re.M | re.S)
    if not m:
        sys.exit(f"release_scope: no '## {minor}' section in {ROADMAP.relative_to(ROOT)}")
    return m.group(1)


def named(cell: str, known: dict) -> set[str]:
    ids: set[str] = set()
    for a, b in RANGE.findall(cell):
        lo, hi = int(a), int(b)
        ids |= {t for t in known if lo <= int(t[4:]) <= hi}
        cell = cell.replace(f"AJ-E{a}", "").replace(f"AJ-E{b}", "")
    ids |= set(SINGLE.findall(cell))
    for epic in EPIC.findall(re.sub(r"AJ-E\d+", "", cell)):
        ids |= {t for t, (_, _, e) in known.items() if e == epic}
    return ids


def scope(minor: str):
    known = tasks()
    body = section(minor)
    wanted: set[str] = set()
    unchecked: list[str] = []
    for line in body.splitlines():
        if not line.startswith("|") or set(line) <= set("|- "):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if cells[0] in ("Area", "Decision", "Version"):
            continue
        ids = named(cells[-1], known)
        if ids:
            wanted |= ids
        else:
            unchecked.append(cells[0])
    deferred: dict[str, str] = {}
    for decision, rest in DEFERRED.findall(body):
        for t in named(rest.split("—")[0], known):
            deferred[t] = decision
    missing = sorted(t for t in wanted if t not in known)
    open_ = sorted(
        (t for t in wanted if t in known and not known[t][1].startswith("DONE") and t not in deferred),
        key=lambda t: int(t[4:]),
    )
    weak = {
        t: why for t in sorted(wanted, key=lambda t: int(t[4:]))
        if t in known and known[t][1].startswith("DONE") and t not in deferred
        and (why := unproven(t))
    }
    return known, wanted, open_, deferred, missing, unchecked, weak


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in ("report", "check"):
        print(__doc__)
        return 2
    mode, version = sys.argv[1], sys.argv[2]
    minor = ".".join(version.split(".")[:2])
    known, wanted, open_, deferred, missing, unchecked, weak = scope(minor)
    done = len(wanted) - len(open_) - len([t for t in wanted if t in deferred]) - len(missing) - len(weak)
    print(f"release scope {minor}: {len(wanted)} tasks named, {done} done and proven, "
          f"{len(weak)} marked done but unproven, {len(deferred)} deferred, {len(open_)} open")
    for t in open_:
        title, status, _ = known[t]
        print(f"  OPEN      {t} {title[:70]} — {status[:60]}")
    for t, why in weak.items():
        print(f"  UNPROVEN  {t} {known[t][0][:60]} — {why}")
    for t, d in sorted(deferred.items()):
        print(f"  deferred  {t} ({d})")
    for t in missing:
        print(f"  UNKNOWN   {t}: named in the roadmap, no task header in docs/tasks")
    for row in unchecked:
        print(f"  UNCHECKED row '{row[:60]}': names no task, so nothing proves it")
    failed = bool(open_ or missing or unchecked or weak)
    if mode == "check" and failed:
        print(f"release_scope: {minor} is not complete. Finish the open tasks, or "
              "list them under '**Deferred (D-NN):**' with the maintainer's decision.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
