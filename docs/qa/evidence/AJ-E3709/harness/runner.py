#!/usr/bin/env python3
"""Model-eval runner (stdlib only).
  runner.py --label apple --cmd ./probe/apple_probe --corpus corpus.json --out results-apple.json [--watch modelmanagerd,...] [--repeats 1]

ENGINE PROTOCOL (how a local model plugs in): --cmd is any program that stays alive, reads one JSON object per stdin line
  {"id":..,"system":..,"prompt":..,"max_tokens":60,"temperature":0}
and writes one JSON object per stdout line
  {"id":..,"text":"..","refused":false,"error":""}   (any other keys are kept; a first {"hello":true,..} line is ignored)
e.g. a thin wrapper around `llama-cli`/`llama-server`, or an MLX script. The runner times each request itself (wall ms),
samples peak RSS of the engine's process tree (+ any --watch process names, as delta over the pre-run baseline) with `ps`,
and scores with checks.py. Energy: powermetrics needs root on macOS ("must be invoked as the superuser"); not used here."""
import argparse, json, os, subprocess, threading, time, statistics, shlex
import checks
SYSTEM = "One plain sentence of at most 20 words. Use only what the message says. No new numbers, names or files."
PROMPT_VERSION = 1
def ps_table():
    out = subprocess.run(["ps","-axo","pid=,ppid=,rss=,comm="],capture_output=True,text=True).stdout
    rows=[]
    for l in out.splitlines():
        p=l.split(None,3)
        if len(p)==4 and p[0].isdigit(): rows.append((int(p[0]),int(p[1]),int(p[2]),p[3]))
    return rows
def tree_rss(root):
    rows=ps_table(); kids={root}; ch=True
    while ch:
        ch=False
        for pid,ppid,_,_ in rows:
            if ppid in kids and pid not in kids: kids.add(pid); ch=True
    return sum(r for pid,_,r,_ in rows if pid in kids)
def watch_rss(names):
    return sum(r for _,_,r,c in ps_table() if any(os.path.basename(c)==n or n in c for n in names))
class Sampler(threading.Thread):
    def __init__(s,pid,watch):
        super().__init__(daemon=True); s.pid=pid; s.watch=watch; s.stop=False; s.peak_own=0; s.peak_watch=0; s.base_watch=watch_rss(watch) if watch else 0
    def run(s):
        while not s.stop:
            try:
                s.peak_own=max(s.peak_own,tree_rss(s.pid))
                if s.watch: s.peak_watch=max(s.peak_watch,watch_rss(s.watch))
            except Exception: pass
            time.sleep(0.1)
def main():
    ap=argparse.ArgumentParser(); ap.add_argument("--label",required=True); ap.add_argument("--cmd",required=True)
    ap.add_argument("--corpus",default="corpus.json"); ap.add_argument("--out",required=True)
    ap.add_argument("--watch",default=""); ap.add_argument("--repeats",type=int,default=1); ap.add_argument("--limit",type=int,default=0); ap.add_argument("--wrap",action="store_true",help="variant: wrap the message in a labelled block (not ADR-0033 default)")
    a=ap.parse_args()
    items=json.load(open(a.corpus)); items=items[:a.limit] if a.limit else items
    watch=[w for w in a.watch.split(",") if w]
    p=subprocess.Popen(shlex.split(a.cmd),stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1)
    hello=p.stdout.readline()
    base_own=tree_rss(p.pid); samp=Sampler(p.pid,watch); samp.start()
    res=[]
    try:
        for n,it in enumerate(items):
            for rep in range(a.repeats):
                t0=time.perf_counter()
                p.stdin.write(json.dumps({"id":it["id"],"system":SYSTEM,"prompt":(("Message to summarise:\n\"\"\"\n"+it["source"]+"\n\"\"\"") if a.wrap else it["source"]),"max_tokens":60,"temperature":0})+"\n"); p.stdin.flush()
                line=p.stdout.readline()
                ms=(time.perf_counter()-t0)*1000
                if not line: raise SystemExit("engine died at "+it["id"])
                r=json.loads(line); text=(r.get("text") or "").strip().replace("\n"," ") if False else (r.get("text") or "").strip()
                c=checks.check_all(it["source"],text,it.get("gold"),r.get("refused",False))
                res.append({"id":it["id"],"bucket":it["bucket"],"rep":rep,"cold_first":(n==0 and rep==0),"source_chars":len(it["source"]),"output":text,"latency_ms":round(ms),
                            "engine_ms":r.get("gen_ms"),"refused":bool(r.get("refused")),"retried_permissive":bool(r.get("retried_permissive")),"error":r.get("error",""),"checks":c})
    finally:
        samp.stop=True; time.sleep(0.2)
        try: p.stdin.close(); p.wait(timeout=5)
        except Exception: p.kill()
    meta={"label":a.label,"hello":hello.strip(),"system_prompt":SYSTEM,"prompt_version":PROMPT_VERSION,"wrapped":a.wrap,"temperature":0,"max_tokens":60,
          "peak_rss_mb_engine_tree":round(samp.peak_own/1024,1),"baseline_rss_mb_engine_tree":round(base_own/1024,1),
          "watched":watch,"peak_rss_mb_watched_total":round(samp.peak_watch/1024,1),"baseline_rss_mb_watched_total":round(samp.base_watch/1024,1),
          "energy":"not measured: powermetrics requires root (sudo); not used","host":subprocess.run(["sw_vers","-productVersion"],capture_output=True,text=True).stdout.strip()+" "+os.uname().machine}
    json.dump({"meta":meta,"results":res},open(a.out,"w"),indent=1,ensure_ascii=False)
    print("wrote",a.out,len(res),"results; peak engine RSS MB",meta["peak_rss_mb_engine_tree"],"watched delta MB",round(meta["peak_rss_mb_watched_total"]-meta["baseline_rss_mb_watched_total"],1))
if __name__=="__main__": main()
