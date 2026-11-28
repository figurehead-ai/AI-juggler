#!/usr/bin/env python3
"""runner_v6.py: AJ-E3709 harness v6's model calls (AJ-E3830, AJ-E3831; D-149). Runbook: v6-runbook.md.

  python3 runner_v6.py --check --corpus watch6=../../heldout/v6/watch-v6.json      counts and unmapped words only, never item text
  python3 runner_v6.py --cmd <scratch>/apple_probe --job watch2 --corpus watch6=../../heldout/v6/watch-v6.json --out <scratch>/v6-apple-watch.json
  python3 runner_v6.py --fake --corpus v6-fixtures=fixtures/v6-selftest.json --out <scratch>/dry-v6.json     (made-up items only)

Per item: v6's job runs its own calls (core6.run6): code decides first; each call is checked, and a failed check regenerates that call with
the reason, at most --retries (2) times, with a seeded sample (core6.attempt_sampling; the first attempt keeps the model's tested
decoding); then the plain path. Every attempt is recorded with its step, so report_v6.py replays the run offline. One model at a time."""
import argparse, hashlib, json, os, sys, time
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import core6                                                         # noqa: E402
import core                                                          # noqa: E402
from core import REGISTRY, norm_item, item_kind                      # noqa: E402
import turns, chat, digest, watch                                    # noqa: E402,F401  (v5: the item kinds and the checks v6 reuses)
import pick, digest_mr, watch2                                       # noqa: E402,F401  (v6's jobs, registered)
from runner_v4 import Probe                                          # noqa: E402
from runner_v2 import WATCH as WATCHED, cpu_seconds, snapshot, rss_of  # noqa: E402
from runner_v3 import machine, sh                                    # noqa: E402
from runner_v5 import _no_home                                       # noqa: E402
WATCHED = WATCHED + ["local_probe"]
V6_JOBS = ("feed_pick", "digest_mr", "watch2")
FAKE_OK = ("v6-fixtures",)

