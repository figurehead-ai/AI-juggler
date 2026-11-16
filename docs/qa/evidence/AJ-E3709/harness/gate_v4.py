"""gate_v4.py: GATE_VERSION "eval-4", A's excerpt with caveats and failures found by structure and the L3 safety net
(ADR-0033 "What the run found in A itself"; AJ-E3708 Req.6b-6d, 855a92ee9). Stdlib only.

gate.py stays frozen at 7acec4f50 as eval-3 and is not edited: this module imports it and changes A's excerpt only.
  6b caveat_kinds(): a negation in the same clause as a verb of running, testing, building, compiling, executing, checking, verifying
     or confirming (kind "unverified"), or of finishing, starting, completing, doing or "get to" (kind "incomplete"), in any tense or
     voice, EN and DE; a verbal negation with "yet"; "only" restricting such a verb; "still a TODO", "stubbed", "left as it was",
     "happy path only", "only ... of ...", "for X only", N-of-M counts (incomplete); plus eval-3's phrase lists. Advice ("should not"),
     orders ("Do not run", "Please do not"), conditions ("If ... not run"), reported content ("saying that ... wasn't started") and a
     reduced relative after an indefinite noun ("An App not stored yet") are not caveats. A shown excerpt carries every kind its text
     states (H008: "3 of 5" does not stand in for "I didn't run the suite").
  6c reports_failure_v4(): a present or past statement that something errors, fails, breaks, crashes, panics, or does not build,
     compile, start, run or work; an error code (E0432, ERR_..., a non-zero exit code); a "Bad news:" or "Leider" lead; German
     separable verbs (bricht ... ab, stürzt ... ab, schlägt ... fehl) and "hat nicht geklappt". Report, not describe: conditionals,
     if/when clauses, modals, relative clauses, the object of a fix, a resolved failure ("the errors are gone") and a zero count
     ("0 failed") stay excluded, as in eval-3's reports_failure.
  6d excerpt_v4(): on an L3 item the excerpt opens on the first sentence that reports a failure and never on a success-only sentence;
     with none, it shows the opening and the record says "facts line carries the failure".
B's gate (steps 0-5) and the ladder are eval-3's unchanged: ladder_v4 runs gate.ladder_v3 and swaps in this excerpt.
`python3 gate_v4.py --selftest` runs the eval-4 self-tests."""
import re, sys
import gate
from gate import (nfkc, _fam3, _is_neg, sentences_v3, _cap, facts_norm, verdict_words_v3, COND_WORDS, MODAL_WORDS, AVOID_WORDS,
                  RELATIVE_WORDS, INDEFINITE, DEFINITE, PAST_FAIL)

GATE_VERSION = "eval-4"
FACTS_LINE_FLAG = "facts line carries the failure"

def _tk(s):
    """words with digits kept ("0 failed", "e0432"); contractions and hyphenated words are one token ("didn't", "built-in")"""
    return re.findall(r"[^\W_]+(?:['-][^\W_]+)*", nfkc(s).lower().replace("’", "'"))
CLAUSE_SPLIT = re.compile(r"[,;:()—–]|\s-\s|\b(?:but|aber|sondern|whereas|während)\b", re.I)
def clauses(sentence): return [c for c in CLAUSE_SPLIT.split(nfkc(sentence)) if c and c.strip()]
CONJ = {"and", "or", "und", "oder", "then", "dann"}

# ---------- 6b: caveats by structure ----------
NEG_PARTICLE = {"not", "never", "cannot", "without", "nicht", "nie", "niemals", "ohne"}     # plus every n't word
NEG_DET = {"no", "none", "nothing", "nobody", "neither", "nor", "kein", "keine", "keinen", "keiner", "keinem", "keines", "nichts"}
def _neg_particle(w): return w in NEG_PARTICLE or w.endswith("n't")
ADVICE = {"should", "must", "need", "needs", "shall", "may", "might", "would", "sollte", "sollten", "soll", "muss", "müssen", "darf", "dürfen"}
ADVICE_NEG = {"shouldn't", "mustn't", "needn't", "wouldn't", "mightn't", "shan't"}
BE_AUX = {"is", "are", "was", "were", "be", "been", "being", "am", "got", "get", "gets", "wurde", "wurden", "wird", "werden", "worden", "ist", "sind", "war", "waren"}
AUX = BE_AUX | {"have", "has", "had", "do", "does", "did", "will", "can", "could", "to", "zu", "habe", "hat", "haben", "hatte", "hatten",
                "bin", "konnte", "konnten", "kann", "können"}
ADVERB = {"yet", "actually", "fully", "even", "really", "also", "ever", "properly", "still", "all", "any", "it", "them", "this", "that",
          "noch", "ja", "auch", "schon", "wirklich", "alle", "es", "sie", "das", "die", "den", "dem"}
# the verbs a caveat negates: unambiguous forms (past, participle, -ing) count anywhere in the window; base and -s forms only right
# after a negation, an auxiliary or "to" (so "no build errors" and "the test run" are nouns, not verbs)
_V = {"run": ("ran running", "run runs"), "test": ("tested testing", "test tests"), "build": ("built building", "build builds"),
      "compile": ("compiled compiling", "compile compiles"), "execute": ("executed executing", "execute executes"),
      "check": ("checked checking", "check checks"), "verify": ("verified verifying", "verify verifies"),
      "finish": ("finished finishing", "finish finishes"), "start": ("started starting", "start starts"),
      "confirm": ("confirmed confirming", "confirm confirms"),
      "complete": ("completed completing", "complete completes"), "do": ("done", "")}
