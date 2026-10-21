#!/usr/bin/env python3
"""AJ-E1010: a release that changes the database ships a downgrade note.

Migrations only go forward and an older build refuses newer data, so a
version whose database schema is newer than the one users come from must
say how to go back: a bullet under "## Known issues" that starts with
"Going back to" (docs/RELEASING.md § Database changes).

  downgrade_note.py schema REF          the schema (migration count) at REF
  downgrade_note.py check NOTES REF FROM
      fails when REF's schema is newer than FROM's and NOTES has no
      downgrade note, and when either schema can't be read (fails closed:
      AIJ_DOWNGRADE_NOTE_UNCHECKED=<reason> lets a release through, saying so)
  downgrade_note.py --selftest
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STORAGE = "crates/juggler-core/src/storage/mod.rs"
MIGRATIONS = re.compile(r"const MIGRATIONS: &\[&str\] = &\[(?P<body>.*?)\];", re.S)


ENTRY = re.compile(r"\bV(\d+)\b")


def count(source: str) -> int:
    """The number of entries in storage's MIGRATIONS list: its `V<n>`
    constants (comments stripped), which must run V1..Vn without a gap."""
    match = MIGRATIONS.search(source)
    if not match:
        raise ValueError("no MIGRATIONS list")
    body = re.sub(r"//[^\n]*|/\*.*?\*/", "", match.group("body"), flags=re.S)
    numbers = [int(n) for n in ENTRY.findall(body)]
    if numbers != list(range(1, len(numbers) + 1)):
        raise ValueError(f"MIGRATIONS isn't V1..Vn in order: {numbers}")
    return len(numbers)


def schema_at(ref: str) -> int | None:
    shown = subprocess.run(
        ["git", "show", f"{ref}:{STORAGE}"], cwd=ROOT, capture_output=True, text=True
    )
    if shown.returncode != 0:
        return None
    try:
        return count(shown.stdout)
    except ValueError:
        return None


def has_note(notes: str) -> bool:
    """A "## Known issues" section with a bullet starting "Going back to"."""
    section = re.search(r"^## Known issues\s*$(.*?)(?=^## |\Z)", notes, re.M | re.S)
    return bool(section) and bool(
        re.search(r"^-\s+(\*\*)?Going back to\b", section.group(1), re.M)
    )


def verdict(
    notes: str, now: int | None, before: int | None, label: str, waiver: str = ""
) -> tuple[bool, str]:
    if now is None or before is None:
        if waiver:
            return True, f"downgrade-note: NOT CHECKED against {label} ({waiver})"
        return False, (
            f"downgrade-note: can't read the database schema at {label if before is None else 'this release'}, "
            "so whether a downgrade note is needed is unknown. Fetch the tags (fetch-depth: 0), or set "
            "AIJ_DOWNGRADE_NOTE_UNCHECKED=<reason> to release without the check."
        )
    if now <= before:
        return True, f"downgrade-note: database schema {now}, as in {label}; no note needed"
    if has_note(notes):
        return True, f"downgrade-note: schema {before} → {now} since {label}; the note is there"
    return False, (
        f"downgrade-note: the database changes since {label} (schema {before} → {now}), so earlier "
        "versions can't open the new data. Add a bullet under '## Known issues' starting "
        "'Going back to …' that says how (docs/RELEASING.md § Database changes)."
    )


def selftest() -> int:
    source = "x\nconst MIGRATIONS: &[&str] = &[\n    V1, V2, V3,\n    V4,\n];\n"
    assert count(source) == 4
    assert count("const MIGRATIONS: &[&str] = &[V1, V2, // V3 later\n V3, /* V9 */ V4];") == 4
    for broken in ("const MIGRATIONS: &[&str] = &[V1, V3];", "nothing here"):
        try:
            count(broken)
            raise AssertionError(broken)
        except ValueError:
            pass
    note = "One sentence.\n\n## Fixed\n- x\n\n## Known issues\n\n- **Going back to 0.0.7** takes steps.\n"
    plain = "One sentence.\n\n## Fixed\n- Going back to 0.0.7 is not a known-issues bullet.\n"
    assert has_note(note) and not has_note(plain)
    assert verdict(plain, 28, 27, "v0.0.7")[0] is False
    assert verdict(note, 28, 27, "v0.0.7")[0] is True
    assert verdict(plain, 28, 28, "v0.0.7")[0] is True
    assert verdict(plain, 28, None, "nothing")[0] is False, "fails closed"
    assert verdict(plain, None, 27, "v0.0.7")[0] is False
    assert verdict(plain, 28, None, "nothing", waiver="first release")[0] is True
    print("downgrade-note selftest: OK")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if len(args) == 2 and args[0] == "schema":
        found = schema_at(args[1])
        print(found if found is not None else "unknown")
        return 0 if found is not None else 1
    if len(args) == 4 and args[0] == "check":
        _, notes, ref, base = args
        waiver = os.environ.get("AIJ_DOWNGRADE_NOTE_UNCHECKED", "").strip()
        ok, message = verdict(
            Path(notes).read_text(), schema_at(ref), schema_at(base), base, waiver
        )
        print(message, file=sys.stdout if ok else sys.stderr)
        return 0 if ok else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
