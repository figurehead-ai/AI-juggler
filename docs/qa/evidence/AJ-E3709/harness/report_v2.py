#!/usr/bin/env python3
"""report_v2.py results-apple-v2.json corpus-v2.json [--top-run F --top-idle F] -> metrics JSON on stdout (stdlib only).
Applies ADR-0033's gate and ladder (gate.py) to the engine's recorded drafts under three facts variants:
  main      = the ADR's design: real facts, section 2 routing (L2/L3 show the agent's own words; no model)
  gate_only = real facts, routing off: what the gate alone does to the model's lines
  unknown   = facts unknown (words only), routing off: the gate when the journal knows nothing
Scorers on what would be shown: the harness's independent v1 scorer (checks.polarity_check with the item's gold, entity_check against the source)."""
import json, re, statistics, sys
import checks, gate
def pct(v, den): return round(100.0*v/den, 1) if den else None
def p95(v):
    v = sorted(v); return v[min(len(v)-1, int(round(0.95*(len(v)-1))))] if v else None
def med(v): return statistics.median(v) if v else None
def top_means(path, names):
    out = {n: [] for n in names}; samples = []; cur = None
    try: txt = open(path).read()
    except OSError: return {}
    for l in txt.splitlines():
        if l.startswith("PID"): cur = {}; samples.append(cur); continue
        m = re.match(r"\s*(\d+)\s+(.+?)\s+([\d.]+)\s+([\d.]+)\s*$", l)
        if m and cur is not None: cur[m.group(2).strip()] = (float(m.group(3)), float(m.group(4)))
    samples = samples[1:]  # the first sample is since-boot
    res = {}
    for n in names:
        vals = []
        for s in samples:
            hit = [v for k, v in s.items() if n.startswith(k) or k.startswith(n[:15])]
            vals.append(hit[0][1] if hit else 0.0)   # not in the top 40 by power: counted as 0.0 (an upper bound is < the 40th's)
        res[n] = {"samples": len(vals), "mean_power": round(statistics.mean(vals), 2) if vals else None, "max_power": max(vals) if vals else None}
    return res
