#!/usr/bin/env python3
"""prefill_score.py (AJ-E3830, 0.5.2): the pre-fill's per-field measurement against a set's gold.

  python3 prefill_score.py <out.json> <set.json> [--md <report.md>] [--json <report.json>] [--items]

Per supported request and field: filled and right, filled and wrong (the bar: 0), or left for the
user. Lifetime unsaid is the form's suggestion (ADR-0036's default, labelled so), counted apart.
Quiet hours: on or off as verified, or left (the switch off, the form says to check it). On an
unsupported request no condition may be filled; a filled target there is listed for the reader's
eye; the nearest offered (and equal to the gold's) is counted. `--items` adds each request's fields
(ids and codes only, never the request's words)."""
import json, sys
from statistics import median

def gold_life(g):
    if g == "once": return {"kind": "once"}
    if g == "until_removed": return {"kind": "always"}
    if isinstance(g, str) and g.startswith("until:"):
        hm = g.split("T")[1][:5]; h, m = int(hm[:2]), int(hm[3:5])
        return {"kind": "today"} if (h, m) == (23, 59) else {"kind": "until", "atMs": h * 3_600_000 + m * 60_000}
    return None

def cond_key(cs, gold=False):
    out = []
    for c in cs or []:
        code = c.get("type") if gold else c.get("code")
        if code == "asks": code = "asks_you"
        p = c.get("param")
        v = p if gold else (p.get("value") if isinstance(p, dict) else None)
        out.append((code, v if isinstance(v, int) else None))
    return sorted(out)

def target_key(t):
    if not t: return None
    return ("any", None) if t.get("kind") == "any" else (t.get("kind"), t.get("id"))

def old_gold(item):
    return item["id"].startswith(("V6-", "V7-"))

