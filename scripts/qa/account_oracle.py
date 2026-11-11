#!/usr/bin/env python3
"""L1 reference model for Accounts and Apps (AJ-E3302; the validation plan, §3).

Written from the rules, not from the code:

- D-62: an Account is a sign-in. Every folder (an App's) signed in as the
  same person, on the same tool, is one Account; a different person is
  another.
- D-35: one login can be a personal plan and a team seat: the same person in
  a team is a separate Account.
- A sign-in that can't be read is its own Account (never merged: nothing
  says whose it is).
- Folders not signed in gather under one "Not signed in" per tool.
- Order: Accounts signed in first (by their first folder), then the ones
  that can't be read, then the ones not signed in.
- AJ-E3202 req. 4: an Account's plan, renewal and fee are the one value its
  folders agree on; when two folders say different things, that term
  "differs" (the user chooses once), and a folder that says nothing doesn't
  disagree.

The case space: three folders, each on a tool (Claude Code or Codex), each
signed in as A, as A in a team, as B, unreadable, or not signed in, each
with a plan of none, Pro or Max: (2 x 5 x 3)^3 = 27 000 cases.

Usage:
  account_oracle.py group        the table: "<case>\\t<expected>" per line
  account_oracle.py --selftest   hand-checked cases
"""
from __future__ import annotations

import itertools
import sys

TOOLS = ("claude-code", "codex")
PROBES = ("in-a", "in-a-team", "in-b", "unreadable", "out")
PLANS = ("none", "pro", "max")


def case_key(folders: tuple[tuple[str, str, str], ...]) -> str:
    return ";".join(f"{tool}/{probe}/{plan}" for tool, probe, plan in folders)


def expected(folders: tuple[tuple[str, str, str], ...]) -> str:
    """The Accounts, in order: state, its folders (p1..p3), its plan."""
    accounts: list[dict] = []
    by_who: dict[tuple, dict] = {}
    for at, (tool, probe, plan) in enumerate(folders, 1):
        if probe.startswith("in-"):
            person = "b" if probe == "in-b" else "a"
            team = probe.endswith("team")
            who = ("in", tool, person, team)
            rank = 0
        elif probe == "unreadable":
            who = ("unreadable", at)
            rank = 1
        else:
            who = ("out", tool)
            rank = 2
        if who not in by_who:
            by_who[who] = {"rank": rank, "state": {0: "signed_in", 1: "unverified", 2: "signed_out"}[rank], "folders": [], "plans": []}
            accounts.append(by_who[who])
        by_who[who]["folders"].append(f"p{at}")
        by_who[who]["plans"].append(plan)
    accounts.sort(key=lambda account: account["rank"])  # stable: first folder's order within a rank
    words = []
    for account in accounts:
        said = sorted({plan for plan in account["plans"] if plan != "none"})
        plan = "differs" if len(said) > 1 else (said[0] if said else "none")
        words.append(f"{account['state']}:{'+'.join(account['folders'])}:{plan}")
    return " | ".join(words)


def table_group():
    one = list(itertools.product(TOOLS, PROBES, PLANS))
    for folders in itertools.product(one, repeat=3):
        yield case_key(folders), expected(folders)


def selftest() -> int:
    checks = {
        # Two folders as the same person: one Account with two Apps.
        (("claude-code", "in-a", "max"), ("claude-code", "in-a", "max"), ("codex", "out", "none")):
            "signed_in:p1+p2:max | signed_out:p3:none",
        # The same person on a team seat is another Account (D-35).
        (("claude-code", "in-a", "pro"), ("claude-code", "in-a-team", "max"), ("claude-code", "in-b", "none")):
            "signed_in:p1:pro | signed_in:p2:max | signed_in:p3:none",
        # The same person on another tool is another Account.
        (("claude-code", "in-a", "none"), ("codex", "in-a", "none"), ("codex", "in-a", "pro")):
            "signed_in:p1:none | signed_in:p2+p3:pro",
        # Unreadable ones never merge; not signed in gathers per tool; signed in first.
        (("claude-code", "out", "none"), ("claude-code", "unreadable", "pro"), ("claude-code", "unreadable", "pro")):
            "unverified:p2:pro | unverified:p3:pro | signed_out:p1:none",
        # Plans that disagree: it differs; one that says nothing doesn't disagree.
        (("codex", "in-b", "pro"), ("codex", "in-b", "max"), ("codex", "in-b", "none")):
            "signed_in:p1+p2+p3:differs",
        (("codex", "in-b", "none"), ("codex", "in-b", "max"), ("codex", "out", "pro")):
            "signed_in:p1+p2:max | signed_out:p3:pro",
    }
    failed = [(case, want, expected(case)) for case, want in checks.items() if expected(case) != want]
    for case, want, got in failed:
        print(f"selftest: {case_key(case)}: got {got!r}, want {want!r}", file=sys.stderr)
    size = sum(1 for _ in table_group())
    print(f"account_oracle selftest: {len(checks) - len(failed)}/{len(checks)} ok; group table {size} cases")
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    if argv[1:] == ["--selftest"]:
        return selftest()
    if argv[1:] != ["group"]:
        print(__doc__)
        return 2
    out = sys.stdout
    for key, want in table_group():
        out.write(f"{key}\t{want}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
