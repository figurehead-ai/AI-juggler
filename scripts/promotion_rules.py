#!/usr/bin/env python3
"""D-24 channel requirements: whether a version may be promoted.

  promotion_rules.py check beta|stable FACTS.json   exit 0 = may promote
  promotion_rules.py --selftest

promote.yml gathers the facts from GitHub and this decides, so the rules
live in one tested place:

- **Beta**: the build is at least BETA_MIN_DAYS old — it has been used as
  AI Juggler Nightly, real days of work, not only CI — and no issue
  labelled `blocker` is open.
- **Stable**: the version has been on beta for at least STABLE_MIN_DAYS,
  and no `blocker` issue is open. (Stable is also a deliberate act: the
  maintainer runs and approves it; promote.yml checks the signed beta
  gate and the CHANGELOG section.)

A failed rule can be overridden only with a written reason, which is
printed with the result and kept in the channel tag's message.

FACTS.json: {"now": ISO, "built_at": ISO, "beta_at": ISO | null,
             "blockers": [{"number": N, "title": "…"}], "override": "…"}
"""
from __future__ import annotations

import datetime as dt
import json
import sys
import tempfile
from pathlib import Path

BETA_MIN_DAYS = 3
STABLE_MIN_DAYS = 14


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
        elif (age := _days(built, now)) < BETA_MIN_DAYS:
            problems.append(
                f"built {age:.1f} days ago; beta needs {BETA_MIN_DAYS}+ days of real use on nightly first"
            )
    elif channel == "stable":
        on_beta = _when(facts.get("beta_at"))
        if on_beta is None:
            problems.append("no record of when it reached beta — promote it to beta first")
        elif (age := _days(on_beta, now)) < STABLE_MIN_DAYS:
            problems.append(f"on beta for {age:.1f} days; stable needs {STABLE_MIN_DAYS}+")
    else:
        raise SystemExit(f"unknown channel {channel!r} (beta or stable)")
    blockers = facts.get("blockers") or []
    if blockers:
        listed = ", ".join(f"#{b['number']} {b['title']}" for b in blockers)
        problems.append(f"{len(blockers)} open blocker issue(s): {listed}")
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
    ok, report = check("beta", {"now": now, "built_at": "2026-10-28T13:00:00Z", "blockers": []})
    assert not ok and "2.0 days" in report, report
    ok, report = check(
        "beta",
        {"now": now, "built_at": "2026-10-20T00:00:00Z", "blockers": [{"number": 7, "title": "wrong account"}]},
    )
    assert not ok and "#7 wrong account" in report, report
    ok, report = check(
        "beta", {"now": now, "built_at": "2026-10-29T00:00:00Z", "blockers": [], "override": "hotfix for a crash"}
    )
    assert ok and "OVERRIDDEN" in report and "hotfix for a crash" in report, report
    ok, report = check("beta", {"now": now, "built_at": "2026-10-29T00:00:00Z", "blockers": [], "override": "  "})
    assert not ok, "a blank reason is no override"
    ok, _ = check("stable", {"now": now, "beta_at": "2026-10-15T12:00:00Z", "blockers": []})
    assert ok, "15 days on beta"
    ok, report = check("stable", {"now": now, "beta_at": "2026-10-20T12:00:00Z", "blockers": []})
    assert not ok and "10.0 days" in report, report
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
