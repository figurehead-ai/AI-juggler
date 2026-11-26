#!/usr/bin/env python3
"""selftest_v8.py: self-tests for harness v8 (AJ-E3830 watch4; AJ-E3831 feed designs A feed_pick3 and B feed_why), made-up items only
(fixtures/v8-selftest.json; heldout/v6, v7 and v8 stay blind to this code). v7's five "unsupported claimed" rows each have a regression
test here, in fresh words of the same shape (v7-results/summary.md quotes W05, W19 and W43; W32 and W51 by their recorded outcomes).

  python3 selftest_v8.py                  run every self-test
  python3 selftest_v8.py --write-prompts  regenerate prompts/watch4.md, feed_pick3.md and feed_why.md
  python3 selftest_v8.py --overlap PATH   the examples' and fixtures' overlap with a corpus file, as a count only"""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                         # noqa: E402
from core import REGISTRY, norm_item, gate                           # noqa: E402
import watch as W                                                    # noqa: E402
import watch4 as W4, pick3, why                                      # noqa: E402
from selftest_v6 import Script, NoCall, TUNED                        # noqa: E402
import selftest_v6                                                   # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "v8-selftest.json")
def fx(): return {x["id"]: x for x in json.load(open(FIXTURES))}
def it(fid): return norm_item(fx()[fid])
class Fail:
    def __init__(s): s.n = 0
    def __call__(s, req): s.n += 1; raise AssertionError("the model was called")
