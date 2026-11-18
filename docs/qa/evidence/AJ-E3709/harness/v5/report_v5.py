#!/usr/bin/env python3
"""report_v5.py: AJ-E3709 harness v5's report, offline, by severity (D-130 as clarified 2026-11-18; design: ../../v5-design.md).

  python3 report_v5.py RESULTS.json --corpus LABEL=PATH ... [--md OUT.md] [--candidates DIR] [--packets DIR] > report-v5.json
  python3 report_v5.py --value REVIEW.json KEY.json                     the value bar from a scored blind review (after scoring only)
  python3 report_v5.py --rescore-v4 V4RESULTS.json --corpus LABEL=PATH ... [--candidates DIR] [--md OUT.md]
                                                                        harness v4's held-out runs re-scored under the D-130 criteria

Per corpus and job: what code decided with no call; the drafts (first-attempt pass, passed after a retry, fallbacks, attempts per item);
latency; and the severity block (severity.py): critical reaching the user (bar: at most 1 % of shown model items, none systematic),
critical caught (rejected drafts, and the items they were on), omissions (bar: at most ~10 %), the structural rows (questions or
failures hidden: 0; a model line on a turn that needs you: 0). Each item is replayed from its recorded attempts (core.replay) so a
rule change re-scores without the model; a replay that disagrees with the run is listed. --candidates writes, per model and job, every
critical and omission candidate (shown, and the critical ones caught) for Opus to confirm by eye: the script flags, Opus decides."""
import argparse, json, os, random, statistics, sys
from collections import Counter, defaultdict
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core                                                           # noqa: E402
from core import j4, gate, REGISTRY, norm_item, item_kind             # noqa: E402
import turns, chat, severity                                          # noqa: E402

def pct(n, d): return round(100.0 * n / d, 2) if d else None
def p95(v): return sorted(v)[max(0, int(round(0.95 * len(v))) - 1)] if v else None
def med(v): return statistics.median(v) if v else None

def load(specs, jobs_wanted=("chat",)):
    corp = {}
    for spec in specs:
        label, _, path = spec.partition("=")
        raw = [x for x in json.load(open(path)) if item_kind(x)]
        if label in ("v2", "turnends") and not any(item_kind(x) == "chat" for x in raw): raw += chat.chat_items_tuned(label, raw)
        corp[label] = {"path": path, "items": {x["id"]: norm_item(x) for x in raw}}
    return corp
def chat_view_extras(item, outcome):
    path, fs, found, _why = REGISTRY["chat"].route(item)
    src = item["question"] + "\n" + "\n".join(chat.found_lines(found)) + "\n" + "\n".join(
        r["text"] + " " + " ".join(f"{k}: {chat.fmt_value(k, v)}" for k, v in r["fields"].items()) for r in item["records"])
    return {"source": src, "found_pairs": found if (path == "lookup" and outcome.get("kind") == "answer") else None}
def view_of(job, item, rec, final):
    v = {"job": job.name, "id": item["id"], "setting": rec.get("setting"), "outcome": final, "attempts": rec.get("attempts"),
         "model_called": rec.get("model_called"), "used": rec.get("used")}
    if job.item_kind == "chat": v.update(item=item, **chat_view_extras(item, final))
    else: v["t"] = item
    return v
def job_stats(recs):
    n = len(recs); called = [r for r in recs if r["model_called"]]
    first = sum(1 for r in called if r["used"] == 0); retried = sum(1 for r in called if isinstance(r["used"], int) and r["used"] > 0)
    fb = sum(1 for r in called if r["used"] == "fallback")
    att = Counter(len(r["attempts"]) for r in called)
    lat1 = [r["attempts"][0]["latency_ms"] for r in called if r["attempts"] and r["attempts"][0].get("latency_ms") is not None]
    tot = [sum(a.get("latency_ms") or 0 for a in r["attempts"]) for r in called]
    valid = sum(1 for r in called for a in r["attempts"] if not any(e.split(":")[0].endswith(("missing", "not in the schema")) or e.startswith(("not JSON", "empty", "unparsed")) for e in a["errors"]))
    return {"items": n, "decided_by_code": n - len(called), "model_called": len(called), "passed_first_attempt": first, "passed_after_a_retry": retried,
            "fell_back": fb, "passed_first_pct": pct(first, len(called)), "passed_with_retries_pct": pct(first + retried, len(called)),
            "fallback_pct": pct(fb, len(called)), "attempts_per_item": dict(sorted(att.items())),
            "drafts_parsed_and_valid_pct": pct(valid, sum(len(r["attempts"]) for r in called)),
            "latency_ms_first_attempt": {"median": med(lat1), "p95": p95(lat1)}, "latency_ms_item_total": {"median": med(tot), "p95": p95(tot)},
            "kinds_shown": dict(Counter(r["final"].get("kind") for r in recs))}
