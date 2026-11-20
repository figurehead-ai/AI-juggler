#!/usr/bin/env python3
"""D-24 channel requirements, as D-99, D-114 and D-138 amend them: whether
a version may be promoted.

  promotion_rules.py check beta|stable FACTS.json   exit 0 = may promote
  promotion_rules.py --selftest

promote.yml gathers the facts from GitHub and this decides, so the rules
live in one tested place:

- **Beta**: the build exists and no issue labelled `blocker` is open.
  Beta is a promotion of the tested nightly, with no rebuild (D-99, from
  0.4.1): the days of real use D-24 asked for before beta happen on the
  beta build itself, and the full hand-walk comes before Stable.
- **Stable**: the version has been on beta for at least STABLE_MIN_DAYS
  (7, D-114; D-24 said 14), and no `blocker` issue is open. (Stable is
  also a deliberate act: the maintainer runs and approves it, and the
  approval is the release's sign-off, D-109; promote.yml checks the
  CHANGELOG section and the roadmap scope.)
- **A blocker fix** (D-114) on a soaked base: a patch that fixes blocker
  issues (`blocker_fix`, each a closed issue labelled `blocker`) needs
  BLOCKER_FIX_MIN_DAYS (3) on its own beta tag, not the whole 7, and the
  Stable candidate it fixes (promote.yml's `candidate` input; its
  annotated beta tag's date is `line_beta_at`) at least STABLE_MIN_DAYS
  on beta, so the line's soak is never skipped.
- **Which version** (D-138): promotions are periodic, not per version. At
  each promotion point the latest version that's been tested long enough
  is the one promoted; the ones in between are skipped. These rules judge
  that one version.

A failed rule can be overridden only with a written reason, which is
printed with the result and kept in the channel tag's message.

FACTS.json: {"now": ISO, "built_at": ISO, "beta_at": ISO | null,
             "blockers": [{"number": N, "title": "…"}], "override": "…",
             "line_beta_at": ISO | null,
             "blocker_fix": [{"number": N, "blocker": bool, "closed": bool}]}
"""
from __future__ import annotations

import datetime as dt
import json
import sys
import tempfile
from pathlib import Path

STABLE_MIN_DAYS = 7
BLOCKER_FIX_MIN_DAYS = 3


def _when(raw: str | None) -> dt.datetime | None:
    if not raw:
        return None
    return dt.datetime.fromisoformat(raw.replace("Z", "+00:00"))


def _days(since: dt.datetime, now: dt.datetime) -> float:
    return (now - since).total_seconds() / 86_400


def failures(channel: str, facts: dict) -> list[str]:
    """Every unmet requirement, in plain words (empty: may promote)."""
    now = _when(facts["now"])
    assert now is not None, "facts need 'now'"
    problems: list[str] = []
    if channel == "beta":
        built = _when(facts.get("built_at"))
        if built is None:
            problems.append("no build date — is this an existing build?")
    elif channel == "stable":
        problems += _stable(facts, now)
    else:
        raise SystemExit(f"unknown channel {channel!r} (beta or stable)")
    blockers = facts.get("blockers") or []
    if blockers:
        listed = ", ".join(f"#{b['number']} {b['title']}" for b in blockers)
        problems.append(f"{len(blockers)} open blocker issue(s): {listed}")
    return problems


def _stable(facts: dict, now: dt.datetime) -> list[str]:
    """Stable's soak: 7 days on beta, or, for a blocker fix on a soaked
    base, 3 on its own tag with its line's first candidate 7 on beta."""
    on_beta = _when(facts.get("beta_at"))
    if on_beta is None:
        return ["no record of when it reached beta — promote it to beta first"]
    fixes = facts.get("blocker_fix") or []
    age = _days(on_beta, now)
    if not fixes:
        return [] if age >= STABLE_MIN_DAYS else [f"on beta for {age:.1f} days; stable needs {STABLE_MIN_DAYS}+"]
    problems = [
        f"#{fix['number']} isn't a closed issue labelled blocker; a blocker fix names only those"
        for fix in fixes
        if not (fix.get("blocker") and fix.get("closed"))
    ]
    if age < BLOCKER_FIX_MIN_DAYS:
        problems.append(f"this blocker fix is on beta for {age:.1f} days; it needs {BLOCKER_FIX_MIN_DAYS}+")
    line = _when(facts.get("line_beta_at"))
    if line is None:
        problems.append("no record of when its line's first candidate reached beta")
    elif (line_age := _days(line, now)) < STABLE_MIN_DAYS:
        problems.append(f"its line is on beta for {line_age:.1f} days; stable needs {STABLE_MIN_DAYS}+")
    return problems


