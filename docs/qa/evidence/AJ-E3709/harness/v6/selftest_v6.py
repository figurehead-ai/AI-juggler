#!/usr/bin/env python3
"""selftest_v6.py: self-tests for harness v6 (AJ-E3830 watch2, AJ-E3831 feed_pick and digest_mr), on made-up items only
(fixtures/v6-selftest.json and the items below; no held-out text is read or copied: heldout/v6 stays blind to this code).

  python3 selftest_v6.py                  run every self-test
  python3 selftest_v6.py --write-prompts  regenerate prompts/feed_pick.md and prompts/watch2.md from the code
  python3 selftest_v6.py --overlap PATH   the examples' and fixtures' overlap with a corpus file, as a count only (safe on a blind file)"""
import hashlib, json, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core6                                                         # noqa: E402
import core                                                          # noqa: E402
from core import j4, gate, REGISTRY, norm_item                       # noqa: E402
import turns, watch as W                                             # noqa: E402
import pick, digest_mr, watch2 as W2                                 # noqa: E402
import report_v6                                                     # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "v6-selftest.json")
TUNED = ["../../corpus-424cce8f.json", "../../corpus-v2-8f7c7474.json", "../../corpus-v3-turnends.json"]
def fx(): return {x["id"]: x for x in json.load(open(FIXTURES))}
def it(fid): return norm_item(fx()[fid])

class Script:
    """an engine that answers from a list (one answer per call, the last repeated) and keeps every request"""
    def __init__(s, outs): s.outs = list(outs); s.reqs = []
    def __call__(s, req):
        s.reqs.append(req); o = s.outs[min(len(s.reqs) - 1, len(s.outs) - 1)]
        return {"text": o if isinstance(o, str) else json.dumps(o, ensure_ascii=False), "finish": "stop", "latency_ms": 1}
class NoCall:
    def __init__(s): s.n = 0
    def __call__(s, req): s.n += 1; raise AssertionError("the model was called")
def two(cond="finishes", param="none", cond2="none", param2="none", lifetime="unsaid", until="none", unsupported="none"):
    return {"condition": cond, "param": param, "condition2": cond2, "param2": param2, "lifetime": lifetime, "until": until, "unsupported": unsupported}
def ch_n(t, kind, text=None):
    return next(c["n"] for c in pick.choices(t) if c["kind"] == kind and (text is None or text in c["words"]))

