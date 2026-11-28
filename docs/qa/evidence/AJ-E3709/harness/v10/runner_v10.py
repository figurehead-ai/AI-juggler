#!/usr/bin/env python3
"""runner_v10.py: harness v10's model calls: v6's runner with v10's job, watch6 (fail closed, the entity wins when it is the condition.s own). Runbook: v10-runbook.md.

  python3 runner_v10.py --check --corpus watch10=../../heldout/v10/watch-v10.json          counts and unmapped words only, never item text
  python3 runner_v10.py --cmd <scratch>/apple_probe --corpus watch10=../../heldout/v10/watch-v10.json --out <scratch>/v10-apple-watch6.json
  python3 runner_v10.py --fake --corpus v10-fixtures=fixtures/v10-selftest.json --out <scratch>/dry-v10.json        (made-up items only)"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch6                                                        # noqa: E402,F401
V10_JOBS = ("watch6",)
def main():
    R6.V6_JOBS = V10_JOBS; R6.FAKE_OK = ("v10-fixtures",)
    if "--job" not in sys.argv: sys.argv += ["--job", "watch6"]
    R6.main()
if __name__ == "__main__": main()
