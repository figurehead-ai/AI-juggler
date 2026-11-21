#!/usr/bin/env python3
"""blind_review_v3.py: the v3 blind review (ADR-0033 section 6.1) for the realistic turn-end corpus, stdlib only.

Build the packet (the reviewer reads INPUT only; KEY stays unopened until the scores are in):
  python3 blind_review_v3.py build CORPUS_TURNENDS.json REPORT_V3.json INPUT.md KEY.json [--corpus-label turnends] [--sample 10]
Count it afterwards:
  python3 blind_review_v3.py tally REVIEW.md KEY.json

Entries, from the 'main' setting (the design), repeat 0:
  pairs   (P###) every calm item (L0/L1) where a B variant's line differs from A's excerpt: A's excerpt and each distinct B line
          (two lines, or three when the ladder and choose-only differ), unlabelled, in shuffled order, with the source and the facts.
          One entry per item, so no line repeats across entries (a repeated line would give A away). Lines are compared and shown as
          plain text (backticks off), as the feed shows a line; the key keeps the originals.
  singles (S###) every L2 and L3 item (the shown line, as in v2; a gate-only line too where it differs), and a seeded sample of the
          other calm items (where B shows A's excerpt).
The count rule (decided): B is preferred only if it wins more calm items than A; a "same" answer is a tie and counts as A's. Zero FAILs
on B's calm lines is required as well. The tally counts each B variant against A apart."""
import json, random, re, sys
import gate
FENCE = "`" * 3
def facts_words(f):
    F = gate.facts_norm(f)
    o = {"finished": "finished", "failed": "failed or broke", "asking": "stopped and asks a question", "stopped": "stopped",
         "unknown": "not known"}[F["outcome"]]
    if str(f.get("outcome")).lower() == "none": o = "finished, no result stated"
    t = {"unknown": "not known to AI Juggler", "not_run": "none were run"}.get(F["tests_state"]) or f"ran, {F['passed']} passed, {F['failed']} failed"
    lv = {"L0": "L0 (calm, finished)", "L1": "L1 (calm, finished, worth a glance)", "L2": "L2 (needs you: asks something)",
          "L3": "L3 (failed or broke)"}.get(F["level"], str(F["level"]))
    extra = f" Partial work: {F['partial'][0]} of {F['partial'][1]}." if F["partial"] else ""
    return f"Turn: {o}. Tests: {t}. Level: {lv}.{extra}"
def block(text): return [FENCE + "text", text.replace(FENCE, "'''"), FENCE]
def plain(s): return re.sub(r"\s+", " ", (s or "").replace("`", "")).strip()
def plain_lines(s): return "\n".join(plain(l) for l in (s or "").split("\n"))
def quote(line): return ["> " + l for l in line.split("\n")]

