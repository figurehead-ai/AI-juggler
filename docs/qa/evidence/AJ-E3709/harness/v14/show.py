#!/usr/bin/env python3
"""AJ-E3831 harness v14: a round's output for the TUNING set, side by side
(the plain excerpt, the line, each attempt's refusal). Never run it on a
held-out round's output before its judging is done.

    python3 show.py <round.json> [--drafts]
"""
from __future__ import annotations

import json
import statistics
import sys

rows = json.load(open(sys.argv[1]))
drafts = "--drafts" in sys.argv
shown = refused_first = 0
first_ms: list[int] = []
total_ms: list[int] = []
reasons: dict[str, int] = {}
for r in rows:
    if "view" not in r:
        print(f"== {r['id']}: {r.get('error')}")
        continue
    v = r["view"]
    print(f"== {r['id']}  allowed={','.join(v['allowed'])}  q={'y' if v.get('question') else 'n'}")
    print(f"   REQ:   {v['request'][:150]!r}")
    print(f"   PLAIN: {r['plain']}")
    print(f"   LINE:  {r.get('line')}")
    atts = r.get("attempts", [])
    if atts:
        first_ms.append(atts[0]["ms"])
        total_ms.append(r.get("ms_total", 0))
    if r.get("line"):
        shown += 1
    for i, a in enumerate(atts):
        if a["refused"]:
            key = a["refused"].split(":")[0] + ":" + a["refused"].split(":")[1].split("(")[0].strip() if ":" in a["refused"] else a["refused"]
            reasons[key] = reasons.get(key, 0) + 1
            if i == 0:
                refused_first += 1
        if drafts or a["refused"]:
            print(f"     #{i} {a['ms']}ms {'REFUSED ' + a['refused'] if a['refused'] else 'kept'}: {a['draft'][:220]}")
n = sum(1 for r in rows if "view" in r)
print(f"\nshown {shown}/{n}; first draft refused {refused_first}")
if first_ms:
    print(f"first attempt median {statistics.median(first_ms)} ms; total median {statistics.median(total_ms)} ms, max {max(total_ms)} ms")
for k, c in sorted(reasons.items(), key=lambda kv: -kv[1]):
    print(f"  {c:3d}  {k}")
