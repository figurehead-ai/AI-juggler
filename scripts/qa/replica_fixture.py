#!/usr/bin/env python3
"""AJ-E1529: the replica as the UI's fixture, made by the backend's own
readers (never hand-written).

Reads the backend harness's output (crates/juggler-app/tests/
qa_real_readonly.rs run against the replica HOME, as `just replica-oracle`
and `just replica-fixture` do) and writes the accounts, projects and
conversations it returned, in the shapes the UI reads, as a Playwright
fixture (`window.__AIJ_FIXTURE__`). Only the replica: never point the
harness at a real HOME for this.

Usage: replica_fixture.py HARNESS.json OUT.json
"""
from __future__ import annotations

import json
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    harness = json.load(open(sys.argv[1]))
    home_marks = {acc["state_dir"] for acc in harness["accounts"]}
    if not all("aij-replica" in path or "/replica" in path for path in home_marks):
        print("replica-fixture: these accounts aren't the replica's: refusing", file=sys.stderr)
        return 1
    fixture = {
        "accounts": harness["accounts_info"],
        "projects": harness["projects"],
        "conversations": harness["conversations"],
        # The replica has no session AI Juggler started; an empty list, not
        # the preview's own mocks.
        "sessions": [],
        "profiles": [],
        "generated_ms": harness["generated_ms"],
        # AJ-E1532: the projects' own icons, as the backend served them.
        "project_icons": harness.get("project_icons", {}),
    }
    with open(sys.argv[2], "w") as out:
        json.dump(fixture, out, indent=1, sort_keys=True)
        out.write("\n")
    live = sum(1 for c in fixture["conversations"] if c["live"])
    print(f"replica-fixture: {len(fixture['accounts'])} accounts, {len(fixture['projects'])} projects, "
          f"{len(fixture['conversations'])} conversations ({live} live) → {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