def build(corpus_path, report_path, out_md, out_key, label="turnends", sample=10):
    C = {i["id"]: i for i in json.load(open(corpus_path))}
    rep = json.load(open(report_path))["corpora"][label]["_detail"]
    def row(var, st, id_): return next(r for r in rep[var][st] if r["id"] == id_ and r["rep"] == 0)
    ids = list(dict.fromkeys(r["id"] for r in rep["ladder"]["main"]))
    pairs, singles, rest = [], [], []
    for id_ in ids:
        if gate.facts_loud(gate.facts_norm(C[id_]["facts"])):
            m = row("ladder", "main", id_); singles.append((id_, "main", m)); seen = {plain_lines(m["shown"])}
            for var in ("ladder", "choose_only"):
                g = row(var, "gate_only", id_)
                if plain_lines(g["shown"]) not in seen: singles.append((id_, f"{var}/gate_only", g)); seen.add(plain_lines(g["shown"]))
            continue
        lines, exc = {}, None
        for var in ("ladder", "choose_only"):
            r = row(var, "main", id_); exc = r["excerpt"]
            if r["kind"] in ("written", "quoted") and plain_lines(r["shown"]) != plain_lines(r["excerpt"]):
                lines.setdefault(plain_lines(r["shown"]), []).append((var, r))
        if not lines: rest.append(id_); continue
        pairs.append((id_, [("A", plain_lines(exc), None)] + [("B:" + "+".join(v for v, _ in vs), shown, vs[0][1]) for shown, vs in lines.items()]))
    pick = random.Random(11).sample(sorted(rest), min(sample, len(rest)))
    singles += [(id_, "main (B shows A's excerpt)", row("ladder", "main", id_)) for id_ in pick]
    entries = [("P", e) for e in pairs] + [("S", e) for e in singles]
    random.Random(5).shuffle(entries); flip = random.Random(7)
    md = ["# Blind review input: AJ-E3709, v3 run (realistic turn-end messages)", "",
          "Judge each line against its source text and AI Juggler's facts. Verdicts: **PASS** faithful, useful and clean. **FAIL** not "
          "faithful: an added fact or outcome, a changed meaning, a flipped verdict, or a dropped failure, \"not run\", \"not done\" or partial "
          "count that makes the turn read better than it was. **WEAK** faithful, but not useful or not clean (a fragment, stray quotes, leftover "
          "markup, over 20 words, a vague \"the message details\").", "",
          "**Pair entries (P…)** show two (sometimes three) candidate lines for the same turn, in no particular order. Give each line a "
          "verdict, then say which a reader of the feed is better served by: `first`, `second`, or `same` when neither is better. With three "
          "lines, give the order instead, best first, `=` for equal, as in `2>1=3`. **Single entries (S…)** show one line: give it a verdict.", "",
          "A line may have two rows (the second carries a caveat). Lines are shown as plain text. Do not guess which engine or path made a "
          "line; that is not in this file.", "",
          "**Answer format.** End your review with these two tables, one row per entry; the tally reads them:", "",
          "| Id | Line 1 | Line 2 | Pick | Reason |", "|---|---|---|---|---|", "| P001 | PASS | WEAK | first | … |",
          "| P002 | PASS | WEAK | FAIL | 1>2=3 | … |", "", "(A three-line entry has three verdicts, then the order.)", "",
          "| Id | Verdict | Reason |", "|---|---|---|", "| S001 | PASS | … |", ""]
    key = []; np = ns = 0
    for kind, (id_, what, *rest_) in entries:
        it = C[id_]
        head = [f"## {{rid}}", "", "**Source text**", ""] + block(it["source"]) + ["", f"**AI Juggler's facts:** {facts_words(it['facts'])}", ""]
        if kind == "P":
            np += 1; rid = f"P{np:03d}"; lines = list(what); flip.shuffle(lines)
            md += [h.replace("{rid}", rid) for h in head]
            for n_, (_who, text, _r) in enumerate(lines, 1): md += [f"**Line {n_}**", ""] + quote(text) + [""]
            key.append({"review_id": rid, "type": "pair", "item": id_, "bucket": it.get("bucket"), "level": it["facts"].get("level"),
                        "order": [who for who, _t, _r in lines], "lines": [t for _w, t, _r in lines],
                        "b_rows": {who: {"kind": br["kind"], "shown": br["shown"], "excerpt": br["excerpt"], "trace": br["trace"],
                                         "draft": br.get("draft"), "choose_pick": br.get("choose")} for who, _t, br in lines if br}})
        else:
            ns += 1; rid = f"S{ns:03d}"; r = rest_[0]
            md += [h.replace("{rid}", rid) for h in head] + ["**Shown line**", ""] + quote(plain_lines(r["shown"])) + [""]
            key.append({"review_id": rid, "type": "single", "item": id_, "bucket": it.get("bucket"), "level": it["facts"].get("level"),
                        "setting": what, "kind": r["kind"], "line": r["shown"], "trace": r["trace"]})
    for k in key: k["engine"] = "Apple on-device Foundation Model (prompt v3, guided output) with gate eval-3"
    open(out_md, "w").write("\n".join(md) + "\n"); json.dump(key, open(out_key, "w"), indent=1, ensure_ascii=False)
    print(f"{np} pair entries, {ns} single entries ({len(pick)} sampled of {len(rest)} calm items where B shows the excerpt); input {out_md}, key {out_key}")