def main():
    out_path, set_path = sys.argv[1], sys.argv[2]
    args = sys.argv[3:]
    R = json.load(open(out_path, encoding="utf-8"))
    items = json.load(open(set_path, encoding="utf-8"))
    items = items["items"] if isinstance(items, dict) else items
    by_id = {r["id"]: r for r in R["results"]}
    F = ("target", "condition", "lifetime", "quiet_hours")
    sup = {f: {"right": 0, "wrong": 0, "left": 0} for f in F}
    sup["lifetime"].update(suggested=0, suggested_right=0)
    wrong, unsup_targets, rows = [], [], []
    n_sup = n_unsup = read_as_watch = complete = cards = cards_right = 0
    near_offered = near_gold = unsup_cond = 0
    for it in items:
        g, r = it["gold"], by_id.get(it["id"])
        p = (r or {}).get("prefill")
        if p: read_as_watch += 1
        if not g.get("supported"):
            n_unsup += 1
            if not p: continue
            d = p["draft"]
            if d["conditions"]: unsup_cond += 1; wrong.append(f"{it['id']} condition on an unsupported request {cond_key(d['conditions'])}")
            if d["targets"]: unsup_targets.append(f"{it['id']} {target_key(d['targets'][0])}")
            if p.get("nearest"):
                near_offered += 1
                if p["nearest"] == g.get("nearest_if_unsupported"): near_gold += 1
            rows.append((it["id"], "unsupported", p.get("why"), p.get("left"), p.get("nearest")))
            continue
        n_sup += 1
        if not p:
            for f in F: sup[f]["left"] += 1
            rows.append((it["id"], "not read as a watch", None, None, None)); continue
        d, left = p["draft"], set(p["left"])
        ok = 0
        # target
        if d["targets"]:
            if target_key(d["targets"][0]) == target_key(g.get("target")): sup["target"]["right"] += 1; ok += 1
            else: sup["target"]["wrong"] += 1; wrong.append(f"{it['id']} target {target_key(d['targets'][0])} gold {target_key(g.get('target'))}")
        else: sup["target"]["left"] += 1
        # condition
        gc = cond_key(g.get("conditions"), gold=True)
        if old_gold(it) and "idle" in it["request"].lower():
            gc = sorted(("idle_for" if c == "stuck" else c, v) for c, v in gc)
        if d["conditions"]:
            if cond_key(d["conditions"]) == gc: sup["condition"]["right"] += 1; ok += 1
            else: sup["condition"]["wrong"] += 1; wrong.append(f"{it['id']} condition {cond_key(d['conditions'])} gold {gc}")
        else: sup["condition"]["left"] += 1
        # lifetime
        gl = gold_life(g.get("lifetime"))
        if "lifetime" in left: sup["lifetime"]["left"] += 1
        elif d["lifetime"]["kind"] == "unsaid":
            sup["lifetime"]["suggested"] += 1
            if d["targets"] and d["conditions"]:
                kinds = {t["kind"] for t in d["targets"]}; codes = {c["code"] for c in d["conditions"]}
                default = {"kind": "once"} if kinds == {"session"} and codes <= {"finishes", "ends"} else {"kind": "today"}
                if default == gl: sup["lifetime"]["suggested_right"] += 1; ok += 1
        elif d["lifetime"] == gl: sup["lifetime"]["right"] += 1; ok += 1
        else: sup["lifetime"]["wrong"] += 1; wrong.append(f"{it['id']} lifetime {d['lifetime']} gold {gl}")
        # quiet hours
        gq = bool(g.get("even_in_quiet_hours"))
        if "quiet_hours" in left: sup["quiet_hours"]["left"] += 1
        elif d["evenInQuietHours"] == gq: sup["quiet_hours"]["right"] += 1; ok += 1
        else: sup["quiet_hours"]["wrong"] += 1; wrong.append(f"{it['id']} quiet hours {d['evenInQuietHours']} gold {gq}")
        if ok == 4: complete += 1
        if p.get("card"):
            cards += 1
            if ok == 4: cards_right += 1
        rows.append((it["id"], "supported", p.get("why"), sorted(left), p.get("nearest")))
    first = [r["call_ms"][0] for r in R["results"] if r.get("call_ms")]
    totals = sorted(r["ms"] for r in R["results"])
    p95 = totals[int(0.95 * (len(totals) - 1))] if totals else None
    rep = {"meta": R["meta"], "supported": n_sup, "unsupported": n_unsup, "read_as_watch": read_as_watch,
           "fields": sup, "wrong_total": len(wrong), "wrong": wrong, "complete": complete, "cards": cards,
           "cards_right": cards_right, "unsupported_condition_filled": unsup_cond, "unsupported_targets": unsup_targets,
           "nearest_offered": near_offered, "nearest_equal_gold": near_gold,
           "calls": sum(r["calls"] for r in R["results"]),
           "latency_ms": {"median_item": median(totals) if totals else None, "p95_item": p95,
                          "median_first_call": median(first) if first else None}}
    if "--items" in args: rep["items"] = rows
    if "--json" in args: json.dump(rep, open(args[args.index("--json") + 1], "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    pct = lambda a, b: f"{a} of {b} = {100 * a / b:.1f} %" if b else "n/a"
    md = [f"# Pre-fill measurement: {R['meta']['engine']} {R['meta']['model']}", "",
          f"{n_sup} supported, {n_unsup} unsupported; read as a watch request: {read_as_watch} of {n_sup + n_unsup}.", "",
          "| Field | Filled right | Filled wrong | Left for the user | Note |", "|---|---|---|---|---|"]
    for f in F:
        s = sup[f]; note = ""
        if f == "lifetime": note = f"the form's suggestion (unsaid) {s['suggested']}, equal to the gold {s['suggested_right']}"
        md.append(f"| {f} | {pct(s['right'], n_sup)} | {s['wrong']} | {s['left']} | {note} |")
    md += ["", f"Wrong pre-filled fields: **{len(wrong)}** (the bar: 0).",
           f"Every field right (as v13's exact counts it, the suggestion included): {pct(complete, n_sup)}.",
           f"Offered as Bot's card (every field verified): {cards}, every field right in {cards_right}.",
           f"Unsupported: condition filled {unsup_cond} (must be 0); nearest offered {pct(near_offered, n_unsup)}, equal to the gold {near_gold}.",
           f"Targets filled on unsupported requests (checked by eye): {', '.join(unsup_targets) or 'none'}.",
           f"Model calls {rep['calls']}; latency per item median {rep['latency_ms']['median_item']} ms, p95 {p95} ms; first call median {rep['latency_ms']['median_first_call']} ms."]
    if wrong: md += ["", "Wrong:"] + [f"- {w}" for w in wrong]
    text = "\n".join(md) + "\n"
    if "--md" in args: open(args[args.index("--md") + 1], "w", encoding="utf-8").write(text)
    print(text)

if __name__ == "__main__":
    main()