class Fake:
    """no model: deterministic drafts by a hash of the prompt (without its retry note), so every path runs: good first time; bad then
    good (a retry); bad every time (the fallback); a cut answer then good"""
    h = {"hello": True, "protocol": 3, "extensions": ["fields", "sampling"], "available": "fake"}; hello = json.dumps(h)
    def call(s, req):
        p = req["prompt"]; base = p.split("\n\nYour previous answer was rejected because:")[0]
        attempt = req.get("retry", 0); k = int(hashlib.sha256((req.get("step", "") + base).encode()).hexdigest(), 16) % 6
        bad = (k == 1 and attempt == 0) or k == 2
        obj = REGISTRY[req["job"]].fake_draft(req, base, bad, k)
        text = json.dumps(obj, ensure_ascii=False, separators=(", ", ": "))
        cut = k == 3 and attempt == 0
        return {"text": text[: len(text) // 2] if cut else text, "finish": "length" if cut else "stop", "out_tokens": len(text) // 4, "refused": False,
                "error": "", "latency_ms": 1, "gen_ms": 1}
    def pid(s): return None
    def close(s): pass

def load_corpus(spec, limit=0, fake=False):
    label, _, path = spec.partition("=")
    if not path: raise SystemExit(f"--corpus wants LABEL=PATH, got {spec}")
    if fake and label not in FAKE_OK: raise SystemExit(f"--fake runs on the made-up fixtures only ({', '.join(FAKE_OK)}), not {label}")
    raw = open(path, "rb").read(); items = [x for x in json.loads(raw) if item_kind(x)]
    unknown = core.vocab_scan(items, label)
    if unknown: raise SystemExit(f"{label}: {len(unknown)} facts word(s) the gate's mappings don't know; refusing to start:\n  " + "\n  ".join(unknown))
    norm = [norm_item(x) for x in items]
    if limit:
        by = {}; out = []
        for it in norm:
            by[it["kind"]] = by.get(it["kind"], 0) + 1
            if by[it["kind"]] <= limit: out.append(it)
        norm = out
    return label, path, hashlib.sha256(raw).hexdigest(), norm, len(items)
def check(specs):
    """counts per item kind, and what the readers can't map: safe on a blind file (no item text is printed)"""
    for spec in specs:
        label, path, digest_sha, items, n = load_corpus(spec)
        kinds = {}
        for it in items: kinds[it["kind"]] = kinds.get(it["kind"], 0) + 1
        extra = {}
        if "watch" in kinds:
            ws = [it for it in items if it["kind"] == "watch"]
            extra["watch: gold condition types unknown to the reader"] = sum(1 for w in ws for c in (w["gold"].get("conditions") or [])
                                                                            if watch.cond_name(c.get("type") if isinstance(c, dict) else c) is None)
            extra["watch: requests in a language without word lists"] = sum(1 for w in ws if not w.get("lang_known", True))
        if "digest" in kinds: extra["digest: agents unmapped"] = sum(len(it.get("unmapped") or []) for it in items if it["kind"] == "digest")
        print(json.dumps({"corpus": label, "file": os.path.basename(path), "sha256": digest_sha, "items_in_file": n, "by_kind": kinds, **extra}))

def sha(path): return hashlib.sha256(open(path, "rb").read()).hexdigest()
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--cmd", help="the engine command (apple_probe, or local_probe --model ...)"); g.add_argument("--fake", action="store_true")
    g.add_argument("--check", action="store_true", help="load the corpora and print counts only (no model, no item text)")
    ap.add_argument("--job", default=",".join(V6_JOBS), help="a comma list of v6's jobs: " + ", ".join(V6_JOBS))
    ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH"); ap.add_argument("--out")
    ap.add_argument("--repeats", type=int, default=1); ap.add_argument("--limit", type=int, default=0, help="first N items of each kind per corpus")
    ap.add_argument("--retries", type=int, default=core6.MAX_RETRIES, help="regenerations of each call after a failed check (D-130: 2)")
    ap.add_argument("--sampling", help="the first attempt's arm from the model's entry in core.MODELS (default: its chosen one)")
    ap.add_argument("--datamark", choices=("on", "off"), help="default: the model's choice (core.MODELS)")
    a = ap.parse_args()
    if not a.corpus: ap.error("at least one --corpus")
    if a.check: check(a.corpus); return
    if not a.out: ap.error("--out")
    job_list = [j.strip() for j in a.job.split(",") if j.strip()]
    for j in job_list:
        if j not in V6_JOBS: ap.error(f"unknown v6 job {j} (v5's jobs run with ../v5/runner_v5.py)")
    eng = Fake() if a.fake else Probe(a.cmd)
    entry = core.MODELS[0] if a.fake else core.model_entry(eng.h)
    if entry is None: raise SystemExit("unknown model: add it to core.MODELS (its card's or GGUF's sampling) first")
    sname = a.sampling or entry["default"]
    if sname not in entry["sampling"]: raise SystemExit(f"no sampling {sname} for {entry['family']}: {list(entry['sampling'])}")
    mark = (a.datamark or entry["datamark"]) == "on"
    corpora = [load_corpus(c, a.limit, a.fake) for c in a.corpus]
    before = {} if a.fake else snapshot(); rss0 = 0 if a.fake else rss_of(WATCHED); peak = rss0; load0 = os.getloadavg()
    t0 = time.time(); res = []; ncalls = [0]
    for label, _path, _d, items, _n in corpora:
        for jn in job_list:
            job = REGISTRY[jn]
            for it in items:
                if it["kind"] != job.item_kind: continue
                for rep in range(a.repeats):
                    key = f"{label}:{it['id']}:{jn}"
                    def call(req, key=key, rep=rep):
                        k = f"{key}:{req['step']}"
                        r = dict(req, id=f"{k}:{rep}:a{req['retry']}", sampling=core6.attempt_sampling(entry, sname, k, rep, req["retry"]))
                        for x in ("est_tokens", "cut", "path"): r.pop(x, None)      # step and retry stay: the probes ignore them
                        out = eng.call(r); ncalls[0] += 1; out["sampling_used"] = r["sampling"]; return out
                    try: rec = core6.run6(job, it, "main", call, mark, a.retries)
                    except core.FactsUnmapped as e: eng.close(); raise SystemExit(f"{label}: {e}")
                    fin = rec["final"]
                    res.append({"corpus": label, "job": jn, "setting": "main", "id": it["id"], "rep": rep, "model_called": rec["model_called"],
                                "used": rec["used"], "attempts": rec["attempts"],
                                "final": {k: fin.get(k) for k in ("kind", "shown", "model_text", "notes") + tuple(getattr(job, "record_keys", ())) if k in fin}})
                    if not a.fake and ncalls[0] % 20 == 0: peak = max(peak, rss_of(WATCHED))
    wall = time.time() - t0
    helper_cpu = cpu_seconds(sh(["ps", "-o", "time=", "-p", str(eng.pid())]) or "0") if eng.pid() else 0.0
    after = {} if a.fake else snapshot(); rss1 = 0 if a.fake else rss_of(WATCHED); eng.close()
    deltas = sorted(({"name": nm, "cpu_s": round(t1 - before[pid][0], 2)} for pid, (t1, nm) in after.items()
                     if pid in before and before[pid][1] == nm and nm in WATCHED and t1 - before[pid][0] >= 0.05), key=lambda d: -d["cpu_s"])
    prompts = {jn: hashlib.sha256(REGISTRY[jn].write_prompts().encode()).hexdigest() for jn in job_list}
    code = {f: sha(os.path.join(HERE, f)) for f in ("core6.py", "pick.py", "digest_mr.py", "watch2.py")}
    code.update({"v5/" + f: sha(os.path.join(core6.V5, f)) for f in ("core.py", "turns.py", "digest.py", "watch.py", "claims_v5.py", "severity.py")})
    meta = {"harness": "v6", "prompt_version": core6.PROMPT_VERSION, "harness_today": core6.HARNESS_TODAY,
            "engine": "fake (no model)" if a.fake else (eng.h.get("engine") or "Apple on-device Foundation Model via apple_probe.swift"),
            "hello": _no_home(eng.hello), "family": entry["family"], "model_label": entry.get("label") or entry["family"], "sampling_name": sname,
            "sampling": entry["sampling"][sname], "retry_sampling": {"temperature": core6.RETRY_TEMPERATURE, "top_k (Apple)": core6.RETRY_TOP_K, "seed": "per item and attempt"},
            "sampling_source": entry["source"], "datamark": "on" if mark else "off", "retries": a.retries, "jobs": job_list, "prompt_sha256": prompts,
            "code_sha256": code, "gate_py_sha256": sha(core.gate.__file__), "frozen_gate_commit": "7acec4f50", "repeats": a.repeats, "limit": a.limit or None,
            "corpora": [{"label": l, "file": os.path.basename(p), "sha256": d, "items_in_file": n, "items_run": len(i)} for l, p, d, i, n in corpora],
            "calls": ncalls[0], "wall_s": round(wall, 1), "machine": {} if a.fake else machine(), "load_avg_start": [round(x, 1) for x in load0],
            "load_avg_end": [round(x, 1) for x in os.getloadavg()], "helper_cpu_s": helper_cpu, "cpu_deltas": deltas,
            "watched_rss_mb": {"before": round(rss0, 1), "peak": round(peak, 1), "after": round(rss1, 1)},
            "started": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t0))}
    json.dump({"meta": meta, "results": res}, open(a.out, "w"), indent=1, ensure_ascii=False)
    print("wrote", a.out, len(res), "records,", ncalls[0], "calls,", round(wall, 1), "s")

if __name__ == "__main__": main()