_VERIFY = {"run", "test", "build", "compile", "execute", "check", "verify", "confirm"}
VERB_SURE = {f for a, _ in _V.values() for f in a.split()}
VERB_BASE = {f for _, b in _V.values() for f in b.split()}
VERIFY_FORMS = {f for k, (a, b) in _V.items() if k in _VERIFY for f in (a + " " + b).split()}
GET_TO = {"get", "got", "gotten", "getting"}
DE_PART = {"getestet", "ausgeführt", "durchgeführt", "gebaut", "kompiliert", "gestartet", "geprüft", "überprüft", "verifiziert", "gelaufen",
           "beendet", "abgeschlossen", "erledigt", "angefangen", "begonnen", "fertiggestellt", "auszuführen", "durchzuführen", "auszuprobieren"}
DE_INF = {"testen", "ausführen", "durchführen", "bauen", "kompilieren", "starten", "prüfen", "überprüfen", "verifizieren", "laufen",
          "beenden", "anfangen", "beginnen", "fertigstellen"}
DE_VERIFY = {"getestet", "ausgeführt", "durchgeführt", "gebaut", "kompiliert", "geprüft", "überprüft", "verifiziert", "gelaufen",
             "auszuführen", "durchzuführen", "auszuprobieren", "testen", "ausführen", "durchführen", "bauen", "kompilieren", "prüfen",
             "überprüfen", "verifizieren", "laufen"}
def _kind_of(verb):
    """unverified: a verb of running, testing, building, compiling, executing, checking; incomplete: finishing, starting, get to, done"""
    return "unverified" if verb in VERIFY_FORMS or verb in DE_VERIFY else "incomplete"
REPORTING = {"say", "says", "saying", "said", "tell", "tells", "telling", "told", "state", "states", "stating", "explain", "explains",
             "ensure", "ensures", "ensuring", "mean", "means", "assert", "asserts", "asserting", "sagt", "sagen", "prüft", "stellt", "heißt"}
def _verb_at(toks, i):
    """is toks[i] one of the caveat verbs used as a verb? -> its family word or None"""
    w = toks[i]; prev = [t for t in toks[:i] if t not in ADVERB][-1:] or [""]
    p = prev[0]
    if w in VERB_SURE or w in DE_PART: return w
    if w in VERB_BASE and (p in AUX or _neg_particle(p)): return w
    if w in DE_INF and (p in AUX or _neg_particle(p) or p == "zu" or (i + 1 < len(toks) and toks[i + 1] in {"lassen", "gelassen"})): return w
    if w in GET_TO and i + 1 < len(toks) and toks[i + 1] in {"to", "round", "around"}: return "get to"
    if w == "gekommen" and "dazu" in toks[max(0, i - 2):i]: return "dazu gekommen"
    return None
def _excluded_neg(toks, j):
    """a negation that advises, sets a condition or gives an order is not a caveat about the turn"""
    if toks and toks[0] in {"please", "bitte", "kindly"}: toks, j = toks[1:], j - 1      # "Please do not publish it yet": an order
    if j < 0: return False
    before = toks[:j]
    if any(t in COND_WORDS for t in before): return True
    if toks[j] in ADVICE_NEG or any(t in ADVICE for t in toks[max(0, j - 2):j]): return True
    if j == 0 and toks[j] in {"don't", "never", "nie"} : return True                       # imperative: "Don't run X", "Never build Y"
    if j == 1 and toks[0] == "do" and toks[1] == "not": return True
    # reported content ("--rules saying plainly that this session wasn't started"): what a text says, not what the turn did
    th = [k for k in range(j) if toks[k] in {"that", "dass"}]
    if th and any(t in REPORTING for t in toks[max(0, th[-1] - 3):th[-1]]): return True
    # a reduced relative after an indefinite noun ("An App not stored yet takes ..."): a kind of thing, not the turn's work
    if toks[j] == "not" and toks and toks[0] in INDEFINITE and ([t for t in toks[:j] if t not in ADVERB][-1:] or [""])[0] not in AUX: return True
    return False
def _negated_verb(clause):
    """-> the matched 'negation ... verb' text, or None"""
    toks = _tk(clause)
    for j, n in enumerate(toks):
        if _neg_particle(n):
            if _excluded_neg(toks, j): continue
            for i in range(j + 1, min(len(toks), j + 5)):
                if toks[i] in CONJ: break
                v = _verb_at(toks, i)
                if v and (i - j <= 3 or toks[i] in DE_PART): return _kind_of(v), f"{n} … {v}"
            verbal = n.endswith("n't") or (n == "not" and ([t for t in toks[:j] if t not in ADVERB][-1:] or [""])[0] in AUX)
            if verbal and "yet" in toks[j + 1:]: return "incomplete", f"{n} … yet"
        elif n in NEG_DET:
            if _excluded_neg(toks, j): continue
            # "no testing", "keine Tests ausgeführt", "No tests were run", "none of the suites were executed"
            for i in range(j + 1, min(len(toks), j + 7)):
                if toks[i] in CONJ: break
                w = toks[i]
                if i == j + 1 and w.endswith("ing") and w in VERB_SURE: return _kind_of(w), f"{n} {w}"
                if w in DE_PART: return _kind_of(w), f"{n} … {w}"
                if (w in VERB_SURE or w in VERB_BASE) and toks[i - 1] in BE_AUX and not w.endswith("ing"): return _kind_of(w), f"{n} … {toks[i - 1]} {w}"
    return None
