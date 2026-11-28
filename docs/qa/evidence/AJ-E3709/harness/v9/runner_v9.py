#!/usr/bin/env python3
"""runner_v9.py: harness v9's model calls: v6's runner with v9's job, watch5 (fail closed; D-149 amendment 2). Runbook: v9-runbook.md.

  python3 runner_v9.py --check --corpus watch9=../../heldout/v9/watch-v9.json          counts and unmapped words only, never item text
  python3 runner_v9.py --cmd <scratch>/apple_probe --corpus watch9=../../heldout/v9/watch-v9.json --out <scratch>/v9-apple-watch5.json
  python3 runner_v9.py --fake --corpus v9-fixtures=fixtures/v9-selftest.json --out <scratch>/dry-v9.json        (made-up items only)"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch5                                                        # noqa: E402,F401
V9_JOBS = ("watch5",)
def main():
    R6.V6_JOBS = V9_JOBS; R6.FAKE_OK = ("v9-fixtures",)
    if "--job" not in sys.argv: sys.argv += ["--job", "watch5"]
    R6.main()
if __name__ == "__main__": main()
