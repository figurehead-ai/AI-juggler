#!/usr/bin/env python3
"""AJ-E2011 / ENGINEERING-GUIDE A1, A6: the layering check.

1. Every workspace crate's dependencies on other workspace crates (from
   `cargo metadata --no-deps`) must be allowed by `docs/plan/layering.toml`
   [allow]. An edge not listed fails; so does a listed edge no crate has
   any more (the file stays the truth).
2. No adapter module imports a sibling tool's module: a `use crate::<m>`
   in `crates/juggler-adapters/src/<n>.rs` where n and m belong to
   different [adapters.families] fails, unless it is in
   [adapters].allow_sibling — which only shrinks (a listed import that
   went away must leave the list).

Usage: check_layering.py [--selftest]
"""
from __future__ import annotations

from typing import Dict, List, Optional, Set, Tuple  # noqa: F401  (3.9)

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LAYERING = ROOT / "docs" / "plan" / "layering.toml"
USE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+crate::(\w+)", re.M)


def load_toml(text: str) -> dict:
    """The subset layering.toml uses: [tables] (dotted), `key = [...]`
    string lists (one or many lines) and comments (Python 3.9 has no
    tomllib, and CI and local runs must agree)."""
    root: dict = {}
    table = root
    pending: tuple[str, list[str]] | None = None
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if pending is not None:
            key, items = pending
            items += re.findall(r'"([^"]*)"', line)
            if "]" in line:
                table[key] = items
                pending = None
            continue
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            table = root
            for part in line[1:-1].split("."):
                table = table.setdefault(part.strip(), {})
            continue
        key, _, value = (piece.strip() for piece in line.partition("="))
        items = re.findall(r'"([^"]*)"', value)
        if value.startswith("[") and "]" not in value:
            pending = (key, items)
        else:
            table[key] = items
    return root


def workspace_edges(root: Path) -> dict[str, set[str]]:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root, capture_output=True, text=True, check=True,
    ).stdout
    packages = json.loads(out)["packages"]
    names = {package["name"] for package in packages}
    return {
        package["name"]: {
            dep["name"] for dep in package["dependencies"]
            if dep["name"] in names and dep.get("kind") is None
        }
        for package in packages
    }


def check_edges(edges: dict[str, set[str]], allow: dict[str, list[str]]) -> list[str]:
    problems = []
    for crate, deps in sorted(edges.items()):
        allowed = set(allow.get(crate, []))
        if crate not in allow:
            problems.append(f"{crate}: not in layering.toml — add it with its allowed dependencies")
        for dep in sorted(deps - allowed):
            problems.append(f"{crate} -> {dep}: not an allowed edge (ENGINEERING-GUIDE A1); add it to layering.toml only with a reason")
        for dep in sorted(allowed - deps):
            problems.append(f"{crate} -> {dep}: listed but gone — remove it from layering.toml")
    return problems


def sibling_imports(src: Path, families: dict[str, list[str]]) -> set[str]:
    family_of = {module: family for family, modules in families.items() for module in modules}
    found = set()
    for path in sorted(src.glob("*.rs")):
        module = path.stem
        mine = family_of.get(module)
        if mine is None:
            continue
        for used in USE.findall(path.read_text(encoding="utf-8")):
            theirs = family_of.get(used)
            if theirs is not None and theirs != mine:
                found.add(f"{module} -> {used}")
    return found


def check_siblings(found: set[str], allowed: list[str]) -> list[str]:
    problems = []
    for edge in sorted(found - set(allowed)):
        problems.append(f"juggler-adapters: {edge} imports a sibling tool's module (A6) — move the shared piece to a neutral module")
    for edge in sorted(set(allowed) - found):
        problems.append(f"juggler-adapters: {edge} is gone — remove it from allow_sibling (it only shrinks)")
    return problems


def run(root: Path, layering: Path, edges: dict[str, set[str]] | None = None) -> list[str]:
    config = load_toml(layering.read_text(encoding="utf-8"))
    problems = check_edges(edges if edges is not None else workspace_edges(root), config.get("allow", {}))
    adapters = config.get("adapters", {})
    problems += check_siblings(
        sibling_imports(root / "crates" / "juggler-adapters" / "src", adapters.get("families", {})),
        adapters.get("allow_sibling", []),
    )
    return problems


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "crates" / "juggler-adapters" / "src"
        src.mkdir(parents=True)
        (src / "acme.rs").write_text("use crate::beta::Thing;\nuse crate::transcript::parse;\n")
        (src / "beta.rs").write_text("pub struct Thing;\n")
        layering = root / "layering.toml"
        base = '[allow]\ncore = []\napp = ["core"]\n[adapters.families]\nacme = ["acme"]\nbeta = ["beta"]\n'
        edges = {"core": set(), "app": {"core"}}
        cases = [
            (base + '[adapters]\nallow_sibling = ["acme -> beta"]\n', edges, 0, "allowed graph and listed sibling pass"),
            (base + "[adapters]\nallow_sibling = []\n", edges, 1, "a new sibling import fails"),
            (base + '[adapters]\nallow_sibling = ["acme -> beta", "beta -> acme"]\n', edges, 1, "a gone sibling must leave the list"),
            (base + '[adapters]\nallow_sibling = ["acme -> beta"]\n', {"core": {"app"}, "app": {"core"}}, 1, "an inward-pointing edge fails"),
            (base + '[adapters]\nallow_sibling = ["acme -> beta"]\n', {"core": set(), "app": set()}, 1, "a listed edge that's gone fails"),
        ]
        bad = 0
        for text, case_edges, want, what in cases:
            layering.write_text(text)
            got = len(run(root, layering, case_edges))
            if got != want:
                print(f"layering selftest FAILED: {what}: {got} problem(s), wanted {want}")
                bad += 1
    if bad:
        return 1
    print("layering selftest: OK")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    problems = run(ROOT, LAYERING)
    for problem in problems:
        print(f"layering: {problem}")
    if problems:
        return 1
    print("layering: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
