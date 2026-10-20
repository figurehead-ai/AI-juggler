#!/usr/bin/env python3
"""AJ-E1010: a release that changes the database ships a downgrade note.

Migrations only go forward and an older build refuses newer data, so a
version whose database schema is newer than the one users come from must
say how to go back: a bullet under "## Known issues" that starts with
"Going back to" (docs/RELEASING.md § Database changes).

  downgrade_note.py schema REF          the schema (migration count) at REF
  downgrade_note.py check NOTES REF FROM
      fails when REF's schema is newer than FROM's and NOTES has no
      downgrade note; passes (saying why) when FROM can't be read
  downgrade_note.py --selftest
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STORAGE = "crates/juggler-core/src/storage/mod.rs"
MIGRATIONS = re.compile(r"const MIGRATIONS: &\[&str\] = &\[(?P<body>.*?)\];", re.S)


def count(source: str) -> int:
    """The number of entries in storage's MIGRATIONS list."""
    match = MIGRATIONS.search(source)
    if not match:
        raise ValueError("no MIGRATIONS list")
    return len([item for item in match.group("body").split(",") if item.strip()])


def schema_at(ref: str) -> int | None:
    shown = subprocess.run(
        ["git", "show", f"{ref}:{STORAGE}"], cwd=ROOT, capture_output=True, text=True
    )
    return count(shown.stdout) if shown.returncode == 0 else None


def has_note(notes: str) -> bool:
    """A "## Known issues" section with a bullet starting "Going back to"."""
    section = re.search(r"^## Known issues\s*$(.*?)(?=^## |\Z)", notes, re.M | re.S)
    return bool(section) and bool(
        re.search(r"^-\s+(\*\*)?Going back to\b", section.group(1), re.M)
    )


def verdict(notes: str, now: int | None, before: int | None, label: str) -> tuple[bool, str]:
    if now is None or before is None:
        return True, f"downgrade-note: can't compare database schemas with {label}; not checked"
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
    note = "One sentence.\n\n## Fixed\n- x\n\n## Known issues\n\n- **Going back to 0.0.7** takes steps.\n"
    plain = "One sentence.\n\n## Fixed\n- Going back to 0.0.7 is not a known-issues bullet.\n"
    assert has_note(note) and not has_note(plain)
    assert verdict(plain, 28, 27, "v0.0.7")[0] is False
    assert verdict(note, 28, 27, "v0.0.7")[0] is True
    assert verdict(plain, 28, 28, "v0.0.7")[0] is True
    assert verdict(plain, 28, None, "nothing")[0] is True
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
        ok, message = verdict(Path(notes).read_text(), schema_at(ref), schema_at(base), base)
        print(message, file=sys.stdout if ok else sys.stderr)
        return 0 if ok else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
