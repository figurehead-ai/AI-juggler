#!/usr/bin/env python3
"""runner_v5.py: AJ-E3709 harness v5's model calls: every registered job, facts first, verify and retry (design: ../../v5-design.md).

  python3 runner_v5.py --cmd <scratch>/apple_probe --corpus heldout5=../../heldout/v5/<file>.json --out <scratch>/v5-apple.json
  python3 runner_v5.py --cmd "<target>/release/local_probe --model <file.gguf>" --corpus ... --out ...     (the model's v4 variant by default)
  python3 runner_v5.py --fake --corpus v2=../../corpus-v2-8f7c7474.json --corpus turnends=../../corpus-v3-turnends.json --out <scratch>/dry-v5.json

Per item and job: code decides first (job.plan: routing, the attention decision, Chat's look-up with "I don't know" and no call); else one
call, the job's checks, and on a failed check a regeneration with the reason appended (core.retry_note), at most --retries (2) times,
then the non-AI fallback. Every attempt is recorded (text, finish, latency, the reasons it was rejected); report_v5.py replays them
offline, so a rule change re-scores without the model. Models (core.MODELS): Apple (apple_probe), Qwen3-1.7B Q8, Nemotron 3 Nano 4B
(their v4 sweep variants), Granite 4.1 3B and Qwen3-4B-Instruct-2507 (local_probe, each GGUF's own template, thinking off).
--fake runs on the tuned corpora only (v2, turnends): never on a held-out file."""
import argparse, hashlib, json, os, sys, time
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
def _no_home(text):
    """A probe's hello may name a path under the home folder (e.g. its manifest's); the recorded result never does."""
    home = os.path.expanduser("~")
    return text.replace(home, "~") if isinstance(text, str) and home and home != "/" else text
import core                                                          # noqa: E402
from core import j4, gate, REGISTRY, norm_item, item_kind            # noqa: E402
import turns, chat                                                   # noqa: E402,F401  (registers the jobs)
import digest, watch                                                 # noqa: E402,F401  (ADR-0036's (e) and (f): registered the same way)
from runner_v4 import Probe                                          # noqa: E402  (protocol 3 with the v4 extensions: fields, sampling)
from runner_v2 import WATCH, cpu_seconds, snapshot, rss_of           # noqa: E402
from runner_v3 import machine, sh                                    # noqa: E402
WATCH = WATCH + ["local_probe"]
TUNED = ("v2", "turnends")
FAKE_OK = TUNED + ("ef-fixtures",)                                   # --fake also runs (e) and (f) on their selftest fixtures (fixtures/ef-selftest.json)

