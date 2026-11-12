#!/usr/bin/env python3
"""Which of CI's jobs a change needs (AJ-E2020 R2, D-99 point 2).

CI's `plan` job runs this and every filtered job asks it by id (`if:
... contains(fromJSON(needs.plan.outputs.jobs || '[]'), '<job id>')`), so there is
no workflow-level path filter: every job reports, and `verify`, the one
aggregate check, accepts a skip only for a job this left out. From the
files a pull request changes against its base:

- **Unknown means everything.** Only the paths in NARROW may narrow the
  run; any other change (`.github/`, `scripts/`, the build as a whole:
  the toolchain, `Cargo.lock`, `Cargo.toml`, `.cargo/`, `.config/`, the
  justfile, `fuzz/`, the dependency policy) runs every job.
- **Rust sources** (`crates/`) build: the Rust static checks, the three
  platform builds, the doctests and dev-bridge, coverage and the release
  candidate.
- **Tests** run for the packages the change can affect, dependents
  included (`scripts/affected_crates.py`, shared with the local preflight:
  a file outside `crates/` that tests read, such as `ui/` or `docs/qa/`,
  counts). The archive is still built for the whole workspace (the same
  binaries and cache as a full run); only the slices are filtered.
- **The interface** (`ui/` and the files its tests read: UI_READS) runs
  `ui`, the visual and accessibility gates, and the release candidate.
- **Documentation** alone runs only `static`'s lints.

`--full` (every push, the weekly run, release pull requests: the
workflow's FULL) runs every job, whatever changed.

Prints GitHub output lines: `jobs` (a JSON list of job ids), `tests` (a
nextest filterset for the slices; empty for every test), and `summary`.

Usage: scripts/ci_scope.py --full
       scripts/ci_scope.py BASE [HEAD]    (the files `git diff BASE HEAD` names)
       scripts/ci_scope.py --check [WORKFLOW]   (the filtered jobs of ci.yml are JOBS)
       scripts/ci_scope.py --selftest
"""
from __future__ import annotations

import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_crates  # noqa: E402  (the one map of what tests read)

ROOT = Path(__file__).resolve().parent.parent

# The paths that may narrow a run; anything else runs every job.
NARROW = (
    "crates/*",
    "ui/*",
    "docs/*",
    "stats/*",
    "plugins/*",
    "*.md",
    "LICENSE",
)
# Narrowing never applies under these, even to a Markdown file.
NEVER_NARROW = (".github/*",)

# Files outside ui/ that the interface's tests read (`grep -rn "\.\./"
# ui/tests`; keep this in step).
UI_READS = (
    "ui/*",
    "docs/qa/*",
    "crates/juggler-core/src/storage/entities.rs",
    "crates/juggler-app/tauri.conf.json",
)

# The Floor's golden frames: the package whose tests draw them.
FLOOR = "juggler-floor-render"

# Each filtered job of ci.yml and what runs it. The jobs not named here
# (plan, static, and the aggregates verify, visual, a11y, full-run) always
# run. `--check` holds this and the workflow's `if:`s in step.
JOBS = {
    "static-rust": "rust",
    "rust-build": "tests",
    "rust-test": "tests",
    "dev-bridge": "rust",
    "coverage": "rust",
    "floor-gallery": "floor",
    "ui": "ui",
    "visual-shard": "ui",
    "a11y-shard": "ui",
    "build-macos": "rust",
    "build-linux": "rust",
    "build-windows": "rust",
    "rc-dev-app": "app",
}


def matches(path: str, patterns: tuple[str, ...]) -> bool:
    return any(fnmatch.fnmatch(path, p) for p in patterns)


EVERYTHING = {"everything": True, "rust": True, "tests": None, "ui": True}


def scope(changed: list[str], packages: dict[str, tuple[str, list[str]]] | None) -> dict:
    """The areas a change touches. `packages` None (the workspace couldn't
    be read) runs everything."""
    everything = packages is None or any(
        matches(p, NEVER_NARROW) or not matches(p, NARROW) for p in changed
    )
    if everything:
        return dict(EVERYTHING)
    tests = affected_crates.affected(changed, packages)
    return {
        "everything": False,
        "rust": tests is None or any(p.startswith("crates/") for p in changed),
        "tests": tests,  # None: every package
        "ui": any(matches(p, UI_READS) for p in changed),
    }


