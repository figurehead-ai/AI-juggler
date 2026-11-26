#!/usr/bin/env python3
"""selftest_v13.py: harness v13's self-test, on made-up items and every past miss only (never a blind set). It runs the app's own drafting
(watch9: the example binary `watch_round`, at v13's commit) over every watch fixture of v5..v13, six ways, scored with v6's watch scoring:

  no model       every call fails: code alone;
  labeller       an honest labeller (each span by code's own reading; an unplaced word labelled other);
  filler-model   calls every unplaced word filler (a model that sees nothing in them): the second guard decides;
  liar-target    calls every unplaced word part of the target (a negation, an "unless" clause, a count);
  liar-cond      calls every unplaced word part of the condition;
  wrong-length   answers one slot short (refused, asked again, then code's own; never padded or cut).
Labels are word by word (v13): one slot for each word code couldn't place.

The checks: 0 unsupported claimed on every fixture, every way (verified labels may draft, so no labeller may ever draft a request gold
calls unsupported); every unsupported answer has a nearest option; v11's and v12's misses and the made-up items decided as gold says
(with no model, or with a labeller calling unknown small words filler); prompts/watch9.md equal to the app's prompt files. CARGO_TARGET_DIR (or AIJ_WATCH_ROUND) names the binary."""
import glob, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import watch9 as W9                                                 # noqa: E402
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
def labeller(w, liar=False, lie="filler"):
    """word by word: each word the app asks about gets `lie` (a liar) or other (honest); "short" answers one slot short"""
    def call(req):
        if req["step"] != "label": return no_model(req)
        n = int(req["prompt"].split("LABEL (", 1)[1].split(" ", 1)[0])
        if lie == "short": slots = ["filler"] * max(0, n - 1)
        else: slots = [lie if liar else "other"] * n
        return {"text": json.dumps({"slots": slots}), "finish": "stop", "error": "", "latency_ms": 0}
    return call

def run(items, mk):
    views = []
    for w in items:
        rec = core6.run6(W9.WATCH9, w, "main", mk(w))
        views.append({"job": "watch9", "id": w["id"], "outcome": rec["final"], "attempts": rec["attempts"], "model_called": rec["model_called"],
                      "used": rec["used"], "item": w})
    return views, W2.summarise(views)

def main():
    if "--write-prompts" in sys.argv:
        open(os.path.join(HERE, "prompts", "watch9.md"), "w", encoding="utf-8").write(W9.WATCH9.write_prompts() + "\n"); print("wrote prompts/watch9.md"); return
    items = fixtures(); ok = fail = 0; outs = {}
    def check(name, cond, detail=""):
        nonlocal ok, fail
        if cond: ok += 1
        else: fail += 1; print("FAIL", name, detail)
    for way, mk in (("no model", lambda w: no_model), ("labeller", labeller),
                    ("liar-target", lambda w: labeller(w, True, "target")), ("liar-cond", lambda w: labeller(w, True, "condition")),
                    ("wrong-length", lambda w: labeller(w, True, "short")), ("filler-model", lambda w: labeller(w, True, "filler"))):
        views, s = run(items, mk); outs[way] = {v["id"]: v["outcome"].get("kind") for v in views}
        print(f"{way:11s} items {s['items']}: unsupported claimed {s['unsupported_claimed']}; exact {s['exact_rule_matches_with_defaults']} of "
              f"{s['supported_items']} ({s['exact_rule_match_with_defaults_pct']} %, strict {s['exact_rule_match_pct']} %); nearest offered "
              f"{s['nearest_offered']} of {s['nearest_due']} ({s['nearest_offered_pct']} %), equal to gold {s['nearest_matches_gold']}")
        check(f"{way}: 0 unsupported claimed", s["unsupported_claimed"] == 0, [m for m in s["candidates"] if m["detector"] == "unsupported_claimed"])
        bare = [v["id"] for v in views if v["outcome"].get("kind") == "unsupported" and not v["outcome"].get("nearest")]
        check(f"{way}: every unsupported answer has a nearest option", not bare, bare)
        if way == "filler-model":
            for v in views:
                if not str(v["id"]).startswith(("V10-", "N11-", "V11-", "N12-", "V12-", "N13-")): continue
                g = W.gold_norm(v["item"]); o = v["outcome"]
                if g["supported"]:
                    good = o.get("kind") == "watch" and W.match(v["item"], o, g)[0] or (o.get("kind") == "watch" and W.match(v["item"], o, g)[2])
                    check(f"{v['id']} drafted as gold", good, f"{o.get('kind')}: {o.get('shown')} {W.match(v['item'], o, g)[1] if o.get('kind') == 'watch' else ''}")
                else:
                    check(f"{v['id']} told back with the nearest", o.get("kind") == "unsupported" and o.get("nearest"), f"{o.get('kind')}: {o.get('shown')}")
    check("a liar's target or condition drafts no more than the honest labeller or code alone", all(outs[l][k] != "watch" or outs["labeller"][k] == "watch" or outs["no model"][k] == "watch"
                                                   for l in ("liar-target", "liar-cond") for k in outs[l]),
          [(l, k) for l in ("liar-target", "liar-cond") for k in outs[l]
           if outs[l][k] == "watch" and outs["labeller"][k] != "watch" and outs["no model"][k] != "watch"])
    check("a wrong length drafts no more than no model at all", all(outs["wrong-length"][k] == outs["no model"][k] for k in outs["no model"]),
          [k for k in outs["no model"] if outs["wrong-length"][k] != outs["no model"][k]])
    have = open(os.path.join(HERE, "prompts", "watch9.md"), encoding="utf-8").read()
    check("prompts/watch9.md is the app's prompt files", have == W9.WATCH9.write_prompts() + "\n")
    print(f"selftest_v13: {ok}/{ok + fail} passed"); sys.exit(1 if fail else 0)
if __name__ == "__main__": main()