def report(results, corp):
    meta = results["meta"]; out = {"meta": meta, "criteria": "D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only)", "corpora": {}}
    by = defaultdict(list); views = defaultdict(list); mismatches = []
    for r in results["results"]:
        C = corp.get(r["corpus"])
        if C is None or r["id"] not in C["items"]: mismatches.append({"id": r["id"], "why": "item not in the corpora given"}); continue
        job = REGISTRY[r["job"]]; item = C["items"][r["id"]]
        rep = core.replay(job, item, r["setting"], r, meta.get("datamark", "on") == "on")
        if (rep["used"], rep["final"].get("kind")) != (r["used"], r["final"].get("kind")):
            mismatches.append({"id": r["id"], "job": r["job"], "run": [r["used"], r["final"].get("kind")], "replay": [rep["used"], rep["final"].get("kind")]})
        rr = dict(r, used=rep["used"], final=rep["final"], attempts=[dict(a, ok=b["ok"], errors=b["errors"]) for a, b in zip(r["attempts"], rep["attempts"])] or r["attempts"])
        by[(r["corpus"], r["job"])].append(rr); views[(r["corpus"], r["job"])].append(view_of(job, item, rr, rep["final"]))
    for key, recs in sorted(by.items()):
        sev = severity.summarise(views[key])
        block = {"stats": job_stats(recs), "severity": {k: v for k, v in sev.items() if k != "candidates"}, "bar_check": severity.bar_rows(sev)}
        out["corpora"].setdefault(key[0], {})[key[1]] = block
        block["_candidates"] = sev["candidates"]
    out["replay_mismatches"] = mismatches
    return out, views
def write_candidates(rep, outdir, family):
    os.makedirs(outdir, exist_ok=True); per_job = defaultdict(list)
    for label, js in rep["corpora"].items():
        for job, b in js.items(): per_job[job] += [dict(c, corpus=label) for c in b.pop("_candidates", [])]
    for job, cs in per_job.items():
        json.dump({"family": family, "job": job, "note": "script flags: candidates for Opus to confirm by eye, never verdicts", "candidates": cs},
                  open(os.path.join(outdir, f"candidates-{family}-{job}.json"), "w"), indent=1, ensure_ascii=False)
def strip_candidates(rep):
    for js in rep["corpora"].values():
        for b in js.values(): b.pop("_candidates", None)
def packets(views, corp, outdir, seed=5, key_name="KEY-do-not-open-before-scoring.json"):
    """blind pairs: the AI output against the job's non-AI alternative (A's excerpt; the code-only review; the facts-only attention; the
    looked-up values or the found rows), shuffled and unlabelled; the key, with each pair's job, in a separate file"""
    os.makedirs(outdir, exist_ok=True); rnd = random.Random(seed); key = []
    for (label, jn), vs in sorted(views.items()):
        job = REGISTRY[jn]; pk = []
        for v in vs:
            o = v["outcome"]
            if not o.get("model_text") or (v.get("setting") not in (None, "main")): continue
            item = v.get("t") or v.get("item"); alt = job.alternative(item, v.get("setting"))
            if not alt or alt.strip() == (o.get("shown") or "").strip(): continue
            ctxt = ({"question": item["question"], "records": [{"id": r["id"], "kind": r["kind"], "fields": r["fields"], "text": r["text"]} for r in item["records"]]}
                    if job.item_kind == "chat" else {"source": item["source"], "facts": item["raw"].get("facts"), "git": item["raw"].get("git")})
            pair = [("model", o["shown"]), ("non-AI", alt)]; rnd.shuffle(pair); pid = f"{label}-{jn}-{len(pk) + 1:03d}"
            pk.append({"pair": pid, "item": v["id"], **ctxt, "first": pair[0][1], "second": pair[1][1]})
            key.append({"pair": pid, "job": jn, "first": pair[0][0], "second": pair[1][0]})
        if pk: json.dump(pk, open(os.path.join(outdir, f"packet-{label}-{jn}.json"), "w"), indent=1, ensure_ascii=False)
    json.dump(key, open(os.path.join(outdir, key_name), "w"), indent=1)
