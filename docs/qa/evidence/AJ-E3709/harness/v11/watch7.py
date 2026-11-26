#!/usr/bin/env python3
"""watch7.py: harness v11's watch drafting (AJ-E3830, D-149 amendment 3; the coordinator's v11 design, 2026-11-26).

v10 round: 3 unsupported claimed (a German "nicht von der Wetterstation" read as the target; "unless it ran for under an hour" dropped; "8
failed retries in a row" lost its count), nearest offered 72 % (two forms with no nearest), exact 85.7 %. The design is now **full coverage,
failing closed by construction**:
  1. the code parser stays first and claims a request only when it places every word (a target's name, kind word or "any" before one;
     a condition's cue word; a condition's number with its unit; a lifetime or quiet-hours phrase; a closed filler list); a word it
     can't place, or a number no condition said takes, and code doesn't claim it;
  2. then the model labels: a guided schema splits the request into spans, each with one slot (target, exclusion, condition,
     condition_param, lifetime, quiet_hours, filler, other); code checks the spans repeat the request exactly and reads them only for what
     to tell back and the nearest target. The model never makes a request supported;
  3. what can't be watched always comes with the nearest option (a test-proven invariant in the app).

**v11 mirrors the app by running it.** Earlier rounds re-implemented the app's rules in Python and drifted from it (v10's report showed
targets the app would have read otherwise). This job drives the app's own `watches::draft::draft` through the example binary
`crates/juggler-app/examples/watch_round.rs` over stdin and stdout; the harness makes every model call it asks for (the runner's engine,
sampling, retries and attempt records), so `replay6` re-scores a run offline from its recorded answers, as before.

The binary: `AIJ_WATCH_ROUND` (a path), else `$CARGO_TARGET_DIR/debug/examples/watch_round`; build it with
`scripts/heavy.sh cargo build --offline -p juggler-app --example watch_round` (the runbook)."""
import atexit, json, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), os.path.join(H, "v10"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402,F401
from core6 import MAX_RETRIES                                       # noqa: E402
from core import register                                           # noqa: E402
import watch as W                                                   # noqa: E402
import watch3 as W3, watch6 as W6                                   # noqa: E402

PROMPT_VERSION = 11
REPO = os.path.abspath(os.path.join(H, "..", "..", "..", "..", ".."))
APP_PROMPTS = os.path.join(REPO, "crates", "juggler-app", "src", "features", "watches", "draft_prompts")
DAY_MS = 86_400_000; NOW_MS = 9 * 3_600_000          # the harness's clock: 09:00 on its day (midnight is 0)

def binary():
    p = os.environ.get("AIJ_WATCH_ROUND")
    if not p and os.environ.get("CARGO_TARGET_DIR"): p = os.path.join(os.environ["CARGO_TARGET_DIR"], "debug", "examples", "watch_round")
    if not p or not os.path.exists(p):
        raise SystemExit("watch7: the app's drafting binary is missing: build it with `scripts/heavy.sh cargo build --offline -p juggler-app "
                         "--example watch_round` and set CARGO_TARGET_DIR (or AIJ_WATCH_ROUND to its path)")
    return p

class Drafter:
    """the app's drafting, one process for the run"""
    def __init__(self): self.p = None
    def start(self):
        if self.p is None or self.p.poll() is not None:
            self.p = subprocess.Popen([binary()], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1, encoding="utf-8")
            atexit.register(self.close)
        return self.p
    def send(self, obj):
        p = self.start(); p.stdin.write(json.dumps(obj, ensure_ascii=False) + "\n"); p.stdin.flush()
    def recv(self):
        line = self.p.stdout.readline()
        if not line: raise SystemExit("watch7: the app's drafting binary stopped")
        return json.loads(line)
    def close(self):
        if self.p is not None and self.p.poll() is None:
            self.p.stdin.close(); self.p.wait(timeout=10)
DRAFTER = Drafter()

def candidates(w):
    """the item's targets as the app's `Candidate`s; offered: the harness's own reading of what each can report (watch3.offered_for)"""
    out = []
    for e in w["entries"]:
        if e["kind"] == "any": continue
        raw = e.get("raw") or {}
        out.append({"kind": e["kind"], "id": e["id"], "name": e["raw_name"], "aliases": [a for a in (raw.get("aliases") or []) if isinstance(a, str)],
                    "app": e.get("app") if e["kind"] == "session" else None, "project": e.get("project") if e["kind"] == "session" else None,
                    "offered": W3.offered_for(w, e)})
    return out
def job_of(w, op):
    return {"op": op, "id": w["id"], "request": w["request"], "candidates": candidates(w), "midnight_ms": 0, "now_ms": NOW_MS}
def reading(w):
    """code's reading only: each word's slot and what code makes of the request (no model call)"""
    DRAFTER.send(job_of(w, "read")); return DRAFTER.recv()

def entity(w, t):
    if t.get("kind") == "any": return W.typed_target(w["by_ref"]["any"])
    e = next((x for x in w["entries"] if x["kind"] == t.get("kind") and x["id"] == t.get("id")), None)
    return W.typed_target(e) if e else {"kind": t.get("kind"), "id": t.get("id"), "ref": None, "name": None}
def watch_of(w, d):
    """the app's WatchDraftInfo -> the harness's watch (v5's form the scorer reads)"""
    tg = [entity(w, t) for t in d.get("targets") or []]
    conds = []
    for c in d.get("conditions") or []:
        p = c.get("param"); v = p.get("value") if isinstance(p, dict) else None
        conds.append({"type": c.get("code"), "param": v if isinstance(v, int) else (v if isinstance(v, str) else None)})
    lt = (d.get("lifetime") or {}).get("kind") or "unsaid"; until = None; said = lt != "unsaid"
    if lt == "until":
        at = int(d["lifetime"].get("atMs") or 0) % DAY_MS; lt = "until_time"; until = f"{at // 3_600_000:02d}:{at % 3_600_000 // 60_000:02d}"
    if lt == "unsaid": lt = W.default_lifetime(tg, conds)
    return {"targets": tg, "matching": d.get("matching") or "each", "conditions": conds, "lifetime": lt, "until": until, "lifetime_said": said,
            "even_in_quiet_hours": bool(d.get("evenInQuietHours", d.get("even_in_quiet_hours")))}
def shown_of(wt):
    tg = ", ".join(f"{t['kind']} {t.get('name') or t.get('id')}" for t in wt["targets"]) or "(the form asks)"
    cs = ", ".join(c["type"] + (f" {c['param']}" if c["param"] is not None else "") for c in wt["conditions"])
    return f"{tg} · {cs} · {wt['lifetime']}{' ' + wt['until'] if wt['until'] else ''}{' · even during quiet hours' if wt['even_in_quiet_hours'] else ''}"

class WatchFullCover(W6.WatchEntityWins):
    name, letter, title = "watch7", "f11", "watch drafting: the app's own code, full coverage failing closed"
    def run(self, w, setting, ask, mark=True, retries=MAX_RETRIES):
        DRAFTER.send(job_of(w, "draft")); n = {}
        while True:
            msg = DRAFTER.recv()
            if msg.get("op") == "call":
                step = msg["step"]; k = n.get(step, 0); n[step] = k + 1
                req = {"system": msg["system"], "prompt": msg["prompt"], "mode": "fields", "schema": msg["schema"], "max_tokens": msg["max_tokens"],
                       "job": self.name}
                out, a = ask(step, req, k)
                try: json.loads(out.get("text") or ""); a["ok"] = out.get("finish") != "length" and not out.get("error")
                except ValueError: a["errors"].append("shape: not JSON")
                DRAFTER.send({"op": "reply", "text": out.get("text") or "", "finish": out.get("finish") or "stop", "error": out.get("error") or "",
                              "refused": bool(out.get("refused"))})
                continue
            if msg.get("op") != "final": raise SystemExit(f"watch7: unexpected message from the app: {str(msg)[:200]}")
            break
        cover = msg.get("cover"); calls = msg.get("calls") or 0
        how = cover if isinstance(cover, str) else next(iter(cover), "?")
        decided = [f"cover: {how}"] + ([f"labels: model ({calls} call{'s' if calls != 1 else ''})"] if calls else [])
        used = "code" if not calls else max(n.values()) - 1
        o = msg.get("outcome")
        if o == "not_a_watch": return dict(W.WATCH._done(w, "not_a_watch", "", []), decided=decided), "code"
        kind = (o or {}).get("kind")
        if kind == "filled":
            wt = watch_of(w, o["draft"])
            return dict(W.WATCH._done(w, "watch", shown_of(wt), [], watch=wt), decided=decided), used
        if kind == "pick":
            return dict(W.WATCH._done(w, "ask", "Two match that name: the Watches form asks which one.", [], picker=[]), decided=decided), used
        if kind == "cant_watch":
            d = o.get("draft"); near = (d.get("conditions") or [{}])[0].get("code") if d else None
            nw = watch_of(w, d) if d else None
            shown = f"I can't watch for “{o.get('asked')}”." + (f" Nearest: {shown_of(nw)}." if nw else "")
            return dict(W.WATCH._done(w, "unsupported", shown, [], nearest=near, via="model labels" if calls else "code", asked=o.get("asked")),
                        decided=decided), used
        return dict(W.WATCH.fallback(w, setting), decided=decided), ("fallback" if calls else "code")
    def fake_draft(self, req, prompt, bad, k):
        """runner --fake: the label call calls the whole request other (or, when bad, a reworded filler span); call 1 says none"""
        if req.get("step") != "label": return {"target": "none"}
        request = prompt.split("REQUEST (from the developer): ", 1)[-1].split("\n", 1)[0]
        return {"spans": [{"text": ("please " if bad else "") + request, "slot": "filler" if bad else "other", "number": 0, "unit": "none"}]}
    def write_prompts(self):
        def text(f): return open(os.path.join(APP_PROMPTS, f), encoding="utf-8").read().rstrip("\n")
        doc = ["# Harness v11, job (f11) watch7: prompt version 11", "",
               "Generated by `selftest_v11.py --write-prompts` from the app's own prompt files (`crates/juggler-app/src/features/watches/"
               "draft_prompts/`); the app's code (`watches::draft`, `watches::draft::cover`) builds the prompts and the schemas, and this round "
               "runs that code. Order: code reads every word first; only a request it can't account for gets the labelling call; v7's "
               "call 1 (the target among several, each named in full) stays for what code claims; v7's call 2 (the model picking a "
               "condition) is gone: no cue word, no condition, the form.", ""]
        for title, f in (("The labelling call (v11)", "label.txt"), ("Call 1: the target among several (v7)", "target.txt")):
            doc += [f"## {title}", "", "### System prompt (instructions)", "", "```text", text(f), "```", ""]
        return "\n".join(doc)
WATCH7 = register(WatchFullCover())
