#!/usr/bin/env python3
"""AJ-E803: the visual gate's summary (cells passed / failed / not
captured) from Playwright's JSON report, as Markdown."""
import json
import sys


def walk(suite, out):
    for spec in suite.get("specs", []):
        for test in spec.get("tests", []):
            status = test.get("status") or (test.get("results") or [{}])[-1].get("status")
            out.append((spec.get("title", ""), status))
    for child in suite.get("suites", []):
        walk(child, out)


def main() -> int:
    try:
        report = json.load(open(sys.argv[1]))
    except (OSError, ValueError, IndexError):
        print("Visual gate: no report (the comparison didn't run).")
        return 0
    cells = []
    for suite in report.get("suites", []):
        walk(suite, cells)
    passed = [t for t, s in cells if s in ("expected", "passed")]
    failed = [t for t, s in cells if s in ("unexpected", "failed", "timedOut")]
    skipped = [t for t, s in cells if s == "skipped"]
    print("### Visual gate")
    print(f"- passed: {len(passed)}")
    print(f"- failed: {len(failed)}")
    print(f"- not captured: {len(skipped)}")
    for title in failed:
        print(f"  - ✗ {title} — see the visual-triptychs artifact")
    return 0


if __name__ == "__main__":
    sys.exit(main())