def main():
    R = json.load(open(sys.argv[1])); C = {i["id"]: i for i in json.load(open(sys.argv[2]))}
    args = sys.argv[3:]; top_run = args[args.index("--top-run")+1] if "--top-run" in args else None; top_idle = args[args.index("--top-idle")+1] if "--top-idle" in args else None
    res = R["results"]; meta = R["meta"]
    out = {"meta": meta, "n_records": len(res)}
    # ---- guard on the commit bodies
    bodies = [i for i in C.values() if i["bucket"] != "adversarial"]
    out["guard"] = {"commit_bodies": len(bodies), "removed_anything": sum(1 for i in bodies if gate.guard(i["source"])[1]),
                    "adversarial_removed": {i["id"]: gate.guard(i["source"])[1] for i in C.values() if i["bucket"] == "adversarial" and gate.guard(i["source"])[1]}}
    # ---- before the gate: the engine's own drafts (items where the guard removed nothing, so a write was made)
    dr = [r for r in res if r["write"]]; n = len(dr)
    na = []; ent_den = 0; ent_ok = 0; invented = []; fmt_ok = 0; fmt_fail = []; pol_script = []; s3rej = []; s5rej = []; s4rej = []; s2rej = []; quoted_ge90 = 0
    for r in dr:
        it = C[r["id"]]; cleaned, removed = gate.guard(it["source"]); w = r["write"]; t = w["text"]
        if checks.non_answer(t, w["refused"]): na.append(r["id"]); fmt_fail.append((r["id"], "non-answer")); continue
        ent_den += 1
        eok, ebad = gate.step2(cleaned, t)
        if eok: ent_ok += 1
        else: invented.append((r["id"], r["rep"], ebad))
        s4 = gate.step4(cleaned, t)
        if s4[0]: fmt_ok += 1
        else: fmt_fail.append((r["id"], s4[1]))
        if s4[2]: quoted_ge90 += 1
        pok, perr = checks.polarity_check(it["source"], t, it.get("gold"))
        if not pok: pol_script.append((r["id"], r["rep"], t, perr))
        if not gate.step3(cleaned, t, it["facts"])[0]: s3rej.append(r["id"])
        if not gate.step5(removed, t)[0]: s5rej.append(r["id"])
    out["before_gate"] = {"drafts": n, "non_answers": len(na), "non_answer_ids": sorted(set(na)),
        "entity_pass": ent_ok, "entity_den": ent_den, "invented_drafts": len(invented), "invented_detail": invented,
        "format_pass": fmt_ok, "format_fail": len(fmt_fail), "format_fail_detail": [(a, b if isinstance(b, str) else "; ".join(b)) for a, b in fmt_fail][:40],
        "polarity_script_errors": len(pol_script), "polarity_script_per_100": pct(len(pol_script), n), "polarity_script_detail": pol_script,
        "step3_would_reject": len(s3rej), "step3_reject_per_100": pct(len(s3rej), n), "copy_ge_90": quoted_ge90,
        "refusals": sum(1 for r in dr if r["write"]["refused"]), "permissive_retries": sum(1 for r in res for k in ("write","choose","frame_only") if r[k] and r[k]["retried_permissive"]),
        "errors": [(r["id"], r["write"]["error"]) for r in dr if r["write"]["error"]]}
    # ---- the ladder under three variants
    variants = {"main": (True, False), "gate_only": (False, False), "unknown": (False, True)}
    out["variants"] = {}; detail = {}
    for vn, (route, unk) in variants.items():
        kinds = {"written": 0, "quoted": 0, "excerpt": 0, "excerpt_routed": 0}; stepfail = {"2": 0, "3": 0, "4": 0, "5": 0}
        errs = []; inv = []; forbidden = []; bybucket = {}; lat_path = []; reached = 0; lost = []
        rows = []
        for r in res:
            it = C[r["id"]]; facts = None if unk else it["facts"]
            L = gate.ladder(it, facts, r["write"], r["choose"], route=route)
            kinds[L["kind"]] += 1
            for k, v in (L["write_gate"] or {}).items():
                if not v[0]: stepfail[k] += 1
            shown = L["shown"]
            used_choose = any(("choose" in t or "chosen" in t) and "no choose possible" not in t and "straight to the excerpt" not in t for t in L["trace"])
            lat = (r["write"]["latency_ms"] if r["write"] else 0) + (r["choose"]["latency_ms"] if used_choose and r["choose"] else 0)
            if used_choose: reached += 1
            if not r["cold_first"]: lat_path.append(lat)
            pok, perr = checks.polarity_check(it["source"], shown, it.get("gold")) if L["kind"] in ("written", "quoted") else (True, [])
            # excerpts: the 'failure lost / question lost' read of the shown text against the gold verdict
            xok, xerr = (True, [])
            if L["kind"] in ("excerpt", "excerpt_routed") and it.get("gold"):
                g = it["gold"]; sh = shown
                if g["verdict"] == "fail" and not (gate.verdict_words(sh)["fail"] or gate.verdict_words(sh)["nsucc"]): xok, xerr = False, ["failure lost in the excerpt"]
                if g["verdict"] == "question" and "?" not in sh: xok, xerr = False, ["question lost in the excerpt"]
            eok, ebad = checks.entity_check(it["source"], shown) if L["kind"] in ("written",) else (True, [])
            if not pok: errs.append((r["id"], r["rep"], L["kind"], shown, perr))
            if not xok: lost.append((r["id"], r["rep"], shown, xerr))
            if not eok: inv.append((r["id"], r["rep"], shown, ebad))
            if it.get("gold") and L["kind"] in ("written", "quoted"):
                for pat in it["gold"].get("forbidden", []):
                    if re.search(pat, shown, re.I): forbidden.append((r["id"], r["rep"], shown))
            rows.append({"id": r["id"], "rep": r["rep"], "kind": L["kind"], "shown": shown, "trace": L["trace"],
                         "gate": {k: [v[0], v[1]] for k, v in L["gate"].items()}, "draft": (r["write"] or {}).get("text"), "choose": (r["choose"] or {}).get("text")})
        N = len(res)
        out["variants"][vn] = {"trials": N, "shown_kinds": kinds, "model_line_pct": pct(kinds["written"], N), "quoted_pct": pct(kinds["quoted"], N),
            "excerpt_pct": pct(kinds["excerpt"] + kinds["excerpt_routed"], N), "write_drafts_failing_step": stepfail,
            "polarity_errors_script": len(errs), "polarity_per_100": pct(len(errs), N), "polarity_detail": errs,
            "failure_or_question_lost_in_excerpt": len(lost), "lost_detail": lost,
            "invented_in_written": len(inv), "invented_detail": inv, "forbidden_shown": forbidden,
            "reached_choose": reached, "path_latency_ms_median": med(lat_path), "path_latency_ms_p95": p95(lat_path)}
        detail[vn] = rows
    # per-bucket model-line rate (main and gate_only)
    for vn in ("main", "gate_only"):
        bb = {}
        for row in detail[vn]:
            b = C[row["id"]]["bucket"]; d = bb.setdefault(b, {"trials": 0, "written": 0, "quoted": 0, "excerpt": 0})
            d["trials"] += 1; d["written" if row["kind"] == "written" else "quoted" if row["kind"] == "quoted" else "excerpt"] += 1
        out["variants"][vn]["by_bucket"] = bb
    # ---- latency of the engine itself (write call, choose call), warm = all but the very first call of the run
    wl = [r["write"]["latency_ms"] for r in res if r["write"] and not r["cold_first"]]; cl = [r["choose"]["latency_ms"] for r in res if r["choose"] and not r["cold_first"]]
    out["latency"] = {"write_median_ms": med(wl), "write_p95_ms": p95(wl), "choose_median_ms": med(cl), "choose_p95_ms": p95(cl),
                      "cold_first_write_ms": next((r["write"]["latency_ms"] for r in res if r["cold_first"] and r["write"]), None), "writes": len(wl), "chooses": len(cl),
                      "by_bucket_write_median": {b: med([r["write"]["latency_ms"] for r in res if r["write"] and r["bucket"] == b and not r["cold_first"]]) for b in ("short", "medium", "long", "adversarial")}}
    # ---- frame-only diagnostic (A06/A07, frame v2 WITHOUT the guard)
    out["frame_only"] = [(r["id"], r["rep"], r["frame_only"]["text"]) for r in res if r["frame_only"]]
    # ---- repeats: how stable is the engine at temperature 0
    byid = {}
    for r in res:
        if r["write"]: byid.setdefault(r["id"], set()).add(r["write"]["text"])
    out["repeat_stability"] = {"items_with_one_distinct_draft": sum(1 for v in byid.values() if len(v) == 1), "items": len(byid), "items_with_variation": sorted(k for k, v in byid.items() if len(v) > 1)}
    # ---- energy inputs
    names = ["modelmanagerd", "generativeexperiencesd", "GenerativeExperiencesSafetyInferenceProvider", "intelligenceplatformd", "IntelligencePlatformComputeService", "mlhostd", "modelcatalogd"]
    ev = {}
    for d in meta["cpu_deltas"]:
        pass
    named = {d["name"]: d["cpu_s"] for d in meta["cpu_deltas"] if d["name"] in names}
    out["energy"] = {"wall_s": meta["wall_s"], "calls": meta["calls"], "helper_cpu_s": meta["helper_cpu_s"], "named_daemon_cpu_s": named,
                     "named_total_cpu_s": round(sum(named.values()) + meta["helper_cpu_s"], 2), "top_cpu_deltas_all_processes": meta["cpu_deltas"][:12],
                     "rss_mb": meta["watched_rss_mb"]}
    if top_run and top_idle:
        out["energy"]["top_power_run"] = top_means(top_run, names + ["apple_probe"]); out["energy"]["top_power_idle"] = top_means(top_idle, names + ["apple_probe"])
    out["_detail"] = detail
    print(json.dumps(out, indent=1, default=str, ensure_ascii=False))
if __name__ == "__main__": main()
