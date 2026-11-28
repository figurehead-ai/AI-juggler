#!/usr/bin/env python3
"""AJ-E3831 harness v14: the blind material for gold writers and judges,
and the value count. Every reader sees the same turn text (the request,
the goal, every agent message whole, the commands, the files changed, the
tests: what the app's reader read, in full) and the one definition of good
(`../../../AJ-E3831/definition-of-good.md`). Nobody sees prompts, code or
which line is the model's.

    python3 packets.py turns <read.json> <out.md>             # for the gold writers
    python3 packets.py pairs <round.json> <gold.json> <out-dir> --seed S
    python3 packets.py value <review.json> <key.json> [<review2.json> …]
    python3 packets.py anonymize <read.json> <out.json>        # the committed copy

`pairs` writes `<out-dir>/packet.md` (turn, reference line, A, B) and
`<out-dir>-key.json` (which side is the model's), kept apart. A review is
`{"<pair>": {"pick": "A"|"B"|"same", "critical": {"A": "…"|null, "B": "…"|null}}}`.
"""
from __future__ import annotations

import json
import random
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]


def turn_text(v: dict) -> str:
    out = [f"REQUEST (the user's message that started the turn):\n{v['request']}"]
    if v.get("goal") and v["goal"].strip() != v["request"].strip():
        out.append(f"SESSION GOAL (the session's opening request):\n{v['goal']}")
    msgs = v["messages"]
    for i, m in enumerate(msgs):
        label = "AGENT'S REPLY (its last message)" if i == len(msgs) - 1 else f"AGENT MESSAGE {i + 1}"
        out.append(f"{label}:\n{m}")
    c = v["calls"]
    tools = ", ".join(f"{t} {n}" for t, n in c["by_tool"]) or "none"
    if c.get("hidden"):
        tools += f", {c['hidden']} earlier calls not listed"
    cmds = "\n".join(f"  - `{cmd}`{' (failed)' if failed else ''}" for cmd, failed in c["commands"]) or "  none"
    out.append(f"TOOL CALLS: {tools}\nCOMMANDS RUN:\n{cmds}")
    ch = v.get("changes")
    if ch:
        files = "\n".join(
            f"  - {f['name']}" + (f" (+{f['added']} -{f['removed']})" if f.get("added") is not None else "")
            for f in ch["per_file"])
        out.append(f"FILES CHANGED BY EDIT CALLS ({ch['files']}):\n{files}")
    else:
        out.append("FILES CHANGED BY EDIT CALLS: none seen (changes made through shell commands aren't counted)")
    last = c.get("last_test_failed")
    out.append("TEST RUNS: " + ("none" if last is None else f"{c['test_runs']}; the last one {'failed' if last else 'passed'}"))
    return "\n\n".join(out)


def turns(read: str, out: str) -> None:
    rows = [r for r in json.load(open(read)) if "view" in r]
    parts = [f"## Turn {r['id']}\n\n{turn_text(r['view'])}\n" for r in rows]
    Path(out).write_text("\n\n---\n\n".join(parts))
    print(f"{len(rows)} turns -> {out}")


def pairs(round_: str, gold: str, out_dir: str, seed: int) -> None:
    rows = [r for r in json.load(open(round_)) if r.get("line") and r.get("plain")]
    ref = json.load(open(gold))
    rnd = random.Random(seed)
    rnd.shuffle(rows)
    key, parts = {}, []
    for n, r in enumerate(rows, 1):
        pid = f"P{n:02d}"
        ai_first = rnd.random() < 0.5
        a, b = (r["line"], r["plain"]) if ai_first else (r["plain"], r["line"])
        key[pid] = {"turn": r["id"], "ai": "A" if ai_first else "B"}
        g = ref.get(r["id"], {})
        parts.append(
            f"## Pair {pid}\n\n{turn_text(r['view'])}\n\nREFERENCE LINE (written blind from the definition): {g.get('line', '(none)')}\n"
            f"REFERENCE OUTCOME: {g.get('status', '?')}: {g.get('note', '')}\n\n**A:** {a}\n\n**B:** {b}\n")
    d = Path(out_dir)
    d.mkdir(parents=True, exist_ok=True)
    (d / "packet.md").write_text("\n\n---\n\n".join(parts))
    Path(str(d) + "-key.json").write_text(json.dumps(key, indent=1))
    print(f"{len(rows)} pairs -> {d}/packet.md (key apart)")


def value(review_paths: list[str], key_path: str) -> None:
    key = json.load(open(key_path))
    pooled = {"ai": 0, "plain": 0, "same": 0}
    crit_ai: dict[str, list[str]] = {}
    crit_plain: dict[str, list[str]] = {}
    for p in review_paths:
        rv = json.load(open(p))
        c = {"ai": 0, "plain": 0, "same": 0}
        for pid, k in key.items():
            r = rv.get(pid)
            if not r:
                continue
            pick = r["pick"]
            c["same" if pick == "same" else ("ai" if pick == k["ai"] else "plain")] += 1
            other = "B" if k["ai"] == "A" else "A"
            if (r.get("critical") or {}).get(k["ai"]):
                crit_ai.setdefault(pid, []).append(f"{Path(p).stem}: {r['critical'][k['ai']]}")
            if (r.get("critical") or {}).get(other):
                crit_plain.setdefault(pid, []).append(f"{Path(p).stem}: {r['critical'][other]}")
        dec = c["ai"] + c["plain"]
        print(f"{Path(p).stem}: AI {c['ai']} / plain {c['plain']} / same {c['same']} -> {100 * c['ai'] / dec if dec else 0:.1f} % of decided")
        for k2 in pooled:
            pooled[k2] += c[k2]
    dec = pooled["ai"] + pooled["plain"]
    print(f"POOLED: AI {pooled['ai']} / plain {pooled['plain']} / same {pooled['same']} -> {100 * pooled['ai'] / dec if dec else 0:.1f} % of decided (bar 60 %)")
    print(f"shown AI lines: {len(key)}; flagged critical by a judge: {len(crit_ai)} ({100 * len(crit_ai) / max(1, len(key)):.1f} %)")
    for pid, why in sorted(crit_ai.items()):
        print(f"  AI {pid} ({key[pid]['turn']}): {' | '.join(why)}")
    for pid, why in sorted(crit_plain.items()):
        print(f"  plain {pid} ({key[pid]['turn']}): {' | '.join(why)}")


def anonymize(read: str, out: str) -> None:
    text = Path(read).read_text()
    clean = subprocess.run([sys.executable, str(ROOT / "scripts/qa/anonymize.py")], input=text,
                           capture_output=True, text=True, check=True).stdout
    rows = json.loads(clean)
    for r in rows:
        r.pop("transcript", None)
    Path(out).write_text(json.dumps(rows, indent=1, ensure_ascii=False))
    print(f"{len(rows)} rows -> {out}")


if __name__ == "__main__":
    a = sys.argv[1:]
    if a[:1] == ["turns"]:
        turns(a[1], a[2])
    elif a[:1] == ["pairs"]:
        pairs(a[1], a[2], a[3], int(a[a.index("--seed") + 1]) if "--seed" in a else 14)
    elif a[:1] == ["value"]:
        value(a[1:-1], a[-1])
    elif a[:1] == ["anonymize"]:
        anonymize(a[1], a[2])
    else:
        print(__doc__)
        sys.exit(2)
