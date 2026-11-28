#!/usr/bin/env python3
"""runner_v7.py: harness v7's model calls (AJ-E3830, AJ-E3831; D-149's amendment). v6's runner (../v6/runner_v6.py) with v7's jobs:
watch3 (code decides the target, conditions, numbers and lifetime; the model picks only where code can't), feed_pick2 (the pick from
code's top three) and digest_mr2 (map and reduce, G7). Runbook: v7-runbook.md.

  python3 runner_v7.py --check --corpus watch7=../../heldout/v7/watch-v7.json        counts and unmapped words only, never item text
  python3 runner_v7.py --cmd <scratch>/apple_probe --job watch3 --corpus watch7=../../heldout/v7/watch-v7.json --out <scratch>/v7-apple-watch3.json
  python3 runner_v7.py --fake --corpus v7-fixtures=fixtures/v7-selftest.json --out <scratch>/dry-v7.json        (made-up items only)"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); V6 = os.path.join(os.path.dirname(HERE), "v6")
for p in (V6, HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch3, pick2, digest_mr2                                     # noqa: E402,F401  (v7's jobs, registered)
V7_JOBS = ("watch3", "feed_pick2", "digest_mr2")
def main():
    R6.V6_JOBS = V7_JOBS                                             # the v6 runner's job list, v7's jobs in it
    R6.FAKE_OK = ("v7-fixtures",)
    if not any(a.startswith("--job") for a in sys.argv): sys.argv += ["--job", ",".join(V7_JOBS)]
    R6.main()
if __name__ == "__main__": main()
