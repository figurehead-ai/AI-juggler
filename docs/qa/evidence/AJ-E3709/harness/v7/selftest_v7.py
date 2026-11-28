#!/usr/bin/env python3
"""selftest_v7.py: self-tests for harness v7 (AJ-E3830 watch3, AJ-E3831 feed_pick2 and digest_mr2), on made-up items only
(fixtures/v7-selftest.json and the items below; heldout/v6 and heldout/v7 stay blind to this code).

  python3 selftest_v7.py                  run every self-test
  python3 selftest_v7.py --write-prompts  regenerate prompts/watch3.md and prompts/feed_pick2.md from the code
  python3 selftest_v7.py --overlap PATH   the examples' and fixtures' overlap with a corpus file, as a count only (safe on a blind file)"""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); V6 = os.path.join(os.path.dirname(HERE), "v6")
for p in (V6, HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                         # noqa: E402
from core import j4, gate, REGISTRY, norm_item                       # noqa: E402
import watch as W                                                    # noqa: E402
import watch3 as W3, pick2, digest_mr2                               # noqa: E402
from selftest_v6 import Script, NoCall, TUNED                        # noqa: E402
import selftest_v6                                                   # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "v7-selftest.json")
def fx(): return {x["id"]: x for x in json.load(open(FIXTURES))}
def it(fid): return norm_item(fx()[fid])
class Fail:
    def __init__(s): s.n = 0
    def __call__(s, req): s.n += 1; raise AssertionError(f"the model was called ({req.get('step')})")

def selftest():
    fails = []; n = [0]
    def check(name, cond, detail=""):
        n[0] += 1
        if not cond: fails.append(f"{name}: {detail}")
    # ============================================================================================================ watch3: code decides
    expect = {"V7-W01": ("watch", "S-cedar", {"asks_you"}), "V7-W02": ("watch", "S-cedar", {"finishes", "fails"}), "V7-W03": ("watch", "birch-docs", {"stuck"}),
              "V7-W04": ("watch", "any", {"idle_for"}), "V7-W05": ("watch", "S-cedar", {"fails"}), "V7-W06": ("watch", "acc-w", {"hits_limit"}),
              "V7-W07": ("unsupported", None, None), "V7-W08": ("watch", "S-api-c", {"finishes"}), "V7-W09": ("unsupported", None, None),
              "V7-W10": ("watch", "S-cedar", {"finishes"}), "V7-W11": ("watch", "birch-docs", {"stuck"}), "V7-W12": ("watch", "S-aspen", {"finishes"})}
    views = []
    for fid, (kind, tid, conds) in expect.items():
        w = it(fid); eng = Fail(); rec = core6.run6(W3.WATCH3, w, "main", eng); o = rec["final"]
        got = (o.get("kind"), (o.get("watch") or {}).get("targets", [{}])[0].get("id") if o.get("watch") else None,
               {c["type"] for c in o["watch"]["conditions"]} if o.get("watch") else None)
        check(f"watch3 {fid}: code decides with no model call ({fx()[fid]['request']})", eng.n == 0 and got == (kind, tid, conds), got)
        views.append({"job": "watch3", "id": fid, "outcome": o, "attempts": rec["attempts"], "model_called": rec["model_called"], "used": rec["used"], "item": w})
    w13 = it("V7-W13"); eng = Script([{"condition": "idle_for", "condition2": "none", "unsupported": "none"}]); rec = core6.run6(W3.WATCH3, w13, "main", eng)
    check("watch3: no condition word: the model picks one, from the enum with definitions", [r["step"] for r in eng.reqs] == ["when"]
          and "idle_for · no activity at all" in eng.reqs[0]["prompt"] and "stuck · AI Juggler sees it stuck" in eng.reqs[0]["prompt"], [r["step"] for r in eng.reqs])
    check("watch3: the number is code's", rec["final"].get("kind") == "watch" and rec["final"]["watch"]["conditions"] == [{"type": "idle_for", "param": 20}], rec["final"].get("watch"))
    views.append({"job": "watch3", "id": "V7-W13", "outcome": rec["final"], "attempts": rec["attempts"], "model_called": True, "used": rec["used"], "item": w13})
    sm = W3.WATCH3.summarise(views)
    check("watch3 scoring: every made-up supported request exact", sm["exact_rule_matches_with_defaults"] == sm["supported_items"] == 11,
          (sm["exact_rule_matches_with_defaults"], sm["supported_items"], sm["mismatches"]))
    check("watch3 scoring: an unsaid lifetime stays unsaid (code reads the lifetime; v6's 23 'today' misses)", sm["exact_rule_matches"] == sm["exact_rule_matches_with_defaults"], sm["mismatches"])
    check("watch3 scoring: unsupported requests all get the nearest option", sm["nearest_offered"] == sm["nearest_due"] == 2, (sm["nearest_offered"], sm["nearest_due"]))
    check("watch3: never writes a watch", sm["store_writes_by_the_job"] == 0)
    # the resolver
    w = it("V7-W01"); r = W3.resolve(w)
    check("resolve: an ignored session is no candidate", [e["id"] for e in r["candidates"]] == ["S-cedar"] and any(e["id"] == "S-aspen" for e in r["excluded"]), r)
    r = W3.resolve(it("V7-W03"))
    check("resolve: 'anything in <project>' is the project, not one of its sessions", [e["id"] for e in r["candidates"]] == ["birch-docs"], [e["id"] for e in r["candidates"]])
    r = W3.resolve(it("V7-W06"))
    check("resolve: an alias names its account", [e["id"] for e in r["candidates"]] == ["acc-w"], [e["id"] for e in r["candidates"]])
    wx = norm_item(dict(fx()["V7-W08"], id="X", request="Let me know when api finishes."))
    o = core6.run6(W3.WATCH3, wx, "main", Fail())["final"]
    check("resolve: a shared name nothing settles asks with the picker, no call", o.get("kind") == "ask" and len(o.get("picker") or []) == 2, o)
    wx = norm_item(dict(fx()["V7-W08"], id="X", request="Tell me when cedar-api or aspen-board finishes."))
    eng = Script([{"target": "s2"}]); rec = core6.run6(W3.WATCH3, wx, "main", eng)
    check("resolve: two named candidates: the model picks one of them (an enum of the candidates only)", eng.reqs and eng.reqs[0]["step"] == "target"
          and sorted(eng.reqs[0]["schema"]["properties"][0]["schema"]["values"]) == ["none", "s1", "s2"], eng.reqs and eng.reqs[0]["schema"])
    # the conditions
    e = it("V7-W01")["by_ref"]["s1"]
    check("conditions: the session's enum is the app's table for a session, hits_limit included", W3.kind_conditions(e) ==
          ["finishes", "ends", "asks_you", "fails", "stuck", "idle_for", "hits_limit", "leaves_processes"], W3.kind_conditions(e))
    check("conditions: every condition has its one-line definition", all(c in W3.DEFINE for c in W.COND if c not in W.NOT_YET))
    for req, want in (("tell me when it finishes or errors", ["finishes", "fails"]), ("ping me if it stops moving", ["stuck"]), ("tell me if it is idle for 30 min", ["idle_for"]),
                      ("warn me when we hit the rate limit", ["hits_limit"]), ("tell me when the account drops below 10 percent", ["runway_below"]),
                      ("sag mir, wenn es fehlschlägt", ["fails"]), ("tell me once it ends up done", ["finishes"])):
        w = norm_item({"id": "c", "request": req, "context": fx()["V7-W01"]["context"]})
        check(f"conditions: code's cue words: {req!r}", W3.cue_conditions(w) == want, W3.cue_conditions(w))
    for req, want in (("Tell me when x finishes.", ("unsaid", None)), ("Today, tell me when x finishes.", ("today", None)), ("Tell me until 18:30 if x fails.", ("until_time", "18:30")),
                      ("Tell me every time x fails.", ("always", None)), ("Tell me once, when x finishes.", ("once", None)), ("Tell me if x fails until it ends.", ("until_ends", None))):
        got = W3.lifetime_for({"request": req})
        check(f"lifetime: code's reading of {req!r}", got == want, got)
    w = norm_item({"id": "c", "request": "Tell me when the work max account hits its limit, the costs matter.", "context": fx()["V7-W01"]["context"]})
    check("cue net: a usage limit beside money words is the limit, not unsupported money", not W3.cues_fixed(w), W3.cues_fixed(w))
    w = norm_item({"id": "c", "request": "Wake me even at 3am if cedar-api fails.", "context": fx()["V7-W01"]["context"]})
    check("cue net: 'even at 3am' is quiet hours, not a clock time", not W3.cues_fixed(w), W3.cues_fixed(w))

    # ============================================================================================================ feed_pick2: code's top three
    t = it("V7-T01"); chs = pick2.renumber(pick2.rank(t, pick2.P6.choices(t)))
    check("pick2: the outcome first, the side detail and git's files out of the top three", chs[0]["words"].startswith("the big sheets now import")
          and not any(c["kind"] == "git" for c in chs) and not any("cron" in c["words"] for c in chs) and len(chs) <= 3, [c["words"] for c in chs])
    eng = Script([{"fact": "1", "verb": "none"}]); rec = core6.run6(pick2.FEED_PICK2, t, "main", eng)
    check("pick2: the model's enum is the top three only", eng.reqs[0]["schema"]["properties"][0]["schema"]["values"] == ["1", "2", "3"], eng.reqs[0]["schema"])
    check("pick2: code renders the pick", (rec["final"].get("line") or "").startswith("The big sheets now import") and rec["final"]["pick"]["rank"] == 1, rec["final"].get("line"))
    t2 = it("V7-T02"); c2 = pick2.renumber(pick2.rank(t2, pick2.P6.choices(t2)))
    check("pick2: the change ranks over a bump and a reformat", c2[0]["verb"] == "switched", [(c["verb"], c["words"]) for c in c2])
    for fid in ("V6-T06", "V6-T07", "V6-T08"):
        eng = NoCall(); rec = core6.run6(pick2.FEED_PICK2, it(fid), "main", eng)
        check(f"pick2: {fid} stays code's (routing, as v6)", eng.n == 0 and rec["used"] == "code")
    # ============================================================================================================ digest_mr2: G7
    d = it("V7-D01"); eng = Script([{"fact": "1", "verb": "none"}]); rec = core6.run6(digest_mr2.DIGEST_MR2, d, "main", eng); o = rec["final"]
    yew = next(a for a in d["agents"] if a["session_id"] == "ses-yew-1")
    check("G7: a rolled-back turn keeps the plain words (no pick for it)", yew["ref"] in o["kept_plain_g7"] and len(eng.reqs) == 1
          and any(x["agent"] == yew["ref"] and x["source"] == "rollup" for x in o["items"]), (o["kept_plain_g7"], len(eng.reqs)))
    check("G7: its line says what was tried", "bigger cache" in o["shown"], o["shown"])
    check("G7: the other agent is picked", any(x["source"] == "ai" for x in o["items"]))
    # ============================================================================================================ replay, run, report, prompts
    for job, item, outs in ((W3.WATCH3, w13, [{"condition": "idle_for", "condition2": "none", "unsupported": "none"}]),
                            (pick2.FEED_PICK2, t, [{"fact": "9", "verb": "none"}, {"fact": "2", "verb": "none"}]), (digest_mr2.DIGEST_MR2, d, [{"fact": "1", "verb": "none"}])):
        rec = core6.run6(job, item, "main", Script(outs)); rp = core6.replay6(job, item, "main", rec)
        check(f"replay: {job.name} replays to the same outcome", (rp["used"], rp["final"].get("shown")) == (rec["used"], rec["final"].get("shown")))
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "dry.json")
        r = subprocess.run([sys.executable, os.path.join(HERE, "runner_v7.py"), "--fake", "--corpus", f"v7-fixtures={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake on the fixtures", r.returncode == 0, r.stderr[-400:])
        r2 = subprocess.run([sys.executable, os.path.join(HERE, "report_v7.py"), out, "--corpus", f"v7-fixtures={FIXTURES}", "--packets", os.path.join(tmp, "blind-review", "p")],
                            capture_output=True, text=True)
        check("report on the fake run", r2.returncode == 0, r2.stderr[-400:])
        if r2.returncode == 0:
            rep = json.loads(r2.stdout)
            check("report: every replay agrees with the run", not rep["replay_mismatches"], rep["replay_mismatches"][:3])
            check("report: the three jobs are scored", set(rep["corpora"]["v7-fixtures"]) == {"watch3", "feed_pick2", "digest_mr2"}, list(rep["corpora"]["v7-fixtures"]))
        r3 = subprocess.run([sys.executable, os.path.join(HERE, "runner_v7.py"), "--fake", "--corpus", f"watch7={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake refuses anything but the made-up fixtures", r3.returncode != 0)
    for jn in ("watch3", "feed_pick2"):
        p = os.path.join(HERE, "prompts", f"{jn}.md")
        check(f"prompts/{jn}.md is current", os.path.exists(p) and open(p).read() == REGISTRY[jn].write_prompts(), "run --write-prompts")
    ov = overlap(TUNED)
    check("examples and fixtures: no overlap with the tuned corpora", not ov, ov[:6])
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True).stdout.strip()
    if root:
        dd = subprocess.run(["git", "diff", "--quiet", "7acec4f50", "--", os.path.relpath(gate.__file__, root)], cwd=root)
        check("gate.py is byte-identical to 7acec4f50", dd.returncode == 0)
    print(f"selftest_v7: {n[0] - len(fails)}/{n[0]} passed")
    for f in fails: print("FAIL", f)
    return not fails

def texts():
    for ex in pick2.EXAMPLES: yield ex["label"], ex["message"]
    for ex in W3.EXAMPLES1 + W3.EXAMPLES2: yield "watch3 example", ex["request"]
    for x in json.load(open(FIXTURES)):
        if str(x.get("id", "")).startswith("V6-"): continue
        yield x["id"], (x.get("source") or "") + " " + (x.get("request") or "") + " " + " ".join(t["source"] for a in x.get("agents") or [] for t in a["turns"])
def overlap(paths):
    selftest_v6.texts = texts
    return selftest_v6.overlap(paths)

if __name__ == "__main__":
    if "--write-prompts" in sys.argv:
        for jn in ("watch3", "feed_pick2"): open(os.path.join(HERE, "prompts", f"{jn}.md"), "w").write(REGISTRY[jn].write_prompts())
        print("wrote prompts/watch3.md, prompts/feed_pick2.md")
    elif "--overlap" in sys.argv:
        print(json.dumps({"overlaps": len(overlap([a for a in sys.argv[sys.argv.index("--overlap") + 1:] if not a.startswith("--")]))}))
    else: sys.exit(0 if selftest() else 1)
