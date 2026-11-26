#!/usr/bin/env python3
"""selftest_v11.py: harness v11's self-test, on made-up items and every past miss only (never a blind set). It runs the app's own drafting
(watch7: the example binary `watch_round`) over every watch fixture of v5..v11, three ways, and scores each with v6's watch scoring:

  no model     every call fails: code alone (a request code can't place is told back by code);
  labeller     a well-behaved labeller (each span by code's own reading; an unplaced word labelled other);
  liar         a labeller that hides what can't be watched: it calls every unplaced word filler or condition, or rewords a span.

The checks: 0 unsupported claimed on every fixture, every way (the labels never make a request supported); every unsupported answer has
a nearest option; the same outcomes with and without the labeller (code decides); the v10 misses decided as gold says; prompts/watch7.md
equal to what the app's prompt files give (`--write-prompts` writes it). CARGO_TARGET_DIR (or AIJ_WATCH_ROUND) must name the built binary."""
import glob, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import watch7 as W7                                                 # noqa: E402
import core6, watch as W, watch2 as W2                              # noqa: E402

def fixtures():
    seen = set(); out = []
    for f in sorted(glob.glob(os.path.join(H, "v*", "fixtures", "*.json"))):
        try: d = json.load(open(f, encoding="utf-8"))
        except ValueError: continue
        for it in d if isinstance(d, list) else d.get("items", []):
            if not isinstance(it, dict) or "context" not in it or not it.get("gold") or it.get("request") in seen: continue
            seen.add(it["request"]); out.append(W.norm_watch(it))
    return out

def no_model(req): return {"text": None, "finish": "stop", "error": "no model (selftest)", "latency_ms": 0}
def spans_by(w, liar=False):
    r = W7.reading(w); spans = []
    for word, slot in r["slots"]:
        s = slot if slot != "unplaced" else ("filler" if liar else "other")
        if spans and spans[-1]["slot"] == s: spans[-1]["text"] += " " + word
        else: spans.append({"text": word, "slot": s, "number": 0, "unit": "none"})
    if liar and spans: spans[0]["text"] = "please " + spans[0]["text"]        # a reworded span: code must not take it
    return spans
def labeller(w, liar=False):
    first, honest = spans_by(w, liar), spans_by(w)          # read now: the app is busy drafting once the calls come
    def call(req):
        if req["step"] != "label": return no_model(req)
        spans = first if req["retry"] == 0 else honest
        if liar and req["retry"] > 0: spans = [dict(s, slot="condition" if s["slot"] == "other" else s["slot"]) for s in spans]
        return {"text": json.dumps({"spans": spans}), "finish": "stop", "error": "", "latency_ms": 0}
    return call

def run(items, mk):
    views = []
    for w in items:
        rec = core6.run6(W7.WATCH7, w, "main", mk(w))
        views.append({"job": "watch7", "id": w["id"], "outcome": rec["final"], "attempts": rec["attempts"], "model_called": rec["model_called"],
                      "used": rec["used"], "item": w})
    return views, W2.summarise(views)

def main():
    if "--write-prompts" in sys.argv:
        open(os.path.join(HERE, "prompts", "watch7.md"), "w", encoding="utf-8").write(W7.WATCH7.write_prompts() + "\n"); print("wrote prompts/watch7.md"); return
    items = fixtures(); ok = fail = 0; outs = {}
    def check(name, cond, detail=""):
        nonlocal ok, fail
        if cond: ok += 1
        else: fail += 1; print("FAIL", name, detail)
    for way, mk in (("no model", lambda w: no_model), ("labeller", labeller), ("liar", lambda w: labeller(w, True))):
        views, s = run(items, mk); outs[way] = {v["id"]: v["outcome"].get("kind") for v in views}
        print(f"{way:9s} items {s['items']}: unsupported claimed {s['unsupported_claimed']}; exact {s['exact_rule_matches_with_defaults']} of "
              f"{s['supported_items']} ({s['exact_rule_match_with_defaults_pct']} %, strict {s['exact_rule_match_pct']} %); nearest offered "
              f"{s['nearest_offered']} of {s['nearest_due']} ({s['nearest_offered_pct']} %), equal to gold {s['nearest_matches_gold']}")
        check(f"{way}: 0 unsupported claimed", s["unsupported_claimed"] == 0, [m for m in s["candidates"] if m["detector"] == "unsupported_claimed"])
        bare = [v["id"] for v in views if v["outcome"].get("kind") == "unsupported" and not v["outcome"].get("nearest")]
        check(f"{way}: every unsupported answer has a nearest option", not bare, bare)
        if way == "no model":
            for v in views:
                if not str(v["id"]).startswith(("V10-", "N11-")): continue
                g = W.gold_norm(v["item"]); o = v["outcome"]
                if g["supported"]:
                    good = o.get("kind") == "watch" and W.match(v["item"], o, g)[0] or (o.get("kind") == "watch" and W.match(v["item"], o, g)[2])
                    check(f"{v['id']} drafted as gold", good, f"{o.get('kind')}: {o.get('shown')} {W.match(v['item'], o, g)[1] if o.get('kind') == 'watch' else ''}")
                else:
                    check(f"{v['id']} told back with the nearest", o.get("kind") == "unsupported" and o.get("nearest"), f"{o.get('kind')}: {o.get('shown')}")
    check("the labels change no outcome (code decides)", outs["no model"] == outs["labeller"] == outs["liar"],
          [k for k in outs["no model"] if not (outs["no model"][k] == outs["labeller"][k] == outs["liar"][k])])
    have = open(os.path.join(HERE, "prompts", "watch7.md"), encoding="utf-8").read()
    check("prompts/watch7.md is the app's prompt files", have == W7.WATCH7.write_prompts() + "\n")
    print(f"selftest_v11: {ok}/{ok + fail} passed"); sys.exit(1 if fail else 0)
if __name__ == "__main__": main()
