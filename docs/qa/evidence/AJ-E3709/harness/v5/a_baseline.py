#!/usr/bin/env python3
"""a_baseline.py: AJ-E3709 A alone, the non-AI output 0.5 ships as the default, scored on a v5 set (no model is called).

  python3 a_baseline.py --corpus turns=PATH --corpus chat=PATH [--md OUT.md] [--outputs OUTPUTS.json] > OUT.json

--outputs scores A's outputs recorded by another code version (a JSON {"<job>:<id>": final}), so a fix's before and after are
scored by the same detectors; without it, A runs on this code.

A per job is what each job shows when no model text is shown: (a) the feed's excerpt with git's lead and the facts' question and
failure lines; (b) the code-only review; (c) attention from the facts; (d) Chat's look-up by code (the looked-up values, the listing,
"I don't know" with the found rows). Every item runs its job's code path with an engine that never answers, so a model-path item
shows its fallback, exactly as a user without the AI opt-in sees it.

A writes no words of its own, so the model-text detectors don't apply; A is checked for what code can get wrong:
  critical  a failure line AI Juggler didn't record (a "Failed:" line on a turn whose facts show no failure, not a facts line);
            an agent claim the facts or git contradict, shown unattributed (turns.claims_vs_facts: committed or pushed while git shows
            uncommitted work, tests or a check passed or clean while it failed, done or works on a failed turn, an undone change
            stated as done);
  omission  a feed must-have word missing (gold feed_must), a changed file missing from the review (gold review_changed), an
            attention miss (gold needs_user), Chat's no answer or a missing must-have (severity.chat_findings);
  hidden    a question or failure from the facts not shown (structural, must be 0).
Script flags are candidates; Opus confirms each by eye."""
import argparse, json, os, re, sys
from collections import Counter
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core                                                   # noqa: E402
from core import REGISTRY, norm_item, item_kind, run_item, j4  # noqa: E402
import turns, chat, severity                                  # noqa: E402

def no_model(_req): return {"text": None, "finish": "stop", "error": "A alone: no model"}

FAILED_HEADS = (turns.HEAD["en"]["failure"] + ":", turns.HEAD["de"]["failure"] + ":")
def invented_failures(t, shown):
    """'Failed:' lines whose words are no facts failure, on a turn the facts don't show failed"""
    if turns.turn_failed(t): return []
    return [l for l in (shown or "").split("\n") if l.startswith(FAILED_HEADS) and not any(f in l for f in t["failures"])]
AGENT_HEADS = tuple(turns.HEAD[l]["agent_says"] for l in ("en", "de"))
def agent_lines(t, job, shown):
    """the lines of A's output that carry the agent's own sentences, unattributed (not git's or AI Juggler's facts lines)"""
    out = []
    for l in (shown or "").split("\n"):
        if not l.strip() or l.startswith(("git:",) + AGENT_HEADS): continue
        if job == "review":
            H = turns.heads(t)
            for k in ("left", "not_checked"):
                if l.startswith(H[k] + ":"): out += [x for x in l.split(":", 1)[1].split("; ") if x.strip() and x.strip() not in (H["nothing"], H["nothing_else"], H["tests_not_run"])]
            continue
        if job == "feed_line" and not l.startswith(tuple(turns.HEAD[x][k] + ":" for x in ("en", "de") for k in ("question", "failure"))): out.append(l)
    return out
