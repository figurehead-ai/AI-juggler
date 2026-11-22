#!/usr/bin/env python3
"""excerpt_only.py: A's excerpt alone, eval-3 beside eval-4, over the corpora; no engine is called (the excerpt needs none). Stdlib only.

  python3 excerpt_only.py --corpus v2=../corpus-v2-8f7c7474.json --corpus turnends=../corpus-v3-turnends.json \
      --corpus heldout=../heldout/corpus-heldout.json [--md OUT.md] > OUT.json

Per corpus, facts setting (main and gate_only use the item's facts, so the excerpt is the same in both; unknown passes none) and gate:
  caveat dropped   the text states a caveat and the shown excerpt carries none, read by eval-3's lists (report_v3's yardstick) and by
                   eval-4's structure (every caveat kind the text states, unverified and incomplete, must be on a shown line)
  lost             a gold 'fail' item whose excerpt shows no failure, or a gold 'question' item whose excerpt asks nothing; eval-3's
                   reading (report_v3: failure words) and eval-4's (a line that reports a failure by structure, or a negated success)
  Each gate's own reading flatters it; gold misses (hand-set must_match patterns) are the reading neither gate wrote.
  gold misses      information: gold must_match patterns the cleaned text matches and the excerpt does not
  flags            eval-4's record (the L3 net's "facts line carries the failure"; a caveat with no room)
  calm changes     calm items (facts L0, finished) whose excerpt eval-4 changes: the lines it adds, to read for false positives
Counts are items (the excerpt is deterministic; report_v3 counts trials, three per item). Never run on the real held-out set to tune
a rule (D-124)."""
import argparse, hashlib, json, re
import gate, gate_v4
SETTINGS = ("main", "gate_only", "unknown")
def excerpt(g, cleaned, facts):
    if g == "eval-3":
        t, sec = gate.excerpt_v3(cleaned, facts); return t, sec, []
    return gate_v4.excerpt_v4(cleaned, facts)
def lost(reader, gold, shown):
    if not gold: return None
    if gold.get("verdict") == "fail":
        v = gate.verdict_words_v3(shown)
        says = (v["fail"] or v["nsucc"]) if reader == "eval-3" else gate_v4.shown_says_failure(shown)
        return None if says else "failure"
    if gold.get("verdict") == "question" and "?" not in shown: return "question"
    return None
def dropped(reader, cleaned, shown):
    if reader == "eval-3":
        req = gate.caveat_required(cleaned, None)
        return [f"{k}: {v}" for k, v in req] if req and not gate.says_caveat(shown, req) else []
    return gate_v4.caveat_dropped(cleaned, shown)
def calm(F): return bool(F) and F["level"] == "L0" and F["outcome"] == "finished"
def corpus(C):
    out = {"items": len(C), "settings": {}}
    ctx = {i: gate.prepare(C[i]["source"]) for i in C}
    for st in SETTINGS:
        row = {}
        shown = {g: {} for g in ("eval-3", "eval-4")}
        for g in ("eval-3", "eval-4"):
            d = {"caveat_dropped_by_eval3": [], "caveat_dropped_by_eval4": [], "lost_by_eval3": [], "lost_by_eval4": [], "gold_misses": [],
                 "flags": {}, "lines": {}}
            for i, it in C.items():
                facts = None if st == "unknown" else it["facts"]; cl = ctx[i]["cleaned"]
                t, sec, fl = excerpt(g, cl, facts); shown[g][i] = t
                d["lines"][t.count("\n") + 1] = d["lines"].get(t.count("\n") + 1, 0) + 1
                for f in fl: d["flags"].setdefault(f.split(":")[0], []).append(i)
                for reader in ("eval-3", "eval-4"):
                    x = dropped(reader, cl, t)
                    if x: d[f"caveat_dropped_by_{reader.replace('-', '')}"].append({"id": i, "missing": x, "shown": t})
                    y = lost(reader, it.get("gold"), t)
                    if y: d[f"lost_by_{reader.replace('-', '')}"].append({"id": i, "lost": y, "shown": t})
                for p in (it.get("gold") or {}).get("must_match", []):
                    if re.search(p, cl, re.I) and not re.search(p, t, re.I): d["gold_misses"].append({"id": i, "pattern": p})
            row[g] = d
        F = {i: gate.facts_norm(C[i]["facts"]) for i in C}
        row["calm_changes"] = [{"id": i, "level": (F[i] or {}).get("level"), "eval-3": shown["eval-3"][i], "eval-4": shown["eval-4"][i]}
                               for i in C if calm(F[i]) and shown["eval-3"][i] != shown["eval-4"][i]]
        row["changed_items"] = sorted(i for i in C if shown["eval-3"][i] != shown["eval-4"][i])
        out["settings"][st] = row
    return out
def table(rep):
    L = ["| Corpus | Setting | Gate | Caveat dropped (eval-3 reading / eval-4 reading) | Lost failure or question (eval-3 / eval-4 reading) | Gold misses | L3 flagged | Calm items changed |",
         "|---|---|---|---|---|---|---|---|"]
    for c, o in rep.items():
        for st in SETTINGS:
            for g in ("eval-3", "eval-4"):
                d = o["settings"][st][g]
                L.append(f"| {c} ({o['items']}) | {st} | {g} | {len(d['caveat_dropped_by_eval3'])} / {len(d['caveat_dropped_by_eval4'])} | "
                         f"{len(d['lost_by_eval3'])} / {len(d['lost_by_eval4'])} | {len(d['gold_misses'])} | "
                         f"{len(d['flags'].get(gate_v4.FACTS_LINE_FLAG, []))} | {len(o['settings'][st]['calm_changes']) if g == 'eval-4' else '—'} |")
    return "\n".join(L) + "\n"
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus", action="append", required=True, metavar="LABEL=PATH"); ap.add_argument("--md")
    a = ap.parse_args(); rep = {}; files = []
    for spec in a.corpus:
        label, _, path = spec.partition("=")
        raw = open(path, "rb").read(); files.append({"label": label, "file": path.rsplit("/", 1)[-1], "sha256": hashlib.sha256(raw).hexdigest()})
        rep[label] = corpus({i["id"]: i for i in json.loads(raw)})
    sha = lambda m: hashlib.sha256(open(m.__file__, "rb").read()).hexdigest()
    out = {"meta": {"gates": ["eval-3", "eval-4"], "gate_py_sha256": sha(gate), "gate_v4_py_sha256": sha(gate_v4), "corpora": files,
                    "engine": "none (excerpt only)"}, "corpora": rep, "table_md": table(rep)}
    if a.md: open(a.md, "w").write("# A's excerpt alone: eval-3 beside eval-4\n\n" + table(rep))
    print(json.dumps(out, indent=1, ensure_ascii=False))
if __name__ == "__main__": main()
