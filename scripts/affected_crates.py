#!/usr/bin/env python3
"""The workspace packages whose tests a change can affect (ADR-0024).

Preflight's fast mode runs only these packages' tests; GitHub runs every
test on every pull request. From the files changed between BASE and HEAD:

- a change to the build as a whole (the root manifest, the lockfile, the
  toolchain, `.cargo/`, nextest's settings) affects every package: ALL;
- a file under `crates/<dir>/` affects that package;
- a file outside `crates/` that tests read (READ_BY below: the UI's
  sources, the workflows, a few scripts and fixtures) affects the
  packages whose tests read it;
- then every package that depends on an affected one, dev-dependencies
  included, is affected too (its tests build against it).

Anything else (documentation, the justfile, other scripts) affects no
package's tests. Prints one package per line, `ALL`, or nothing.

Usage: scripts/affected_crates.py BASE [HEAD]   (HEAD defaults to HEAD)
       scripts/affected_crates.py --selftest
"""
from __future__ import annotations

import fnmatch
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Changes that rebuild everything.
EVERYTHING = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/*",
    ".config/nextest.toml",
)

# Files outside crates/ that tests read, and the packages whose tests read
# them (`grep -rn '\.\./\.\./' crates` finds them; keep this in step).
READ_BY: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("ui/*", ("juggler-app",)),
    ("docs/qa/*", ("juggler-app", "juggler-handover")),
    (".github/workflows/*", ("juggler-app",)),
    (".gitignore", ("juggler-app",)),
    ("scripts/sign_macos.sh", ("juggler-app",)),
    ("scripts/replica.py", ("juggler-adapters",)),
    ("scripts/gen_stats_events.py", ("juggler-stats",)),
    ("stats/*", ("juggler-stats",)),
    ("plugins/*", ("juggler-agentbridge",)),
)


def affected(changed: list[str], packages: dict[str, tuple[str, list[str]]]) -> list[str] | None:
    """`packages`: name -> (its folder relative to the root, the workspace
    packages it depends on). None means every package."""
    hit: set[str] = set()
    for path in changed:
        if any(fnmatch.fnmatch(path, pattern) for pattern in EVERYTHING):
            return None
        for name, (folder, _) in packages.items():
            if path.startswith(folder.rstrip("/") + "/"):
                hit.add(name)
        for pattern, readers in READ_BY:
            if fnmatch.fnmatch(path, pattern):
                hit.update(r for r in readers if r in packages)
    # Every package that depends on an affected one, until nothing new.
    grew = True
    while grew:
        grew = False
        for name, (_, deps) in packages.items():
            if name not in hit and hit.intersection(deps):
                hit.add(name)
                grew = True
    return sorted(hit)


def workspace() -> dict[str, tuple[str, list[str]]]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    meta = json.loads(out)
    root = Path(meta["workspace_root"])
    packages = {}
    for package in meta["packages"]:
        folder = Path(package["manifest_path"]).parent.relative_to(root).as_posix()
        deps = sorted({d["name"] for d in package["dependencies"] if d.get("path")})
        packages[package["name"]] = (folder, deps)
    return packages


def changed_files(base: str, head: str) -> list[str]:
    merge_base = subprocess.run(
        ["git", "merge-base", base, head], cwd=ROOT, capture_output=True, text=True,
    ).stdout.strip()
    if not merge_base:
        return ["Cargo.toml"]  # no common history: treat as a change to everything
    out = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", merge_base, head],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def selftest() -> None:
    packages = {
        "core": ("crates/core", []),
        "adapters": ("crates/adapters", ["core"]),
        "app": ("crates/app", ["adapters", "core", "bridge"]),
        "bridge": ("crates/bridge", ["app", "core"]),  # a dev-dependency cycle
        "stats": ("crates/stats", []),
        "juggler-app": ("crates/juggler-app", []),
    }
    assert affected(["Cargo.lock"], packages) is None
    assert affected(["crates/core/src/lib.rs"], packages) == ["adapters", "app", "bridge", "core"]
    assert affected(["crates/stats/tests/a.rs"], packages) == ["stats"]
    assert affected(["crates/app/src/main.rs"], packages) == ["app", "bridge"], "the cycle ends"
    assert affected(["docs/RELEASING.md", "justfile", "scripts/visual_gate.sh"], packages) == []
    assert affected(["ui/src/App.tsx"], packages) == ["juggler-app"]
    assert affected([".config/nextest.toml"], packages) is None
    assert affected(["crates/core2/src/lib.rs"], packages) == [], "a folder name is matched whole"
    print("affected-crates selftest: OK")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    if len(sys.argv) < 2:
        print(__doc__.strip().splitlines()[-2], file=sys.stderr)
        return 2
    base = sys.argv[1]
    head = sys.argv[2] if len(sys.argv) > 2 else "HEAD"
    result = affected(changed_files(base, head), workspace())
    if result is None:
        print("ALL")
    else:
        print("\n".join(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