def _only_verb(clause):
    """'only' restricting a caveat verb: "I only checked three screens", "tested on macOS only", "nur die Unit-Tests ausgeführt" """
    toks = _tk(clause)
    for j, n in enumerate(toks):
        if n not in {"only", "nur"}: continue
        for i in range(j + 1, min(len(toks), j + 5)):
            if (toks[i] in VERB_SURE and not toks[i].endswith("ing")) or toks[i] in DE_PART: return f"{n} {toks[i]}"
        for i in range(max(0, j - 5), j):
            if toks[i] in VERB_SURE and not toks[i].endswith("ing") and not any(t in CONJ for t in toks[i:j]): return f"{toks[i]} … {n}"
    return None
NUMWORD = r"(?:\d+|one|two|three|four|five|six|seven|eight|nine|ten|some|half|part|a\s+few|a\s+couple|ein\w*|zwei|drei|vier|fünf|teil\w*)"
PARTIAL_RX = re.compile(r"""
    \b(?:still|is|are|remains?|left|stays?)\s+(?:a\s+|an\s+|as\s+a\s+)?(?:open\s+)?TODO\b | \bnoch\s+(?:ein\s+)?TODO\b
  | \b(?:is|are|was|were|still|remains?|left|stays?)\s+(?:\w+\s+)?stub(?:bed)?\b
  | \bleft\s+as\s+(?:it|they)\s+(?:was|were)\b
  | \bhappy[- ]path\s+only\b | \bonly\s+(?:the\s+)?happy[- ]path\b | \bnur\s+(?:den\s+|der\s+)?happy[- ]path\b
  | \bonly\s+(?:[\w-]+\s+){0,3}?""" + NUMWORD + r"""\s+(?:[\w./-]+\s+){0,3}?of\b
  | \bnur\s+(?:\w+\s+){0,3}?""" + NUMWORD + r"""\s+(?:\w+\s+){0,3}?von\b
  | \bfor\s+(?:[\w./-]+\s+){1,3}only\b
  | \bteilweise\b
""", re.I | re.X)
def caveat_matches(sentence):
    """-> [(kind, matched text)] for one sentence. Kinds: unverified (not run, not tested, not built or checked) and incomplete (not
    finished or started, not yet, only part of it, a TODO, a stub). A shown excerpt carries every kind its text states."""
    out = []; t = nfkc(sentence).replace("’", "'")
    if gate._counts(t): out.append(("incomplete", "N of M"))
    for c in clauses(t):
        p = gate._caveat_phrase(c)        # eval-3's phrases, under the same advice / condition / order exclusions as the structure
        if p:
            toks = _tk(c); j = next((k for k, w in enumerate(toks) if _neg_particle(w) or w in {"noch", "cannot"}), None)
            if j is None or not _excluded_neg(toks, j):
                out.append(("unverified" if re.search(r"(?i)run|test|unverified|cannot say|can't say|ausgeführt", p) else "incomplete", p))
        m = _negated_verb(c)
        if m: out.append(m)
        m = _only_verb(c)
        if m: out.append(("incomplete", m))
    m = PARTIAL_RX.search(t)
    if m: out.append(("incomplete", m.group(0)))
    return out
def caveat_kinds(sentence): return {k for k, _ in caveat_matches(sentence)}
def text_caveat_kinds(cleaned):
    return set().union(*(caveat_kinds(s) for s in sentences_v3(cleaned))) if sentences_v3(cleaned) else set()

# ---------- 6c: failures by structure ----------
FAIL_STEMS = list(gate.OUTCOME_FAIL) + ["killed", "timed", "exceed*"]     # eval-3's outcome stems; "timed" only in "timed out"
def _is_fail_word(w, nxt):
    if w == "timed": return nxt == "out"
    return any((x.endswith("*") and w.startswith(x[:-1])) or w == x for x in FAIL_STEMS)
ZERO = {"0", "zero", "null"}
RESOLVED_AFTER = {"gone", "fixed", "resolved", "behoben", "weg", "verschwunden"}
COMPOUND_NEXT = {"message", "messages", "handling", "handler", "handlers", "mapping", "mappings", "path", "paths", "case", "cases",
                 "type", "types", "text", "texts", "page", "pages", "state", "states", "mode", "modes", "rate", "budget", "reporting",
                 "reporter", "boundary", "boundaries", "variant", "variants", "branch", "branches", "class", "classes", "display",
                 "behaviour", "behavior", "banner", "toast", "dialog", "screen", "view", "meldung", "meldungen", "behandlung"}
