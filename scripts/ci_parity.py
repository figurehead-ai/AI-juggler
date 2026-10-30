#!/usr/bin/env python3
"""The local gate runs everything CI's verify job runs.

`just verify` (and so `just preflight`) is meant to be CI's verify job on
this Mac; the two drifted once (CI ran `cargo deny check`, the justfile
didn't) and a push found out. This reads the `run:` commands of the
verify job in .github/workflows/ci.yml, expands the justfile's `verify`
recipe (its `just <recipe>` lines included), and fails naming each CI
command the local gate doesn't run. Steps that only set the runner up
(installs, disk, coverage) are listed in RUNNER_ONLY.

Usage: scripts/ci_parity.py          (fails listing each missing command)
       scripts/ci_parity.py --selftest
"""
from __future__ import annotations

import re
import shlex
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Commands that prepare the runner or make the coverage artifact; nothing
# a Mac's local gate needs to repeat.
RUNNER_ONLY = (
    "df -h",
    "free=",
    "sudo ",
    "git config",
    "mkdir -p",
    "curl ",
    "echo ",
    "tar ",
    "mv ",
    "deny=",
    "rustup component add",
    "rm -rf target",
    "cargo llvm-cov",
    "set -euo pipefail",
    "if ",
    "fi",
    "then",
    "else",
)


def job_runs(workflow: str, job: str) -> list[str]:
    """The shell lines of every `run:` in one job (a plain indent reader:
    the workflow is ours and keeps GitHub's usual layout)."""
    lines = workflow.splitlines()
    out: list[str] = []
    in_job = False
    block_indent = None
    for line in lines:
        if re.match(r"^  [A-Za-z0-9_-]+:\s*$", line):
            in_job = line.strip() == f"{job}:"
            block_indent = None
            continue
        if not in_job:
            continue
        if block_indent is not None:
            indent = len(line) - len(line.lstrip())
            if line.strip() and indent < block_indent:
                block_indent = None
            elif line.strip():
                # A line ending in "\" goes on in the next.
                if out and out[-1].endswith("\\"):
                    out[-1] = out[-1][:-1].rstrip() + " " + line.strip()
                else:
                    out.append(line.strip())
                continue
            else:
                continue
        m = re.match(r"^(\s*)(?:- )?run:\s*(.*)$", line)
        if m:
            rest = m.group(2).strip()
            if rest in ("|", ">"):
                block_indent = len(m.group(1)) + 2
            elif rest:
                out.append(rest)
    return out


def recipe(justfile: str, name: str, seen: set[str] | None = None) -> list[str]:
    """A recipe's commands, `just <other>` lines expanded."""
    seen = seen or set()
    if name in seen:
        return []
    seen.add(name)
    lines = justfile.splitlines()
    out: list[str] = []
    for i, line in enumerate(lines):
        if re.match(rf"^{re.escape(name)}\s*:", line):
            for body in lines[i + 1 :]:
                if not body.startswith((" ", "\t")):
                    break
                cmd = body.strip()
                if not cmd or cmd.startswith("#"):
                    continue
                sub = re.match(r"^just\s+([A-Za-z0-9_-]+)\s*$", cmd)
                out.extend(recipe(justfile, sub.group(1), seen) if sub else [cmd])
            break
    return out


def normal(cmd: str) -> str:
    """A command without leading VAR=value words, `env`, or flags that
    only shape the output, with single spaces."""
    try:
        words = shlex.split(cmd)
    except ValueError:
        words = cmd.split()
    while words and (words[0] == "env" or re.match(r"^[A-Z_][A-Z0-9_]*=", words[0])):
        words.pop(0)
    words = [w for w in words if w not in ("--no-fail-fast", "-q")]
    return " ".join(words)


def missing(workflow: str, justfile: str) -> list[str]:
    local = {normal(c) for c in recipe(justfile, "verify")}
    out = []
    for cmd in job_runs(workflow, "verify"):
        if cmd.startswith(RUNNER_ONLY) or cmd.startswith("#"):
            continue
        if normal(cmd) not in local:
            out.append(cmd)
    return out


def selftest() -> None:
    workflow = """jobs:
  verify:
    steps:
      - name: Format
        run: cargo fmt --all --check
      - name: Tests
        run: |
          cargo test --workspace --no-fail-fast
          ./scripts/leaked_fixtures.sh
      - name: Install
        run: |
          sudo apt-get install -y \\
            libgtk-3-dev
      - name: Docs
        env:
          RUSTDOCFLAGS: -D warnings
        run: cargo doc --workspace --no-deps
      - name: Deny
        run: cargo deny check
      - name: Disk
        run: df -h /
  ui:
    steps:
      - run: pnpm test
"""
    justfile = """verify:
    cargo fmt --all --check
    cargo test --workspace
    ./scripts/leaked_fixtures.sh
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    just lints

lints:
    echo hi
"""
    assert job_runs(workflow, "verify")[1:3] == ["cargo test --workspace --no-fail-fast", "./scripts/leaked_fixtures.sh"]
    assert "sudo apt-get install -y libgtk-3-dev" in job_runs(workflow, "verify"), "continued lines join"
    assert "pnpm test" not in job_runs(workflow, "verify"), "another job's runs are not the verify job's"
    assert recipe(justfile, "verify")[-1] == "echo hi", "just <recipe> lines expand"
    assert missing(workflow, justfile) == ["cargo deny check"], missing(workflow, justfile)
    assert missing(workflow, justfile.replace("    just lints", "    cargo deny check")) == []
    print("ci-parity selftest: OK")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    justfile = (ROOT / "justfile").read_text(encoding="utf-8")
    gaps = missing(workflow, justfile)
    for cmd in gaps:
        print(f"ci-parity: CI's verify job runs `{cmd}`; `just verify` doesn't")
    if gaps:
        print("ci-parity: add each to the justfile's verify recipe (or to RUNNER_ONLY if it only prepares the runner)")
        return 1
    print(f"ci-parity: OK (`just verify` runs every command of CI's verify job)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
