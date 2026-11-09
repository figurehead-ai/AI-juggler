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

Work added for a later patch goes under its own sub-section of the
minor's section (`### 0.4.2 — …`). `check 0.4.1` then needs the section's
own rows and the sub-sections up to 0.4.1, not 0.4.2's; `check 0.4.2`
needs all of them up to 0.4.2. A Stable promotion (`--channel stable`)
needs the whole section, every sub-section included: the minor's first
Stable is the minor complete (D-85).

Usage:
    release_scope.py report 0.4      # the list, exit 0
    release_scope.py check 0.4.1     # exit 1 when anything is open
    release_scope.py check 0.4.3 --channel stable   # the whole 0.4
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
    # 0.4.1's audit: nine manifests labelled their rows "Req.1", which this
    # check can't read, and their tasks stayed open for no visible reason.
    rows = [l for l in lines if l.startswith("|") and not re.match(r"^\|[\s:|-]*$", l)]
    if REQS.get(task, 0) and rows and not any(re.search(r"\bR\d+\b", l) for l in rows):
        other = "Req.N" if any(re.search(r"\bReq\.?\s*\d", l) for l in rows) else "no requirement ids"
        return f"manifest rows carry {other}, not R1, R2…: label each row with the R<N> it proves"
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


def named_ids(cell: str, known: dict) -> set[str]:
    """The tasks `cell` names by id or range, its epics left out."""
    return named(re.sub(r"(?<![\w-])E\d{1,2}(?![\d\w])", "", cell), known)


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


PATCH = re.compile(r"^### (\d+)\.(\d+)\.(\d+)\b")


def promised(body: str, upto: tuple[int, int, int] | None) -> tuple[str, str]:
    """The section's lines a release of `upto` must prove (its own, and
    each patch sub-section up to `upto`; all of them for `None`), and the
    later patches' lines."""
    keep, later, current = [], [], None
    for line in body.splitlines():
        m = PATCH.match(line)
        if m:
            current = tuple(int(part) for part in m.groups())
        elif line.startswith("### "):
            current = None
        if upto is None or current is None or current <= upto:
            keep.append(line)
        else:
            later.append(line)
    return "\n".join(keep), "\n".join(later)


def rows(body: str):
    """Each table row's first and last cells (header rows left out)."""
    for line in body.splitlines():
        if not line.startswith("|") or set(line) <= set("|- "):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if cells[0] in ("Area", "Decision", "Version"):
            continue
        yield cells[0], cells[-1]


def scope(minor: str, upto: tuple[int, int, int] | None = None):
    known = tasks()
    body, later_body = promised(section(minor), upto)
    wanted: set[str] = set()
    unchecked: list[str] = []
    for area, cell in rows(body):
        ids = named(cell, known)
        if ids:
            wanted |= ids
        else:
            unchecked.append(area)
    # A task a later patch names by id is that patch's, even where an
    # earlier row's whole epic covers it (E35's 0.4.2 tasks).
    for _, cell in rows(later_body):
        wanted -= named_ids(cell, known)
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


def selftest() -> int:
    """The manifest rules on made-up manifests (no repository state)."""
    import tempfile

    global EVIDENCE
    saved = EVIDENCE
    try:
        EVIDENCE = pathlib.Path(tempfile.mkdtemp())
        (EVIDENCE / "AJ-E9").mkdir()
        manifest = EVIDENCE / "AJ-E9" / "manifest.md"
        REQS["AJ-E9"] = 2
        cases = [
            ("| Claim | Verdict |\n|---|---|\n| Req.1 a | PASS |\n| Req.2 b | PASS |\n", "carry Req.N"),
            ("| Claim | Verdict |\n|---|---|\n| a | PASS |\n", "carry no requirement ids"),
            ("| Claim | Verdict |\n|---|---|\n| R1 a | PASS |\n", "no PASS line for R2"),
            ("| Claim | Verdict |\n|---|---|\n| R1 a | PASS |\n| R2–R3 b | PASS |\n", None),
        ]
        for text, want in cases:
            manifest.write_text(text, encoding="utf-8")
            got = unproven("AJ-E9")
            assert (got is None) if want is None else (got is not None and want in got), (text, got)
    finally:
        EVIDENCE = saved
        REQS.pop("AJ-E9", None)
    print("release-scope selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    channel = None
    if len(args) == 4 and args[2] == "--channel":
        channel = args.pop()
        args.pop()
    if len(args) != 2 or args[0] not in ("report", "check"):
        print(__doc__)
        return 2
    mode, version = args
    minor = ".".join(version.split(".")[:2])
    parts = version.split(".")
    # A patch is held to the patches up to it; a minor alone, or a Stable
    # promotion, to the whole section.
    upto = (
        tuple(int(part) for part in parts)
        if len(parts) == 3 and all(part.isdigit() for part in parts) and channel != "stable"
        else None
    )
    known, wanted, open_, deferred, missing, unchecked, weak = scope(minor, upto)
    done = len(wanted) - len(open_) - len([t for t in wanted if t in deferred]) - len(missing) - len(weak)
    print(f"release scope {version}{' (stable: the whole ' + minor + ')' if channel == 'stable' else ''}: {len(wanted)} tasks named, {done} done and proven, "
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
