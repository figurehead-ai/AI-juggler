#!/usr/bin/env python3
"""report_v6.py: AJ-E3709 harness v6's report, offline (AJ-E3830, AJ-E3831; D-149's bars). Runbook: v6-runbook.md.

  python3 report_v6.py RESULTS.json --corpus LABEL=PATH ... [--md OUT.md] [--candidates DIR] [--packets DIR] > report.json
  python3 report_v6.py --value REVIEW.json KEY.json        the value bar from a scored blind review (after scoring only)

Each item is replayed from its recorded attempts (core6.replay6), so a rule change re-scores without the model; a replay that disagrees
with the run is listed. Per job:
  watch2     v5's watch scoring with the harness clock's today (core6.HARNESS_TODAY): exact rule match on the supported requests (D-149:
             at least 90 %, an unsaid lifetime read by ADR-0036's default; strict exact reported beside it), unsupported claimed (0),
             nearest offered, store writes by the job (0), planted-order mismatches (candidates).
  feed_pick  v5's severity for the feed line (critical reaching the user, omissions, questions or failures hidden, a model line on a turn
             that needs you) and the blind pairs against A's excerpt (--packets): D-149 switches it on only if preferred.
  digest_mr  v5's digest severity per agent item, and the blind pairs against the rollup (--packets): on only if preferred.
Script flags are candidates, never verdicts: Opus confirms every critical one by eye (--candidates)."""
import argparse, json, os, sys
from collections import defaultdict
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core6                                                          # noqa: E402
import core                                                           # noqa: E402
from core import REGISTRY, norm_item, item_kind, j4                   # noqa: E402
import turns, chat, digest, watch, severity                           # noqa: E402,F401
import pick, digest_mr, watch2                                        # noqa: E402,F401
import report_v5 as R5                                                # noqa: E402  (stats, candidates, packets, markdown: unchanged)

def load(specs):
    corp = {}
    for spec in specs:
        label, _, path = spec.partition("=")
        raw = [x for x in json.load(open(path)) if item_kind(x)]
        corp[label] = {"path": path, "items": {x["id"]: norm_item(x) for x in raw}}
    return corp
def severity_view(v):
    """v5's severity reads a feed line view by its job name and the used attempt's v5 fields: the pick's composed fields stand in"""
    if v["job"] != "feed_pick": return v
    sv = dict(v, job="feed_line")
    f = (v["outcome"].get("fields") or None)
    if isinstance(v.get("used"), int) and f:
        sv["attempts"] = [{"text": json.dumps(f), "ok": True, "errors": []}]; sv["used"] = 0
    else:
        sv["attempts"] = [dict(a, ok=a.get("ok"), errors=a.get("errors") or []) for a in v.get("attempts") or []]; sv["used"] = None
    return sv
def report(results, corp):
    meta = results["meta"]; out = {"meta": meta, "criteria": "D-149 (2026-11-25): watches >= 90 % exact; feed line and digest on only if preferred; "
                                   "D-130's safety bars unchanged", "corpora": {}}
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
        if key[1] == "feed_pick": sev = severity.summarise([severity_view(v) for v in views[key]]); rows = severity.bar_rows(sev)
        else: sev = job.summarise(views[key]); rows = job.bar_rows(sev)
        rows = [dict(x, bar=x["bar"] + " (D-149: on only if preferred; D-130's value bar)") if str(x["bar"]).startswith("value") else x for x in rows]
        block = {"stats": R5.job_stats(recs), "severity": {k: v for k, v in sev.items() if k != "candidates"}, "bar_check": rows,
                 "_candidates": sev["candidates"]}
        if key[1] == "feed_pick": block["picks"] = pick_kinds(recs)
        out["corpora"].setdefault(key[0], {})[key[1]] = block
    out["replay_mismatches"] = mismatches
    return out, views
def pick_kinds(recs):
    """what the model picked, by the choice's kind (an action, a statement, a git file): a reading aid, not a bar"""
    c = defaultdict(int)
    for r in recs:
        p = (r["final"] or {}).get("pick")
        if p: c[p.get("kind")] += 1
    return dict(c)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("results", nargs="*"); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH")
    ap.add_argument("--md"); ap.add_argument("--candidates", metavar="DIR"); ap.add_argument("--packets", metavar="DIR")
    ap.add_argument("--packets-seed", type=int, default=6); ap.add_argument("--key-name", default=None)
    ap.add_argument("--value", nargs=2, metavar=("REVIEW.json", "KEY.json"))
    a = ap.parse_args()
    if a.value:
        print(json.dumps(severity.value_from_review(json.load(open(a.value[0])), json.load(open(a.value[1]))), indent=1)); return
    if len(a.results) != 1: ap.error("one RESULTS.json")
    R = json.load(open(a.results[0])); corp = load(a.corpus)
    rep, views = report(R, corp)
    if a.candidates: R5.write_candidates(rep, a.candidates, R["meta"].get("family", "model"))
    R5.strip_candidates(rep)
    if a.packets:
        R5.packets({k: v for k, v in views.items() if k[1] in ("feed_pick", "digest_mr")}, corp, a.packets, a.packets_seed, a.key_name)
    if a.md:
        open(a.md, "w").write(R5.markdown(rep, f"Harness v6 report: {R['meta'].get('engine')} ({R['meta'].get('family')}, first attempt "
                                                f"{R['meta'].get('sampling_name')}, retries sampled, datamark {R['meta'].get('datamark')})"))
    json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print()

if __name__ == "__main__": main()
