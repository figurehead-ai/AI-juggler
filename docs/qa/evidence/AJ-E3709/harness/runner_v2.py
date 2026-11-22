#!/usr/bin/env python3
"""runner_v2.py --cmd probe/apple_probe --corpus corpus-v2.json --out results-apple-v2.json [--repeats 3]   (stdlib only)
Runs the engine through ADR-0033's whole path's model calls: guard (gate.py) -> write (frame v2) and choose (same frame), for every item and repeat.
The gate and the ladder are applied offline by report_v2.py (pure functions in gate.py), so facts variants can be replayed on the same drafts.
Choose is always called (even when the ladder might not reach it) so the ladder can be replayed under other facts; it costs a call, not a verdict.
Also records: CPU time of the helper and every process before/after (energy estimate input), and `top` power samples during the run and idle."""
import argparse, json, os, re, shlex, subprocess, time
import gate
WATCH = ["modelmanagerd","generativeexperiencesd","GenerativeExperiencesSafetyInferenceProvider","intelligenceplatformd","IntelligencePlatformComputeService","mlhostd","modelcatalogd","apple_probe"]
def cpu_seconds(t):
    sec = 0.0
    for p in t.split(":"): sec = sec*60 + float(p)
    return sec
def snapshot():
    out = subprocess.run(["ps","-axo","pid=,time=,comm="],capture_output=True,text=True).stdout
    d = {}
    for l in out.splitlines():
        p = l.split(None, 2)
        if len(p) == 3 and p[0].isdigit():
            try: d[int(p[0])] = (cpu_seconds(p[1]), os.path.basename(p[2]))
            except ValueError: pass
    return d
def rss_of(names):
    out = subprocess.run(["ps","-axo","rss=,comm="],capture_output=True,text=True).stdout; tot = 0
    for l in out.splitlines():
        p = l.split(None, 1)
        if len(p) == 2 and any(os.path.basename(p[1]) == n for n in names): tot += int(p[0])
    return tot/1024
class Top:
    """top -stats pid,command,cpu,power sampled every 3 s into a file. POWER is top's Energy Impact index, not watts."""
    def __init__(s, path):
        s.f = open(path, "w")
        s.p = subprocess.Popen(["top","-l","0","-s","3","-n","40","-o","power","-stats","pid,command,cpu,power"], stdout=s.f, stderr=subprocess.DEVNULL)
    def stop(s):
        s.p.terminate(); s.p.wait(); s.f.close()
def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--cmd", required=True); ap.add_argument("--corpus", required=True); ap.add_argument("--out", required=True)
    ap.add_argument("--repeats", type=int, default=3); ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--idle", type=int, default=0, help="idle seconds of top sampling before the run (0 = skip)"); ap.add_argument("--topdir", default=".")
    a = ap.parse_args()
    items = json.load(open(a.corpus)); items = items[:a.limit] if a.limit else items
    if a.idle:
        t = Top(os.path.join(a.topdir, "top-idle.txt")); time.sleep(a.idle); t.stop()
    p = subprocess.Popen(shlex.split(a.cmd), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
    hello = p.stdout.readline()
    def call(system, prompt, max_tokens):
        t0 = time.perf_counter()
        p.stdin.write(json.dumps({"id":"x","system":system,"prompt":prompt,"max_tokens":max_tokens,"temperature":0})+"\n"); p.stdin.flush()
        line = p.stdout.readline(); ms = (time.perf_counter()-t0)*1000
        if not line: raise SystemExit("engine died")
        r = json.loads(line)
        return {"text":(r.get("text") or "").strip(),"refused":bool(r.get("refused")),"retried_permissive":bool(r.get("retried_permissive")),"error":r.get("error",""),"latency_ms":round(ms),"engine_ms":r.get("gen_ms")}
    before = snapshot(); rss0 = rss_of(WATCH); top = Top(os.path.join(a.topdir, "top-run.txt")); t_start = time.time()
    res = []; peak = rss0; first = True
    for it in items:
        cleaned, removed = gate.guard(it["source"]); cands = gate.candidates(cleaned)
        for rep in range(a.repeats):
            rec = {"id":it["id"],"bucket":it["bucket"],"rep":rep,"cold_first":first,"removed":removed,"n_cands":len(cands)}; first = False
            rec["write"] = None if removed else call(gate.INSTRUCTIONS_V2, gate.frame(cleaned), 60)
            rec["choose"] = call(gate.CHOOSE_INSTRUCTIONS_V2, gate.choose_frame(cands), 8) if cands else None
            rec["frame_only"] = call(gate.INSTRUCTIONS_V2, gate.frame(it["source"]), 60) if removed else None
            res.append(rec)
        peak = max(peak, rss_of(WATCH))
    wall = time.time() - t_start
    time.sleep(1)
    helper_cpu = cpu_seconds(subprocess.run(["ps","-o","time=","-p",str(p.pid)],capture_output=True,text=True).stdout.strip() or "0")
    after = snapshot(); rss1 = rss_of(WATCH); top.stop()
    try: p.stdin.close(); p.wait(timeout=5)
    except Exception: p.kill()
    deltas = []
    for pid, (t1, name) in after.items():
        if pid in before and before[pid][1] == name and t1 - before[pid][0] >= 0.05:
            deltas.append({"pid":pid,"name":name,"cpu_s":round(t1 - before[pid][0],2)})
    deltas.sort(key=lambda x: -x["cpu_s"])
    ncalls = sum(1 for r in res for k in ("write","choose","frame_only") if r[k])
    meta = {"hello":hello.strip(),"prompt_version":gate.PROMPT_VERSION_V2,"gate_version":gate.GATE_VERSION_V2,"instructions":gate.INSTRUCTIONS_V2,"label":gate.LABEL,"choose_instructions":gate.CHOOSE_INSTRUCTIONS_V2,
            "temperature":0,"max_tokens_write":60,"max_tokens_choose":8,"repeats":a.repeats,"corpus":os.path.basename(a.corpus),"calls":ncalls,"wall_s":round(wall,1),
            "host":subprocess.run(["sw_vers","-productVersion"],capture_output=True,text=True).stdout.strip()+" "+os.uname().machine,
            "helper_cpu_s":helper_cpu,"watched_rss_mb":{"before":round(rss0,1),"peak":round(peak,1),"after":round(rss1,1)},"cpu_deltas":deltas[:25],
            "started":time.strftime("%Y-%m-%d %H:%M:%S",time.localtime(t_start))}
    json.dump({"meta":meta,"results":res}, open(a.out,"w"), indent=1, ensure_ascii=False)
    print("wrote", a.out, len(res), "records,", ncalls, "calls,", round(wall,1), "s; helper cpu", helper_cpu)
if __name__ == "__main__": main()
