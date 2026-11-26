#!/usr/bin/env python3
"""report_v13.py: harness v13's report, offline: watch9 (the app's own drafting, word-indexed labels) by v6's watch scoring (harness clock's today), "unsupported claimed" first (a
hard bar of 0, ahead of 90 % exact), with who decided each part. Runbook: v13-runbook.md.

  python3 report_v13.py RESULTS.json --corpus watch13=PATH [--md OUT.md] [--candidates DIR] > report.json"""
import argparse, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import report_v6 as R6, report_v5 as R5, report_v8 as R8, report_v7 as R7             # noqa: E402
import watch9                                                        # noqa: E402,F401
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("results"); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH")
    ap.add_argument("--md"); ap.add_argument("--candidates", metavar="DIR")
    a = ap.parse_args()
    R = json.load(open(a.results)); corp = R6.load(a.corpus)
    rep, _views = R8.report(R, corp)          # v8's report: watch jobs get the hard bar; who decided, below
    for blocks in rep["corpora"].values():
        b = blocks.get("watch9")
        if b and not str(b["bar_check"][0]["bar"]).startswith("unsupported claimed"):
            sev = b["severity"]; b["bar_check"].insert(0, {"bar": "unsupported claimed (a hard bar, ahead of exact)", "op": "==", "bar_value": 0,
                                                            "measured": sev.get("unsupported_claimed"), "ok": sev.get("unsupported_claimed") == 0})
    rep["meta"]["criteria_v13"] = ("D-149 amendment 3 as corrected: word-indexed labels, a second guard on filler, the negation guard; "
                                   "0 unsupported claimed, then >= 90 % exact; nearest offered >= 90 %")
    for blocks in rep["corpora"].values():
        if "watch9" in blocks: blocks["watch9"]["decided_by"] = R7.decided_counts([dict(r, final=r["final"]) for r in R["results"] if r["job"] == "watch9"])
    if a.candidates: R5.write_candidates(rep, a.candidates, R["meta"].get("family", "model"))
    R5.strip_candidates(rep)
    if a.md: open(a.md, "w").write(R5.markdown(rep, f"Harness v13 report: {R['meta'].get('engine')} ({R['meta'].get('family')}, retries sampled)"))
    json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print()
if __name__ == "__main__": main()
