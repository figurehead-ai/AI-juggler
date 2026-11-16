#!/usr/bin/env python3
"""runner_v3.py: ADR-0033 v3's model calls for AJ-E3709's v3 run (section 6.1), stdlib only.

  python3 runner_v3.py --cmd <scratch>/apple_probe --corpus v2=../corpus-v2-8f7c7474.json \
      --corpus turnends=../corpus-v3-turnends.json --out ../results-apple-v3.json [--repeats 3]
  python3 runner_v3.py --fake --limit 3 --corpus v2=... --corpus turnends=... --out <scratch>/dry.json   (no Apple calls)

For every item of every corpus and every repeat: the text is normalised (step 0), its language judged, guarded (step 1), then
  write  = the v3 frame, guided output (one `line` field, description from gate.py), temperature 0, 60 tokens; skipped when the guard
           removed anything (section 6, the ladder);
  choose = the same frame around at most 8 numbered candidate sentences, guided to one integer.
Both are made for every item, whatever its facts, so report_v3.py can replay the gate and the ladder offline under the three facts
settings (main, gate only, facts unknown) and both B variants (the ladder; choose only) from the same drafts. Routing (section 2) is
applied by the report: under 'main' an L2/L3 item's calls are simply not used, and under 'unknown' step 0 shows A's excerpt
(ADR-0033 ffb384489), so those calls serve only to measure the drafts (format, entity check, non-answers).
Records the probe's hello, prompt and gate versions, macOS version and the machine (no host name), CPU of the probe and Apple's named
model daemons, their resident memory, and the load average (the Mac is shared)."""
import argparse, hashlib, json, os, shlex, subprocess, time
import gate
from runner_v2 import WATCH, cpu_seconds, snapshot, rss_of, Top

def sh(cmd):
    try: return subprocess.run(cmd, capture_output=True, text=True, timeout=10).stdout.strip()
    except Exception: return ""
def machine():
    """macOS version and the machine, never the host name"""
    return {"macos": sh(["sw_vers", "-productVersion"]), "macos_build": sh(["sw_vers", "-buildVersion"]), "arch": os.uname().machine,
            "model": sh(["sysctl", "-n", "hw.model"]), "chip": sh(["sysctl", "-n", "machdep.cpu.brand_string"]),
            "ram_gb": round(int(sh(["sysctl", "-n", "hw.memsize"]) or 0) / 2**30, 1), "cpus": os.cpu_count()}

