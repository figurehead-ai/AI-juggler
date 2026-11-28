#!/usr/bin/env python3
"""runner_v12.py: harness v12's model calls: v6's runner with v12's job, watch8 (the app's own drafting; verified labels may draft). The
results' meta says harness v12 and the round's own day (watch8.TODAY). Runbook: v12-runbook.md.

  python3 runner_v12.py --check --corpus watch12=../../heldout/v12/watch-v12.json          counts and unmapped words only, never item text
  python3 runner_v12.py --cmd <scratch>/apple_probe --corpus watch12=../../heldout/v12/watch-v12.json --out <scratch>/v12-apple-watch8.json
  python3 runner_v12.py --fake --corpus v12-fixtures=fixtures/v12-selftest.json --out <scratch>/dry-v12.json        (made-up items only)"""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), os.path.join(H, "v10"),
          os.path.join(H, "v11"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch8                                                        # noqa: E402
def main():
    R6.V6_JOBS = ("watch8",); R6.FAKE_OK = ("v12-fixtures",)
    if "--job" not in sys.argv: sys.argv += ["--job", "watch8"]
    R6.main()
    if "--out" in sys.argv:
        out = sys.argv[sys.argv.index("--out") + 1]; R = json.load(open(out, encoding="utf-8"))
        R["meta"].update(harness="v12", prompt_version=watch8.PROMPT_VERSION, harness_today=watch8.TODAY)
        json.dump(R, open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
if __name__ == "__main__": main()
