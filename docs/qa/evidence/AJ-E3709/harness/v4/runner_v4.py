#!/usr/bin/env python3
"""runner_v4.py: AJ-E3709 harness v4's model calls, per job and model (design: docs/qa/evidence/AJ-E3709/v4-design.md). Stdlib only.

  python3 runner_v4.py --cmd <scratch>/apple_probe --job all --corpus v2=../../corpus-v2-8f7c7474.json \
      --corpus turnends=../../corpus-v3-turnends.json --out <scratch>/results-apple-v4.json [--repeats 1]
  python3 runner_v4.py --cmd "<target>/release/local_probe --model <file.gguf>" --sampling low --job all --corpus ... --out ...
  python3 runner_v4.py --fake --limit 2 --job all --corpus v2=... --corpus turnends=... --out <scratch>/dry-v4.json   (no model)
  python3 runner_v4.py --check-grammars <scratch>/grammars.jsonl     (then: local_probe --model <f> --check-grammar that file)

The tuned-only sampling sweep (the coordinating session's rule; design doc "Sampling"): run each open model once per config with
--subset sweep (turnends, the v2 hard cases, the first 18 other v2 items) and pick with report_v4.py --select. Never on held-out.

Every call: one job, one fresh session (Apple) or a cleared context (llama.cpp); mode "fields"; the job's schema (Apple builds its
DynamicGenerationSchema from it) and its GBNF (llama.cpp) are both sent and each probe uses its own; the sampling of the chosen config.
  feed_line  every item, with its facts (report_v4 applies the routing: L2/L3 drafts measure the gate only)
  review     every item, with its facts
  attention  'facts_given' for calm items (L0/L1), and 'facts_hidden' (outcome, level and tests unknown) for every item: the case
             the suggestion exists for, a turn whose journal missed the need
  chat       the look-up items jobs.chat_items() builds from each corpus (or a corpus's own chat cases)
The drafts are recorded raw; report_v4.py composes, gates and scores them offline, so a rule change replays without the model."""
import argparse, hashlib, json, os, shlex, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, os.path.dirname(HERE))
import jobs, gate                                                    # noqa: E402
from runner_v2 import WATCH, cpu_seconds, snapshot, rss_of           # noqa: E402
from runner_v3 import machine, sh                                    # noqa: E402
WATCH = WATCH + ["local_probe"]