class Probe:
    """the Apple probe over its JSON-lines protocol (apple_probe.swift, protocol 3)"""
    def __init__(s, cmd):
        s.p = subprocess.Popen(shlex.split(cmd), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        s.hello = s.p.stdout.readline().strip()
        h = json.loads(s.hello or "{}")
        if h.get("protocol") != 3: raise SystemExit(f"the probe speaks protocol {h.get('protocol')}, not 3: rebuild apple_probe.swift")
        if "available" not in str(h.get("available", "")).lower() or "unavailable" in str(h.get("available", "")).lower():
            raise SystemExit(f"Apple's model is not available here: {h.get('available')}")
    def call(s, req):
        t0 = time.perf_counter()
        s.p.stdin.write(json.dumps(req) + "\n"); s.p.stdin.flush()
        line = s.p.stdout.readline(); ms = (time.perf_counter() - t0) * 1000
        if not line: raise SystemExit("engine died")
        r = json.loads(line); r["latency_ms"] = round(ms); return r
    def pid(s): return s.p.pid
    def close(s):
        try: s.p.stdin.close(); s.p.wait(timeout=5)
        except Exception: s.p.kill()

class Fake:
    """no Apple calls: deterministic adversarial drafts, to prove the plumbing (a wrapped line, a dropped caveat, a polarity flip,
    a long line, a cut line) and a first-sentence pick"""
    hello = json.dumps({"hello": True, "protocol": 3, "guided": True, "available": "fake"})
    def __init__(s): s.n = 0
    def call(s, req):
        s.n += 1
        if req["mode"] == "pick": return {"text": "1", "finish": "stop", "out_tokens": 13, "refused": False, "retried_permissive": False, "error": "", "latency_ms": 1, "gen_ms": 1}
        body = req["prompt"].split(gate.QUOTE + "\n", 1)[1].rsplit("\n" + gate.QUOTE, 1)[0]
        first = (gate.sentences_v3(body) or [body])[0]
        k = int(hashlib.sha256(body.encode()).hexdigest(), 16) % 5
        text, fin = [(f'"{first}"', "stop"), (first.rstrip(".") + ", and all tests pass.", "stop"), (" ".join((body + " ") .split()[:32]), "stop"),
                     (" ".join(first.split()[:6]), "length"), (first, "stop")][k]
        return {"text": text, "finish": fin, "out_tokens": 20, "refused": False, "retried_permissive": False, "error": "", "latency_ms": 1, "gen_ms": 1}
    def pid(s): return None
    def close(s): pass

def write_req(ctx, pattern):
    r = {"id": "w", "mode": "line", "system": gate.INSTRUCTIONS, "prompt": gate.frame(ctx["cleaned"]), "max_tokens": gate.WRITE_MAX_TOKENS,
         "temperature": 0, "field": gate.LINE_FIELD, "field_description": gate.LINE_FIELD_DESCRIPTION}
    if pattern: r["pattern"] = gate.LINE_PATTERN
    return r
def choose_req(ctx):
    return {"id": "c", "mode": "pick", "system": gate.CHOOSE_INSTRUCTIONS, "prompt": gate.choose_frame(ctx["cands"]), "max_tokens": gate.CHOOSE_MAX_TOKENS,
            "temperature": 0, "field": "number", "field_description": "The number of the chosen sentence.", "count": len(ctx["cands"])}
def slim(r):
    return {k: r.get(k) for k in ("text", "finish", "out_tokens", "refused", "retried_permissive", "error", "raw", "latency_ms", "gen_ms") if k in r}

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--cmd", help="the engine command (the built apple_probe)"); g.add_argument("--fake", action="store_true", help="no Apple calls")
    ap.add_argument("--corpus", action="append", required=True, metavar="LABEL=PATH",
                    help="repeatable; slots v2, turnends, heldout (synthetic held-out), real-heldout (real held-out), each reported apart")
    ap.add_argument("--out", required=True); ap.add_argument("--repeats", type=int, default=3); ap.add_argument("--limit", type=int, default=0, help="first N items of each corpus")
    ap.add_argument("--pattern", action="store_true", help="also send gate.LINE_PATTERN as a pattern guide (UnsupportedGuide on macOS 27.0.1)")
    ap.add_argument("--top", action="store_true", help="sample `top` power into --topdir during the run"); ap.add_argument("--topdir", default=".")
    a = ap.parse_args()
    corpora = []
    for spec in a.corpus:
        label, _, path = spec.partition("=")
        if not path: raise SystemExit(f"--corpus wants LABEL=PATH, got {spec}")
        raw = open(path, "rb").read(); items = json.loads(raw)
        corpora.append((label, path, hashlib.sha256(raw).hexdigest(), items[:a.limit] if a.limit else items, len(items)))
    eng = Fake() if a.fake else Probe(a.cmd)
    before = snapshot(); rss0 = rss_of(WATCH); peak = rss0; load0 = os.getloadavg()
    top = Top(os.path.join(a.topdir, "top-run-v3.txt")) if a.top and not a.fake else None
    t_start = time.time(); res = []; first = True; ncalls = 0
    for label, path, digest, items, _n in corpora:
        for it in items:
            ctx = gate.prepare(it["source"])
            for rep in range(a.repeats):
                rec = {"corpus": label, "id": it["id"], "bucket": it.get("bucket"), "rep": rep, "cold_first": first, "lang": ctx["lang"],
                       "removed": ctx["removed"], "n_cands": len(ctx["cands"])}
                first = False
                rec["write"] = None if ctx["removed"] else slim(eng.call(write_req(ctx, a.pattern)))
                rec["choose"] = slim(eng.call(choose_req(ctx))) if ctx["cands"] else None
                ncalls += (rec["write"] is not None) + (rec["choose"] is not None)
                res.append(rec)
            if not a.fake: peak = max(peak, rss_of(WATCH))
    wall = time.time() - t_start
    if not a.fake: time.sleep(1)
    helper_cpu = cpu_seconds(sh(["ps", "-o", "time=", "-p", str(eng.pid())]) or "0") if eng.pid() else 0.0
    after = snapshot(); rss1 = rss_of(WATCH)
    if top: top.stop()
    eng.close()
    deltas = sorted(({"name": name, "cpu_s": round(t1 - before[pid][0], 2)} for pid, (t1, name) in after.items()
                     if pid in before and before[pid][1] == name and name in WATCH and t1 - before[pid][0] >= 0.05), key=lambda d: -d["cpu_s"])
    meta = {"engine": "fake (no Apple calls)" if a.fake else "Apple on-device Foundation Model (SystemLanguageModel.default) via apple_probe.swift",
            "hello": eng.hello, "prompt_version": gate.PROMPT_VERSION, "gate_version": gate.GATE_VERSION, "instructions": gate.INSTRUCTIONS,
            "label": gate.LABEL, "choose_instructions": gate.CHOOSE_INSTRUCTIONS, "guided": {"field": gate.LINE_FIELD, "description": gate.LINE_FIELD_DESCRIPTION,
            "pattern": gate.LINE_PATTERN if a.pattern else None, "choose": "one Int field guided to 1...count"},
            "line_max_words": gate.LINE_MAX_WORDS, "line_max_chars": gate.LINE_MAX_CHARS, "temperature": 0,
            "max_tokens_write": gate.WRITE_MAX_TOKENS, "max_tokens_choose": gate.CHOOSE_MAX_TOKENS, "repeats": a.repeats, "limit": a.limit or None,
            "corpora": [{"label": l, "file": os.path.basename(p), "sha256": d, "items_in_file": n, "items_run": len(i)} for l, p, d, i, n in corpora],
            "calls": ncalls, "wall_s": round(wall, 1), "machine": machine(), "load_avg_start": [round(x, 1) for x in load0],
            "load_avg_end": [round(x, 1) for x in os.getloadavg()], "helper_cpu_s": helper_cpu, "cpu_deltas": deltas,
            "watched_rss_mb": {"before": round(rss0, 1), "peak": round(peak, 1), "after": round(rss1, 1)},
            "started": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(t_start))}
    json.dump({"meta": meta, "results": res}, open(a.out, "w"), indent=1, ensure_ascii=False)
    print("wrote", a.out, len(res), "records,", ncalls, "calls,", round(wall, 1), "s")
if __name__ == "__main__": main()