FIRST_PERSON = {"i", "we", "ich", "wir"}
WORKV_RUN = {"build", "builds", "compile", "compiles", "start", "starts", "run", "runs", "launch", "launches", "boot", "boots",
             "gebaut", "kompiliert", "gestartet", "gelaufen", "baut", "startet", "läuft", "lief"}
WORKV_OTHER = {"work", "works", "load", "loads", "pass", "passes", "geklappt", "klappt", "klappte", "funktioniert", "funktionierte", "bestanden"}
DO_NEG = {"doesn't", "don't", "didn't", "won't", "can't", "cannot", "couldn't"}
DO_AUX = {"does", "do", "did", "will", "can", "could"}
CODE_RX = re.compile(r"(?<![\w.-])E0\d{3}\b|\bERR_[A-Z0-9_]+\b|\bexit(?:ed)?\s+(?:with\s+)?(?:code|status)\s*[:=]?\s*(-?\d+)|\bnon-?zero\s+exit\b|\bExit-Code\s+(-?\d+)", re.I)
CODE_EXCLUDE = {"for", "handle", "handles", "handled", "handling", "map", "maps", "mapped", "mapping", "catch", "catches", "caught",
                "on", "about", "document", "documents", "documented", "explain", "explains", "explained"}
# the object of a fix or an inquiry ("Root-causes the intermittent failures") describes; a design idiom ("fails closed") is no event
FIX_LIKE = {"root-cause", "root-causes", "root-caused", "diagnose", "diagnoses", "diagnosed", "investigate", "investigates",
            "investigated", "debug", "debugs", "debugged", "explain", "explains", "explained", "untersucht", "erklärt"}
IDIOM_NEXT = {"closed", "open", "safe", "fast", "soft", "gracefully", "loudly"}
LEAD_RX = re.compile(r"^\W*(?:bad\s+news\b|leider\b)", re.I)
def _common_excluded(toks, i, past, governed, ci):
    """eval-3's 'report, not describe' exclusions, at a failure word's position"""
    before = toks[:i]
    if governed and ci > 0 and not past: return True
    if any(_is_neg(t) for t in toks[max(0, i - 3):i]) or (i and toks[i - 1] in ZERO): return True
    if any(t in COND_WORDS for t in before): return True
    if any(t in MODAL_WORDS for t in toks[max(0, i - 3):i]): return True
    if any(t in AVOID_WORDS for t in toks[max(0, i - 3):i]): return True
    if any(_fam3(t) == "fixed" or t in FIX_LIKE for t in before): return True
    if not past and any(t in RELATIVE_WORDS for t in before): return True
    return False
def reports_failure_v4(sentence):
    """-> the reason (a short string) the sentence reports a failure, or None"""
    s = nfkc(sentence)
    if LEAD_RX.search(s): return "lead: " + LEAD_RX.search(s).group(0).strip(" :")
    governed = False
    for ci, clause in enumerate(re.split(r"[,;:()—–]|\s-\s", s)):
        toks = _tk(clause)
        if ci == 0 and toks and toks[0] in COND_WORDS: governed = True
        for i, w in enumerate(toks):
            nxt = toks[i + 1] if i + 1 < len(toks) else ""
            # 1. a failure word, reported (eval-3's exclusions, plus: resolved after it, a zero count, a compound noun)
            if _is_fail_word(w, nxt):
                past = w.endswith("ed") or w in PAST_FAIL or (w == "fehl" and bool({"schlug", "schlugen"} & set(toks)))
                if i and toks[i - 1] in INDEFINITE: continue
                if not past and i and toks[i - 1] in DEFINITE: continue
                if not past and nxt in COMPOUND_NEXT: continue
                if w.startswith("fail") and nxt in IDIOM_NEXT: continue
                if _common_excluded(toks, i, past, governed, ci): continue
                if not past and any(t in COND_WORDS for t in toks[i + 1:i + 3]): continue
                if any(t in RESOLVED_AFTER and not any(_is_neg(x) for x in toks[max(i + 1, k - 2):k]) for k, t in enumerate(toks) if k > i): continue
                return f"failure word: {w}"
            # 2. a negated "works" verb: doesn't build / didn't work / klappt nicht / hat nicht geklappt / läuft nicht
            if w in WORKV_RUN or w in WORKV_OTHER:
                if any(t in COND_WORDS for t in toks[:i]) or (governed and ci > 0): continue
                if w in WORKV_RUN and FIRST_PERSON & set(toks[:i]): continue        # "I didn't build it": a caveat, not a failure
                prev = [t for t in toks[max(0, i - 3):i] if t not in ADVERB]
                en = (prev and prev[-1] in DO_NEG) or (len(prev) >= 2 and prev[-2] in DO_AUX and prev[-1] == "not") \
                     or (len(prev) >= 2 and prev[-2:] == ["no", "longer"] and w.endswith("s"))
                de = (w in {"geklappt", "gebaut", "kompiliert", "gestartet", "gelaufen", "bestanden", "funktioniert"} and "nicht" in toks[max(0, i - 4):i]) \
                     or (w in {"klappt", "klappte", "funktioniert", "funktionierte", "läuft", "lief", "baut", "startet", "kompiliert"} and "nicht" in toks[i + 1:i + 4])
                if (en or de) and not (any(t in RELATIVE_WORDS for t in toks[:i]) and w.endswith("s")):
                    return f"negated: {w}"
            # 3. German separable verbs: bricht ... ab, stürzt ... ab
            if w in {"bricht", "brechen", "brach", "brachen", "stürzt", "stürzen", "stürzte", "stürzten"} and "ab" in toks[i + 1:]:
                k = toks.index("ab", i + 1)
                if any(_is_neg(t) for t in toks[i:k]): continue
                past = w in {"brach", "brachen", "stürzte", "stürzten"}
                if _common_excluded(toks, i, past, governed, ci): continue
                return f"separable: {w} … ab"
            if w in {"abgebrochen", "abgestürzt"} and not _common_excluded(toks, i, True, governed, ci): return f"failure word: {w}"
        # 4. an error code or a non-zero exit code
        for m in CODE_RX.finditer(clause):
            code = m.group(1) or m.group(2)
            if code is not None and int(code) == 0: continue
            before = _tk(clause[:m.start()])
            if set(before[-3:]) & CODE_EXCLUDE: continue
            if _common_excluded(before + ["x"], len(before), True, governed, ci): continue
            return f"error code: {m.group(0)}"
    return None

