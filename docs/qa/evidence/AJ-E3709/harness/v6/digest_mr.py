#!/usr/bin/env python3
"""digest_mr.py: harness v6's agent digest, map and reduce (AJ-E3831 R2, D-149 point 3). Runbook: v6-runbook.md.

v5 round 3: asked to compose a whole digest in one go, Apple's digests were safe but preferred only 33 % of the time over the code
rollup. v6 never lets the model compose it:

  map     each changed agent that doesn't need you, with a turn: its newest turn's feed line by pick.py ("pick, don't write"), one small
          call per agent, retried and sampled as the feed line is. An agent that needs you, has no turn, or whose pick falls back keeps
          its rollup line (v5's rollup_line, code).
  reduce  code: each agent's line is its name, its state word, the facts that want a look (stuck, runway, processes left), the picked
          line, and what git undid; then v5's compose: needs and failures first, then notable, then by newest activity; at most 6;
          "and N more agents · Q quiet".

Scored as v5's digest (severity per agent item; must_surface; quiet agents; the structural rows), and blind-paired against the rollup."""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core6                                                        # noqa: E402
from core6 import MAX_RETRIES                                       # noqa: E402
from core import Job, register                                      # noqa: E402
import digest as D                                                  # noqa: E402  (v5: the window, the rollup, compose, findings)
import pick                                                         # noqa: E402

class SubAsk:
    """an asker for one agent's pick: the step is named after the agent, so a replay finds each call in order"""
    def __init__(self, ask, ref): self.ask = ask; self.ref = ref
    def __call__(self, step, req, retry): return self.ask(f"{step}:{self.ref}", req, retry)

def agent_line(a, picked):
    bits = [a["name"], a["state"]] + D.fact_bits(a) + [picked]
    uw = D.undo_word(a)
    if uw and uw not in picked: bits.append(uw)
    tw = D.tests_word(a)
    if tw and not any(w in picked.lower() for w in ("passed", "failed", "not run", "tests")): bits.append(tw)
    return " · ".join(bits)

class DigestMapReduce(Job):
    name, letter, title, item_kind = "digest_mr", "e6", "agent digest, map and reduce", "digest"
    record_keys = ("items", "visible")
    def run(self, d, setting, ask, mark=True, retries=MAX_RETRIES):
        items = []; worst = 0; called = 0; ai = 0; fell = 0; picks = {}
        for a in d["agents"]:
            if not a["changed"]: continue
            n = a["newest"]; line = None
            if not a["loud"] and n is not None:
                final, used = pick.FEED_PICK.run(n, setting, SubAsk(ask, a["ref"]), mark, retries)
                if used != "code": called += 1
                if isinstance(used, int): worst = max(worst, used)
                if used == "fallback": fell += 1
                if final.get("model_text") and final.get("line"):
                    line = final["line"]; picks[a["ref"]] = final.get("pick")
            if line:
                ai += 1
                items.append({"agent": a["ref"], "line": agent_line(a, line), "notable": bool(a["notable_facts"]), "source": "ai", "words": [line],
                              "cites": [{"session_id": a["session_id"], "turn_id": n["turn_id"]}]})
            else:
                items.append({"agent": a["ref"], "line": D.rollup_line(a), "notable": bool(a["notable_facts"]) or a["loud"], "source": "rollup"})
        c = D.compose(d, items)
        out = {"ok": True, "kind": "ai" if ai else "rollup", "shown": c["shown"], "model_text": [w for x in items if x["source"] == "ai" for w in x["words"]],
               "items": c["items"], "visible": [x["agent"] for x in c["visible"]], "errors": [], "dropped_loud": 0, "fell_back_items": fell,
               "picks": picks, "notes": [f"{ai} agent line(s) picked by the model, {len(items) - ai} by code"]}
        if not called: return out, "code"
        return out, ("fallback" if not ai else worst)
    def fallback(self, d, setting): return D.DIGEST._rollup_outcome(d, "the rollup")
    def alternative(self, d, setting): return D.rollup(d)["shown"]
    def view_extras(self, d, outcome): return {"item": d}
    def packet_context(self, d): return D.DIGEST.packet_context(d)
    def summarise(self, views): return D.summarise(D.DIGEST, views)
    def bar_rows(self, sev, value_pct=None): return D.bar_rows(sev, value_pct)
    def write_prompts(self): return ""            # the model's only task is pick.py's (prompts/feed_pick.md)
    def fake_draft(self, req, prompt, bad, k): return pick.FEED_PICK.fake_draft(req, prompt, bad, k)
DIGEST_MR = register(DigestMapReduce())