def markdown(rep, title):
    L = [f"# {title}", "", f"Criteria: {rep.get('criteria')}. Critical = fabrication only; omissions tolerated to ~10 %. "
         "Script flags are candidates: Opus confirms every critical by eye.", ""]
    for label, js in rep["corpora"].items():
        for job, b in js.items():
            s = b.get("stats") or {}; v = b["severity"]
            L += [f"## {label} / {job}" + (f" ({b['setting']})" if b.get("setting") else ""), ""]
            if s: L.append(f"Items {s['items']}; decided by code {s['decided_by_code']}; model called {s['model_called']}: first attempt {s['passed_first_attempt']}, "
                           f"after a retry {s['passed_after_a_retry']}, fallback {s['fell_back']}.")
            L += [f"Shown model items {v['model_shown_items']}; critical reaching the user {v['critical_reaching_the_user']} ({v['critical_by_detector']}); "
                  f"critical caught on {v['critical_caught_items']} items ({v['critical_caught_by_detector']}); omissions {v['omission_items']} ({v['omission_by_detector']}).", "",
                  "| Bar | Bar value | Measured | Holds |", "|---|---|---|---|"]
            for r in b["bar_check"]: L.append(f"| {r['bar']} | {r['op']} {r['bar_value']} | {r['measured']} | {'yes' if r['ok'] else 'NO' if r['ok'] is False else 'n/a'} |")
            L.append("")
    if rep.get("replay_mismatches"): L += [f"Replay mismatches: {len(rep['replay_mismatches'])} (listed in the JSON).", ""]
    return "\n".join(L) + "\n"

# ---------------------------------------------------------------------------------------------------------------- harness v4's results
def rescore_v4(path, specs):
    R = json.load(open(path)); corp = {}
    for spec in specs:
        label, _, p = spec.partition("=")
        raw = j4.adapt_items(json.load(open(p)))
        corp[label] = {"items": {x["id"]: x for x in raw if x.get("id")}, "chat": {c["id"]: c for c in j4.chat_items(label, raw)}}
    views = defaultdict(list); missing = []
    for r in R["results"]:
        C = corp.get(r["corpus"])
        if C is None: missing.append(r["id"]); continue
        if r["job"] == "chat":
            ci = r.get("chat_item") or C["chat"].get(r["id"])
            if ci is None: missing.append(r["id"]); continue
            v = severity.rescore_v4_view(r, None, ci)
        else:
            it = C["items"].get(r["id"])
            if it is None: missing.append(r["id"]); continue
            v = severity.rescore_v4_view(r, it)
        views[(r["corpus"], r["job"], r.get("setting") or "main")].append(v)
    out = {"meta": {"rescored": os.path.basename(path), "v4_meta": R["meta"], "criteria": "D-130 as clarified 2026-11-18",
                    "note": "v4's drafts and v4's composition (what v4 would have shown), read by v5's severity detectors; v4 had no retries, so nothing is 'caught' here"},
           "criteria": "D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only)", "corpora": {}, "missing_items": missing}
    for (label, job, setting), vs in sorted(views.items()):
        sev = severity.summarise(vs)
        out["corpora"].setdefault(label, {})[job if setting == "main" else f"{job}:{setting}"] = {
            "setting": setting, "severity": {k: v for k, v in sev.items() if k != "candidates"}, "bar_check": severity.bar_rows(sev), "_candidates": sev["candidates"]}
    return out

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("results", nargs="*"); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH")
    ap.add_argument("--md"); ap.add_argument("--candidates", metavar="DIR"); ap.add_argument("--packets", metavar="DIR")
    ap.add_argument("--packets-seed", type=int, default=5); ap.add_argument("--key-name", default="KEY-do-not-open-before-scoring.json")
    ap.add_argument("--value", nargs=2, metavar=("REVIEW.json", "KEY.json"), help="the value bar from a scored blind review")
    ap.add_argument("--rescore-v4", metavar="V4RESULTS.json")
    a = ap.parse_args()
    if a.value:
        print(json.dumps(severity.value_from_review(json.load(open(a.value[0])), json.load(open(a.value[1]))), indent=1)); return
    if a.rescore_v4:
        rep = rescore_v4(a.rescore_v4, a.corpus); fam = rep["meta"]["v4_meta"].get("family", "model")
        if a.candidates: write_candidates(rep, a.candidates, fam + "-v4")
        strip_candidates(rep)
        if a.md: open(a.md, "w").write(markdown(rep, f"Harness v4's {os.path.basename(a.rescore_v4)} re-scored by severity (D-130)"))
        json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print(); return
    if len(a.results) != 1: ap.error("one RESULTS.json")
    R = json.load(open(a.results[0])); corp = load(a.corpus)
    rep, views = report(R, corp)
    if a.candidates: write_candidates(rep, a.candidates, R["meta"].get("family", "model"))
    strip_candidates(rep)
    if a.packets: packets(views, corp, a.packets, a.packets_seed, a.key_name)
    if a.md: open(a.md, "w").write(markdown(rep, f"Harness v5 report: {R['meta'].get('engine')} ({R['meta'].get('family')}, sampling {R['meta'].get('sampling_name')}, datamark {R['meta'].get('datamark')})"))
    json.dump(rep, sys.stdout, indent=1, ensure_ascii=False); print()

if __name__ == "__main__": main()
