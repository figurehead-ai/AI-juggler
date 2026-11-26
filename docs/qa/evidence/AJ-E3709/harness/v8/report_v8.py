#!/usr/bin/env python3
"""report_v8.py: harness v8's report, offline. watch4 as v6's watch scoring, with "unsupported claimed" (bar 0, a hard bar ahead of 90 %
exact) first; feed_pick3 (A) and feed_why (B) each by v5's feed severity and the same blind pairs against the plain excerpt. For B the
fabrication rate (D-130's critical, at most 1 % of shown model items) is its own row. Runbook: v8-runbook.md.

  python3 report_v8.py RESULTS.json --corpus LABEL=PATH ... [--md OUT.md] [--candidates DIR] [--packets DIR] > report.json
  python3 report_v8.py --value REVIEW.json KEY.json"""
import argparse, json, os, sys
from collections import defaultdict
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                          # noqa: E402
from core import REGISTRY                                             # noqa: E402
import severity                                                       # noqa: E402
import report_v6 as R6, report_v5 as R5, report_v7 as R7              # noqa: E402
import watch4, pick3, why                                             # noqa: E402,F401
def report(results, corp):
    meta = results["meta"]; out = {"meta": meta, "criteria": "D-149 and the v8 calls (2026-11-26): watches: 0 unsupported claimed, then >= 90 % exact; "
                                   "the feed line (A or B) on only if preferred (60 % of decided pairs, pooled); B's fabrication <= 1 %", "corpora": {}}
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
        if key[1] == "feed_pick3": sev = severity.summarise([R6.severity_view(dict(v, job="feed_pick")) for v in views[key]]); rows = severity.bar_rows(sev)
        elif key[1] == "feed_why":
            sev = severity.summarise(views[key]); rows = severity.bar_rows(sev)
            rows.insert(0, {"bar": "fabrication (D-130 critical), % of shown model half-sentences", "op": "<=", "bar_value": 1.0,
                            "measured": sev["critical_reaching_pct"], "ok": sev["critical_reaching_pct"] <= 1.0})
        else:
            sev = job.summarise(views[key]); rows = job.bar_rows(sev)
            rows.insert(0, {"bar": "unsupported claimed (a hard bar, ahead of exact)", "op": "==", "bar_value": 0, "measured": sev.get("unsupported_claimed"),
                            "ok": sev.get("unsupported_claimed") == 0})
        block = {"stats": R5.job_stats(recs), "severity": {k: v for k, v in sev.items() if k != "candidates"}, "bar_check": rows, "_candidates": sev["candidates"]}
        if key[1] == "watch4": block["decided_by"] = R7.decided_counts(recs)
        if key[1] == "feed_why": block["kinds"] = dict(R5.job_stats(recs)["kinds_shown"])
        out["corpora"].setdefault(key[0], {})[key[1]] = block
    out["replay_mismatches"] = mismatches
    return out, views
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("results", nargs="*"); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH")
    ap.add_argument("--md"); ap.add_argument("--candidates", metavar="DIR"); ap.add_argument("--packets", metavar="DIR")
    ap.add_argument("--packets-seed", type=int, default=8); ap.add_argument("--key-name", default=None)
    ap.add_argument("--value", nargs=2, metavar=("REVIEW.json", "KEY.json"))
    a = ap.parse_args()
    if a.value: print(json.dumps(severity.value_from_review(json.load(open(a.value[0])), json.load(open(a.value[1]))), indent=1)); return
    if len(a.results) != 1: ap.error("one RESULTS.json")
    R = json.load(open(a.results[0])); corp = R6.load(a.corpus)
    rep, views = report(R, corp)
    if a.candidates: R5.write_candidates(rep, a.candidates, R["meta"].get("family", "model"))
    R5.strip_candidates(rep)
    if a.packets: R5.packets({k: v for k, v in views.items() if k[1] in ("feed_pick3", "feed_why")}, corp, a.packets, a.packets_seed, a.key_name)
    if a.md: open(a.md, "w").write(R5.markdown(rep, f"Harness v8 report: {R['meta'].get('engine')} ({R['meta'].get('family')}, retries sampled)"))
    json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print()
if __name__ == "__main__": main()
