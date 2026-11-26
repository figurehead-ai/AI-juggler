#!/usr/bin/env python3
"""watch4.py: harness v8's watch drafting (AJ-E3830; the coordinator's v8 calls, 2026-11-26). v7's watch3 with its five "unsupported
claimed" rows closed. 0 unsupported claimed is a hard bar, ahead of 90 % exact.

v7 round: 77.1 % exact; 5 unsupported claimed (V7-W05, W19, W32, W43, W51), nearest offered 3 of 10. The causes, by code:
  - an exclusion over everything ("all sessions … except the retry logic one", "every project except X", "alle … außer …"): v7's
    resolver took the excluded one out and, with nothing else named, widened to "any". A watch can't exclude: that request is now
    unsupported, with the nearest option, and is never widened.
  - a numeric threshold no condition takes ("more than 50k tokens"; a cost; files or lines changed): v7 let the model map it to
    hits_limit. Now unsupported, with the nearest option (tokens or calls: hits_limit; money: runway_below; files, lines, commits:
    finishes), decided by code with no call.
  - "always" was no lifetime word (v5's list lacked it): "always tell me if anything fails" drafted today.
Never widen: whatever the path, a drafted watch on "any" (or on a project or App that holds what the request excluded) for a request
that excludes something is refused and answered as unsupported."""
import contextlib, os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402
from core6 import MAX_RETRIES                                       # noqa: E402
from core import register                                           # noqa: E402
import watch as W                                                   # noqa: E402
import watch3 as W3                                                 # noqa: E402

PROMPT_VERSION = 8
EXCLUSION = re.compile(r"\b(?:except(?:\s+for)?|excluding|apart\s+from|other\s+than|but\s+not|not\s+the\s+\S+(?:\s+\S+)?\s+one|außer|ausgenommen|bis\s+auf)\b[^.,;!?]*", re.I)
EVERYTHING = re.compile(r"\b(?:all|every|each|any|anything|everything|alle[snr]?|jede[snr]?|sämtliche)\b", re.I)
THRESHOLD = re.compile(
    r"(?:\b(?:more|over|above|exceeds?|exceeding|beyond|past|less|fewer|under|reaches?|hits?|crosses|goes\s+over|mehr|über|weniger)\s+(?:than\s+|als\s+)?)?"
    r"(?:\$|€|£)?\s?\d[\d,.]*\s*(?:k|m|tsd\.?|tausend)?\s*"
    r"(?P<unit>tokens?|dollars?|usd|eur|euros?|bucks|files?(?:\s+changed)?|lines?(?:\s+changed)?|commits?|requests?|api\s+calls?|calls?|messages?|mb|gb|kb|"
    r"dateien|zeilen|anfragen)\b"
    r"|(?:\b(?:more|over|above|exceeds?|exceeding|beyond|past|less|under|mehr|über|weniger)\s+(?:than\s+|als\s+)?)?(?:\$|€|£)\s?\d[\d,]*(?:\.\d+)?", re.I)
THRESHOLD_NEAR = [(r"token|request|call|anfrage|message", ("hits_limit", "runway_below")), (r"\$|€|£|dollar|usd|eur|euro|buck", ("runway_below", "hits_limit")),
                  (r"file|line|commit|datei|zeile", ("finishes", "ends"))]
def exclusion_over_everything(w, res):
    """the request excludes something from a set it doesn't name one by one: "all … except X", "every project except X" """
    m = EXCLUSION.search(w["request"])
    if not m or not res["excluded"]: return None
    before = w["request"][:m.start()]
    named_kept = [e for e in res["candidates"] if e["kind"] != "any" and W3._said(W3.masked_request(w["request"]), e["raw_name"])]
    if EVERYTHING.search(before) or not named_kept:
        return m.group(0).strip(" ,")
    return None
def threshold(w):
    """a number with a unit no condition takes (tokens, money, files or lines changed, calls): -> (the request's words, nearest options)"""
    r = W._mask_names(w)
    for m in THRESHOLD.finditer(r):
        words = m.group(0).strip()
        for rx, near in THRESHOLD_NEAR:
            if re.search(rx, words, re.I): return words, near
    return None
ALWAYS = re.compile(r"\b(?:always|immer)\b", re.I)
def lifetime_for(w):
    lt = W3.LIFETIME_FOR_V7(w)
    if lt == ("unsaid", None) and ALWAYS.search(w["request"]): return "always", None
    return lt
W3.LIFETIME_FOR_V7 = getattr(W3, "LIFETIME_FOR_V7", W3.lifetime_for)
@contextlib.contextmanager
def v8_reading():
    keep = W3.lifetime_for; W3.lifetime_for = lifetime_for
    try: yield
    finally: W3.lifetime_for = keep

class WatchNeverWidens(W3.WatchCodeDecides):
    name, letter, title = "watch4", "f8", "watch drafting: code decides, never widens, every unsupported request with its nearest"
    def unsupported(self, w, tg, words, nearest_order, why, decided):
        near = next((c for c in nearest_order if (tg and W.WATCH._offered_all(w, tg, c)) or (not tg and c in W3.offered_for(w, None))), None)
        if near is None: near = next((c for c in W3.ALWAYS_NEAR if c in W3.offered_for(w, tg[0] if tg else None)), None)
        o = W.WATCH._unsupported(w, tg, near, words, "code", [])
        o = dict(o, nearest=near, nearest_draft=None if not tg else o.get("nearest_draft"), shown=W.nearest_line(w, tg, near, words, w["lang"]) if near else o["shown"])
        return dict(o, decided=decided + [why]), "code"
    def run(self, w, setting, ask, mark=True, retries=MAX_RETRIES):
        if w.get("lang_known", True) and W.asks_to_be_told(w["request"]):
            res = W3.resolve(w)
            ex = exclusion_over_everything(w, res)
            conds = W3.cue_conditions(w)
            if ex:   # never widen: the exclusion is said back, the nearest offered with no target (the form asks which)
                return self.unsupported(w, [], ex, conds + list(W3.ALWAYS_NEAR), "unsupported: an exclusion over everything (code)", [])
            th = threshold(w)
            if th:
                tg = [c for c in res["candidates"] if c["kind"] != "any"][:1] if len(res["candidates"]) == 1 else []
                return self.unsupported(w, tg, th[0], list(th[1]), "unsupported: a threshold no condition takes (code)", [])
        with v8_reading():
            o, used = super().run(w, setting, ask, mark, retries)
        # never widen, whatever the path: a watch broader than the request for a request that excludes something is refused
        if o.get("kind") == "watch" and EXCLUSION.search(w["request"]):
            tgs = (o.get("watch") or {}).get("targets") or []
            if any(t["kind"] in ("any", "project", "app", "pool") for t in tgs):
                return self.unsupported(w, [], EXCLUSION.search(w["request"]).group(0).strip(" ,"), [c["type"] for c in o["watch"]["conditions"]] + list(W3.ALWAYS_NEAR),
                                        "unsupported: refused a widened watch (code)", o.get("decided") or [])
        return o, used
    def write_prompts(self):
        return W3.WatchCodeDecides.write_prompts(self).replace("Harness v7, job (f7) watch3: prompt version 7", "Harness v8, job (f8) watch4: prompt version 8").replace(
            "`selftest_v7.py --write-prompts`", "`selftest_v8.py --write-prompts`")
WATCH4 = register(WatchNeverWidens())
