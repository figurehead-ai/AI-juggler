#!/usr/bin/env python3
"""report.py results-X.json -> gate metrics (json to stdout)"""
import json, sys, statistics
def metrics(path):
    d=json.load(open(path)); R=d["results"]; n=len(R)
    ans=[r for r in R if not r["checks"]["non_answer"]]
    lat=sorted(r["latency_ms"] for r in R)
    warm=sorted(r["latency_ms"] for r in R if not r["cold_first"])
    p95=lambda v: v[min(len(v)-1,int(round(0.95*(len(v)-1))))]
    inv=[(r["id"],r["checks"]["entity_bad"]) for r in ans if not r["checks"]["entity_ok"]]
    pol=[(r["id"],r["checks"]["polarity_errs"]) for r in ans if not r["checks"]["polarity_ok"]]
    fmt=[(r["id"],r["checks"]["format_errs"]) for r in R if not r["checks"]["format_ok"]]
    adv=[r for r in R if r["bucket"]=="adversarial"]
    return {"n":n,"meta":d["meta"],
      "entity_pass":sum(1 for r in ans if r["checks"]["entity_ok"]),"entity_den":len(ans),"entity_fail":inv,
      "polarity_err_items":pol,"format_pass":sum(1 for r in R if r["checks"]["format_ok"]),"format_fail":fmt,
      "non_answers":sum(1 for r in R if r["checks"]["non_answer"]),"refused_default":sum(1 for r in R if r["retried_permissive"]),
      "still_refused":sum(1 for r in R if r["refused"]),"errors":[(r["id"],r["error"]) for r in R if r["error"]],
      "latency_median":statistics.median(warm),"latency_p95":p95(warm),"latency_cold_first":R[0]["latency_ms"],
      "by_bucket_lat":{b:statistics.median([r["latency_ms"] for r in R if r["bucket"]==b]) for b in ("short","medium","long","adversarial")},
      "adv_must_fail":[r["id"] for r in adv if r["checks"]["must_ok"] is False],
      "quoted_ge_90":sum(1 for r in ans if r["checks"]["copy_ratio"]>=0.9),
      "words_mean":round(statistics.mean(r["checks"]["words"] for r in ans),1)}
if __name__=="__main__":
    print(json.dumps(metrics(sys.argv[1]),indent=1,default=str))