def jobs(areas: dict) -> list[str]:
    on = {
        "rust": areas["rust"],
        "tests": areas["tests"] is None or bool(areas["tests"]),
        "floor": areas["tests"] is None or FLOOR in areas["tests"],
        "ui": areas["ui"],
        "app": areas["rust"] or areas["ui"],
    }
    return [job for job, area in JOBS.items() if on[area]]


def filterset(areas: dict) -> str:
    """The slices' nextest filter: empty for every test."""
    if areas["tests"] is None:
        return ""
    return " | ".join(f"package(={name})" for name in areas["tests"])


def summary(areas: dict, run: list[str]) -> str:
    if areas["everything"]:
        return "Every job runs (a full run, or a change outside the areas that narrow it)."
    tests = "every package" if areas["tests"] is None else (", ".join(areas["tests"]) or "none")
    skipped = [job for job in JOBS if job not in run]
    return f"Jobs: {', '.join(run) or 'only the lints'}. Tests: {tests}. Left out: {', '.join(skipped) or 'none'}."


def outputs(areas: dict) -> str:
    run = jobs(areas)
    return "\n".join(
        [f"jobs={json.dumps(run, separators=(',', ':'))}", f"tests={filterset(areas)}", f"summary={summary(areas, run)}"]
    )


def changed_files(base: str, head: str) -> list[str] | None:
    """`git diff BASE HEAD`'s files; None when git can't say (everything
    runs). For a pull request HEAD is GitHub's merge of it into BASE, so
    the diff is exactly the pull request's change."""
    result = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", base, head], cwd=ROOT, capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"ci-scope: git diff failed ({result.stderr.strip()}): every job runs", file=sys.stderr)
        return None
    return [line for line in result.stdout.splitlines() if line]


def workspace() -> dict[str, tuple[str, list[str]]] | None:
    try:
        return affected_crates.workspace()
    except (OSError, subprocess.CalledProcessError, ValueError) as err:
        print(f"ci-scope: the workspace couldn't be read ({err}): every job runs", file=sys.stderr)
        return None


GUARD = re.compile(r"contains\(fromJSON\(needs\.plan\.outputs\.jobs \|\| '\[\]'\),\s*'([A-Za-z0-9_-]+)'\)")


def check(workflow: str) -> list[str]:
    """Every filtered job asks for its own id, and JOBS names exactly the
    filtered jobs (a job missing from JOBS would never run)."""
    problems = []
    guarded: dict[str, str] = {}
    job = None
    for line in workflow.splitlines():
        m = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if m:
            job = m.group(1)
            continue
        if job and re.match(r"^    if:", line):
            for asked in GUARD.findall(line):
                if asked != job:
                    problems.append(f"job `{job}` runs when plan lists `{asked}`: a job asks for its own id")
                guarded[job] = asked
    for name in JOBS:
        if name not in guarded:
            problems.append(f"JOBS names `{name}`, but no job of that id asks plan for it")
    for name in guarded:
        if name not in JOBS:
            problems.append(f"job `{name}` asks plan for itself, but JOBS doesn't name it: it would never run")
    return problems