class Fake:
    """no model: deterministic drafts by a hash of the prompt (without its retry note) and the attempt number, so every path runs:
    good first time; bad then good (the retry path); bad every time (the fallback); cut then good; a critical-class draft then good"""
    h = {"hello": True, "protocol": 3, "extensions": ["fields", "sampling"], "available": "fake"}; hello = json.dumps(h)
    def call(s, req):
        p = req["prompt"]; base = p.split("\n\nYour previous answer was rejected because:")[0]
        attempt = p.count("Your previous answer was rejected because:"); k = int(hashlib.sha256(base.encode()).hexdigest(), 16) % 6
        bad = (k == 1 and attempt == 0) or k == 2 or (k == 4 and attempt == 0)
        own = getattr(s, "_" + req["job"], None)
        obj = own(req, base, bad, k) if own else REGISTRY[req["job"]].fake_draft(req, base, bad, k)    # a registered job brings its own fake drafts
        text = json.dumps(obj, ensure_ascii=False, separators=(", ", ": "))
        if k == 3 and attempt == 0: text = text[: len(text) // 2]
        return {"text": text, "finish": "length" if (k == 3 and attempt == 0) else "stop", "out_tokens": len(text) // 4, "refused": False,
                "retried_permissive": False, "error": "", "latency_ms": 1, "gen_ms": 1}
    @staticmethod
    def _msg(p):
        return j4.unmark(p.split(">>>\n", 1)[1].rsplit("\n<<<END-", 1)[0]) if ">>>\n" in p else ""
    def _feed_line(s, req, p, bad, k):
        msg = s._msg(p); first = (gate.sentences_v3(msg) or [msg or "did nothing"])[0]
        w = first.rstrip(".").split(); w = w[1:] if w and w[0].lower() in ("i", "we", "ich", "wir") else w
        cav = j4.caveat_clause(msg, "unverified") or j4.caveat_clause(msg, "incomplete")
        o = {"action": (w[0].lower() if w else "did")[:30], "object": (" ".join(w[1:9]) or "the work")[:70], "result": "none",
             "caveat_kind": "not_run" if cav else "none", "caveat": (cav or "none")[:60]}
        if bad: o["result"] = "all 37 tests pass" if k != 4 else "the 9 failing checks now pass"
        return o
    def _review(s, req, p, bad, k):
        msg = s._msg(p); first = (gate.sentences_v3(msg) or ["none"])[0]
        paths = req["schema"]["properties"][0]["schema"]["items"]["properties"][0]["schema"]["values"]
        words = " ".join(first.rstrip(".").split()[:7]).strip() or "none"
        o = {"notes": [{"path": paths[0], "says": words[:90]}] if words != "none" else [], "risk_reason": "none", "risk": "unknown"}
        if bad: o["notes"] = [{"path": paths[0], "says": "rewrote src/invented_file.rs in 12 places"}]; o["risk"] = "low" if k == 4 else "unknown"
        return o
    def _attention(s, req, p, bad, k):
        msg = s._msg(p); q = next((x for x in gate.sentences_v3(msg) if x.rstrip().endswith("?")), None)
        o = {"evidence": q[:100], "reason": "asks_question", "suggest": "needs_you"} if q else {"evidence": "none", "reason": "work_done", "suggest": "can_wait"}
        if bad: o = {"evidence": "Should I " + (q or "keep the old one")[:40].lower(), "reason": "asks_question", "suggest": "needs_you"}
        return o
    def _chat(s, req, p, bad, k):
        if req["schema"].get("name") == "ChatPhrase":
            lines = [l for l in p.split("\n") if l.startswith("[")]
            parts = []
            for l in lines:
                head, _, vals = l.partition(" | "); label = head.split('"')[1] if '"' in head else head
                parts.append(f"{label}: " + ", ".join(x.split(": ", 1)[1] for x in vals.split("; ")))
            ans = "; ".join(parts) + "."
            if bad: ans = ("I restarted it. " + ans) if k == 4 else ans.replace(next((c for c in ans if c.isdigit()), "0"), "9", 1) if any(c.isdigit() for c in ans) else ans + " It took 52 min."
            return {"answer": ans[:200], "cites": list(req["refs"])}
        if bad: return {"asks_about": "other", "subject": "none", "covered": "yes", "parts": [{"states": "other", "text": "I restarted the session.", "cites": [req["refs"][0]]}]}
        return {"asks_about": "other", "subject": "none", "covered": "no", "parts": []}
    def pid(s): return None
    def close(s): pass

def load_corpus(spec, limit, fake, jobs_wanted):
    label, _, path = spec.partition("=")
    if not path: raise SystemExit(f"--corpus wants LABEL=PATH, got {spec}")
    if fake and label not in FAKE_OK: raise SystemExit(f"--fake runs on the tuned corpora and the (e)/(f) fixtures only ({', '.join(FAKE_OK)}), not {label}")
    raw = open(path, "rb").read(); items = json.loads(raw)
    items = [x for x in items if item_kind(x)]
    unknown = core.vocab_scan(items, label)          # round 3: every facts word must map before a single model call
    if unknown: raise SystemExit(f"{label}: {len(unknown)} facts word(s) the gate's mappings don't know; refusing to start:\n  " + "\n  ".join(unknown))
    if label in TUNED and "chat" in jobs_wanted and not any(item_kind(x) == "chat" for x in items):
        items = items + chat.chat_items_tuned(label, items)          # the tuned corpora's Chat items, built by code (chat.py)
    norm = [norm_item(x) for x in items]
    if limit:
        by = {}; out = []
        for it in norm:
            by.setdefault(it["kind"], 0)
            if by[it["kind"]] < limit: out.append(it); by[it["kind"]] += 1
        norm = out
    return label, path, hashlib.sha256(raw).hexdigest(), norm, len(items)

def sha(path): return hashlib.sha256(open(path, "rb").read()).hexdigest()
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--cmd", help="the engine command (apple_probe, or local_probe --model ...)"); g.add_argument("--fake", action="store_true")
    ap.add_argument("--job", default="all", help="all, or a comma list of registered jobs: " + ", ".join(REGISTRY))
    ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH"); ap.add_argument("--out", required=True)
    ap.add_argument("--repeats", type=int, default=1); ap.add_argument("--limit", type=int, default=0, help="first N items of each kind per corpus")
    ap.add_argument("--retries", type=int, default=core.MAX_RETRIES, help="regenerations after a failed check (D-130: 2)")
    ap.add_argument("--sampling", help="a config name from the model's entry in core.MODELS (default: its v4 choice or card)")
    ap.add_argument("--datamark", choices=("on", "off"), help="default: the model's v4 choice (core.MODELS)")
    ap.add_argument("--context", type=int, default=4096, help="the context tier for jobs with budget tiers (the digest: 4096 or 8192; "
                    "local_probe needs the same --ctx in --cmd). Other jobs keep their 4,096 budget")
    a = ap.parse_args()
    if not a.corpus: ap.error("at least one --corpus")
    job_list = list(REGISTRY) if a.job == "all" else [j.strip() for j in a.job.split(",")]
    for j in job_list:
        if j not in REGISTRY: ap.error(f"unknown job {j}")
    eng = Fake() if a.fake else Probe(a.cmd)
    entry = core.MODELS[0] if a.fake else core.model_entry(eng.h)
    if entry is None: raise SystemExit("unknown model: add it to core.MODELS (its card's or GGUF's sampling) first")
    sname = a.sampling or entry["default"]
    if sname not in entry["sampling"]: raise SystemExit(f"no sampling {sname} for {entry['family']}: {list(entry['sampling'])}")
    mark = (a.datamark or entry["datamark"]) == "on"
    for j in job_list:
        if hasattr(REGISTRY[j], "context_tiers"):
            if a.context not in REGISTRY[j].context_tiers: ap.error(f"--context {a.context}: {j} has budgets for {REGISTRY[j].context_tiers}")
            REGISTRY[j].context_tokens = a.context
    corpora = [load_corpus(c, a.limit, a.fake, job_list) for c in a.corpus]
    before = {} if a.fake else snapshot(); rss0 = 0 if a.fake else rss_of(WATCH); peak = rss0; load0 = os.getloadavg()
    t0 = time.time(); res = []; ncalls = [0]
    for label, _path, _d, items, _n in corpora:
        for jn in job_list:
            job = REGISTRY[jn]
            for it in items:
                if it["kind"] != job.item_kind: continue
                for setting in job.settings(it):
                    for rep in range(a.repeats):
                        key = f"{label}:{it['id']}:{jn}:{setting}"
                        def call(req, key=key, rep=rep):
                            n = req["prompt"].count("Your previous answer was rejected because:")
                            r = dict(req, id=f"{key}:{rep}:a{n}", sampling=core.sampling_for(entry, sname, key + f":a{n}", rep))
                            for k in ("est_tokens", "cut", "path"): r.pop(k, None)
                            out = eng.call(r); ncalls[0] += 1; return out
                        try: rec = core.run_item(job, it, setting, call, mark, a.retries)
                        except core.FactsUnmapped as e: eng.close(); raise SystemExit(f"{label}: {e}")
                        fin = rec["final"]
                        res.append({"corpus": label, "job": jn, "setting": setting, "id": it["id"], "rep": rep, "model_called": rec["model_called"],
                                    "used": rec["used"], "attempts": rec["attempts"],
                                    "final": {k: fin.get(k) for k in ("kind", "shown", "model_text", "refs", "risk", "needs_you", "notes") + tuple(getattr(job, "record_keys", ())) if k in fin}})
                        if not a.fake and ncalls[0] % 20 == 0: peak = max(peak, rss_of(WATCH))
    wall = time.time() - t0
    helper_cpu = cpu_seconds(sh(["ps", "-o", "time=", "-p", str(eng.pid())]) or "0") if eng.pid() else 0.0
    after = {} if a.fake else snapshot(); rss1 = 0 if a.fake else rss_of(WATCH); eng.close()
    deltas = sorted(({"name": nm, "cpu_s": round(t1 - before[pid][0], 2)} for pid, (t1, nm) in after.items()
                     if pid in before and before[pid][1] == nm and nm in WATCH and t1 - before[pid][0] >= 0.05), key=lambda d: -d["cpu_s"])
    prompts = {jn: hashlib.sha256(REGISTRY[jn].write_prompts().encode()).hexdigest() for jn in job_list}
    meta = {"harness": "v5", "prompt_version": core.PROMPT_VERSION, "engine": "fake (no model)" if a.fake else (eng.h.get("engine") or "Apple on-device Foundation Model via apple_probe.swift"),
            "hello": _no_home(eng.hello), "family": entry["family"], "model_label": entry.get("label") or entry["family"], "sampling_name": sname, "sampling": entry["sampling"][sname], "sampling_source": entry["source"],
            "datamark": "on" if mark else "off", "retries": a.retries, "jobs": job_list, "prompt_sha256": prompts, "context_tokens": a.context,
            "code_sha256": {f: sha(os.path.join(HERE, f)) for f in ("core.py", "turns.py", "chat.py", "severity.py", "digest.py", "watch.py")},
            "gate_py_sha256": sha(gate.__file__), "frozen_gate_commit": "7acec4f50", "jobs_v4_sha256": sha(j4.__file__),
            "repeats": a.repeats, "limit": a.limit or None,
            "corpora": [{"label": l, "file": os.path.basename(p), "sha256": d, "items_in_file": n, "items_run": len(i)} for l, p, d, i, n in corpora],
            "calls": ncalls[0], "wall_s": round(wall, 1), "machine": {} if a.fake else machine(), "load_avg_start": [round(x, 1) for x in load0],
            "load_avg_end": [round(x, 1) for x in os.getloadavg()], "helper_cpu_s": helper_cpu, "cpu_deltas": deltas,
            "watched_rss_mb": {"before": round(rss0, 1), "peak": round(peak, 1), "after": round(rss1, 1)},
            "started": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t0))}
    json.dump({"meta": meta, "results": res}, open(a.out, "w"), indent=1, ensure_ascii=False)
    print("wrote", a.out, len(res), "records,", ncalls[0], "calls,", round(wall, 1), "s")

if __name__ == "__main__": main()
