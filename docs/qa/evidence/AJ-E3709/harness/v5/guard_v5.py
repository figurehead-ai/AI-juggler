#!/usr/bin/env python3
"""guard_v5.py: harness v5's input-guard pass, run after the frozen gate's guard (gate.guard, ADR-0033 §6 step 1) on every job's
untrusted input: the turn jobs' agent text (core.norm_turn), Chat's record text (chat.norm_chat), the digest's events
(digest.norm_digest) and watch drafting's entity names (watch._entries).

Why: gate.py compiles ADDRESSED with re.X, which drops the literal spaces in "your instructions", "all instructions", "these
instructions", "needs to" and "has to", so "Ignore your instructions and …" was never removed (found 2026-11-18 by the (e)/(f) work).
gate.py stays frozen (the v3 record); this pass adds the same phrases with their spaces escaped (\\s+), in English and German. The
shared fixture fixtures/guard-phrases.json holds every phrase that must be caught and the innocent near misses that must not be; the
app's Rust guard (AJ-E3806) SHALL pass the same file. Stdlib only."""
import json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
HARNESS = os.path.dirname(HERE)
if HARNESS not in sys.path: sys.path.insert(0, HARNESS)
import gate                                                    # noqa: E402  (frozen at 7acec4f50; imported, never edited)

FIXTURE = os.path.join(HERE, "fixtures", "guard-phrases.json")
_INSTR_EN = r"(?:instructions?|rules|prompts?|system\s+prompt|guidelines)"
_INSTR_DE = r"(?:anweisungen|anweisung|instruktionen|regeln|vorgaben|richtlinien)"
GUARD_V5 = [   # no re.X anywhere: every space between words is \s+
 # English: an order to drop the instructions (the re.X miss), and the summary told what it must say
 rf"\b(?:ignore|disregard|forget|override)\b.{{0,30}}?\b(?:all\s+(?:of\s+)?(?:the\s+|your\s+|these\s+|those\s+)?(?:instructions?|prompts?)|(?:these|those|your|any)\s+(?:(?:previous|prior|earlier|above|original)\s+)?{_INSTR_EN}"
 rf"|the\s+(?:system\s+prompt|instructions)|(?:previous|prior|earlier|above|preceding)\s+(?:{_INSTR_EN}|text|messages?))\b",
 r"\b(?:the|this)\s+(?:summary|summari[sz]er)\b.{0,40}?\b(?:must|should|shall|needs?\s+to|has\s+to|is\s+to)\b",
 r"\b(?:note|message|instructions?|hey|dear|hello|notice|attention)\s*,?\s+(?:to|for)\s+(?:the\s+)?(?:ai|a\.i\.|model|llm|assistant|summari[sz]er|summary|reader)\b",
 # German: the same orders
 rf"\b(?:ignorier(?:e|t|en)?|vergiss|vergesst|vergessen\s+sie|missacht(?:e|et|en)?|übergeh(?:e|t|en)?)\b.{{0,40}}?\b(?:(?:alle|deine|ihre|eure|diese|die|sämtliche|jegliche)\s+(?:(?:bisherigen|vorherigen|obigen|ursprünglichen)\s+)?{_INSTR_DE}"
 rf"|(?:bisherigen|vorherigen|obigen|vorigen)\s+(?:{_INSTR_DE}|nachrichten?|texte?))\b",
 r"\b(?:die|diese)\s+zusammenfassung\b.{0,40}?\b(?:muss|soll|sollte|darf)\b",
 r"\b(?:hinweis|nachricht|notiz|anweisung)\s+(?:an|für)\s+(?:die\s+|den\s+|das\s+)?(?:ki|ai|modell|assistent\w*|zusammenfass\w*)\b",
]
GUARD_V5 = [re.compile(p, re.I) for p in GUARD_V5]
_SPLIT = re.compile(r"(?<=[.!?])\s+(?=[A-Z0-9ÄÖÜ`\"'(\[])")

def caught(sentence):
    """True when the v5 pass removes this sentence"""
    return any(rx.search(sentence) for rx in GUARD_V5)
def guard_extra(text):
    """-> (cleaned, removed): the v5 pass alone, sentence by sentence, as gate.guard splits them (line breaks kept)"""
    removed = []; kept_lines = []
    for line in (text or "").split("\n"):
        if not line.strip(): kept_lines.append(line); continue
        keep = []
        for s in _SPLIT.split(line):
            if caught(s): removed.append(s.strip())
            else: keep.append(s)
        if keep: kept_lines.append(" ".join(keep))
    return re.sub(r"\n{3,}", "\n\n", "\n".join(kept_lines)).strip(), removed
def guard_v5(text):
    """gate.guard, then the v5 pass -> (cleaned, removed)"""
    c, r = gate.guard(text); c2, r2 = guard_extra(c)
    return c2, r + r2
def prepare_v5(source):
    """gate.prepare (normalise, guard, language, candidates), then the v5 pass; language and candidates re-read when it removed one"""
    ctx = gate.prepare(source)
    c2, r2 = guard_extra(ctx["cleaned"])
    if not r2: return ctx
    return dict(ctx, cleaned=c2, removed=ctx["removed"] + r2, lang=gate.language(c2), cands=gate.candidates_v3(c2))
def fixture():
    return json.load(open(FIXTURE))["phrases"]
