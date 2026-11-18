#!/usr/bin/env python3
"""No two branches claim the same schema migration version.

The migration runner (`crates/juggler-core/src/storage/mod.rs`) applies
`MIGRATIONS` in order and the version is the list index + 1, so versions
are contiguous by construction and a version means one batch of SQL on
every branch. Two release branches that each add their own `V53` would
both ship "schema 53" with different tables: a database written by one
would be taken as up to date by the other. ADR-0037 met this (V53 on
release/0.4.6, the memory store's V54 on release/0.5).

This compares the versions this tree claims (every `const V<N>: &str`
in the storage folder, registered in `MIGRATIONS` or not yet: the memory
store's `memory::V54` waits for the 0.4.6 merge) with each other branch below:
a version both sides added since their merge base, with different
definitions, is a clash. Refs that aren't here (a CI checkout, an old
clone) are skipped, and so is a ref without a merge base (a shallow
clone). It also checks this tree's `MIGRATIONS` is V1..Vk
in order, as the runner needs.

Usage: scripts/migration_clash.py              (fails naming each clash)
       scripts/migration_clash.py --selftest
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIR = "crates/juggler-core/src/storage"
PATH = f"{DIR}/mod.rs"  # holds MIGRATIONS
REFS = ("release/0.4.6", "origin/release/0.4.6", "main", "origin/main")

DEF = re.compile(r"^(?:pub(?:\([^)]*\))? )?const V(\d+): &str = ", re.M)
LIST = re.compile(r"^const MIGRATIONS: &\[&str\] = &\[(.*?)\];", re.M | re.S)


def _rhs(text: str, start: int) -> str:
    """The definition after `= `: a string literal (plain or raw), or an
    expression up to its `;`."""
    raw = re.match(r'r(#*)"', text[start:])
    if raw:
        close = '"' + raw.group(1)
        end = text.index(close, start + raw.end())
        return text[start : end + len(close)]
    if text.startswith('"', start):
        i = start + 1
        while text[i] != '"':
            i += 2 if text[i] == "\\" else 1
        return text[start : i + 1]
    return text[start : text.index(";", start)].strip()


def claims(text: str) -> dict[int, str]:
    """Version -> its definition, whitespace-normalised."""
    return {
        int(m.group(1)): " ".join(_rhs(text, m.end()).split())
        for m in DEF.finditer(text)
    }


def registered(text: str) -> list[str]:
    m = LIST.search(text)
    names = [n.strip() for n in m.group(1).split(",") if n.strip()] if m else []
    return [n.split("::")[-1] for n in names]


def clashes(head: dict[int, str], other: dict[int, str], base: dict[int, str]) -> list[int]:
    """Versions both sides added since `base`, defined differently."""
    return sorted(
        v for v in head.keys() & other.keys() if v not in base and head[v] != other[v]
    )


def contiguity(names: list[str]) -> str | None:
    want = [f"V{i}" for i in range(1, len(names) + 1)]
    if names != want:
        return f"MIGRATIONS must be V1..V{len(names)} in order (the runner's version is index + 1)"
    return None


def _git(*args: str) -> str | None:
    run = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    return run.stdout if run.returncode == 0 else None


def _merged(texts: list[str]) -> dict[int, str]:
    out: dict[int, str] = {}
    for text in texts:
        out.update(claims(text))
    return out


def _at(ref: str) -> dict[int, str]:
    names = (_git("ls-tree", "-r", "--name-only", ref, f"{DIR}/") or "").split()
    texts = [_git("show", f"{ref}:{name}") or "" for name in names if name.endswith(".rs")]
    return _merged(texts)


def check() -> list[str]:
    text = (ROOT / PATH).read_text()
    problems = []
    if bad := contiguity(registered(text)):
        problems.append(bad)
    head = _merged([f.read_text() for f in sorted((ROOT / DIR).rglob("*.rs"))])
    for ref in REFS:
        if _git("rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}") is None:
            print(f"migration_clash: {ref} isn't here; skipped")
            continue
        base = (_git("merge-base", "HEAD", ref) or "").strip()
        if not base:
            # A shallow clone: without the merge base, versions both
            # sides share can't be told from versions both added.
            print(f"migration_clash: no merge base with {ref} (shallow?); skipped")
            continue
        for v in clashes(head, _at(ref), _at(base)):
            problems.append(
                f"V{v} is claimed here and on {ref} with different migrations; "
                f"take the next free version after both"
            )
    return problems


def selftest() -> None:
    text = '''
const V1: &str = "CREATE TABLE a (x TEXT DEFAULT ';');";
const V2: &str = r#"
CREATE TABLE b (y TEXT); -- a "quoted" word
"#;
const V3: &str = pools::LEAD_NEXT_MIGRATION;
const V4: &str = "it\\"s; fine";
const MIGRATIONS: &[&str] = &[
    V1, V2,
    V3,
];
'''
    got = claims(text)
    assert sorted(got) == [1, 2, 3, 4], got
    assert got[1] == '''"CREATE TABLE a (x TEXT DEFAULT ';');"''', got[1]
    assert got[2].startswith('r#" CREATE TABLE b') and got[2].endswith('"#'), got[2]
    assert got[3] == "pools::LEAD_NEXT_MIGRATION", got[3]
    assert got[4] == '"it\\"s; fine"', got[4]
    assert registered(text) == ["V1", "V2", "V3"]
    assert registered(text.replace("    V3,", "    V3, memory::V4,")) == ["V1", "V2", "V3", "V4"]
    assert claims("pub const V54: &str = r#\"x\"#;") == {54: 'r#"x"#'}
    assert claims("pub(crate) const V9: &str = a::B;") == {9: "a::B"}
    assert contiguity(["V1", "V2", "V3"]) is None
    assert contiguity(["V1", "V3"]) is not None
    assert contiguity(["V2", "V1"]) is not None
    base = {1: "a", 52: "x"}
    # The clash this exists for: both sides added V53, differently.
    assert clashes({**base, 53: "memory"}, {**base, 53: "pools"}, base) == [53]
    # A gap is no clash: this side claims V54 while the other has V53.
    assert clashes({**base, 54: "memory"}, {**base, 53: "pools"}, base) == []
    # The same migration on both (merged or picked across) is no clash.
    assert clashes({**base, 53: "pools"}, {**base, 53: "pools"}, base) == []
    # An older shared version edited on one side isn't this check's job.
    assert clashes({1: "a2", 52: "x"}, base, base) == []
    # An empty base: every shared version that differs counts.
    assert clashes({1: "a"}, {1: "b"}, {}) == [1]
    print("migration_clash selftest: ok")


def main() -> int:
    if "--selftest" in sys.argv[1:]:
        selftest()
        return 0
    problems = check()
    for problem in problems:
        print(f"migration_clash: {problem}", file=sys.stderr)
    if not problems:
        print("migration_clash: ok")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
