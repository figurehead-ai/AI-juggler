#!/usr/bin/env python3
"""AJ-E803: a PR that changes visual baselines must explain each one.

The PR body needs a "## Visual baselines" section naming the task ID
(AJ-E…) and, for every changed baseline, a line that names it (the file
name, without ".png") with a note on the intentional change. A bulk
"updated all baselines" without per-screen notes fails.

Usage: PR_BODY="$body" scripts/check_baseline_notes.py BASE_REF
"""
import os
import re
import subprocess
import sys

BASELINES = "ui/tests/visual/__baselines__/"


def changed(base: str) -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--name-only", f"{base}...HEAD", "--", BASELINES],
        capture_output=True, text=True, check=True,
    ).stdout
    return [line for line in out.splitlines() if line.endswith(".png")]


def main() -> int:
    base = sys.argv[1] if len(sys.argv) > 1 else "origin/main"
    files = changed(base)
    if not files:
        print("baseline-notes: no baseline changed")
        return 0
    body = os.environ.get("PR_BODY", "")
    match = re.search(r"^##\s*Visual baselines\s*$(.*?)(?=^##\s|\Z)", body, re.M | re.S)
    if not match:
        print(f"baseline-notes: {len(files)} baseline(s) changed but the PR has no '## Visual baselines' section")
        return 1
    section = match.group(1)
    problems = []
    if not re.search(r"\bAJ-E\d{3,4}\b", section):
        problems.append("the section names no task ID (AJ-E…)")
    for path in files:
        name = os.path.basename(path)[: -len(".png")]
        line = next((l for l in section.splitlines() if name in l), None)
        if line is None:
            problems.append(f"no note for {name}")
        elif len(line.replace(name, "").strip(" -*:`|")) < 12:
            problems.append(f"the note for {name} doesn't say what changed")
    if problems:
        print("baseline-notes: " + "; ".join(problems))
        return 1
    print(f"baseline-notes: {len(files)} baseline change(s), each explained")
    return 0


if __name__ == "__main__":
    sys.exit(main())
