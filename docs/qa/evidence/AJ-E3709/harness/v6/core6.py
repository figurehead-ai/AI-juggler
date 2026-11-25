#!/usr/bin/env python3
"""core6.py: AJ-E3709 harness v6's shared parts (D-149; AJ-E3830, AJ-E3831). Stdlib only. Runbook: v6-runbook.md.

v6 changes WHAT the model is asked, not who checks it. v5's code supplies the truth and every check (imported, never copied, never
edited: ../v5/); v6's jobs ask for less:

  - watch2      (AJ-E3830) two small calls: the one target (an enum of this request's refs), then the condition and lifetime (enums
                 built for that target: the conditions it reports, the numbers and times the request says, a short enum for what can't
                 be watched). v5's WatchDraft checks the result as it checked v5's one-call draft.
  - feed_pick   (AJ-E3831) "pick, don't write": code lists the turn's facts (the agent's own clauses that passed every v5 check, and
                 git's kept files); the model picks one by its number and a verb from an enum; code renders the line, and v5's feed
                 line checks read it.
  - digest_mr   (AJ-E3831) map and reduce: one feed_pick per calm agent's newest turn; code orders, groups, counts and assembles (v5's
                 rollup and compose). The model never writes the digest.

What lives here: the run loop for jobs of several calls (run6, replay6), sampled retries (D-149: retries sample, they are not
greedy), and the harness clock (the blind writer resolved "today" against HARNESS_TODAY)."""
import hashlib, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
V5 = os.path.join(os.path.dirname(HERE), "v5")
if V5 not in sys.path: sys.path.insert(0, V5)
import core                                         # noqa: E402  (v5: the registry, the item kinds, norm_turn, MODELS, retry_note)
from core import j4, gate                           # noqa: E402,F401

PROMPT_VERSION = 6
MAX_RETRIES = 2                                     # D-130 lever 2, per call: at most two regenerations of each call, then the fallback
HARNESS_TODAY = "2026-11-18"                        # the blind writer's "today": gold "today" lifetimes end on this day

# ======================================================================================================================
# Sampled retries (D-149 point 2). The first attempt keeps the model's tested decoding (Apple: greedy, pinned; a GGUF model: its
# chosen arm); a retry samples, with a seed from the item and the attempt, so a rejected draft is not regenerated as the same draft.
# Apple's random mode takes a temperature, a top-k and a seed (apple_probe.swift's "sampling"); a GGUF model keeps its arm's top-p,
# top-k and min-p, its temperature raised to at least RETRY_TEMPERATURE. Neither value is tuned on a held-out item: they are the
# harness's choice, measured by the round.
# ======================================================================================================================
RETRY_TEMPERATURE = 0.6
RETRY_TOP_K = 40
def seed_of(key):
    return int(hashlib.sha256(key.encode()).hexdigest(), 16) % 2_000_000_000
def attempt_sampling(entry, sname, key, rep, retry):
    """the sampling for one call: the model's tested arm on the first attempt, a seeded sample on a retry"""
    base = core.sampling_for(entry, sname, key, rep)
    if retry == 0: return base
    s = seed_of(f"{key}:retry{retry}:rep{rep}")
    if entry["family"] == "apple":
        return {"mode": "random", "temperature": RETRY_TEMPERATURE, "top_k": RETRY_TOP_K, "seed": s}
    return dict(base, temperature=max(float(base.get("temperature") or 0.0), RETRY_TEMPERATURE), seed=s)

# ======================================================================================================================
# The run loop for v6's jobs. A job brings run(item, setting, ask, mark, retries) -> (final outcome, used); `ask(step, request)` makes
# one call and records it. used: 0 when every call passed at its first attempt; n > 0 when the worst call needed n retries; "fallback"
# when the plain path is shown after the retries; "code" when code decided with no call. Deterministic given the engine's outputs, so
# replay6 re-scores a run offline from its recorded attempts (a rule change replays without the model).
# ======================================================================================================================
class Asker:
    def __init__(self, call):
        self.call = call; self.attempts = []; self.retry_of = {}
    def __call__(self, step, req, retry):
        out = self.call(dict(req, step=step, retry=retry))
        a = {"step": step, "attempt": retry, "text": out.get("text"), "finish": out.get("finish"), "latency_ms": out.get("latency_ms"),
             "gen_ms": out.get("gen_ms"), "out_tokens": out.get("out_tokens"), "error": out.get("error") or "", "refused": bool(out.get("refused")),
             "est_tokens": req.get("est_tokens"), "retry_note": retry > 0, "errors": [], "ok": False}
        self.attempts.append(a)
        return out, a
def run6(job, item, setting, call, mark=True, retries=MAX_RETRIES):
    """-> record {model_called, attempts: [{step, attempt, text, finish, latency_ms, errors, ok}], final, used}"""
    ask = Asker(call)
    final, used = job.run(item, setting, ask, mark, retries)
    return {"model_called": bool(ask.attempts), "attempts": ask.attempts, "final": final, "used": used}
def replay6(job, item, setting, rec, mark=True, retries=MAX_RETRIES):
    recorded = list(rec.get("attempts") or []); n = [0]
    def call(req):
        k = n[0]; n[0] += 1
        if k >= len(recorded): return {"text": None, "finish": "stop", "error": "no recorded attempt (the rules changed since the run)"}
        a = recorded[k]
        if a.get("step") not in (None, req.get("step")):
            return {"text": None, "finish": "stop", "error": f"recorded step {a.get('step')} where the rules now ask {req.get('step')}"}
        return {"text": a.get("text"), "finish": a.get("finish") or "stop", "refused": a.get("refused"), "error": a.get("error"),
                "latency_ms": a.get("latency_ms")}
    return run6(job, item, setting, call, mark, retries)

def parse(schema, text):
    """the engine's text -> (fields or None, hard errors): v4's parser and validator, DATAMARK unmarked"""
    obj, err = j4.parse_fields(text)
    if obj is None: return None, [err or "unparsed"]
    obj = j4.unmark_fields(obj)
    hard, _soft = j4.validate(schema, obj)
    return (None, hard) if hard else (obj, [])
def note(prev_text, reasons, task="the fields"):
    """the retry note: v5's (the previous answer and our checks' reasons), with the task it asks again"""
    return core.retry_note(prev_text, reasons).replace("Write the fields again", f"Choose {task} again")