def a_turn_findings(t, job, o):
    fs = []
    for l in invented_failures(t, o.get("shown")): fs.append(("critical", "invented_failure", l[:200]))
    if job in ("feed_line", "review") and t["ctx"].get("lang") in ("en", "de"):
        for s in agent_lines(t, job, o.get("shown")):
            for kind, words, fact in turns.claims_vs_facts(s, t):
                fs.append(("critical", f"claim_vs_facts:{kind}", f"{s[:140]!r} ({words}; {fact})"))
    g = t["gold"]; shown = (o.get("shown") or "").lower()
    if job == "feed_line":
        miss = [w for w in g.get("feed_must") or [] if w.lower() not in shown]
        if miss: fs.append(("omission", "feed_must_missing", ", ".join(miss)))
    if job == "review":
        miss = [p for p in g.get("review_changed") or [] if p.lower() not in shown]
        if miss: fs.append(("omission", "changed_missing", ", ".join(miss)))
    if job == "attention":
        if g.get("needs_user") is True and not o.get("needs_you"): fs.append(("omission", "attention_miss", "gold: needs the user"))
        if g.get("needs_user") is False and o.get("needs_you"): fs.append(("omission", "attention_false_alarm", "gold: calm"))
    if job in ("feed_line", "review"):
        hid = [q for q in t["questions"] + t["failures"] if not j4._contains([o.get("shown") or ""], q)]
        fs += [("hidden", "question_or_failure_hidden", q[:160]) for q in hid]
    return fs

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH"); ap.add_argument("--md")
    ap.add_argument("--outputs", help="score these recorded A outputs ({job:id: final}) instead of running A on this code")
    a = ap.parse_args(); given = json.load(open(a.outputs)) if a.outputs else None
    out = {"what": "A alone (no model text), scored on the named sets; synthetic held-out only", "corpora": {}}
    for spec in a.corpus:
        label, _, path = spec.partition("=")
        items = [norm_item(x) for x in json.load(open(path)) if item_kind(x)]
        jobs = {}
        for it in items:
            for job in core.jobs_for_kind(it["kind"]):
                for setting in job.settings(it):
                    o = given[f"{job.name}:{it['id']}"] if given is not None else run_item(job, it, setting, no_model)["final"]
                    if it["kind"] == "chat":
                        v = {"job": "chat", "item": it, "outcome": o, "source": ""}
                        fs = [(f["class"], f["detector"], f["detail"]) for f in severity.chat_findings(v)]
                    else: fs = a_turn_findings(it, job.name, o)
                    jobs.setdefault(job.name, []).append({"id": it["id"], "kind": o.get("kind"), "shown": o.get("shown"), "findings": fs})
        summ = {}
        for jn, rows in jobs.items():
            n = len(rows); c = Counter(f[1] for r in rows for f in r["findings"])
            crit = [r["id"] for r in rows if any(f[0] == "critical" for f in r["findings"])]
            om = [r["id"] for r in rows if any(f[0] == "omission" for f in r["findings"])]
            hid = sum(1 for r in rows for f in r["findings"] if f[0] == "hidden")
            summ[jn] = {"items": n, "critical_items": len(crit), "critical_pct": round(100.0 * len(crit) / n, 2) if n else 0.0, "critical_ids": crit,
                        "omission_items": len(om), "omission_pct": round(100.0 * len(om) / n, 2) if n else 0.0, "omission_ids": om,
                        "hidden": hid, "by_detector": dict(c)}
        out["corpora"][label] = {"path": os.path.basename(path), "summary": summ, "items": jobs}
    print(json.dumps(out, indent=1, ensure_ascii=False))
    if a.md:
        L = ["# A alone (the non-AI default), scored", "", out["what"] + ". Script flags are candidates; Opus confirms each by eye.", "",
             "| Set | Job | Items | Critical items | Omission items | Hidden | Detectors |", "|---|---|---|---|---|---|---|"]
        for label, c in out["corpora"].items():
            for jn, s in c["summary"].items():
                L.append(f"| {label} | {jn} | {s['items']} | {s['critical_items']} ({s['critical_pct']} %) | {s['omission_items']} ({s['omission_pct']} %) | {s['hidden']} | "
                         + ", ".join(f"{k} {v}" for k, v in sorted(s["by_detector"].items())) + " |")
        open(a.md, "w").write("\n".join(L) + "\n")

if __name__ == "__main__": main()