def selftest() -> None:
    packages = {
        "juggler-core": ("crates/juggler-core", []),
        "juggler-app": ("crates/juggler-app", ["juggler-core"]),
        "juggler-floor-render": ("crates/juggler-floor-render", ["juggler-core"]),
        "juggler-stats": ("crates/juggler-stats", []),
    }
    every = list(JOBS)

    # A docs-only change: only static's lints (no filtered job).
    docs = scope(["docs/RELEASING.md", "README.md", "docs/tasks/E20.md"], packages)
    assert jobs(docs) == [], jobs(docs)
    # A UI-only change: the UI gates, the release candidate, and the tests
    # of the package that reads ui/ (juggler-app); no Rust build or lint.
    ui = scope(["ui/src/App.tsx"], packages)
    assert jobs(ui) == ["rust-build", "rust-test", "ui", "visual-shard", "a11y-shard", "rc-dev-app"], jobs(ui)
    assert filterset(ui) == "package(=juggler-app)", filterset(ui)
    # A Rust-only change: the Rust jobs, its package's tests and its
    # dependents'; no UI gate, no Floor gallery unless the Floor depends on it.
    stats = scope(["crates/juggler-stats/src/lib.rs"], packages)
    assert "ui" not in jobs(stats) and "visual-shard" not in jobs(stats) and "floor-gallery" not in jobs(stats)
    assert {"static-rust", "rust-build", "rust-test", "build-macos", "build-linux", "build-windows", "rc-dev-app"} <= set(jobs(stats))
    assert filterset(stats) == "package(=juggler-stats)"
    core = scope(["crates/juggler-core/src/lib.rs"], packages)
    assert "floor-gallery" in jobs(core) and "ui" not in jobs(core)
    assert filterset(core) == "package(=juggler-app) | package(=juggler-core) | package(=juggler-floor-render)"
    # A Rust file the interface's tests read runs the UI gates too.
    assert "ui" in jobs(scope(["crates/juggler-core/src/storage/entities.rs"], packages))
    # docs/qa is read by tests on both sides.
    qa = scope(["docs/qa/pairwise/canvas.csv"], packages)
    assert "ui" in jobs(qa) and "rust-build" in jobs(qa) and "build-macos" not in jobs(qa)
    # Everything: the workflows (Markdown under .github too), the build,
    # scripts, anything unknown, an unreadable workspace.
    for path in (".github/workflows/ci.yml", ".github/PULL_REQUEST_TEMPLATE.md", "Cargo.lock",
                 "rust-toolchain.toml", ".config/nextest.toml", "scripts/a11y_gate.sh", "justfile",
                 "fuzz/Cargo.toml", "deny.toml"):
        areas = scope(["docs/x.md", path], packages)
        assert jobs(areas) == every and filterset(areas) == "", path
    assert jobs(scope(["docs/x.md"], None)) == every
    # The build as a whole, inside the narrowing areas, still tests everything.
    assert filterset(scope(["Cargo.toml"], packages)) == ""
    # Outputs are one line each.
    out = outputs(docs).splitlines()
    assert out[0] == "jobs=[]" and out[1] == "tests=" and out[2].startswith("summary="), out

    workflow = """jobs:
  plan:
    runs-on: x
  rust-build:
    needs: plan
    if: needs.plan.outputs.tested == '' && contains(fromJSON(needs.plan.outputs.jobs || '[]'), 'rust-build')
  ui:
    needs: plan
    if: contains(fromJSON(needs.plan.outputs.jobs || '[]'), 'visual-shard')
  extra:
    if: contains(fromJSON(needs.plan.outputs.jobs || '[]'), 'extra')
"""
    problems = check(workflow)
    assert any("job `ui` runs when plan lists `visual-shard`" in p for p in problems), problems
    assert any("job `extra` asks plan for itself, but JOBS doesn't name it" in p for p in problems), problems
    assert any("JOBS names `rust-test`" in p for p in problems), problems
    assert not any("`rust-build`" in p for p in problems), problems
    print("ci-scope selftest: OK")


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        selftest()
        return 0
    if args[:1] == ["--check"]:
        path = Path(args[1]) if len(args) > 1 else ROOT / ".github/workflows/ci.yml"
        problems = check(path.read_text(encoding="utf-8"))
        for problem in problems:
            print(f"ci-scope: {problem}")
        if not problems:
            print(f"ci-scope: OK (the {len(JOBS)} filtered jobs of {path.name} are JOBS)")
        return 1 if problems else 0
    if args == ["--full"]:
        areas = dict(EVERYTHING)
    elif 1 <= len(args) <= 2 and not args[0].startswith("-"):
        changed = changed_files(args[0], args[1] if len(args) > 1 else "HEAD")
        areas = scope(changed, workspace()) if changed is not None else dict(EVERYTHING)
    else:
        print(__doc__.strip().split("Usage:")[1], file=sys.stderr)
        return 2
    print(outputs(areas))
    return 0


if __name__ == "__main__":
    sys.exit(main())
