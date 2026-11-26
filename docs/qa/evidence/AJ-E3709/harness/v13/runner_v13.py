#!/usr/bin/env python3
"""runner_v13.py: harness v13's model calls: v6's runner with v13's job, watch9 (the app's own drafting; word-indexed labels). The
results' meta says harness v13 and the round's own day (watch9.TODAY). Runbook: v13-runbook.md.

  python3 runner_v13.py --check --corpus watch13=../../heldout/v13/watch-v13.json          counts and unmapped words only, never item text
  python3 runner_v13.py --cmd <scratch>/apple_probe --corpus watch13=../../heldout/v13/watch-v13.json --out <scratch>/v13-apple-watch9.json
  python3 runner_v13.py --fake --corpus v13-fixtures=fixtures/v13-selftest.json --out <scratch>/dry-v13.json        (made-up items only)"""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), os.path.join(H, "v10"),
          os.path.join(H, "v11"), os.path.join(H, "v12"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import runner_v6 as R6                                               # noqa: E402
import watch9                                                        # noqa: E402
def main():
    R6.V6_JOBS = ("watch9",); R6.FAKE_OK = ("v13-fixtures",)
    if "--job" not in sys.argv: sys.argv += ["--job", "watch9"]
    R6.main()
    if "--out" in sys.argv:
        out = sys.argv[sys.argv.index("--out") + 1]; R = json.load(open(out, encoding="utf-8"))
        R["meta"].update(harness="v13", prompt_version=watch9.PROMPT_VERSION, harness_today=watch9.TODAY)
        json.dump(R, open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
if __name__ == "__main__": main()
