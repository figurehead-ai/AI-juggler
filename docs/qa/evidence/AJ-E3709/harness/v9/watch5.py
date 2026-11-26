#!/usr/bin/env python3
"""watch5.py: harness v9's watch drafting, fail closed (AJ-E3830; D-149 amendment 2, 2026-11-26).

v8 round: 3 unsupported claimed (V8-W27 "…, skipping the glossary hub" drafted on any session; V8-W54 "everything but form-kit" drafted
ON form-kit, the inverse; V8-W60 "the token count … passes one million" drafted as a usage limit), exact 75 %. Amendment 2: code accepts a
draft only when the request parses into a shape it fully understands; otherwise unsupported with the nearest option, or the form:

  1. a meaning-changing word anywhere outside a name (but, except, skipping, other than, besides, apart from, aside from, excluding,
     leave out, not the … one, without; außer, ohne, bis auf, nicht das/die/den/der): unsupported, said back, with the nearest; never a
     draft ("but only" / "but just", which narrows to what follows, is the one reading kept);
  2. a broad quantifier (all, every, everything, any, anything, each; alle, jede, irgendein) beside a named entity: unsupported, unless
     the entity is clearly the target: a selection ("of all my sessions, … checkout-api"), or the quantifier inside it ("anything in X");
  3. a number in a unit no condition takes (tokens, a token count, cost or money, files, lines, commits), the number in digits or words,
     before or after its unit: refused before any model call (v8's watch4 needed the number first);
  4. two or more entities named: only the clause that asks to be told is read (its sentence, from the last comma before the ask); still
     more than one: the form (code doesn't guess);
  5. "once" is a lifetime only as "just once", "only once", "one time", "the first time"; a bare "once" is "when";
  6. never widen (v8's guard, kept).
"""
import os, re, sys, unicodedata
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402,F401
from core6 import MAX_RETRIES                                       # noqa: E402
from core import register                                           # noqa: E402
import watch as W                                                   # noqa: E402
import watch3 as W3                                                 # noqa: E402
import watch4 as W4                                                 # noqa: E402

PROMPT_VERSION = 9
MEANING = re.compile(r"\b(?:but(?!\s+(?:only|just)\b)|except|skipping|skip|other\s+than|besides|apart\s+from|aside\s+from|excluding|leave\s+out|leaving\s+out|"
                     r"not\s+the\s+\S+(?:\s+\S+)?\s+one|without|außer|ohne|bis\s+auf|nicht\s+(?:das|die|den|der))\b", re.I)
QUANT = re.compile(r"\b(?:all|every|everything|any|anything|each|alle[snr]?|jede[snr]?|irgendein\w*)\b", re.I)
SELECTION = re.compile(r"\b(?:of|out\s+of|among|from|von|unter)\s+(?:all|every|alle[nr]?)\b", re.I)
WITHIN = re.compile(r"\b(?:any|anything|everything|all|every|alle[snr]?|irgendein\w*|jede[snr]?)\b(?:\s+\S+){0,3}?\s+(?:in|im|of|from|von|aus|inside|within)\s+", re.I)
NUMBER_WORDS = r"(?:\d[\d,.]*\s*(?:k|m|million|thousand|tausend|millionen?)?|(?:one|a|eine?|two|three|five|ten|twenty|fifty)\s+(?:hundred|thousand|million|millionen?|tausend)|one|two|three|five|ten|twenty|fifty|hundred)"
UNIT = r"(?:tokens?|token\s+count|token\s+usage|cost|costs|spend(?:ing)?|money|dollars?|euros?|bucks|files?|lines?|commits?|requests?|api\s+calls?|tokenanzahl|kosten|dateien|zeilen)"
UNIT_FIRST = re.compile(r"\b" + UNIT + r"\b[^.;!?]{0,60}?\b(?:passes|exceeds|reaches|hits|goes\s+(?:over|above|past)|is\s+(?:over|above|more\s+than)|tops|crosses|über(?:steigt|schreitet)?)\s+" + NUMBER_WORDS, re.I)
def meaning_word(w):
    m = MEANING.search(W._mask_names(w))
    return m.group(0) if m else None
def named_full(w):
    r = W3.masked_request(w["request"])
    return [e for e in w["entries"] if e["kind"] != "any" and any(W3._said(r, a) for a in W3.aliases(e))]
def quantifier_beside_name(w):
    r = W._mask_names(w)
    m = QUANT.search(r)
    if not m or not named_full(w): return None
    if SELECTION.search(r) or WITHIN.search(W3.masked_request(w["request"])): return None
    return m.group(0)
def threshold(w):
    th = W4.threshold(w)
    if th: return th
    m = UNIT_FIRST.search(W._mask_names(w))
    if not m: return None
    words = m.group(0).strip()
    for rx, near in W4.THRESHOLD_NEAR:
        if re.search(rx, words, re.I): return words, near
    return words, ("hits_limit", "finishes")