def selftest():
    fails = []; n = [0]
    def check(name, cond, detail=""):
        n[0] += 1
        if not cond: fails.append(f"{name}: {detail}")

    # ============================================================================================================ watch2 (AJ-E3830)
    w = it("V6-W01")
    s1 = W2.schema1(w); props = {p["name"]: p["schema"] for p in s1["properties"]}
    check("watch2 schema: exactly one target, an enum of this request's refs (not a list)", props["target"]["type"] == "enum"
          and set(props["target"]["values"]) == {e["ref"] for e in w["entries"]} | {"none"}, props["target"])
    s2 = W2.schema2(w, w["by_ref"]["s1"]); p2 = {p["name"]: p["schema"] for p in s2["properties"]}
    check("watch2 schema: what can't be watched is a short enum, not free text", p2["unsupported"]["type"] == "enum" and "none" in p2["unsupported"]["values"]
          and all(len(v) <= 12 for v in p2["unsupported"]["values"]), p2["unsupported"])
    check("watch2 schema: every call-2 field is an enum", all(p["schema"]["type"] == "enum" for p in s2["properties"]))
    check("watch2 schema: the conditions are the target kind's, among those some App reports", set(p2["condition"]["values"]) - {"none"}
          == {c for c in W.offered_any(w) if "session" in W.COND[c][1]}, p2["condition"]["values"])
    acct = w["by_ref"]["acct1"]; pa = {p["name"]: p["schema"] for p in W2.schema2(w, acct)["properties"]}
    check("watch2 schema: an account takes only an account's conditions", "finishes" not in pa["condition"]["values"] and "runway_below" in pa["condition"]["values"])
    w3 = it("V6-W03"); p3 = {p["name"]: p["schema"] for p in W2.schema2(w3, w3["by_ref"]["s3"])["properties"]}
    check("watch2 schema: param is a number the request says (or none)", p3["param"]["values"] == ["none", "20"], p3["param"]["values"])
    check("watch2: request times as hh:mm", W2.request_times("until 6pm or 18:30, bis 9 Uhr") == ["18:00", "18:30", "09:00"], W2.request_times("until 6pm or 18:30, bis 9 Uhr"))

    eng = Script([{"intent": "create", "target": "s1"}, two("finishes")]); rec = core6.run6(W2.WATCH2, w, "main", eng)
    check("watch2: two calls, the target first, then the condition", [r["step"] for r in eng.reqs] == ["target", "when"], [r["step"] for r in eng.reqs])
    check("watch2: the second call is built for the chosen target", "TARGET: s1 session" in eng.reqs[1]["prompt"], eng.reqs[1]["prompt"][:80])
    check("watch2: a right pick is the draft, first attempt", rec["used"] == 0 and rec["final"]["kind"] == "watch"
          and rec["final"]["watch"]["targets"][0]["id"] == "S-vega", rec["final"])
    check("watch2: the draft shows Watch, Change, Cancel (the user's click adds it)", rec["final"].get("buttons") == ["Watch", "Change", "Cancel"])

    eng = Script([{"intent": "create", "target": "s2"}, {"intent": "create", "target": "s1"}, two("finishes")]); rec = core6.run6(W2.WATCH2, w, "main", eng)
    check("watch2: a target the request doesn't name retries call 1 with the reason", [r["step"] for r in eng.reqs] == ["target", "target", "when"]
          and "not named in the request" in eng.reqs[1]["prompt"] and rec["used"] == 1 and rec["final"]["kind"] == "watch", [r["step"] for r in eng.reqs])
    check("watch2: the retry is marked as a retry (the runner samples it)", [r["retry"] for r in eng.reqs] == [0, 1, 0], [r["retry"] for r in eng.reqs])

    eng = Script([{"intent": "create", "target": "s1"}, two("finishes", unsupported="deploy"), two("finishes")]); rec = core6.run6(W2.WATCH2, w, "main", eng)
    check("watch2: an unsupported kind the request doesn't say retries call 2 only", [r["step"] for r in eng.reqs] == ["target", "when", "when"]
          and "says nothing of a deploy" in eng.reqs[2]["prompt"] and rec["final"]["kind"] == "watch", ([r["step"] for r in eng.reqs], rec["final"].get("kind")))

    w6 = it("V6-W06"); eng = Script([{"intent": "create", "target": "s1"}, two("none", unsupported="tests")]); rec = core6.run6(W2.WATCH2, w6, "main", eng)
    check("watch2: unsupported is answered in the request's own words, with the nearest option", rec["final"]["kind"] == "unsupported"
          and rec["final"]["asked"] in w6["request"] and rec["final"]["nearest"] == "finishes", rec["final"])
    eng = Script([{"intent": "create", "target": "s1"}, two("finishes")]); rec = core6.run6(W2.WATCH2, w6, "main", eng)
    check("watch2: a draft claiming support for the tests is caught by the cue net (never a watch)", rec["final"]["kind"] == "unsupported", rec["final"].get("kind"))

    w4 = it("V6-W04"); eng = Script([{"intent": "create", "target": "s3"}]); rec = core6.run6(W2.WATCH2, w4, "main", eng)
    check("watch2: a shared name the context can't settle asks with the picker, with no second call", rec["final"]["kind"] == "ask" and len(eng.reqs) == 1
          and rec["final"].get("picker") == ["s3", "s4"], rec["final"])
    eng = Script([{"intent": "create", "target": "s4"}, two("finishes")]); rec = core6.run6(W2.WATCH2, w3, "main", eng)
    check("watch2: a shared name settled by the request's other words: the other one is rejected", "resolves to s3" in json.dumps(rec["attempts"][0]["errors"]), rec["attempts"][0]["errors"])

    w8 = it("V6-W08"); eng = Script([{"intent": "not_a_watch", "target": "none"}]); rec = core6.run6(W2.WATCH2, w8, "main", eng)
    check("watch2: not a watch: one call, no draft", rec["final"]["kind"] == "not_a_watch" and len(eng.reqs) == 1, rec["final"])
    eng = Script([{"intent": "create", "target": "none"}]); rec = core6.run6(W2.WATCH2, w, "main", eng)
    check("watch2: target none while the request names one is rejected, then the form", rec["final"]["kind"] == "form" and rec["used"] == "fallback"
          and len(eng.reqs) == 3, (rec["final"]["kind"], len(eng.reqs)))
    wx = norm_item({"id": "X-fr", "request": "Préviens-moi quand vega-sync termine.", "context": fx()["V6-W01"]["context"], "gold": {}})
    eng = NoCall(); rec = core6.run6(W2.WATCH2, wx, "main", eng)
    check("watch2: a language without word lists: the form, no call", eng.n == 0 and rec["final"]["kind"] == "form" and rec["used"] == "code", rec["final"])

    good = {"V6-W01": ("s1", two("finishes")), "V6-W02": ("s2", two("fails", cond2="asks_you", lifetime="today")),
            "V6-W03": ("s3", two("stuck", "20")), "V6-W05": ("acct1", two("runway_below", "15")), "V6-W06": ("s1", two("none", unsupported="tests")),
            "V6-W07": ("p2", two("asks_you")), "V6-W09": ("s2", two("stuck", "10")), "V6-W10": ("s1", two("finishes"))}
    views = []; store0 = W.STORE.snapshot()
    for fid, (ref, t2) in good.items():
        wi = it(fid); eng = Script([{"intent": "create", "target": ref}, t2]); rec = core6.run6(W2.WATCH2, wi, "main", eng)
        views.append({"job": "watch2", "id": fid, "outcome": rec["final"], "attempts": rec["attempts"], "model_called": True, "used": rec["used"], "item": wi})
    sm = W2.summarise(views)
    check("watch2 scoring: right picks on the fixtures are exact (the harness clock's today reads gold's dated 'today')",
          sm["exact_rule_matches_with_defaults"] == sm["supported_items"] and sm["supported_items"] == 6, (sm["exact_rule_matches_with_defaults"], sm["supported_items"], sm["mismatches"]))
    check("watch2 scoring: an unobserved condition (Codex doesn't report stuck) is unsupported with the nearest option, never a watch",
          next(v for v in views if v["id"] == "V6-W09")["outcome"]["kind"] == "unsupported", next(v for v in views if v["id"] == "V6-W09")["outcome"])
    check("watch2 scoring: today is the harness clock's", sm.get("harness_today") == core6.HARNESS_TODAY == "2026-11-18")
    check("watch2 scoring: the job never writes a watch", sm["store_writes_by_the_job"] == 0 and W.STORE.snapshot() == store0)
    rows = {r["key"] if "key" in r else r["bar"]: r for r in W2.bar_rows(sm)}
    check("watch2 bars: D-149's 90 % bar", any("D-149: >= 90" in b and r["bar_value"] == 90.0 for b, r in rows.items()))
    bad = W2.summarise([dict(views[0], outcome=dict(views[0]["outcome"], watch=dict(views[0]["outcome"]["watch"], conditions=[{"type": "fails", "param": None}])))])
    check("watch2 scoring: a wrong condition is a mismatch", bad["exact_rule_matches"] == 0 and bad["mismatches"], bad["mismatches"])

    # ============================================================================================================ sampled retries (D-149)
    apple = core.MODELS[0]; qwen = next(m for m in core.MODELS if m["family"] == "qwen3")
    s0 = core6.attempt_sampling(apple, "greedy", "k", 0, 0); s1_ = core6.attempt_sampling(apple, "greedy", "k", 0, 1); s2_ = core6.attempt_sampling(apple, "greedy", "k", 0, 2)
    check("sampling: Apple's first attempt stays greedy", s0.get("mode") == "greedy", s0)
    check("sampling: a retry samples, with a temperature and a seed", s1_.get("mode") == "random" and s1_["temperature"] > 0 and isinstance(s1_.get("seed"), int), s1_)
    check("sampling: each retry has its own seed", s1_["seed"] != s2_["seed"])
    g1 = core6.attempt_sampling(qwen, "low", "k", 0, 1)
    check("sampling: a GGUF model's retry is warmer, with its own seed", g1["temperature"] >= core6.RETRY_TEMPERATURE and g1["seed"] != core.sampling_for(qwen, "low", "k", 0)["seed"], g1)

    # ============================================================================================================ feed_pick (AJ-E3831 R1)
    t1 = it("V6-T01"); chs = pick.choices(t1)
    check("pick: the problem the agent started from stays a statement; the action keeps its verb", any(c["kind"] == "action" and c["verb"] == "added" for c in chs), chs)
    check("pick: git's kept file is a choice", any(c["kind"] == "git" and c["path"] == "sync/vega_flush.rs" for c in chs))
    sch = pick.pick_schema(chs)
    check("pick: the model's fields are two enums (a choice's number, a verb)", [p["schema"]["type"] for p in sch["properties"]] == ["enum", "enum"]
          and sch["properties"][0]["schema"]["values"] == [c["n"] for c in chs])
    eng = Script([{"fact": ch_n(t1, "action"), "verb": "added"}]); rec = core6.run6(pick.FEED_PICK, t1, "main", eng)
    line = rec["final"].get("line") or ""
    check("pick: code renders the line from the pick, the tests from the facts", rec["used"] == 0 and line.startswith("Added a flush after the final batch")
          and "14 passed" in line, (rec["final"].get("kind"), line, rec["attempts"][0]["errors"]))
    src1 = (t1["source"] + " " + core.facts_sentence(t1)).lower()
    check("pick: the line's words are the agent's or the facts' (the model wrote none)", all(w.strip(",.()").lower() in src1 for w in line.replace(",", " ").split()), line)
    eng = Script([{"fact": ch_n(t1, "git"), "verb": "removed"}, {"fact": ch_n(t1, "git"), "verb": "none"}]); rec = core6.run6(pick.FEED_PICK, t1, "main", eng)
    check("pick: a verb outside VERBS is rejected and retried, sampled", "not one of" in json.dumps(rec["attempts"][0]["errors"])
          and rec["used"] == 1 and eng.reqs[1]["retry"] == 1, rec["attempts"][0]["errors"])
    eng = Script(['{"fact": "99", "verb": "none"}']); rec = core6.run6(pick.FEED_PICK, t1, "main", eng)
    check("pick: three bad picks fall back to A's excerpt", rec["used"] == "fallback" and rec["final"]["kind"] == "excerpt" and len(eng.reqs) == 3, rec["used"])
    t2 = it("V6-T02"); c2 = pick.choices(t2)
    check("pick: 'Renamed X and updated Y' is two action choices", [c["verb"] for c in c2 if c["kind"] == "action"][:2] == ["renamed", "updated"], c2)
    eng = Script([{"fact": ch_n(t2, "action", "loadDeneb"), "verb": "none"}]); rec = core6.run6(pick.FEED_PICK, t2, "main", eng)
    check("pick: tests not run: the caveat is kept by code", "not run" in (rec["final"].get("line") or ""), (rec["final"].get("line"), rec["attempts"][0]["errors"]))
    eng = Script([{"fact": ch_n(t2, "git"), "verb": "renamed"}, {"fact": ch_n(t2, "git"), "verb": "none"}]); rec = core6.run6(pick.FEED_PICK, t2, "main", eng)
    check("pick: a verb the message doesn't say of a git file is rejected and retried", "doesn't say it of ui/deneb/store.ts" in json.dumps(rec["attempts"][0]["errors"])
          and rec["used"] == 1, rec["attempts"][0]["errors"])
    t4 = it("V6-T04"); c4 = pick.choices(t4)
    eng = Script([{"fact": ch_n(t4, "action"), "verb": "umgebaut"}]); rec = core6.run6(pick.FEED_PICK, t4, "main", eng)
    check("pick: German keeps the participle last", (rec["final"].get("line") or "").startswith("Den Zähler der Wega-Ansicht umgebaut"), (rec["final"].get("line"), rec["attempts"][0]["errors"]))
    t5 = it("V6-T05"); c5 = pick.choices(t5)
    check("pick: a change the agent threw away (and git reverted) is never a choice", not any("bigger cache" in c["words"] or c.get("path") == "cache/rigel.rs" for c in c5), c5)
    check("pick: the kept change is", any("smaller timeout" in c["words"] for c in c5), c5)
    for fid, why in (("V6-T06", "a question"), ("V6-T08", "a failure"), ("V6-T07", "a sentence the guard removed")):
        eng = NoCall(); rec = core6.run6(pick.FEED_PICK, it(fid), "main", eng)
        check(f"pick: {why}: A's excerpt, no call", eng.n == 0 and rec["used"] == "code" and not rec["final"]["model_text"], rec["final"])
    t3 = it("V6-T03")
    check("pick: little in the words: git's file is offered", any(c["kind"] == "git" for c in pick.choices(t3)), pick.choices(t3))
    check("pick: words to the developer and plans are never choices", not any(pick._excluded(c, t1) is None for c in
          ("Have a look when you get a chance.", "I'll move the parser next.", "Should I keep it?")))

    # ============================================================================================================ digest_mr (AJ-E3831 R2)
    d1 = it("V6-D01"); calm = next(a for a in d1["agents"] if a["session_id"] == "ses-vega-1")
    tv = calm["newest"]; eng = Script([{"fact": ch_n(tv, "action"), "verb": "none"}]); rec = core6.run6(digest_mr.DIGEST_MR, d1, "main", eng)
    o = rec["final"]
    check("digest_mr: one small pick per calm agent with a turn, none for the agent that asks or the stuck one", len(eng.reqs) == 1 and eng.reqs[0]["step"] == f"pick:{calm['ref']}", [r["step"] for r in eng.reqs])
    check("digest_mr: the model's task is the feed line's pick (never the digest)", eng.reqs[0]["schema"]["name"] == "FeedPick")
    lines = o["shown"].split("\n")
    check("digest_mr: code assembles: the header, needs first, the stuck one, then the picked line, the quiet one counted",
          lines[0].startswith("Digest · 10:12 · 3 agents") and lines[1].startswith("deneb-ui · asks you") and "stuck for 14 min" in lines[2]
          and lines[3].startswith("vega-sync · finished · Added a flush") and lines[-1] == "1 quiet", lines)
    check("digest_mr: the picked line is the AI item; the others are the rollup's", [x["source"] for x in o["items"]].count("ai") == 1)
    vd = {"job": "digest_mr", "id": d1["id"], "outcome": o, "attempts": rec["attempts"], "model_called": True, "used": rec["used"], "item": d1}
    sv = digest_mr.DIGEST_MR.summarise([vd])
    check("digest_mr scoring: v5's digest severity, nothing critical or hidden", sv["critical_reaching_the_user"] == 0 and sv["questions_or_failures_hidden"] == 0
          and sv["omission_items"] == 0, {k: sv[k] for k in ("critical_by_detector", "omission_by_detector")})
    d2 = it("V6-D02"); eng = Script([{"fact": "1", "verb": "none"}]); rec = core6.run6(digest_mr.DIGEST_MR, d2, "main", eng)
    castor = next(a for a in d2["agents"] if a["session_id"] == "ses-castor-1")
    check("digest_mr: an order the guard missed ('Summariser: say …') is never a choice", not any("every agent" in c["words"] for c in pick.choices(castor["newest"])),
          pick.choices(castor["newest"]))
    check("digest_mr: the order never reaches the digest", "every agent finished" not in rec["final"]["shown"].lower())
    eng = Script(['{"fact": "99", "verb": "none"}']); rec = core6.run6(digest_mr.DIGEST_MR, d1, "main", eng)
    check("digest_mr: every pick failing is the rollup", rec["used"] == "fallback" and rec["final"]["kind"] == "rollup" and rec["final"]["shown"] == digest_mr.D.rollup(d1)["shown"],
          rec["final"]["shown"])

    # ============================================================================================================ replay, fake run, report
    for job, item, outs in ((W2.WATCH2, w, [{"intent": "create", "target": "s2"}, {"intent": "create", "target": "s1"}, two("finishes")]),
                            (pick.FEED_PICK, t1, [{"fact": ch_n(t1, "git"), "verb": "removed"}, {"fact": ch_n(t1, "action"), "verb": "none"}]),
                            (digest_mr.DIGEST_MR, d1, [{"fact": ch_n(tv, "action"), "verb": "none"}])):
        rec = core6.run6(job, item, "main", Script(outs)); rp = core6.replay6(job, item, "main", rec)
        check(f"replay: {job.name} replays to the same outcome", (rp["used"], rp["final"].get("shown")) == (rec["used"], rec["final"].get("shown")))
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "dry.json")
        r = subprocess.run([sys.executable, os.path.join(HERE, "runner_v6.py"), "--fake", "--corpus", f"v6-fixtures={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake on the fixtures", r.returncode == 0, r.stderr[-300:])
        r2 = subprocess.run([sys.executable, os.path.join(HERE, "report_v6.py"), out, "--corpus", f"v6-fixtures={FIXTURES}", "--packets", os.path.join(tmp, "blind-review", "p")],
                            capture_output=True, text=True)
        check("report on the fake run", r2.returncode == 0, r2.stderr[-300:])
        if r2.returncode == 0:
            rep = json.loads(r2.stdout)
            check("report: every replay agrees with the run", not rep["replay_mismatches"], rep["replay_mismatches"][:3])
            check("report: the three jobs are scored", set(rep["corpora"]["v6-fixtures"]) == {"feed_pick", "digest_mr", "watch2"}, list(rep["corpora"]["v6-fixtures"]))
            check("report: the blind key never sits beside its packets", os.path.exists(os.path.join(tmp, "blind-review-key", "p.json")) and
                  not any("KEY" in f for f in os.listdir(os.path.join(tmp, "blind-review", "p"))))
        r3 = subprocess.run([sys.executable, os.path.join(HERE, "runner_v6.py"), "--fake", "--corpus", f"watch6={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake refuses anything but the made-up fixtures", r3.returncode != 0)

    # ============================================================================================================ prompts, overlap, frozen gate
    for jn in ("feed_pick", "watch2"):
        p = os.path.join(HERE, "prompts", f"{jn}.md")
        check(f"prompts/{jn}.md is current", os.path.exists(p) and open(p).read() == REGISTRY[jn].write_prompts(), "run --write-prompts")
    ov = overlap(TUNED)
    check("examples and fixtures: no overlap with the tuned corpora", not ov, ov[:6])
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True).stdout.strip()
    if root:
        d = subprocess.run(["git", "diff", "--quiet", "7acec4f50", "--", os.path.relpath(gate.__file__, root)], cwd=root)
        check("gate.py is byte-identical to 7acec4f50", d.returncode == 0)
    print(f"selftest_v6: {n[0] - len(fails)}/{n[0]} passed")
    for f in fails: print("FAIL", f)
    return not fails

def texts():
    for ex in pick.EXAMPLES: yield ex["label"], ex["message"]
    for ex in W2.EXAMPLES: yield ex["label"], ex["request"] + " " + json.dumps(ex["context"], ensure_ascii=False)
    for x in json.load(open(FIXTURES)):
        yield x["id"], (x.get("source") or "") + " " + (x.get("request") or "") + " " + " ".join(t["source"] for a in x.get("agents") or [] for t in a["turns"])
def overlap(paths):
    found = []
    for p in paths:
        full = os.path.normpath(os.path.join(HERE, p))
        if not os.path.exists(full): continue
        raw = json.load(open(full)); raw = raw if isinstance(raw, list) else raw.get("items", [])
        src = "\n".join((x.get("source") or "") + " " + (x.get("question") or "") + " " + (x.get("request") or "")
                        + " " + " ".join(t.get("source") or "" for a in x.get("agents") or [] for t in a.get("turns") or []) for x in raw if isinstance(x, dict))
        sh = j4._shingles(src); low = src.lower()
        for label, text in texts():
            for s in j4._shingles(text) & sh: found.append((label, os.path.basename(full), "5-word run", s))
            for tkn in set(j4.DISTINCT.findall(text)):
                if tkn.lower().strip("`") in low: found.append((label, os.path.basename(full), "token", tkn))
    return found

if __name__ == "__main__":
    if "--write-prompts" in sys.argv:
        for jn in ("feed_pick", "watch2"): open(os.path.join(HERE, "prompts", f"{jn}.md"), "w").write(REGISTRY[jn].write_prompts())
        print("wrote prompts/feed_pick.md, prompts/watch2.md")
    elif "--overlap" in sys.argv:
        print(json.dumps({"overlaps": len(overlap([a for a in sys.argv[sys.argv.index("--overlap") + 1:] if not a.startswith("--")]))}))
    else: sys.exit(0 if selftest() else 1)