def check(channel: str, facts: dict) -> tuple[bool, str]:
    """(may promote, the report to print)."""
    problems = failures(channel, facts)
    override = (facts.get("override") or "").strip()
    if not problems:
        return True, f"{channel}: every requirement met"
    lines = [f"{channel}: requirement not met — {p}" for p in problems]
    if override:
        lines.append(f"{channel}: OVERRIDDEN by the maintainer: {override}")
        return True, "\n".join(lines)
    lines.append(f"{channel}: not promoting (override only with a written reason)")
    return False, "\n".join(lines)


def selftest() -> int:
    now = "2026-10-30T12:00:00Z"
    ok, _ = check("beta", {"now": now, "built_at": "2026-10-26T12:00:00Z", "blockers": []})
    assert ok, "4 days old, no blocker"
    ok, _ = check("beta", {"now": now, "built_at": "2026-10-30T11:00:00Z", "blockers": []})
    assert ok, "an hour-old nightly may go to beta (D-99)"
    ok, report = check("beta", {"now": now, "built_at": None, "blockers": []})
    assert not ok and "existing build" in report, report
    ok, report = check(
        "beta",
        {"now": now, "built_at": "2026-10-20T00:00:00Z", "blockers": [{"number": 7, "title": "wrong account"}]},
    )
    assert not ok and "#7 wrong account" in report, report
    blocked = [{"number": 9, "title": "crash at launch"}]
    ok, report = check(
        "beta", {"now": now, "built_at": "2026-10-29T00:00:00Z", "blockers": blocked, "override": "hotfix for a crash"}
    )
    assert ok and "OVERRIDDEN" in report and "hotfix for a crash" in report, report
    ok, report = check("beta", {"now": now, "built_at": "2026-10-29T00:00:00Z", "blockers": blocked, "override": "  "})
    assert not ok, "a blank reason is no override"
    ok, _ = check("stable", {"now": now, "beta_at": "2026-10-23T12:00:00Z", "blockers": []})
    assert ok, "7 days on beta (D-114)"
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-24T12:00:00Z", "blockers": []})
    assert not ok and "6.0 days" in report and "7+" in report, report
    # D-114: a blocker fix needs 3 days on its own tag, its line 7.
    fix = [{"number": 12, "blocker": True, "closed": True}]
    line = "2026-10-21T12:00:00Z"
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-27T12:00:00Z", "line_beta_at": line, "blocker_fix": fix, "blockers": []})
    assert ok, report
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-28T12:00:00Z", "line_beta_at": line, "blocker_fix": fix, "blockers": []})
    assert not ok and "2.0 days" in report and "3+" in report, report
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-27T12:00:00Z", "line_beta_at": "2026-10-26T12:00:00Z", "blocker_fix": fix, "blockers": []})
    assert not ok and "its line is on beta for 4.0 days" in report, "a fix never skips the line's soak"
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-27T12:00:00Z", "line_beta_at": None, "blocker_fix": fix, "blockers": []})
    assert not ok and "first candidate" in report, report
    for wrong in ({"number": 13, "blocker": False, "closed": True}, {"number": 14, "blocker": True, "closed": False}):
        ok, report = check("stable", {"now": now, "beta_at": "2026-10-27T12:00:00Z", "line_beta_at": line, "blocker_fix": [wrong], "blockers": []})
        assert not ok and f"#{wrong['number']} isn't a closed issue labelled blocker" in report, report
    ok, report = check("stable", {"now": now, "beta_at": None, "blockers": []})
    assert not ok and "promote it to beta first" in report, report
    with tempfile.TemporaryDirectory() as tmp:
        facts = Path(tmp) / "facts.json"
        facts.write_text(json.dumps({"now": now, "built_at": "2026-10-01T00:00:00Z", "blockers": []}))
        assert main(["check", "beta", str(facts)]) == 0
    print("promotion-rules selftest: OK")
    return 0


def main(args: list[str]) -> int:
    if args == ["--selftest"]:
        return selftest()
    if len(args) == 3 and args[0] == "check":
        ok, report = check(args[1], json.loads(Path(args[2]).read_text()))
        print(report)
        return 0 if ok else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
