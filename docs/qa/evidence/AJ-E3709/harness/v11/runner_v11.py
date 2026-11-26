#!/usr/bin/env python3
"""runner_v11.py: harness v11's model calls: v6's runner with v11's job, watch7 (the app's own drafting, full coverage failing closed). Runbook: v11-runbook.md.

  python3 runner_v11.py --check --corpus watch11=../../heldout/v11/watch-v11.json          counts and unmapped words only, never item text
  python3 runner_v11.py --cmd <scratch>/apple_probe --corpus watch11=../../heldout/v11/watch-v11.json --out <scratch>/v11-apple-watch7.json
  python3 runner_v11.py --fake --corpus v11-fixtures=fixtures/v11-selftest.json --out <scratch>/dry-v11.json        (made-up items only)"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch7                                                        # noqa: E402,F401
V11_JOBS = ("watch7",)
def main():
    R6.V6_JOBS = V11_JOBS; R6.FAKE_OK = ("v11-fixtures",)
    if "--job" not in sys.argv: sys.argv += ["--job", "watch7"]
    R6.main()
if __name__ == "__main__": main()
