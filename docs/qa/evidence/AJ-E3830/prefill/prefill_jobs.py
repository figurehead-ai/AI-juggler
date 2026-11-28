#!/usr/bin/env python3
"""prefill_jobs.py (AJ-E3830, 0.5.2): a round's set as jobs for the app's own pre-fill
(`crates/juggler-app/examples/watch_prefill_round.rs`), with the harness's own candidates
(harness v11's `watch7.candidates`: each target as the app's `Candidate`, its offer as the
harness reads what its App reports) and the harness's clock (midnight 0, 09:00).

  python3 prefill_jobs.py <set.json> <jobs.json>

Item text goes only into the jobs file (scratch); nothing is printed but counts."""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
H = os.path.abspath(os.path.join(HERE, "..", "..", "AJ-E3709", "harness"))
for p in ("v5", "v6", "v7", "v8", "v9", "v10", "v11", "v12", "v13"):
    sys.path.insert(0, os.path.join(H, p))
sys.path.insert(0, H)
import watch9                                          # noqa: E402,F401  (the job chain v13 ran)
import watch7                                          # noqa: E402
import watch as W                                      # noqa: E402

def main():
    src, dst = sys.argv[1], sys.argv[2]
    items = json.load(open(src, encoding="utf-8"))
    items = items["items"] if isinstance(items, dict) else items
    jobs = [watch7.job_of(W.norm_watch(it), "draft") for it in items]
    json.dump(jobs, open(dst, "w", encoding="utf-8"), ensure_ascii=False)
    print(f"{len(jobs)} jobs")

if __name__ == "__main__":
    main()
