#!/usr/bin/env python3
"""The local gate runs everything CI's verify runs.

`just verify` is meant to be CI's verify on this Mac; the two drifted once
(CI ran `cargo deny check`, the justfile didn't) and a push found out.
CI's verify is split across jobs (ADR-0025): VERIFY_JOBS, which the
`verify` job needs, so the required check stays one name. This reads the
`run:` commands of those jobs in .github/workflows/ci.yml, expands the
justfile's `verify` recipe (its `just <recipe>` lines included), and fails
naming each CI command the local gate doesn't run, and each of those jobs
`verify` doesn't need. Steps that only set the runner up (installs, disk,
coverage) are listed in RUNNER_ONLY; CI's nextest archive and its
partitions are, together, `cargo nextest run --workspace` (canonical()).

Usage: scripts/ci_parity.py          (fails listing each missing command)
       scripts/ci_parity.py --selftest
"""
from __future__ import annotations

import re
import shlex
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The jobs that make up CI's verify (the `verify` job needs each).
VERIFY_JOBS = ("static", "static-rust", "rust-build", "rust-test", "dev-bridge")

# What preflight.sh runs itself, before `just verify-static`.
PREFLIGHT_ONLY = ("./scripts/release_notes_changed.sh",)

# Commands that prepare the runner or make the coverage artifact; nothing
# a Mac's local gate needs to repeat.
RUNNER_ONLY = (
    # A release pull request's scope report (D-99: report only), for the
    # pull request's own branch; `just verify` reports 0.4 (release-selftest).
    'python3 scripts/release_scope.py report "${HEAD_REF',
    # Installs a pinned tool on the runner (AJ-E2007); locally it's
    # installed once by hand.
    "./scripts/install_release_tool.sh",
    # The runner's system packages, bounded and retried; locally the
    # Mac needs none of them.
    "./scripts/ci/apt_install.sh",
    # The RC's version stamp (it reads the pull request's branch).
    "./scripts/ci/rc_version.sh",
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
    "cargo fetch",
    "rm -rf target",
    "find target",
    "du -sh",
    "for ",
    "done",
    "base=",
    "test ",
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


def canonical(cmd: str) -> str:
    """normal(), and CI's build-once nextest (`cargo nextest archive`, then
    `nextest run --archive-file ... --partition ...` on each runner) as the
    one local command they stand for."""
    words = normal(cmd).split()
    if words[:3] == ["cargo", "nextest", "archive"] or (
        words[1:3] == ["nextest", "run"] and words[0] in ("cargo", "cargo-nextest") and "--archive-file" in words
    ):
        return "cargo nextest run --workspace"
    return " ".join(words)


def job_needs(workflow: str, job: str) -> list[str]:
    """A job's `needs:` (the one-line list form)."""
    in_job = False
    for line in workflow.splitlines():
        if re.match(r"^  [A-Za-z0-9_-]+:\s*$", line):
            in_job = line.strip() == f"{job}:"
            continue
        m = re.match(r"^    needs:\s*\[(.*)\]\s*$", line) if in_job else None
        if m:
            return [n.strip() for n in m.group(1).split(",") if n.strip()]
    return []


def missing(workflow: str, justfile: str) -> list[str]:
    local = {canonical(c) for c in recipe(justfile, "verify")}
    out = []
    for job in VERIFY_JOBS:
        for cmd in job_runs(workflow, job):
            if cmd.startswith(RUNNER_ONLY + PREFLIGHT_ONLY) or cmd.startswith("#"):
                continue
            if canonical(cmd) not in local:
                out.append(cmd)
    needs = job_needs(workflow, "verify")
    out.extend(f"(the verify job doesn't need `{job}`)" for job in VERIFY_JOBS if job not in needs)
    return out


def selftest() -> None:
    workflow = """jobs:
  static:
    steps:
      - name: Format
        run: cargo fmt --all --check
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
      - name: Notes
        run: ./scripts/release_notes_changed.sh "origin/$BASE_REF"
  static-rust:
    steps:
      - run: cargo fmt --all --check
  rust-build:
    steps:
      - run: cargo nextest archive --workspace --archive-file "$RUNNER_TEMP/n.tar.zst"
      - run: cargo test --workspace --doc
  rust-test:
    steps:
      - name: Tests
        run: |
          cargo-nextest nextest run --archive-file "$RUNNER_TEMP/n.tar.zst" --workspace-remap . --partition hash:1/4 --no-fail-fast
          ./scripts/leaked_fixtures.sh
      - name: Disk
        run: df -h /
  dev-bridge:
    steps:
      - run: ./scripts/dev_bridge_check.sh
  verify:
    needs: [static, static-rust, rust-build, rust-test, dev-bridge]
    steps:
      - run: echo ok
  ui:
    steps:
      - run: pnpm test
"""
    justfile = """verify:
    just verify-static
    cargo nextest run --workspace --no-fail-fast
    cargo test --workspace --doc
    ./scripts/leaked_fixtures.sh
    ./scripts/dev_bridge_check.sh

verify-static:
    cargo fmt --all --check
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    just lints

lints:
    echo hi
"""
    assert job_runs(workflow, "rust-test")[:2] == [
        'cargo-nextest nextest run --archive-file "$RUNNER_TEMP/n.tar.zst" --workspace-remap . --partition hash:1/4 --no-fail-fast',
        "./scripts/leaked_fixtures.sh",
    ]
    assert "sudo apt-get install -y libgtk-3-dev" in job_runs(workflow, "static"), "continued lines join"
    assert "pnpm test" not in job_runs(workflow, "static"), "another job's runs are not this job's"
    assert recipe(justfile, "verify")[2] == "echo hi", "just <recipe> lines expand"
    assert canonical('cargo nextest archive --workspace --archive-file "x"') == "cargo nextest run --workspace"
    assert canonical("cargo-nextest nextest run --archive-file x --partition hash:2/4") == "cargo nextest run --workspace"
    assert canonical("cargo nextest run --workspace --no-fail-fast") == "cargo nextest run --workspace"
    assert job_needs(workflow, "verify") == ["static", "static-rust", "rust-build", "rust-test", "dev-bridge"]
    assert missing(workflow, justfile) == ["cargo deny check"], missing(workflow, justfile)
    assert missing(workflow, justfile.replace("    just lints", "    cargo deny check")) == []
    unneeded = workflow.replace("needs: [static, static-rust, rust-build, rust-test, dev-bridge]", "needs: [static, static-rust, rust-build, rust-test]")
    assert missing(unneeded, justfile.replace("    just lints", "    cargo deny check")) == [
        "(the verify job doesn't need `dev-bridge`)"
    ]
    print("ci-parity selftest: OK")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    justfile = (ROOT / "justfile").read_text(encoding="utf-8")
    gaps = missing(workflow, justfile)
    for cmd in gaps:
        print(f"ci-parity: CI's verify runs `{cmd}`; `just verify` doesn't" if not cmd.startswith("(") else f"ci-parity: {cmd}")
    if gaps:
        print("ci-parity: add each to the justfile's verify recipes (or to RUNNER_ONLY if it only prepares the runner)")
        return 1
    print(f"ci-parity: OK (`just verify` runs every command of CI's verify jobs: {', '.join(VERIFY_JOBS)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