# ---------- the excerpt (eval-4) ----------
def _asks(s): return s.rstrip().rstrip("\"'”’)]").endswith("?")
def _trim(s, matches):
    if len(s) <= 200: return s
    key = next((m for _, m in matches if m and m in s), None)
    if key is None:
        words = [w for _, m in matches for w in (m or "").split() if w not in {"…", "..."} and w in s.lower()]
        i = s.lower().find(words[0]) if words else -1
    else: i = s.find(key)
    if i < 0: return _cap(s)
    i = max(0, i - 80); return ("..." if i else "") + s[i:i + 190]
def excerpt_v4(cleaned, facts=None):
    """-> (text, second, flags). The opening: the first sentence (L2: the first question; L3: the first sentence that reports a failure).
    L3 safety net: the opening never only reports success while a failure sentence exists; with none, the opening is shown and flagged
    FACTS_LINE_FLAG. Facts unknown: a later question, else a later failure, is the second line (as eval-3). Then every caveat kind the
    text states (unverified, incomplete) that no shown line carries gets the first sentence that carries it, up to three lines in all; a
    kind that does not fit is flagged. `second` names the lines after the opening, e.g. 'failure+caveat', or None."""
    F = facts_norm(facts); level = F["level"] if F else None; flags = []
    alls = sentences_v3(cleaned)
    sents = [s for s in alls if len(s.split()) >= 3] or alls
    if not sents: return _cap(cleaned.strip(), 160), None, ([FACTS_LINE_FLAG] if level == "L3" else [])
    pick = None
    if level == "L2": pick = next((s for s in sents if "?" in s), None)
    if level == "L3": pick = next((s for s in sents if reports_failure_v4(s)), None)
    pick = pick or sents[0]
    if level == "L3" and not reports_failure_v4(pick):
        if verdict_words_v3(pick)["succ"]:
            fail_s = next((s for s in alls if reports_failure_v4(s)), None)      # the net; unreachable while the pick above holds
            if fail_s: pick = fail_s; flags.append("L3 net: a success-only opening replaced by the failure sentence")
        if not reports_failure_v4(pick): flags.append(FACTS_LINE_FLAG)
    lines = [pick]; kinds = []
    if F is None:
        after = alls[alls.index(pick) + 1:] if pick in alls else alls
        q = next((s for s in after if _asks(s)), None)
        f = None if q else next((s for s in after if reports_failure_v4(s)), None)
        if q or f: lines.append(q or f); kinds.append("question" if q else "failure")
    have = set().union(*(caveat_kinds(x) for x in lines)); need = text_caveat_kinds(cleaned)
    shown = [_cap(x) for x in lines]
    for s in alls:
        if not (need - have): break
        if s in lines: continue
        ms = caveat_matches(s); k = {kk for kk, _ in ms}
        if not (k & (need - have)): continue
        if len(shown) >= 3: flags.append("caveat not shown, no room: " + ",".join(sorted(need - have))); break
        shown.append(_trim(s, ms)); lines.append(s); kinds.append("caveat"); have |= k
    return "\n".join(shown), ("+".join(kinds) or None), flags

def ladder_v4(item, facts, write_out, choose_out, route=True, variant="ladder", ctx=None, unknown_rule=True):
    """gate.ladder_v3 (B's gate and ladder, eval-3) with A's excerpt from excerpt_v4 wherever the excerpt is shown"""
    ctx = ctx or gate.prepare(item["source"])
    L = gate.ladder_v3(item, facts, write_out, choose_out, route=route, variant=variant, ctx=ctx, unknown_rule=unknown_rule)
    exc, second, flags = excerpt_v4(ctx["cleaned"], facts)
    L.update(excerpt=exc, second_line=second, caveat_line="caveat" in (second or ""), excerpt_flags=flags)
    if L["kind"] in ("excerpt", "excerpt_routed"): L["shown"] = exc
    return L

# ---------- yardsticks for the reports (eval-4's own reading of a shown text) ----------
def shown_says_failure(shown):
    """eval-4's reading: a shown line reports a failure by structure, or negates a success ("I haven't fixed it yet")"""
    return verdict_words_v3(shown)["nsucc"] or any(reports_failure_v4(x) for x in shown.split("\n"))
