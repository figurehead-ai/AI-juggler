#!/usr/bin/env python3
"""replay_v1.py corpus-v2.json results-apple-wrapped-*.json results-apple-*.json : AJ-E3709's v1 misses as fake drafts through the v2 gate
(ADR-0033 section 6 'Tests': the wrapped run's six polarity misses plus run A's A01; each must be rejected). Stdlib only."""
import json, sys
import gate
C = {i["id"]: i for i in json.load(open(sys.argv[1]))}
def outputs(path):
    d = json.load(open(path)); return {r["id"]: r["output"] for r in d["results"]}
B = outputs(sys.argv[2]); A = outputs(sys.argv[3])
cases = [("M04","B"),("M29","B"),("L21","B"),("A06","B"),("A07","B"),("A13","B"),("A01","A"),("A06","A"),("A07","A")]
rows = []
for id_, run in cases:
    it = C[id_]; draft = (B if run == "B" else A)[id_]
    cleaned, removed = gate.guard(it["source"])
    steps = {"2": gate.step2(cleaned, draft), "3": gate.step3(cleaned, draft, it["facts"]), "4": gate.step4(cleaned, draft)[:2], "5": gate.step5(removed, draft)}
    steps_unk = gate.step3(cleaned, draft, None)
    failed = [k for k, v in steps.items() if not v[0]]
    rows.append({"item": id_, "run": run, "draft": draft, "rejected_by_steps": failed, "reasons": {k: steps[k][1] for k in failed},
                 "rejected": bool(failed), "step3_with_facts_unknown": [steps_unk[0], steps_unk[1]]})
json.dump(rows, open(sys.argv[4] if len(sys.argv) > 4 else "/dev/stdout", "w"), indent=1, ensure_ascii=False)
