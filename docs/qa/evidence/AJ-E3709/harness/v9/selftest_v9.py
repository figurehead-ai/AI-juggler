#!/usr/bin/env python3
"""selftest_v9.py: self-tests for harness v9 (AJ-E3830 watch5, fail closed; G11's gate), on made-up items and the misses' own requests
(v7 and v8 `summary.md`; W32 and W51 in fresh words of their recorded drafts' shape). heldout/v9 stays blind to this code.

  python3 selftest_v9.py                  run every self-test
  python3 selftest_v9.py --write-prompts  regenerate prompts/watch5.md"""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                         # noqa: E402
from core import REGISTRY, norm_item, gate                           # noqa: E402
import watch5 as W5, g11                                             # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "v9-selftest.json")
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
        w = norm_item(raw); eng = Fail(); rec = core6.run6(W5.WATCH5, w, "main", eng); o = rec["final"]
        views.append({"job": "watch5", "id": fid, "outcome": o, "attempts": rec["attempts"], "model_called": False, "used": rec["used"], "item": w})
        if fid.startswith("M-"):
            tg = [t["kind"] for t in (o.get("watch") or {}).get("targets") or []]
            check(f"{fid}: told back with the nearest, never a watch, no call ({raw['request']})", o.get("kind") == "unsupported" and o.get("nearest") and eng.n == 0,
                  (o.get("kind"), o.get("shown")))
            check(f"{fid}: never the excluded one, never widened", not tg, tg)
    sm = W5.WATCH5.summarise(views)
    check("scoring: 0 unsupported claimed", sm["unsupported_claimed"] == 0, sm["unsupported_claimed"])
    check("scoring: every kept shape exact (selection, a clause that asks, a session over its namesake, a bare once)",
          sm["exact_rule_matches_with_defaults"] == sm["supported_items"] == 6, sm["mismatches"])
    check("scoring: the nearest on every unsupported request", sm["nearest_offered"] == sm["nearest_due"] == 8, (sm["nearest_offered"], sm["nearest_due"]))
    # G11
    check("G11 selftest", g11.selftest())
    d = g11.digest_v6() if os.path.exists(os.path.join(H, "..", "heldout", "v6", "digest-v6.json")) else None
    if d: check("G11 over v6's digest lines: counted", d["ai_lines_shown"] == 25, d["ai_lines_shown"])
    b = g11.bot_e3829()
    check("G11 over Bot's recorded real answers: 0 refused", b["refused_by_g11"] == 0, b)
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "dry.json")
        r = subprocess.run([sys.executable, os.path.join(HERE, "runner_v9.py"), "--fake", "--corpus", f"v9-fixtures={FIXTURES}", "--out", out], capture_output=True, text=True)
        check("runner --fake on the fixtures", r.returncode == 0, r.stderr[-400:])
        r2 = subprocess.run([sys.executable, os.path.join(HERE, "report_v9.py"), out, "--corpus", f"v9-fixtures={FIXTURES}"], capture_output=True, text=True)
        check("report on the fake run", r2.returncode == 0, r2.stderr[-400:])
        if r2.returncode == 0:
            rep = json.loads(r2.stdout); b9 = rep["corpora"]["v9-fixtures"]["watch5"]
            check("report: replays agree; the hard bar first", not rep["replay_mismatches"] and b9["bar_check"][0]["bar"].startswith("unsupported claimed"), rep["replay_mismatches"][:2])
    p = os.path.join(HERE, "prompts", "watch5.md")
    check("prompts/watch5.md is current", os.path.exists(p) and open(p).read() == REGISTRY["watch5"].write_prompts(), "run --write-prompts")
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True).stdout.strip()
    if root:
        dd = subprocess.run(["git", "diff", "--quiet", "7acec4f50", "--", os.path.relpath(gate.__file__, root)], cwd=root)
        check("gate.py is byte-identical to 7acec4f50", dd.returncode == 0)
    print(f"selftest_v9: {n[0] - len(fails)}/{n[0]} passed")
    for f in fails: print("FAIL", f)
    return not fails
if __name__ == "__main__":
    if "--write-prompts" in sys.argv:
        open(os.path.join(HERE, "prompts", "watch5.md"), "w").write(REGISTRY["watch5"].write_prompts()); print("wrote prompts/watch5.md")
    else: sys.exit(0 if selftest() else 1)