def watch_clause(w):
    """the part of the request that asks to be told: its sentence (split at . ; ! ?), from the last comma before the ask when what is left
    still names a condition ("…, so ping me just once when X finishes"); None when that is the whole request"""
    req = unicodedata.normalize("NFKC", w["request"])
    for sent in re.split(r"(?<=[.;!?])\s+|;\s*", req):
        m = next((x for x in W.WATCH_CUE.finditer(sent) if not re.match(r"remind|erinner", x.group(0), re.I)), None)
        if not m: continue
        start = sent.rfind(",", 0, m.start()) + 1
        clip = sent[start:].strip(" ,")
        part = clip if start and W3.cue_conditions(dict(w, request=clip)) else sent.strip()
        return None if part.strip(" .") == req.strip(" .") else part
    return None
ONCE_LIFE = re.compile(r"\b(?:just\s+once|only\s+once|once\s+only|one\s+time|just\s+the\s+first\s+time|only\s+the\s+first\s+time|nur\s+einmal|ein\s+einziges\s+mal)\b", re.I)
_V8_LIFETIME = W4.lifetime_for
_V7_RESOLVE = W3.resolve
def lifetime_for(w):
    lt = _V8_LIFETIME(w)
    if lt == ("once", None) and not ONCE_LIFE.search(w["request"]): return "unsaid", None
    return lt

class WatchFailsClosed(W4.WatchNeverWidens):
    name, letter, title = "watch5", "f9", "watch drafting: fail closed"
    def run(self, w, setting, ask, mark=True, retries=MAX_RETRIES):
        if w.get("lang_known", True) and W.asks_to_be_told(w["request"]):
            conds = W3.cue_conditions(w)
            mw = meaning_word(w)
            if mw:
                return self.unsupported(w, [], (W4.EXCLUSION.search(w["request"]) or re.search(re.escape(mw) + r"[^.,;!?]*", w["request"], re.I)).group(0).strip(" ,"),
                                        conds + list(W3.ALWAYS_NEAR), f"unsupported: a meaning-changing word ({mw!r}), code", [])
            th = threshold(w)
            if th:
                res = W3.resolve(w)
                tg = [c for c in res["candidates"] if c["kind"] != "any"][:1] if len(res["candidates"]) == 1 else []
                return self.unsupported(w, tg, th[0], list(th[1]), "unsupported: a threshold no condition takes (code, before any call)", [])
            q = quantifier_beside_name(w)
            if q:
                return self.unsupported(w, [], q + " … " + named_full(w)[0]["raw_name"], conds + list(W3.ALWAYS_NEAR),
                                        "unsupported: a broad quantifier beside a named entity (code)", [])
            clause = watch_clause(w)
            if clause is not None and (len(named_full(w)) >= 2 or W.cue_hits(w) != W.cue_hits(dict(w, request=clause)) or W3.cue_conditions(w) != W3.cue_conditions(dict(w, request=clause))):
                wc = dict(w, request=clause)
                if len(named_full(wc)) != 1 and not W3.resolve(wc)["candidates"]:
                    return dict(W.WATCH.fallback(w, setting), decided=["two entities named, the ask names none: the form (code)"]), "code"
                with v9_reading():
                    o, used = W4.WatchNeverWidens.run(self, dict(wc, lifetime_source=w["request"]), setting, ask, mark, retries)
                return o, used
        with v9_reading():
            return W4.WatchNeverWidens.run(self, w, setting, ask, mark, retries)
    def write_prompts(self):
        return W4.WatchNeverWidens.write_prompts(self).replace("Harness v8, job (f8) watch4: prompt version 8", "Harness v9, job (f9) watch5: prompt version 9").replace(
            "`selftest_v8.py --write-prompts`", "`selftest_v9.py --write-prompts`")
import contextlib                                                    # noqa: E402
@contextlib.contextmanager
def v9_reading():
    """v8's reading with the bare-"once" fix; the lifetime read from the whole request when the ask was clipped to its clause"""
    keep4, keep3, keep_r = W4.lifetime_for, W3.lifetime_for, W3.resolve
    def lt(w): return lifetime_for(dict(w, request=w.get("lifetime_source") or w["request"]))
    W4.lifetime_for = lt; W3.lifetime_for = lt; W3.resolve = resolve
    try: yield
    finally: W4.lifetime_for = keep4; W3.lifetime_for = keep3; W3.resolve = keep_r
def resolve(w):
    """v7's resolver; a session and its own project or App named alike, with no kind word: the session (v8's W23, W32)"""
    res = _V7_RESOLVE(w); c = res["candidates"]
    if len(c) == 2 and not any(rx.search(W._mask_names(w)) for rx in W3.KIND_WORDS.values()):
        s = [e for e in c if e["kind"] == "session"]; o = [e for e in c if e["kind"] in ("project", "app")]
        if len(s) == 1 and len(o) == 1 and W._norm(s[0].get(o[0]["kind"]) or "") in W._keys(o[0]) \
                and W._spaced(W._norm(s[0]["raw_name"])) == W._spaced(W._norm(o[0]["raw_name"])):
            return dict(res, candidates=s)
    return res
WATCH5 = register(WatchFailsClosed())
