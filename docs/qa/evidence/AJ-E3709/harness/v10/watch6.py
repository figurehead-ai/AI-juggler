#!/usr/bin/env python3
"""watch6.py: harness v10's watch drafting (AJ-E3830; the coordinator's v10 calls on v9's results, 2026-11-26).

v9 round: 0 unsupported claimed (PASS), nearest 92 % (PASS), exact 68.75 % (FAIL). Code decided 58 of 60, so the misses are code's:
  - lifetimes said in words read as today: "permanently", "keep that on permanently", "dauerhaft" (always); "einmal" and the "once"
    phrasings (once) (V9-W01, W03, W30, W32, W40, W56, W58);
  - a named entity that is clearly the target refused for a broad word nearby ("Bei all meinen Projekten: … wenn gartenplaner scheitert";
    "any … OpenCode") (W02, W18, W25, W36, W37, W55); "but hold it until quiet hours are over" read as a meaning-changer (W21); a bare
    "say" read as "what an agent says" (W24); "I need to know the moment X finishes" read as no watch (W41);
  - an invented parameter: "the Pattern export one" gave asks_you 1 minute (W28);
  - nearest missing: "I want to know about every new session, leave out OpenCode" (W17, no watch), "more than 1.5 million tokens" (W47).
v10, still fail closed:
  1. lifetimes said in words: always ("permanently", "from now on", "every time", "keep watching", "keep that on", "for good",
     "dauerhaft", "immer"), once ("just this once", "the next time", "next time", "einmal", "nur beim nächsten Mal"), today ("for the
     rest of today", "for today", "bis heute Abend", "für heute"); unsaid stays unsaid (ADR-0036's default);
  2. entity wins, only when the entity is the subject or object of the condition clause (that clause names it and holds no broad word,
     or the broad word is "any time", an indefinite "anything/something … with|about|in <entity>", or "<quantifier> <entity> sessions")
     and no meaning-changer is present;
  3. a minutes value only with a time unit ("10 minutes", "an hour"); a percent only with "%"/"percent"/"Prozent";
  4. the asks-to-be-told cue reads "I need/want to know", "the moment"; "but" before quiet-hours words isn't a meaning-changer; a bare
     "say" isn't "what an agent says"; "1.5 million tokens" is a threshold."""
import contextlib, os, re, sys, unicodedata
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402,F401
from core6 import MAX_RETRIES                                       # noqa: E402
from core import register                                           # noqa: E402
import watch as W                                                   # noqa: E402
import watch3 as W3, watch4 as W4, watch5 as W5                     # noqa: E402

PROMPT_VERSION = 10
ALWAYS = re.compile(r"\b(?:permanent(?:ly)?|from\s+now\s+on|every\s+time|each\s+time|keep\s+watching|keep\s+(?:that|it|this)\s+on|for\s+good|forever|indefinitely|"
                    r"until\s+(?:i|you)\s+(?:remove|turn|switch)\w*|always|any\s+time|whenever|dauerhaft|immer|ab\s+jetzt|jedes\s+mal|auf\s+dauer)\b", re.I)
ONCE = re.compile(r"\b(?:just\s+this\s+once|just\s+once|only\s+once|once\s+only|one\s+time|(?:the\s+)?next\s+time|just\s+the\s+first\s+time|only\s+the\s+first\s+time|"
                  r"nur\s+beim\s+nächsten\s+mal|beim\s+nächsten\s+mal|nur\s+einmal|einmal)\b", re.I)
TODAY = re.compile(r"\b(?:for\s+the\s+rest\s+of\s+(?:the\s+)?(?:today|day)|for\s+today|today|tonight|this\s+evening|bis\s+heute\s+abend|für\s+heute|heute(?:\s+abend)?)\b", re.I)
def lifetime_for(w):
    req = unicodedata.normalize("NFKC", w.get("lifetime_source") or w["request"])
    m = W3.UNTIL_TIME.search(req)
    if m:
        t = W3.W2.request_times(m.group(1)) or W3.W2.request_times(m.group(1) + " uhr")
        if t: return "until_time", t[0]
    if W3.LIFE_ENDS.search(req): return "until_ends", None
    if ONCE.search(req): return "once", None
    if ALWAYS.search(req): return "always", None
    if TODAY.search(req): return "today", None
    return "unsaid", None
TIME_UNIT = re.compile(r"(\d{1,4}|an?|one|two|three|four|five|ten|fifteen|twenty|thirty|forty|fifty|sixty|ninety|half\s+an)\s*(?:-\s*)?(min(?:ute)?s?|minuten|h|hrs?|hours?|stunden?)\b", re.I)
PERCENT = re.compile(r"(\d{1,3})\s*(?:%|percent|prozent)", re.I)
def param_for(w, c):
    """a minutes value only with a time unit; a percent only with its sign or word (v9's W28: "the … one" gave 1 minute)"""
    p = W.COND[c][0]
    if p is None or p[1] != "int": return None
    r = W3.blanked(w)
    if c == "runway_below":
        m = PERCENT.search(r); return int(m[1]) if m else None
    for m in TIME_UNIT.finditer(r):
        n = m[1].lower(); unit = m[2].lower()
        v = 30 if n.startswith("half") else 1 if n in ("a", "an") else W.NUM_WORDS.get(n) or (int(n) if n.isdigit() else None)
        if v is None: continue
        return v * 60 if unit.startswith(("h", "stund")) else v
    return None
