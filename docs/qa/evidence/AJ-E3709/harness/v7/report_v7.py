#!/usr/bin/env python3
"""report_v7.py: harness v7's report, offline (AJ-E3830, AJ-E3831; D-149's bars: watches >= 90 % exact; the feed line on only if
preferred, D-130's 60 % of decided pairs pooled over the blind judges). Runbook: v7-runbook.md.

  python3 report_v7.py RESULTS.json --corpus LABEL=PATH ... [--md OUT.md] [--candidates DIR] [--packets DIR] > report.json
  python3 report_v7.py --value REVIEW.json KEY.json

v6's report with v7's job names: each item replayed from its recorded attempts; watch3 scored as v6's watch2 (with the harness clock's
today), feed_pick2 by v5's feed-line severity and blind pairs against A's excerpt, digest_mr2 by v5's digest severity and pairs against
the rollup. Script flags are candidates; Opus confirms every critical one by eye."""
import argparse, json, os, sys
from collections import defaultdict
HERE = os.path.dirname(os.path.abspath(__file__)); V6 = os.path.join(os.path.dirname(HERE), "v6")
for p in (V6, HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                          # noqa: E402
from core import REGISTRY                                             # noqa: E402
import severity                                                       # noqa: E402
import report_v6 as R6                                                # noqa: E402
import report_v5 as R5                                                # noqa: E402
import watch3, pick2, digest_mr2                                      # noqa: E402,F401
FEED = ("feed_pick", "feed_pick2")
def report(results, corp):
    meta = results["meta"]; out = {"meta": meta, "criteria": "D-149 and its amendment (2026-11-26): watches >= 90 % exact; the feed line on only if "
                                   "preferred (60 % of decided pairs, pooled); D-130's safety bars unchanged", "corpora": {}}
    by = defaultdict(list); views = defaultdict(list); mismatches = []
    for r in results["results"]:
        C = corp.get(r["corpus"])
        if C is None or r["id"] not in C["items"]: mismatches.append({"id": r["id"], "why": "item not in the corpora given"}); continue
        job = REGISTRY[r["job"]]; item = C["items"][r["id"]]
        rep = core6.replay6(job, item, r["setting"], r, meta.get("datamark", "on") == "on", meta.get("retries", core6.MAX_RETRIES))
        if (rep["used"], rep["final"].get("kind")) != (r["used"], r["final"].get("kind")):
            mismatches.append({"id": r["id"], "job": r["job"], "run": [r["used"], r["final"].get("kind")], "replay": [rep["used"], rep["final"].get("kind")]})
        rr = dict(r, used=rep["used"], final=rep["final"], attempts=rep["attempts"])
        by[(r["corpus"], r["job"])].append(rr); views[(r["corpus"], r["job"])].append(R5.view_of(job, item, rr, rep["final"]))
    for key, recs in sorted(by.items()):
        job = REGISTRY[key[1]]
        if key[1] in FEED: sev = severity.summarise([R6.severity_view(dict(v, job="feed_pick")) for v in views[key]]); rows = severity.bar_rows(sev)
        else: sev = job.summarise(views[key]); rows = job.bar_rows(sev)
        block = {"stats": R5.job_stats(recs), "severity": {k: v for k, v in sev.items() if k != "candidates"}, "bar_check": rows, "_candidates": sev["candidates"]}
        if key[1] in FEED: block["picks"] = R6.pick_kinds(recs); block["pick_rank"] = rank_counts(recs)
        if key[1] == "watch3": block["decided_by"] = decided_counts(recs)
        out["corpora"].setdefault(key[0], {})[key[1]] = block
    out["replay_mismatches"] = mismatches
    return out, views
def rank_counts(recs):
    c = defaultdict(int)
    for r in recs:
        p = (r["final"] or {}).get("pick")
        if p: c[f"rank {p.get('rank')}"] += 1
    return dict(c)
def decided_counts(recs):
    """who decided each part of each draft (code, the model's pick, the picker): a reading aid, not a bar"""
    c = defaultdict(int)
    for r in recs:
        for d in (r["final"] or {}).get("decided") or []: c[d.split(":")[0] + ": " + d.split(":", 1)[1].strip()[:40] if ":" in d else d] += 1
    return dict(c)
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("results", nargs="*"); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH")
    ap.add_argument("--md"); ap.add_argument("--candidates", metavar="DIR"); ap.add_argument("--packets", metavar="DIR")
    ap.add_argument("--packets-seed", type=int, default=7); ap.add_argument("--key-name", default=None)
    ap.add_argument("--value", nargs=2, metavar=("REVIEW.json", "KEY.json"))
    a = ap.parse_args()
    if a.value: print(json.dumps(severity.value_from_review(json.load(open(a.value[0])), json.load(open(a.value[1]))), indent=1)); return
    if len(a.results) != 1: ap.error("one RESULTS.json")
    R = json.load(open(a.results[0])); corp = R6.load(a.corpus)
    rep, views = report(R, corp)
    if a.candidates: R5.write_candidates(rep, a.candidates, R["meta"].get("family", "model"))
    R5.strip_candidates(rep)
    if a.packets: R5.packets({k: v for k, v in views.items() if k[1] in FEED + ("digest_mr2",)}, corp, a.packets, a.packets_seed, a.key_name)
    if a.md: open(a.md, "w").write(R5.markdown(rep, f"Harness v7 report: {R['meta'].get('engine')} ({R['meta'].get('family')}, retries sampled)"))
    json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print()
if __name__ == "__main__": main()
