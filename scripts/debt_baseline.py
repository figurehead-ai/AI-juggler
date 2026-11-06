#!/usr/bin/env python3
"""AJ-E2011 R7: dead code and copied code may only shrink.

- knip (ui/knip.json): unused files, dependencies, exports, exported
  types, duplicate exports and unresolved imports in the UI. oxlint,
  oxlint-tsgolint and jscpd are listed as used: scripts call them
  (`ui_lint.py`, this one), not the UI's code.
- jscpd: copy-pasted blocks in the UI's TypeScript and the crates' Rust
  (tests, fixtures and generated types left out), counted as each file's
  duplicated lines. A count per file, not per clone, so code that only
  moves doesn't trip it and a new copy does.

Each file's counts are compared with `scripts/debt_baseline.json`. A
count above its baseline fails (new code adds no debt). A count below it
fails too until the baseline is lowered with `--update`, which never
raises one (so the debt paid stays paid).

Usage: debt_baseline.py [--update] [--selftest]
"""
from __future__ import annotations

import collections
import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
UI = ROOT / "ui"
BASELINE = ROOT / "scripts" / "debt_baseline.json"
KINDS = ("files", "dependencies", "devDependencies", "unlisted", "unresolved", "exports", "types", "duplicates", "enumMembers")
JSCPD_IGNORE = "**/types/generated/**,**/*.test.*,**/target/**,**/node_modules/**,**/tests/**,**/fixtures/**"


def knip_counts(report: dict) -> dict[str, dict[str, int]]:
    out: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for issue in report.get("issues", []):
        for kind in KINDS:
            n = len(issue.get(kind) or [])
            if n:
                out[f"ui/{issue['file']}"][f"knip:{kind}"] = n
    return out


def jscpd_counts(report: dict) -> dict[str, dict[str, int]]:
    lines: collections.Counter = collections.Counter()
    for dup in report.get("duplicates", []):
        for side in ("firstFile", "secondFile"):
            f = dup[side]
            name = f["name"]
            rel = f"crates/{name}" if not name.startswith("src/") else f"ui/{name}"
            lines[rel] += f["end"] - f["start"] + 1
    return {rel: {"jscpd:lines": n} for rel, n in lines.items()}


def measure() -> dict[str, dict[str, int]]:
    knip = subprocess.run(["pnpm", "exec", "knip", "--reporter", "json", "--no-exit-code"],
                          cwd=UI, capture_output=True, text=True, check=False)
    try:
        knip_report = json.loads(knip.stdout)
    except json.JSONDecodeError:
        sys.exit(f"debt-baseline: knip didn't run (pnpm install in ui/?): {knip.stderr.strip()[:300]}")
    with tempfile.TemporaryDirectory() as out:
        subprocess.run(["pnpm", "exec", "jscpd", "--silent", "--reporters", "json", "--output", out,
                        "--ignore", JSCPD_IGNORE, "--format", "typescript,tsx,rust", "src", "../crates"],
                       cwd=UI, capture_output=True, text=True, check=False)
        try:
            jscpd_report = json.loads((pathlib.Path(out) / "jscpd-report.json").read_text())
        except (OSError, json.JSONDecodeError):
            sys.exit("debt-baseline: jscpd didn't run (pnpm install in ui/?)")
    merged: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for part in (knip_counts(knip_report), jscpd_counts(jscpd_report)):
        for rel, kinds in part.items():
            merged[rel].update(kinds)
    return {rel: dict(sorted(kinds.items())) for rel, kinds in sorted(merged.items())}


def problems(now: dict, base: dict) -> list[str]:
    out = []
    for rel in sorted(set(now) | set(base)):
        for kind in sorted(set(now.get(rel, {})) | set(base.get(rel, {}))):
            n, b = now.get(rel, {}).get(kind, 0), base.get(rel, {}).get(kind, 0)
            if n > b:
                out.append(f"{rel}: {kind} {n} (baseline {b}) — new debt: remove it")
            elif n < b:
                out.append(f"{rel}: {kind} {n} (baseline {b}) — fewer now: run debt_baseline.py --update to keep it that way")
    return out


def lowered(now: dict, base: dict) -> tuple[dict, list[str]]:
    """The baseline at today's counts where they're lower; what grew."""
    grew = [line for line in problems(now, base) if "new debt" in line]
    out = {rel: {k: n for k, n in kinds.items() if n} for rel, kinds in now.items()} if not grew else base
    return {rel: kinds for rel, kinds in out.items() if kinds}, grew


def selftest() -> None:
    knip = {"issues": [{"file": "src/a.ts", "exports": [{"name": "x"}, {"name": "y"}], "types": [], "files": []}]}
    assert knip_counts(knip) == {"ui/src/a.ts": {"knip:exports": 2}}
    jscpd = {"duplicates": [{"firstFile": {"name": "src/a.ts", "start": 1, "end": 10},
                             "secondFile": {"name": "juggler-core/src/b.rs", "start": 5, "end": 9}}]}
    assert jscpd_counts(jscpd) == {"ui/src/a.ts": {"jscpd:lines": 10}, "crates/juggler-core/src/b.rs": {"jscpd:lines": 5}}
    base = {"a": {"knip:exports": 2}}
    assert not problems({"a": {"knip:exports": 2}}, base)
    assert problems({"a": {"knip:exports": 3}}, base), "grew"
    assert problems({"a": {"knip:exports": 1}}, base), "shrank: the baseline must follow"
    assert problems({"b": {"jscpd:lines": 4}}, base), "a new file"
    new, grew = lowered({"a": {"knip:exports": 1}}, base)
    assert new == {"a": {"knip:exports": 1}} and not grew
    new, grew = lowered({"a": {"knip:exports": 3}}, base)
    assert new == base and grew, "never raised"
    print("debt-baseline: selftest ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    now = measure()
    base = json.loads(BASELINE.read_text()) if BASELINE.exists() else None
    if "--update" in sys.argv:
        if base is None:
            new, grew = now, []
        else:
            new, grew = lowered(now, base)
        for line in grew:
            print(f"debt-baseline: {line}", file=sys.stderr)
        if grew:
            return 1
        BASELINE.write_text(json.dumps(new, indent=1, sort_keys=True) + "\n")
        total = collections.Counter(k for kinds in new.values() for k in kinds for _ in range(kinds[k]))
        print(f"debt-baseline: written ({dict(total)})")
        return 0
    found = problems(now, base or {})
    for line in found:
        print(f"debt-baseline: {line}", file=sys.stderr)
    if not found:
        print(f"debt-baseline: OK ({len(now)} files carry baselined debt)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