ASKS = re.compile(r"\b(?:(?:i\s+)?(?:need|want)\s+to\s+(?:know|hear)|the\s+moment|the\s+minute|as\s+soon\s+as|ich\s+will\s+wissen|sobald)\b", re.I)
_V5_ASKS = W.asks_to_be_told
def asks_to_be_told(request): return _V5_ASKS(request) or bool(ASKS.search(request or ""))
QUIET_BUT = re.compile(r"\bbut\s+(?:hold|wait|keep|save|park)\w*\b[^.;!?]{0,40}\bquiet\s+hours?\b|\bbut\s+(?:not\s+)?(?:during|in)\s+(?:my\s+)?quiet\s+hours?\b", re.I)
def meaning_word(w):
    r = QUIET_BUT.sub(" ", W._mask_names(w))
    m = W5.MEANING.search(r)
    return m.group(0) if m else None
INDEFINITE = re.compile(r"\b(?:anything|something|etwas|irgendwas)\b[^.,;!?]{0,30}?\b(?:with|about|in|on|from|for|mit|bei|an|in)\s+$", re.I)
def entity_wins(w):
    """the named entity is the subject or object of the condition clause, despite a broad word elsewhere: -> True when so"""
    named = W5.named_full(w)
    sp = [set(W._spaced(W._norm(e["raw_name"])).split()) for e in named]
    named = [e for i, e in enumerate(named) if not any(j != i and sp[i] < sp[j] for j in range(len(named)))]   # "Claude Code" inside "Claude Code CLI · Studio"
    if len(named) != 1 or meaning_word(w): return False
    e = named[0]; req = unicodedata.normalize("NFKC", w["request"])
    clauses = [c for c in re.split(r"[,;:.!?]", req) if c.strip()]
    conds = [c for c in clauses if W3.cue_conditions(dict(w, request=c))]
    for cl in conds:
        if not any(W3._said(cl, a) for a in W3.aliases(e)): continue
        q = W5.QUANT.search(cl)
        if not q or re.search(r"\bany\s+time\b", cl, re.I): return True
        at = min(cl.lower().find(a.lower()) for a in W3.aliases(e) if a.lower() in cl.lower()) if any(a.lower() in cl.lower() for a in W3.aliases(e)) else -1
        if at > q.start():
            between = cl[q.start():at]
            if INDEFINITE.search(between): return True                        # "anything goes wrong with Chapter index"
            if re.fullmatch(r"\s*(?:any|all|every|alle\w*|jede\w*)\s+(?:the\s+|of\s+the\s+|my\s+)?", between, re.I): return True   # "any OpenCode session"
    return False
CUE_SAY = re.compile(r"\bsay\b", re.I)
_V7_RESOLVE = W3.resolve
def resolve(w):
    """v7's resolver; of two names said in full where one holds the other ("Pattern export" and "pattern"), the longer one"""
    res = _V7_RESOLVE(w); c = res["candidates"]
    if len(c) >= 2:
        sp = [set(W._spaced(W._norm(e["raw_name"])).split()) for e in c]
        keep = [e for i, e in enumerate(c) if not any(j != i and sp[i] < sp[j] for j in range(len(c)))]
        if len(keep) < len(c): return dict(res, candidates=keep)
    return res
@contextlib.contextmanager
def v10_reading():
    """v9's reading with v10's: lifetimes and parameters said in words, the asks cue, quiet-hours "but", no bare "say", millions"""
    keep = (W3.lifetime_for, W4.lifetime_for, W3.param_for, W.asks_to_be_told, W5.meaning_word, W4.THRESHOLD, W3.V5_CUE_HITS, W5.lifetime_for,
            W5._V7_RESOLVE, list(W3.CUE_CONDITIONS))
    W3.lifetime_for = lifetime_for; W4.lifetime_for = lifetime_for; W3.param_for = param_for; W.asks_to_be_told = asks_to_be_told
    W5.lifetime_for = lifetime_for; W5._V7_RESOLVE = resolve
    W3.CUE_CONDITIONS[:] = [(c, rx + (r"|\bgo(?:es)?\s+wrong\b|\bwent\s+wrong\b|\bschief\w*" if c == "fails" else "")) for c, rx in W3.CUE_CONDITIONS]
    W5.meaning_word = meaning_word
    W4.THRESHOLD = re.compile(W4.THRESHOLD.pattern.replace(r"(?:k|m|tsd\.?|tausend)?", r"(?:k|m|million|millions|billion|thousand|tsd\.?|tausend|millionen?)?"), re.I)
    v5_hits = keep[6]
    W3.V5_CUE_HITS = lambda w: [h for h in v5_hits(w) if not (h[0] == "what an agent says" and CUE_SAY.fullmatch(h[2].strip()))]
    try: yield
    finally:
        (W3.lifetime_for, W4.lifetime_for, W3.param_for, W.asks_to_be_told, W5.meaning_word, W4.THRESHOLD, W3.V5_CUE_HITS, W5.lifetime_for,
         W5._V7_RESOLVE, conds) = keep
        W3.CUE_CONDITIONS[:] = conds

class WatchEntityWins(W5.WatchFailsClosed):
    name, letter, title = "watch6", "f10", "watch drafting: fail closed, the entity wins when it is the condition's own"
    def run(self, w, setting, ask, mark=True, retries=MAX_RETRIES):
        with v10_reading():
            if entity_wins(w):
                keep_q = W5.quantifier_beside_name; W5.quantifier_beside_name = lambda _w: None
                try: return W5.WatchFailsClosed.run(self, w, setting, ask, mark, retries)
                finally: W5.quantifier_beside_name = keep_q
            return W5.WatchFailsClosed.run(self, w, setting, ask, mark, retries)
    def write_prompts(self):
        return W5.WatchFailsClosed.write_prompts(self).replace("Harness v9, job (f9) watch5: prompt version 9", "Harness v10, job (f10) watch6: prompt version 10").replace(
            "`selftest_v9.py --write-prompts`", "`selftest_v10.py --write-prompts`")
WATCH6 = register(WatchEntityWins())