def caveat_dropped(cleaned, shown):
    """-> the caveat kinds the text states that no shown line carries (empty: none dropped)"""
    have = set().union(*(caveat_kinds(x) for x in shown.split("\n")))
    return sorted(text_caveat_kinds(cleaned) - have)

# ---------- self-tests: `python3 gate_v4.py --selftest` ----------
def selftest():
    fails = []; n = [0]
    def check(name, cond, detail=""):
        n[0] += 1
        if not cond: fails.append(f"{name}: {detail}")
    L1 = {"outcome": "none", "tests": {"state": "unknown"}, "level": "L1"}
    L0 = {"outcome": "success", "tests": {"state": "unknown"}, "level": "L0"}
    L3 = {"outcome": "failure", "tests": {"state": "unknown"}, "level": "L3"}
    def ex(src, facts): return excerpt_v4(gate.normalise(src), facts)
    # 6b caveats: each sentence must carry a caveat (and the kind where it matters)
    for s, kind in [
        ("No tests were run and I did not add new ones.", "unverified"),                                   # T029
        ("I haven't compiled with the `strict` feature, and `cargo test` was not executed.", "unverified"),  # T034
        ("The PDF rendering and the email step are not started.", "incomplete"),                          # T015
        ("XLSX is still stubbed: `read_xlsx()` returns `Err(Unsupported)`.", "incomplete"),                 # T013
        ("The retry logic works for GET requests only.", "incomplete"),                                     # T017
        ("I did not get to resumable chunks: that is still a TODO at line 88.", "incomplete"),            # H005
        ("api/routes/refunds.go: happy path only, error mapping left as it was", "incomplete"),             # H007
        ("I didn't run the suite.", "unverified"),                                                         # H008
        ("Pixel output should be the same, but I only checked three screens by eye, not all.", "incomplete"),  # T063
        ("I didn't get to the docs.", "incomplete"), ("The e2e suite is not started yet.", "incomplete"),
        ("The migration has never been run against production data.", "unverified"),
        ("I haven't fixed it yet.", "incomplete"), ("None of the integration suites were executed.", "unverified"),
        ("I committed it without running the tests.", "unverified"), ("No testing was done on Windows.", "unverified"),
        ("I couldn't verify the upload against the real bucket.", "unverified"),
        ("I think the new overlay blocks the click, but I have not confirmed it.", "unverified"),           # T046
        ("Only 2 of the 5 endpoints are migrated.", "incomplete"), ("Only two of the five endpoints are migrated.", "incomplete"),
        ("Ran the unit tests only.", "incomplete"),
        ("Die Migration wurde nicht ausgeführt.", "unverified"), ("Ich habe nicht getestet, ob der Export klappt.", "unverified"),
        ("Das Release ist noch nicht gebaut.", "unverified"), ("Es wurden keine Tests ausgeführt.", "unverified"),
        ("Ich habe die Tests nicht laufen lassen.", "unverified"), ("Ich bin nicht dazu gekommen, die Doku zu schreiben.", "incomplete"),
        ("Der Export ist teilweise umgesetzt.", "incomplete"), ("Ich habe nur 3 von 5 Seiten geprüft.", "incomplete"),
    ]:
        k = caveat_kinds(s)
        check(f"caveat: {s!r} -> {kind}", kind in k, (k, caveat_matches(s)))
    for s in ["The check in ci/imports.sh fails if anything still imports core.config, and it reports none.",   # H023
              "This would fail on older systems.", "58 passed, 0 failed.", "No errors appear in the log.",
              "No build errors.", "Testing shows no regressions.", "Do not run the migration twice.", "Never build from a dirty tree.",
              "If the tests are not run, CI blocks the merge.", "You should not run this against production.",
              "Nothing changed and the tests were run.", "All 41 tests ran and passed.", "Added a test with a stubbed clock.",
              "There were no errors and I ran the full suite.", "The parser no longer crashes on empty input.",
              "Moved the config loader from core/ to platform/config/ and fixed the imports.", "Nothing functional changed.",
              "Keine Fehler beim Bauen.", "Alle 14 Tests bestehen.", "I ran cargo test and cargo clippy; both are clean.",
              # false positives seen on the v2 commit bodies (M14, M19, L20), fixed by structure
              "--rules saying plainly that this session wasn't started with AI Juggler's agent tools.",
              "An App not stored yet takes the free colour most unlike those in use.",
              "The once-only check would have run and been marked done.", "--disallowed-tools takes built-in tools only.",
              "Please do not publish it yet."]:
        check(f"no caveat: {s!r}", not caveat_kinds(s), caveat_matches(s))
    # 6c failures
    for s, want in [
        ("Bad news: while removing unused imports I deleted a re-export that cli/main.rs needs, so cargo build now errors with E0432.", True),  # H016
        ("Good news: the lint errors are gone.", False),                                                  # H016
        ("Das hat nicht geklappt: Die Migration 0042 bricht mit 'column already exists' ab, und die Datenbank steht jetzt halb migriert.", True),  # H014
        ("Die Migration 0042 bricht mit 'column already exists' ab.", True), ("Der Build schlägt beim Linken fehl.", True),
        ("Leider ist die Datenbank jetzt halb migriert.", True), ("Der Server startet nicht.", True), ("Die App stürzt beim Start ab.", True),
        ("cargo build doesn't compile on stable.", True), ("The crate no longer builds.", True), ("The deploy did not work.", True),
        ("The job exited with code 2.", True), ("npm run build fails with ERR_MODULE_NOT_FOUND.", True), ("The server crashed twice.", True),
        ("The check in ci/imports.sh fails if anything still imports core.config, and it reports none.", False),   # H023
        ("This would fail on older systems.", False), ("58 passed, 0 failed.", False), ("No errors appear in the log.", False),
        ("I didn't build the release.", False), ("The tests were not run.", False), ("Rewrote the parser's error messages.", False),
        ("Added error handling for the upload.", False), ("Fixed E0432 in cli/main.rs.", False), ("The job exited with code 0.", False),
        ("Falls der Test fehlschlägt, bricht der Lauf ab.", False), ("When the token expires, the upload fails.", False),
        ("The parser no longer crashes on empty input.", False), ("Added handling for ERR_TIMEOUT.", False),
        ("If it doesn't build, run make clean.", False), ("A check that fails when the schema is newer stops the launch.", False),
        # false positives seen on the v2 commit bodies: task ids are not error codes; a refusal described is not a failure
        ("Plans epic E13 (AJ-E1301..E1305) and lands its first step.", False), ("Groundwork for Muse Code (AJ-E1608).", False),
        ("Keep theirs audited; Clean refused while diverged.", False), ("E2910 R4's row says why.", False),
        ("automated Fleet gate (just gate-e7, fails closed)", False), ("Root-causes the intermittent guardrails failures.", False),
    ]:
        check(f"reports_failure_v4: {s!r} -> {want}", bool(reports_failure_v4(s)) == want, reports_failure_v4(s))
    # eval-3's reports_failure fixtures hold under eval-4 too
    for s, want in [("The build failed on the second run.", True), ("Three tests fail on main.", True), ("It crashes on start.", True),
                    ("The build failed when I ran it under load.", True), ("The job was killed once and retried.", True),
                    ("I changed the timeout from 30 to 60 seconds.", False), ("Der Build ist fehlgeschlagen.", True), ("Zwei Tests schlagen fehl.", True),
                    ("The import fails if a link points outside the folder.", False), ("A broken link is left out instead of failing the import.", False),
                    ("Fixed the null pointer error in the logger.", False), ("Wenn die Datei fehlt, schlägt der Import fehl.", False),
                    ("Der Import würde sonst fehlschlagen.", False)]:
        check(f"reports_failure_v4 keeps eval-3's {s!r} -> {want}", bool(reports_failure_v4(s)) == want, reports_failure_v4(s))
    # the corpus items, as A's excerpt shows them (sources quoted from the corpora)
    h016 = "Good news: the lint errors are gone. Bad news: while removing unused imports I deleted a re-export that cli/main.rs needs, so cargo build now errors with E0432. I haven't fixed it yet."
    for facts in (L3, None):
        t, sec, fl = ex(h016, facts)
        check(f"H016 ({'L3' if facts else 'unknown'}): the E0432 sentence is shown, never the good news alone",
              "errors with E0432" in t and not t.startswith("Good news") if facts else "errors with E0432" in t, (t, sec, fl))
    t, sec, fl = ex(h016, L3)
    check("H016 (L3): opens on the failure, keeps 'haven't fixed it yet', no flag", t.split("\n")[0].startswith("Bad news") and "haven't fixed it yet" in t and not fl, (t, fl))
    t, sec, fl = ex("Das hat nicht geklappt: Die Migration 0042 bricht mit 'column already exists' ab, und die Datenbank steht jetzt halb migriert. Ich habe nichts zurückgerollt.", L3)
    check("H014 (L3, German): the failure sentence opens, no flag", t.startswith("Das hat nicht geklappt") and not fl and shown_says_failure(t), (t, fl))
    t, sec, fl = ex("Added input validation to `POST /orders` (`handlers/orders.go`): quantity must be 1 to 999. No tests were run and I did not add new ones.", L1)
    check("T029: 'No tests were run' is a caveat line", sec == "caveat" and "No tests were run" in t, (t, sec))
    t, sec, fl = ex("I rewrote the parser's error messages in `parser/errors.rs`. I haven't compiled with the `strict` feature, and `cargo test` was not executed.", L1)
    check("T034: the not-compiled / not-executed sentence is a caveat line", sec == "caveat" and "was not executed" in t, (t, sec))
    t, sec, fl = ex("I rewrote the parser's error messages in `parser/errors.rs`. I haven't compiled with the `strict` feature, and `cargo test` was not executed.", None)
    check("T034 (unknown): 'error messages' is not a failure line", sec == "caveat", (t, sec))
    t, sec, fl = ex("First half is in: the `Invoice` model and its migration `0042_invoice.sql`. The PDF rendering and the email step are not started. The model tests pass, 9 of 9.", L1)
    check("T015: 'not started' is a caveat line", "are not started" in t, (t, sec))
    t, sec, fl = ex("The importer now handles CSV and TSV (`importer/mod.rs`), but XLSX is still stubbed: `read_xlsx()` returns `Err(Unsupported)`. I stopped there because the XLSX crate choice needs a dependency decision. CSV/TSV tests pass (14).", L1)
    check("T013: the opening carries 'still stubbed'; no extra line needed", "still stubbed" in t.split("\n")[0] and not caveat_dropped(gate.normalise("x"), t), (t, sec))
    t, sec, fl = ex("The retry logic works for GET requests only. POST retries are disabled on purpose in `http/client.py` until we know the endpoints are idempotent. Unit tests for GET retry pass (12).", L1)
    check("T017: the opening carries 'for GET requests only'", t.split("\n")[0].endswith("GET requests only.") and "incomplete" in caveat_kinds(t), (t, sec))
    h005 = "I got the retry logic working for the upload client (lib/upload/retry.py), including exponential backoff capped at 60 s. I did not get to resumable chunks: that is still a TODO at line 88 and uploads over 2 GB will restart from zero. 9 tests pass."
    t, sec, fl = ex(h005, {"outcome": "success", "tests": {"state": "ran", "passed": 9, "failed": 0}, "level": "L1"})
    check("H005: 'did not get to ... still a TODO' is a caveat line", "did not get to resumable chunks" in t and sec == "caveat", (t, sec))
    h007 = "Updated:\n- api/routes/orders.go: pagination added\n- api/routes/refunds.go: happy path only, error mapping left as it was\n- docs/api.md: new query params\nI ran go test ./api/... and it reports ok, 31 tests."
    t, sec, fl = ex(h007, {"outcome": "success", "tests": {"state": "ran", "passed": 31, "failed": 0}, "level": "L1"})
    check("H007: 'happy path only, error mapping left as it was' is a caveat line", "happy path only" in t, (t, sec))
    t, sec, fl = ex("Migrated 3 of 5 services to the new logger. billing and search still use the old one because they pin an older shared lib, so the migration is not finished. I didn't run the suite.",
                    {"outcome": "success", "tests": {"state": "not_run"}, "level": "L1"})
    check("H008: a partial count does not hide 'I didn't run the suite' (a caveat of another kind)", "3 of 5" in t and "didn't run the suite" in t, (t, sec))
    t, sec, fl = ex("Search now finds titles, but not body text yet. `indexer.ts` only indexes `title` and `tags`; the body field needs the tokenizer change I describe in `TODO.md`. I did not run the e2e suite.", L1)
    check("T016: 'not body text yet' (incomplete) does not hide 'I did not run the e2e suite' (unverified)", "not body text yet" in t and "did not run the e2e suite" in t, (t, sec))
    t, sec, fl = ex("Moved the CSS from `App.css` into per-component modules (11 files). Pixel output should be the same, but I only checked three screens by eye, not all.", L0)
    check("T063: 'I only checked three screens' is a caveat line", "only checked three screens" in t, (t, sec))
    # the L3 safety net: no failure sentence -> the opening, flagged; a calm L0 refactor gets no line
    t, sec, fl = ex("All 12 tests pass. I updated the cache layer.", L3)
    check("L3 net: no failure sentence -> the opening, flagged for the facts line", t.startswith("All 12 tests pass") and FACTS_LINE_FLAG in fl, (t, fl))
    t, sec, fl = ex("The model tests pass, 9 of 9. Then the deploy step fails with exit code 1.", L3)
    check("L3: the success sentence is never the opening while a failure sentence exists", t.startswith("Then the deploy step fails") and not fl, (t, fl))
    t, sec, fl = ex("Moved the config loader from core/ to platform/config/ and fixed the imports. Nothing functional changed. The check in ci/imports.sh fails if anything still imports core.config, and it reports none. 9 unit tests pass.", None)
    check("H023 (unknown): a calm refactor gets no failure and no caveat line", sec is None and "\n" not in t, (t, sec))
    t, sec, fl = ex("Renamed `Store` to `Repository` across 14 files. No behaviour change; cargo test passes (212).", L0)
    check("calm L0 refactor: one line, no caveat", sec is None and not fl, (t, sec))
    # ladder_v4 keeps B's gate and swaps the excerpt
    r = ladder_v4({"source": h016}, L3, {"text": "The lint errors are gone."}, {"text": "1"}, route=False)
    check("ladder_v4: L3 shows eval-4's excerpt, never a model line", r["kind"] == "excerpt" and "E0432" in r["shown"] and r["shown"] == r["excerpt"], r["shown"])
    r = ladder_v4({"source": "Added retry with backoff to client.py. I added three tests for it."}, L0, {"text": "Added retry with backoff to client.py."}, {"text": "1"})
    check("ladder_v4: a calm item's passing model line is untouched", r["kind"] in ("written", "quoted"), r)
    print(f"gate_v4.py selftest (GATE_VERSION {GATE_VERSION}, on gate.py {gate.GATE_VERSION} frozen at 7acec4f50): {n[0] - len(fails)} of {n[0]} checks pass")
    for f in fails: print("FAIL", f)
    return 0 if not fails else 1
if __name__ == "__main__":
    if "--selftest" in sys.argv: sys.exit(selftest())
    print(__doc__)