def parse_order(s, n):
    """'first' | 'second' | 'same' (two lines) or an order like '2>1=3' -> rank per line position (0 = best), or None"""
    s = s.strip().lower().replace(" ", "")
    if n == 2 and s in ("first", "second", "same"): return {"first": [0, 1], "second": [1, 0], "same": [0, 0]}[s]
    if not re.fullmatch(r"\d([>=]\d)*", s): return None
    rank, r = [None] * n, 0
    for grp in s.split(">"):
        for x in grp.split("="):
            i = int(x) - 1
            if not 0 <= i < n or rank[i] is not None: return None
            rank[i] = r
        r += 1
    return rank if None not in rank else None

def tally(review_md, key_path):
    K = {k["review_id"]: k for k in json.load(open(key_path))}; got = {}
    for line in open(review_md):
        cells = [c.strip().strip("*`") for c in line.strip().strip("|").split("|")]
        if cells and re.fullmatch(r"[PS]\d{3}", cells[0] or ""): got[cells[0]] = cells[1:]
    V = {"PASS", "WEAK", "FAIL"}; res = {"missing": sorted(set(K) - set(got)), "unreadable": []}
    per = {v: {"pairs": 0, "b_wins": 0, "a_wins": 0, "same": 0, "b_fail": [], "a_fail": [], "b_verdicts": {}, "a_verdicts": {}}
           for v in ("ladder", "choose_only")}
    singles = {"by_level": {}, "fail": []}
    for rid, cells in got.items():
        k = K.get(rid)
        if not k: res["unreadable"].append((rid, "not in the key")); continue
        if k["type"] == "pair":
            n_ = len(k["order"]); vs = [c.upper() for c in cells[:n_]]; rank = parse_order(cells[n_] if len(cells) > n_ else "", n_)
            if len(vs) < n_ or any(v not in V for v in vs) or rank is None: res["unreadable"].append((rid, cells)); continue
            verdict = dict(zip(k["order"], vs)); rank_of = dict(zip(k["order"], rank)); a = rank_of["A"]
            for who in k["order"]:
                if who == "A": continue
                b = rank_of[who]; winner = "B" if b < a else "A" if a < b else "same"
                for var in who[2:].split("+"):
                    p = per[var]; p["pairs"] += 1
                    p["b_wins" if winner == "B" else "a_wins" if winner == "A" else "same"] += 1
                    p["b_verdicts"][verdict[who]] = p["b_verdicts"].get(verdict[who], 0) + 1
                    p["a_verdicts"][verdict["A"]] = p["a_verdicts"].get(verdict["A"], 0) + 1
                    if verdict[who] == "FAIL": p["b_fail"].append((rid, k["item"]))
                    if verdict["A"] == "FAIL": p["a_fail"].append((rid, k["item"]))
        else:
            if not cells or cells[0].upper() not in V: res["unreadable"].append((rid, cells)); continue
            d = singles["by_level"].setdefault(k["level"], {}); d[cells[0].upper()] = d.get(cells[0].upper(), 0) + 1
            if cells[0].upper() == "FAIL": singles["fail"].append((rid, k["item"], k["level"], k["kind"]))
    for var, p in per.items():
        p["preferred"] = p["b_wins"] > p["a_wins"] + p["same"]     # a tie ('same') counts as A's
        p["zero_b_fails"] = not p["b_fail"]
        p["b_ships_by_the_rule"] = p["preferred"] and p["zero_b_fails"] and not res["missing"] and not res["unreadable"]
    res["variants"] = per; res["singles"] = singles
    res["rule"] = "B ships for calm items only if b_wins > a_wins + same (a 'same' answer is a tie and counts as A's) and B has zero FAILs"
    print(json.dumps(res, indent=1, ensure_ascii=False))

if __name__ == "__main__":
    a = sys.argv[1:]
    if a[:1] == ["build"] and len(a) >= 5:
        label = a[a.index("--corpus-label") + 1] if "--corpus-label" in a else "turnends"
        n = int(a[a.index("--sample") + 1]) if "--sample" in a else 10
        build(a[1], a[2], a[3], a[4], label, n)
    elif a[:1] == ["tally"] and len(a) == 3: tally(a[1], a[2])
    else: print(__doc__); sys.exit(2)
