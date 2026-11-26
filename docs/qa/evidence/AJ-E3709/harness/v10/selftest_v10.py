#!/usr/bin/env python3
"""selftest_v10.py: self-tests for harness v10 (AJ-E3830 watch6), on made-up items and the misses' own requests
(v7 and v8 `summary.md`; W32 and W51 in fresh words of their recorded drafts' shape). heldout/v10 stays blind to this code.

  python3 selftest_v10.py                  run every self-test
  python3 selftest_v10.py --write-prompts  regenerate prompts/watch6.md"""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                         # noqa: E402
from core import REGISTRY, norm_item, gate                           # noqa: E402
import watch6 as W6, g11                                             # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "v10-selftest.json")
def fx(): return {x["id"]: x for x in json.load(open(FIXTURES))}
class Fail:
    def __init__(s): s.n = 0
    def __call__(s, req): s.n += 1; raise AssertionError("the model was called")
def selftest():
    fails = []; n = [0]
    def check(name, cond, detail=""):
        n[0] += 1
        if not cond: fails.append(f"{name}: {detail}")
    views = []
    for fid, raw in fx().items():
        w = norm_item(raw); eng = Fail(); rec = core6.run6(W6.WATCH6, w, "main", eng); o = rec["final"]
        views.append({"job": "watch6", "id": fid, "outcome": o, "attempts": rec["attempts"], "model_called": False, "used": rec["used"], "item": w})
        if fid.startswith("M-") or fid in ("N-W17", "N-W47"):
            tg = [t["kind"] for t in (o.get("watch") or {}).get("targets") or []]
            check(f"{fid}: told back with the nearest, never a watch, no call ({raw['request']})", o.get("kind") == "unsupported" and o.get("nearest") and eng.n == 0,
                  (o.get("kind"), o.get("shown")))
            check(f"{fid}: never the excluded one, never widened", not tg, tg)
    sm = W6.WATCH6.summarise(views)
    check("scoring: 0 unsupported claimed", sm["unsupported_claimed"] == 0, sm["unsupported_claimed"])
    check("scoring: v9's misses and v9's kept shapes exact, strict too (lifetimes said in words, the entity wins, no invented number)",
          sm["exact_rule_matches_with_defaults"] == sm["supported_items"] == 22 and sm["exact_rule_matches"] == 22, sm["mismatches"])
    check("scoring: the nearest on every unsupported request", sm["nearest_offered"] == sm["nearest_due"] == 10, (sm["nearest_offered"], sm["nearest_due"]))
    # G11
    check("G11 selftest", g11.selftest())
    d = g11.digest_v6() if os.path.exists(os.path.join(H, "..", "heldout", "v6", "digest-v6.json")) else None
    if d: check("G11 over v6's digest lines: counted", d["ai_lines_shown"] == 25, d["ai_lines_shown"])
    b = g11.bot_e3829()
    check("G11 over Bot's recorded real answers: 0 refused", b["refused_by_g11"] == 0, b)
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "dry.json")
        r = subprocess.run([sys.executable, os.path.join(HERE, "runner_v10.py"), "--fake", "--corpus", f"v10-fixtures={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake on the fixtures", r.returncode == 0, r.stderr[-400:])
        r2 = subprocess.run([sys.executable, os.path.join(HERE, "report_v10.py"), out, "--corpus", f"v10-fixtures={FIXTURES}"], capture_output=True, text=True)
        check("report on the fake run", r2.returncode == 0, r2.stderr[-400:])
        if r2.returncode == 0:
            rep = json.loads(r2.stdout); b9 = rep["corpora"]["v10-fixtures"]["watch6"]
            check("report: replays agree; the hard bar first", not rep["replay_mismatches"] and b9["bar_check"][0]["bar"].startswith("unsupported claimed"), rep["replay_mismatches"][:2])
    p = os.path.join(HERE, "prompts", "watch6.md")
    check("prompts/watch6.md is current", os.path.exists(p) and open(p).read() == REGISTRY["watch6"].write_prompts(), "run --write-prompts")
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True).stdout.strip()
    if root:
        dd = subprocess.run(["git", "diff", "--quiet", "7acec4f50", "--", os.path.relpath(gate.__file__, root)], cwd=root)
        check("gate.py is byte-identical to 7acec4f50", dd.returncode == 0)
    print(f"selftest_v10: {n[0] - len(fails)}/{n[0]} passed")
    for f in fails: print("FAIL", f)
    return not fails
if __name__ == "__main__":
    if "--write-prompts" in sys.argv:
        open(os.path.join(HERE, "prompts", "watch6.md"), "w").write(REGISTRY["watch6"].write_prompts()); print("wrote prompts/watch6.md")
    else: sys.exit(0 if selftest() else 1)