class Probe:
    """apple_probe or local_probe over protocol 3 with the v4 extensions (fields, sampling)"""
    def __init__(s, cmd):
        s.p = subprocess.Popen(shlex.split(cmd), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        s.hello = s.p.stdout.readline().strip(); h = json.loads(s.hello or "{}"); s.h = h
        if h.get("protocol") != 3: raise SystemExit(f"the probe speaks protocol {h.get('protocol')}, not 3")
        if not {"fields", "sampling"} <= set(h.get("extensions") or []):
            raise SystemExit("the probe lacks the v4 extensions (fields, sampling): rebuild it from this harness")
        av = str(h.get("available", "")).lower()
        if "available" not in av or "unavailable" in av: raise SystemExit(f"the probe's model is not available here: {h.get('available')}")
    def call(s, req):
        t0 = time.perf_counter(); s.p.stdin.write(json.dumps(req) + "\n"); s.p.stdin.flush()
        line = s.p.stdout.readline(); ms = (time.perf_counter() - t0) * 1000
        if not line: raise SystemExit("engine died")
        r = json.loads(line); r["latency_ms"] = round(ms); return r
    def pid(s): return s.p.pid
    def close(s):
        try: s.p.stdin.close(); s.p.wait(timeout=10)
        except Exception: s.p.kill()

class Fake:
    """no model: deterministic drafts that exercise every path of the report: good fields built from the text, a dropped caveat, an
    invented number, a cut (unparseable) answer, a value outside the enum, a paraphrased quote, an invented ref, an acting claim"""
    h = {"hello": True, "protocol": 3, "extensions": ["fields", "sampling"], "available": "fake"}
    hello = json.dumps(h)
    def call(s, req):
        job, p = req["job"], req["prompt"]; k = int(hashlib.sha256(p.encode()).hexdigest(), 16) % 6
        if job == "chat": obj = s._chat(req, k)
        else:
            msg = jobs.unmark(p.split(">>>\n", 1)[1].rsplit("\n<<<END-", 1)[0]) if ">>>\n" in p else ""
            first = (gate.sentences_v3(msg) or [msg or "did nothing"])[0]
            w = first.rstrip(".").split(); w = w[1:] if w and w[0].lower() in ("i", "we", "ich", "wir") else w
            act, obj_ = (w[0].lower() if w else "did"), " ".join(w[1:9]) or "the work"
            cav = jobs.caveat_clause(msg, "unverified") or jobs.caveat_clause(msg, "incomplete")
            if job == "feed_line":
                obj = {"action": act[:30], "object": obj_[:70], "result": "none", "caveat_kind": "not_run" if cav else "none", "caveat": (cav or "none")[:60]}
                if k == 1: obj.update(caveat_kind="none", caveat="none")
                if k == 2: obj["result"] = "all 37 tests pass"
            elif job == "review":
                obj = {"changed": [first[:90]], "not_checked": [cav[:90]] if cav else [], "left": [], "risk_reason": "none", "risk": "medium" if cav else "unknown"}
                if k == 1: obj.update(not_checked=[], risk="low")
                if k == 2: obj["changed"] = [first[:60] + " in src/invented_file.rs"]
            else:
                q = next((x for x in gate.sentences_v3(msg) if x.rstrip().endswith("?")), None)
                obj = ({"evidence": q[:100], "reason": "asks_question", "suggest": "needs_you"} if q else
                       {"evidence": "none", "reason": "work_done", "suggest": "can_wait"})
                if k == 1 and q: obj["evidence"] = "Should I " + q[:40].lower()
                if k == 2: obj = {"evidence": first[:100], "reason": "reports_failure", "suggest": "needs_you"}
        text = json.dumps(obj, ensure_ascii=False, separators=(", ", ": "))
        if k == 3: text = text[: len(text) // 2]
        if k == 4 and job == "feed_line": text = text.replace('"caveat_kind": "none"', '"caveat_kind": "maybe"')
        return {"text": text, "finish": "length" if k == 3 else "stop", "out_tokens": len(text) // 4, "refused": False,
                "retried_permissive": False, "error": "", "latency_ms": 1, "gen_ms": 1}
    def _chat(s, req, k):
        refs = req["refs"]; q = req["question"]
        if "function" in q: return {"covered": "out_of_scope", "parts": []}
        if "cost" in q: return {"covered": "no", "parts": []}
        r = refs[0]
        if k == 1: return {"covered": "yes", "parts": [{"text": "The session finished and all tests passed.", "cites": [r]}]}
        if k == 2: return {"covered": "yes", "parts": [{"text": "I restarted the session.", "cites": [r]}]}
        if k == 5: return {"covered": "yes", "parts": [{"text": "The session ended.", "cites": ["r9"]}]}
        return {"covered": "yes", "parts": [{"text": "The session's state is in the record.", "cites": [r]}]}
    def pid(s): return None
    def close(s): pass

def load_corpus(spec, limit, subset):
    label, _, path = spec.partition("=")
    if not path: raise SystemExit(f"--corpus wants LABEL=PATH, got {spec}")
    raw = open(path, "rb").read(); items = jobs.adapt_items(json.loads(raw))     # the per-job held-out schema, keys renamed only
    if subset == "sweep" and label == "v2":
        items = [x for x in items if x.get("bucket") == "adversarial"] + [x for x in items if x.get("bucket") != "adversarial"][:18]
    elif subset == "sweep" and label not in ("turnends",):
        raise SystemExit(f"--subset sweep is for the tuned corpora only (v2, turnends), not {label}")
    return label, path, hashlib.sha256(raw).hexdigest(), (items[:limit] if limit else items), len(items), items

def build_calls(label, items, job_list, mark, n_examples, full=None, limit=0):
    """-> [(job, setting, id, req-without-sampling, meta)]. Chat's look-up items are built from the whole corpus (a set needs four
    turns), then limited to the first `limit`."""
    calls = []
    for it in items if any(j != "chat" for j in job_list) else []:
        if not it.get("source"): continue
        ctx = gate.prepare(it["source"]); F = gate.facts_norm(it.get("facts"))
        for job in job_list:
            if job == "chat" or (it.get("job") and it["job"] != job): continue      # a per-job item runs only its own job
            settings = [("main", False)]
            if job == "attention":
                settings = ([("facts_given", False)] if F and F["level"] in ("L0", "L1") else []) + [("facts_hidden", True)]
            for setting, hide in settings:
                sys_t, prompt, schema, grammar, mx, est, cut = jobs.request(job, ctx, it.get("facts"), f"{label}:{it['id']}:{job}", mark, hide, n_examples=n_examples)
                calls.append((job, setting, it["id"], {"job": job, "mode": "fields", "system": sys_t, "prompt": prompt, "schema": schema,
                              "grammar": grammar, "max_tokens": mx}, {"est_tokens": est, "cut": cut, "prompt_chars": len(prompt)}))
    if "chat" in job_list:
        cis = jobs.chat_items(label, full if full is not None else items)
        for ci in (cis[:limit] if limit else cis):
            sys_t, prompt, schema, grammar, mx, est, cut = jobs.request("chat", chat_item=ci, mark=mark, n_examples=n_examples)
            calls.append(("chat", "main", ci["id"], {"job": "chat", "mode": "fields", "system": sys_t, "prompt": prompt, "schema": schema,
                          "grammar": grammar, "max_tokens": mx, "refs": [r["ref"] for r in ci["rows"]], "question": ci["question"]},
                          {"est_tokens": est, "cut": cut, "prompt_chars": len(prompt), "chat_item": ci}))
    return calls

def check_grammars(path):
    with open(path, "w") as f:
        for job in jobs.JOBS:
            for refs in (["r1"], ["r1", "r2", "r3", "r4", "r5"]) if job == "chat" else ([None],):
                f.write(json.dumps({"name": job + (f"-{len(refs)}refs" if refs and refs[0] else ""), "grammar": jobs.gbnf(jobs.schema_for(job, refs))}) + "\n")
    print("wrote", path, "- run: local_probe --model <file.gguf> --check-grammar", path)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--cmd", help="the engine command (apple_probe, or local_probe --model ...)")
    g.add_argument("--fake", action="store_true", help="no model calls"); g.add_argument("--check-grammars", metavar="OUT.jsonl")
    ap.add_argument("--job", default="all", help="all, or a comma list of feed_line,review,attention,chat")
    ap.add_argument("--corpus", action="append", default=[], metavar="LABEL=PATH",
                    help="tuned: v2, turnends; judged: heldout (feed line), heldout-jobs (jobs b-d), real-heldout (D-124, last)")
    ap.add_argument("--out"); ap.add_argument("--repeats", type=int, default=1, help="1 for greedy (repeats agreed in v3); 3 when sampling")
    ap.add_argument("--limit", type=int, default=0, help="first N items of each corpus")
    ap.add_argument("--subset", choices=("all", "sweep"), default="all", help="sweep: the tuned subset for the sampling sweep")
    ap.add_argument("--sampling", help="a config name from jobs.SAMPLING for this model (default: jobs.DEFAULT_SAMPLING)")
    ap.add_argument("--mark", choices=("datamark", "plain"), default="datamark", help="the fence's datamarking (research open question 3)")
    ap.add_argument("--datamark", choices=("on", "off"), help="the sweep's name for --mark: off = no marks between words; the fences, "
                    "the untrusted-data rule and the rule restated after the text stay, and the grammar still bans the mark")
    ap.add_argument("--examples", type=int, default=0, help="use only the first N examples per job (0: all)")
    ap.add_argument("--no-schema-in-prompt", action="store_true", help="Apple: includeSchemaInPrompt false (research open question 2)")
    a = ap.parse_args()
    if a.datamark: a.mark = "datamark" if a.datamark == "on" else "plain"
    if a.check_grammars: return check_grammars(a.check_grammars)
    if not a.out or not a.corpus: ap.error("--out and at least one --corpus are needed")
    job_list = list(jobs.JOBS) if a.job == "all" else [j.strip() for j in a.job.split(",")]
    for j in job_list:
        if j not in jobs.JOBS: ap.error(f"unknown job {j}")
    if any(c.split("=", 1)[0] in ("heldout", "heldout-jobs", "real-heldout") for c in a.corpus) and a.subset == "sweep":
        ap.error("the sweep runs on the tuned corpora only")
    eng = Fake() if a.fake else Probe(a.cmd)
    fam = "apple" if a.fake else jobs.model_family(eng.h)
    if fam == "unknown": raise SystemExit("unknown model family: add it to jobs.SAMPLING first")
    sname = a.sampling or jobs.DEFAULT_SAMPLING[fam]
    if sname not in jobs.SAMPLING[fam]: raise SystemExit(f"no sampling config {sname} for {fam}: {list(jobs.SAMPLING[fam])}")
    corpora = [load_corpus(c, a.limit, a.subset) for c in a.corpus]
    before = {} if a.fake else snapshot(); rss0 = 0 if a.fake else rss_of(WATCH); peak = rss0; load0 = os.getloadavg()
    t0 = time.time(); res = []; ncalls = 0; first = True
    for label, path, digest, items, _n, full in corpora:
        for job, setting, iid, req, meta in build_calls(label, items, job_list, a.mark == "datamark", a.examples or None, full, a.limit):
            for rep in range(a.repeats):
                samp = jobs.sampling_for(fam, sname, f"{label}:{iid}:{job}:{setting}", rep)
                r = dict(req, id=f"{label}:{iid}:{job}:{setting}:{rep}", sampling=samp)
                if a.no_schema_in_prompt: r["include_schema_in_prompt"] = False
                out = eng.call(r); ncalls += 1
                res.append({"corpus": label, "job": job, "setting": setting, "id": iid, "rep": rep, "cold_first": first, **meta,
                            **{k: out.get(k) for k in ("text", "finish", "out_tokens", "refused", "retried_permissive", "error", "raw", "latency_ms", "gen_ms", "sampling") if k in out}})
                first = False
            if not a.fake and ncalls % 20 == 0: peak = max(peak, rss_of(WATCH))
    wall = time.time() - t0
    helper_cpu = cpu_seconds(sh(["ps", "-o", "time=", "-p", str(eng.pid())]) or "0") if eng.pid() else 0.0
    after = {} if a.fake else snapshot(); rss1 = 0 if a.fake else rss_of(WATCH)
    eng.close()
    deltas = sorted(({"name": nm, "cpu_s": round(t1 - before[pid][0], 2)} for pid, (t1, nm) in after.items()
                     if pid in before and before[pid][1] == nm and nm in WATCH and t1 - before[pid][0] >= 0.05), key=lambda d: -d["cpu_s"])
    prompts = {j: hashlib.sha256((jobs.instructions(j, a.mark == "datamark", a.examples or None) + json.dumps(jobs.schema_for(j))).encode()).hexdigest()
               for j in job_list}
    meta = {"harness": "v4", "prompt_version": jobs.PROMPT_VERSION, "engine": "fake (no model)" if a.fake else (eng.h.get("engine") or "Apple on-device Foundation Model via apple_probe.swift"),
            "hello": eng.hello, "family": fam, "sampling_name": sname, "sampling": jobs.SAMPLING[fam][sname], "mark": a.mark, "datamark": "on" if a.mark == "datamark" else "off",
            "examples": a.examples or "all", "schema_in_prompt": not a.no_schema_in_prompt, "jobs": job_list, "subset": a.subset,
            "prompt_sha256": prompts, "jobs_py_sha256": hashlib.sha256(open(jobs.__file__, "rb").read()).hexdigest(),
            "gate_py_sha256": hashlib.sha256(open(gate.__file__, "rb").read()).hexdigest(), "frozen_gate_commit": "7acec4f50",
            "repeats": a.repeats, "limit": a.limit or None,
            "corpora": [{"label": l, "file": os.path.basename(p), "sha256": d, "items_in_file": n, "items_run": len(i)} for l, p, d, i, n, _f in corpora],
            "calls": ncalls, "wall_s": round(wall, 1), "machine": {} if a.fake else machine(), "load_avg_start": [round(x, 1) for x in load0],
            "load_avg_end": [round(x, 1) for x in os.getloadavg()], "helper_cpu_s": helper_cpu, "cpu_deltas": deltas,
            "watched_rss_mb": {"before": round(rss0, 1), "peak": round(peak, 1), "after": round(rss1, 1)},
            "started": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t0))}
    json.dump({"meta": meta, "results": res}, open(a.out, "w"), indent=1, ensure_ascii=False)
    print("wrote", a.out, len(res), "records,", ncalls, "calls,", round(wall, 1), "s")

if __name__ == "__main__": main()