def selftest():
    fails = []; n = [0]
    def check(name, cond, detail=""):
        n[0] += 1
        if not cond: fails.append(f"{name}: {detail}")
    # ============================================================================================================ v7's five rows, never widen
    views = []
    for fid, near in (("V8-W05", "finishes"), ("V8-W19", "fails"), ("V8-W32", "fails"), ("V8-W43", "hits_limit"), ("V8-W51", "finishes"),
                      ("V8-W60", "hits_limit"), ("V8-W61", "finishes")):
        w = it(fid); eng = Fail(); rec = core6.run6(W4.WATCH4, w, "main", eng); o = rec["final"]
        check(f"watch4 {fid}: unsupported with the nearest ({near}), by code, never a watch", o.get("kind") == "unsupported" and o.get("nearest") == near and eng.n == 0,
              (o.get("kind"), o.get("nearest"), o.get("shown")))
        views.append({"job": "watch4", "id": fid, "outcome": o, "attempts": rec["attempts"], "model_called": False, "used": rec["used"], "item": w})
    for fid in ("V8-W62", "V8-W63"):
        w = it(fid); rec = core6.run6(W4.WATCH4, w, "main", Fail())
        views.append({"job": "watch4", "id": fid, "outcome": rec["final"], "attempts": rec["attempts"], "model_called": False, "used": rec["used"], "item": w})
    sm = W4.WATCH4.summarise(views)
    check("watch4 scoring: 0 unsupported claimed on v7's five shapes and the thresholds", sm["unsupported_claimed"] == 0, sm["unsupported_claimed"])
    check("watch4 scoring: the nearest offered on every unsupported request", sm["nearest_offered"] == sm["nearest_due"] == 7, (sm["nearest_offered"], sm["nearest_due"]))
    check("watch4 scoring: 'always' is a lifetime; an ignore beside a named target still drafts it", sm["exact_rule_matches_with_defaults"] == sm["supported_items"] == 2, sm["mismatches"])
    # generic never-widen: whatever a path drafts, a request that excludes something never gets a broader watch
    ctx = fx()["V8-W05"]["context"]
    for req in ("Ping me when any session asks me something, except invoices.", "Tell me when everything in heron-hooks finishes but not the retry logic one.",
                "Sag mir, wenn irgendeine Sitzung fehlschlägt, außer map tiles.", "Alert me if anything at all gets stuck, other than Webhook retries."):
        w = norm_item({"id": "g", "request": req, "context": ctx}); rec = core6.run6(W4.WATCH4, w, "main", Script([{"target": "none"}, {"condition": "fails", "condition2": "none", "unsupported": "none"}]))
        o = rec["final"]; tg = [t["kind"] for t in (o.get("watch") or {}).get("targets") or []]
        check(f"never widen: {req!r}", o.get("kind") != "watch" or not ({"any", "project", "app", "pool"} & set(tg)), (o.get("kind"), tg))
        check(f"never widen, with the nearest: {req!r}", o.get("kind") != "unsupported" or o.get("nearest"), o.get("shown"))
    # the v7 behaviour stays (made-up v7 fixtures)
    keep = {"V7-W01": "S-cedar", "V7-W02": "S-cedar", "V7-W08": "S-api-c", "V7-W12": "S-aspen"}
    for fid, sid in keep.items():
        o = core6.run6(W4.WATCH4, it(fid), "main", Fail())["final"]
        check(f"watch4 keeps v7's {fid}", o.get("kind") == "watch" and o["watch"]["targets"][0]["id"] == sid, o.get("shown"))
    # ============================================================================================================ feed A: step counts
    t = it("V8-T01"); chs = pick3.pick2.renumber(pick3.pick2.rank(t, pick3.pick2.P6.choices(t)))
    for c in chs:
        f, _w = pick3.FEED_PICK3.fields_of(t, chs, {"fact": c["n"], "verb": "none"})
        check(f"A: a step count is never a test count ({c['words'][:40]})", f is None or f["result"] == "none", f and f["result"])
    t2 = it("V6-T01"); c2 = pick3.pick2.renumber(pick3.pick2.rank(t2, pick3.pick2.P6.choices(t2)))
    f, _w = pick3.FEED_PICK3.fields_of(t2, c2, {"fact": "1", "verb": "none"})
    check("A: a real test count stays", f and f["result"] == "14 passed", f and f["result"])
    # ============================================================================================================ feed B: the faithfulness gate
    tb = it("V8-T02")
    eng = Script([{"why": "big imports stop running out of memory overnight"}]); rec = core6.run6(why.FEED_WHY, tb, "main", eng)
    check("B: the plain excerpt, then the half-sentence", rec["final"].get("kind") == "written" and rec["final"]["shown"].startswith("Moved the jacana importer")
          and " — big imports stop running out of memory overnight" in rec["final"]["shown"], rec["final"].get("shown"))
    for bad, reason in (("all 12 tests pass now", "faithfulness"), ("so the release is ready to ship", "faithfulness"), ("the importer was moved to the streaming parser", "restates"),
                        ("which fixes the crash in the nightly import", "faithfulness")):
        errs = why.FEED_WHY.gate(tb, bad)
        check(f"B gate refuses {bad!r} ({reason})", any(reason in e for e in errs), errs)
    eng = Script([{"why": "all 12 tests pass now"}]); rec = core6.run6(why.FEED_WHY, tb, "main", eng)
    check("B: refused every time: the plain excerpt alone, after two sampled retries", rec["used"] == "fallback" and len(eng.reqs) == 3
          and not rec["final"]["model_text"] and eng.reqs[1]["retry"] == 1, rec["used"])
    eng = Script([{"why": "none"}]); rec = core6.run6(why.FEED_WHY, tb, "main", eng)
    check("B: none: the plain excerpt alone", rec["final"].get("kind") == "plain_by_model" and not rec["final"]["model_text"])
    for fid in ("V6-T06", "V6-T08"):
        eng = NoCall(); rec = core6.run6(why.FEED_WHY, it(fid), "main", eng)
        check(f"B: {fid} needs you: the agent's own words, no call", eng.n == 0 and rec["used"] == "code")
    # ============================================================================================================ run, report, prompts
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "dry.json")
        r = subprocess.run([sys.executable, os.path.join(HERE, "runner_v8.py"), "--fake", "--corpus", f"v8-fixtures={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake on the fixtures", r.returncode == 0, r.stderr[-400:])
        r2 = subprocess.run([sys.executable, os.path.join(HERE, "report_v8.py"), out, "--corpus", f"v8-fixtures={FIXTURES}", "--packets", os.path.join(tmp, "blind-review", "p")],
                            capture_output=True, text=True)
        check("report on the fake run", r2.returncode == 0, r2.stderr[-400:])
        if r2.returncode == 0:
            rep = json.loads(r2.stdout); c = rep["corpora"]["v8-fixtures"]
            check("report: every replay agrees", not rep["replay_mismatches"], rep["replay_mismatches"][:3])
            check("report: the three jobs, the hard bar and B's fabrication row", set(c) == {"watch4", "feed_pick3", "feed_why"}
                  and c["watch4"]["bar_check"][0]["bar"].startswith("unsupported claimed") and c["feed_why"]["bar_check"][0]["bar"].startswith("fabrication"), list(c))
            check("report: packets for both feed designs", os.path.exists(os.path.join(tmp, "blind-review", "p", "packet-v8-fixtures-feed_why.json"))
                  and os.path.exists(os.path.join(tmp, "blind-review", "p", "packet-v8-fixtures-feed_pick3.json")), os.listdir(os.path.join(tmp, "blind-review", "p")))
    for jn in ("watch4", "feed_pick3", "feed_why"):
        p = os.path.join(HERE, "prompts", f"{jn}.md")
        check(f"prompts/{jn}.md is current", os.path.exists(p) and open(p).read() == REGISTRY[jn].write_prompts(), "run --write-prompts")
    ov = overlap(TUNED)
    check("examples and fixtures: no overlap with the tuned corpora", not ov, ov[:6])
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True).stdout.strip()
    if root:
        dd = subprocess.run(["git", "diff", "--quiet", "7acec4f50", "--", os.path.relpath(gate.__file__, root)], cwd=root)
        check("gate.py is byte-identical to 7acec4f50", dd.returncode == 0)
    print(f"selftest_v8: {n[0] - len(fails)}/{n[0]} passed")
    for f in fails: print("FAIL", f)
    return not fails
def texts():
    for ex in why.EXAMPLES: yield "why example", ex["message"]
    for x in json.load(open(FIXTURES)):
        if str(x.get("id", "")).startswith(("V6-", "V7-")): continue
        yield x["id"], (x.get("source") or "") + " " + (x.get("request") or "")
def overlap(paths):
    selftest_v6.texts = texts
    return selftest_v6.overlap(paths)
if __name__ == "__main__":
    if "--write-prompts" in sys.argv:
        for jn in ("watch4", "feed_pick3", "feed_why"): open(os.path.join(HERE, "prompts", f"{jn}.md"), "w").write(REGISTRY[jn].write_prompts())
        print("wrote prompts/watch4.md, feed_pick3.md, feed_why.md")
    elif "--overlap" in sys.argv:
        print(json.dumps({"overlaps": len(overlap([a for a in sys.argv[sys.argv.index("--overlap") + 1:] if not a.startswith("--")]))}))
    else: sys.exit(0 if selftest() else 1)
