#!/usr/bin/env python3
"""runner_v8.py: harness v8's model calls: v6's runner with v8's jobs, watch4 (never widens; every unsupported request with its nearest),
feed_pick3 (design A) and feed_why (design B). Runbook: v8-runbook.md.

  python3 runner_v8.py --check --corpus watch8=../../heldout/v8/watch-v8.json --corpus turns8=../../heldout/v8/turns-v8.json
  python3 runner_v8.py --cmd <scratch>/apple_probe --job watch4 --corpus watch8=../../heldout/v8/watch-v8.json --out <scratch>/v8-apple-watch4.json
  python3 runner_v8.py --fake --corpus v8-fixtures=fixtures/v8-selftest.json --out <scratch>/dry-v8.json        (made-up items only)"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch4, pick3, why                                            # noqa: E402,F401
V8_JOBS = ("watch4", "feed_pick3", "feed_why")
def main():
    R6.V6_JOBS = V8_JOBS; R6.FAKE_OK = ("v8-fixtures",)
    R6.main()
if __name__ == "__main__": main()
