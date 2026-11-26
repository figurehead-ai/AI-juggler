#!/usr/bin/env python3
"""digest_mr2.py: harness v7's agent digest, map and reduce with G7 (AJ-E3831 R2, D-149's amendment of 2026-11-26).

v6 round: the digest passed (64 % pooled; Beta). Opus's re-judge found one class: for a turn whose change was reverted or abandoned the
picked line dropped what was tried ("No code changes remain"). G7: such a turn keeps the plain excerpt's words (its rollup line, with
git's "reverted" and the agent's last words) in the map step; every other calm agent's line is v7's pick (pick2.py)."""
import os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); V6 = os.path.join(os.path.dirname(HERE), "v6")
for p in (V6, HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402
from core6 import MAX_RETRIES                                       # noqa: E402
from core import register, undo_reported                            # noqa: E402
import claims_v5                                                    # noqa: E402
import digest as D                                                  # noqa: E402
import digest_mr as M6                                              # noqa: E402
import pick2                                                        # noqa: E402

ABANDONED = re.compile(r"\b(?:abandon\w*|gave\s+up|giving\s+up|parked|shelved|backed\s+out|rolled\s+(?:it\s+|them\s+|this\s+|that\s+)?back|"
                       r"no\s+(?:code\s+)?changes?\s+remain|nothing\s+(?:was\s+)?kept|aufgegeben|verworfen|zurückgerollt)\b", re.I)
def reverted_turn(a):
    """G7: the agent's newest turn undid or abandoned its change (git's reverts, the words, the rollup's own undo word)"""
    n = a["newest"]
    if n is None: return False
    if n["git"]["reverted"] or claims_v5.undone_entries(n["git"]): return True
    if D.undo_word(a): return True
    words = n["ctx"]["cleaned"]
    return bool(ABANDONED.search(words)) or any(undo_reported(s) for s in re.split(r"(?<=[.!?])\s+", words) if s)

class DigestMapReduceG7(M6.DigestMapReduce):
    name, letter, title = "digest_mr2", "e7", "agent digest, map and reduce (G7: a reverted turn keeps its plain words)"
    def run(self, d, setting, ask, mark=True, retries=MAX_RETRIES):
        items = []; worst = 0; called = 0; ai = 0; fell = 0; picks = {}; kept_plain = []
        for a in d["agents"]:
            if not a["changed"]: continue
            n = a["newest"]; line = None
            if not a["loud"] and n is not None and reverted_turn(a): kept_plain.append(a["ref"])
            elif not a["loud"] and n is not None:
                final, used = pick2.FEED_PICK2.run(n, setting, M6.SubAsk(ask, a["ref"]), mark, retries)
                if used != "code": called += 1
                if isinstance(used, int): worst = max(worst, used)
                if used == "fallback": fell += 1
                if final.get("model_text") and final.get("line"): line = final["line"]; picks[a["ref"]] = final.get("pick")
            if line:
                ai += 1
                items.append({"agent": a["ref"], "line": M6.agent_line(a, line), "notable": bool(a["notable_facts"]), "source": "ai", "words": [line],
                              "cites": [{"session_id": a["session_id"], "turn_id": n["turn_id"]}]})
            else:
                items.append({"agent": a["ref"], "line": D.rollup_line(a), "notable": bool(a["notable_facts"]) or a["loud"], "source": "rollup"})
        c = D.compose(d, items)
        out = {"ok": True, "kind": "ai" if ai else "rollup", "shown": c["shown"], "model_text": [w for x in items if x["source"] == "ai" for w in x["words"]],
               "items": c["items"], "visible": [x["agent"] for x in c["visible"]], "errors": [], "dropped_loud": 0, "fell_back_items": fell, "picks": picks,
               "kept_plain_g7": kept_plain, "notes": [f"{ai} agent line(s) picked by the model, {len(items) - ai} by code ({len(kept_plain)} kept plain by G7)"]}
        if not called: return out, "code"
        return out, ("fallback" if not ai else worst)
DIGEST_MR2 = register(DigestMapReduceG7())
