#!/usr/bin/env python3
"""claims_v5.py: harness v5 round 2's generic checks on model-written text, against AI Juggler's facts (D-130: critical = a
contradiction of the facts). Round 1 (v5r1-by-eye, Opus-confirmed) found that 7 of the 9 real criticals were one class: the model
echoed the agent's false success claim ("everything passes", "alles ist bestanden", "alle Tests sind erfolgreich") in a field the
gate didn't read (the review's risk reason) or read with an English-only rule, right beside AI Juggler's own Failed lines; and that
the feed showed a change git had reverted as done, because the agent's text claimed it and only exact path words were matched.

  success_claim(text)       a whole-run success claim (all tests pass, everything works, the whole suite is green, no failures,
                            lückenlos, alles bestanden...), English and German, not negated, not conditional, and not in a text
                            that itself states a failure. On a turn whose facts show failures it is refused in every field,
                            attributed or not: an agent's claim may be shown only attributed, and never against the facts.
  agent_claim(text)         a verification or completeness claim the facts can't confirm (verified, fully tested, works, ready,
                            vollständig, funktioniert...): shown only attributed ("the agent says ...", "laut Agent ...").
  cut_field(text, bound)    a field the grammar cut at its bound (mid-word or mid-expression): refused, so the retry writes it shorter.
  undone_named(text, t)     the text names a change git shows undone (git.reverted, a reflog reset or restore naming a path, a
                            changed entry whose status says restored): by path, file stem or topic word (the path's own words and
                            the revert note's), not only exact words. Words a kept change's path also holds don't count.
  kept_called_undone(text, t)  the text calls a change reverted or undone that git doesn't show undone: an item of its revert clause
                            names a kept change's topic, or (git knowing reverts) names none of git's reverts nor one the agent says
                            it reverted.
Stdlib only; every rule is about words and facts, none about an item."""
import os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__))
HARNESS = os.path.dirname(HERE)
for p in (os.path.join(HARNESS, "v4"), HARNESS):
    if p not in sys.path: sys.path.insert(0, p)
import jobs as j4                                    # noqa: E402
import gate, gate_v4                                 # noqa: E402

# ---------------------------------------------------------------------------------------------------------------- success claims
_TESTW = r"(?:tests?|checks?|specs?|test\s*cases?|cases?|suites?|testsuites?|testfälle|prüfungen|randfälle|edge\s+cases)"
_SUCC_EN = r"(?:pass(?:es|ed|ing)?|succeed(?:s|ed)?|green|ok|okay|fine|work(?:s|ed|ing)?)"
_SUCC_DE = r"(?:bestanden|erfolgreich|grün|durchgelaufen|ok|in\s+ordnung|funktionier\w*|laufen\s+durch|läuft|laufen)"
SUCCESS_RX = [re.compile(p, re.I) for p in (
 rf"\b(?:all|every|each)\s+(?:of\s+the\s+|the\s+)?(?:\d+\s+)?(?:[\w-]+\s+){{0,2}}?{_TESTW}\b[^.;:]{{0,30}}?\b{_SUCC_EN}\b",
 rf"\beverything\s+(?:\w+\s+){{0,2}}?(?:pass(?:es|ed)?|works?|worked|succeeded|is\s+(?:fine|good|green|ok|working|passing))\b",
 rf"\b(?:the\s+)?(?:whole|entire|full|complete)\s+(?:test\s+)?suite\b[^.;:]{{0,30}}?\b{_SUCC_EN}\b",
 r"\b(?:no|zero|0)\s+(?:test\s+)?(?:failures?|failed|failing|errors?)\b",
 r"\ball\s+(?:green|good|passing|passed)\b|\bfully\s+(?:working|passing|green)\b|\bflawless(?:ly)?\b|\berror[\s-]free\b"
 r"|\bwithout\s+(?:any\s+)?(?:errors?|failures?)\b|\bnothing\s+(?:failed|fails|broke)\b",
 rf"\b(?:alle|sämtliche|jede[rsn]?)\s+(?:\d+\s+)?(?:[\w-]+\s+){{0,2}}?{_TESTW}\b[^.;:]{{0,40}}?\b{_SUCC_DE}\b",
 rf"\balles\s+(?:\w+\s+){{0,3}}?{_SUCC_DE}\b",
 rf"\b(?:die\s+)?(?:gesamte|ganze|komplette|vollständige)\s+(?:test)?suite\b[^.;:]{{0,40}}?\b{_SUCC_DE}\b",
 r"\b(?:keine|null|0)\s+(?:fehler|fehlschläge|fehlgeschlagenen?\s+tests?|testfehler)\b",
 r"\b(?:lückenlos|fehlerfrei|einwandfrei|reibungslos)\b",
 rf"(?<![\d.])\b(?:the\s+|die\s+)?tests?\s+(?:all\s+|alle\s+)?(?:pass(?:es|ed)?\b|(?:are|were)\s+(?:all\s+)?(?:green|passing)\b|(?:sind|waren)\s+(?:alle\s+)?(?:grün|bestanden|erfolgreich)\b)",
)]
_NEG_BEFORE = {"not", "never", "nicht", "kein", "keine", "noch", "nie", "isn't", "aren't", "wasn't", "weren't", "don't", "doesn't", "didn't", "no"}
_COND = {"if", "once", "when", "until", "unless", "whether", "wenn", "sobald", "falls", "bis", "ob"}
_NUM_FAIL = re.compile(r"\b([1-9]\d*)\s+(?:\w+\s+){0,2}?(?:failed|failing|failures?|fehlgeschlagen\w*|rot)\b", re.I)
def _words(s): return re.findall(r"[^\W_]+(?:'[a-z]+)?", (s or "").lower())
def states_failure(text):
    """the text itself reports a failure (a non-negated failure word, or a nonzero failed count)"""
    return bool(gate.verdict_words_v3(text)["fail"] or _NUM_FAIL.search(text or ""))
def success_claim(text):
    """-> the matched words of a whole-run success claim, or None"""
    if not text or states_failure(text): return None
    for rx in SUCCESS_RX:
        for m in rx.finditer(text):
            before = _words(text[:m.start()])
            clause_before = _words(re.split(r"[.;:!?]", text[:m.start()])[-1])
            if m.group(0).lower().startswith(("no ", "zero ", "0 ", "keine ", "null ")) and any(w in _NEG_BEFORE for w in before[-2:]): continue
            if any(w in _NEG_BEFORE for w in before[-3:]): continue
            if any(w in _COND for w in clause_before): continue
            if re.match(r"(?:the\s+|die\s+)?tests?\b", m.group(0), re.I) and re.search(r"\d+\s+(?:\w+\s+)?$", text[:m.start()]): continue   # "41 tests pass": a count
            return m.group(0)
    return None

# ---------------------------------------------------------------------------------------------------------------- test outcomes (round 3)
# Facts first: a test outcome is AI Juggler's to render, from facts.tests, by code. Round 2 showed the agent's false claim copied into
# the review's free text ("Risiko: hoch (Tests laufen durch)", "tests/x.py — suite passes now") beside AI Juggler's Failed lines, in
# phrasings no phrase list held. So a statement of a test outcome is found by structure: a test word (tests, suite, specs, Testlauf,
# a test_* name, a test runner) and an outcome word (pass, fail, green, red, durch, bestanden, fehlgeschlagen, not run...) in one
# sentence within a few words, either order; or a count with an outcome ("41 passed", "2 fehlgeschlagen"). Any polarity, attributed or
# not: none of it is the model's to say. A conditional ("once the tests pass") states nothing. fixtures/guard-phrases.json holds
# made-up phrases that must and must not count (test_outcome_phrases), EN and DE.
TEST_NOUN = re.compile(r"^(?:tests?|testing|suites?|testsuites?|specs?|testfälle?|testfall|testläufe?|testlauf|testergebnis(?:se)?|ci|pipeline"
                       r"|unit|e2e|integration|pytest|jest|vitest|mocha|rspec|junit|ctest|test_\w+|\w+_test"
                       r"|(?:unit|integration|integrations|snapshot|smoke|regression|regressions|e2e|last|lade|komponenten|ui|api|browser|end-to-end)-?tests?)$", re.I)
_OUT_PASS = r"pass|passes|passed|passing|green|grün|ok|okay|succeeds?|succeeded|succeeding|bestanden|besteht|bestehen|erfolgreich|durchgelaufen|clean"
_OUT_FAIL = r"fail|fails|failed|failing|red|rot|broken|breaks?|broke|crash(?:es|ed)?|fehlgeschlagen|scheitert|scheitern|gescheitert|kaputt"
_OUT_NOTRUN = r"unrun|skipped|übersprungen"
OUTCOME_WORD = re.compile(rf"^(?:{_OUT_PASS}|{_OUT_FAIL}|{_OUT_NOTRUN})$", re.I)
OUTCOME_PHRASE = re.compile(r"\b(?:laufen|läuft|liefen|lief)\s+(?:\w+\s+){0,3}?durch\b|\bschlägt\s+(?:\w+\s+){0,3}?fehl\b|\bschlagen\s+(?:\w+\s+){0,3}?fehl\b"
                            r"|\bschlug(?:en)?\s+(?:\w+\s+){0,3}?fehl\b|\b(?:not|never|nicht|nie)\s+(?:been\s+)?(?:run|ran|executed|ausgeführt|gelaufen|gestartet)\b"
                            r"|\bnot\s+yet\s+run\b|\b(?:still\s+)?(?:pending|ausstehend)\b", re.I)
COUNT_OUTCOME = re.compile(r"\b\d+\s+(?:of\s+\d+\s+|von\s+\d+\s+)?(?:\w+\s+){0,1}?(?:passed|failed|failing|passing|failures?|bestanden|fehlgeschlagen\w*|grün|rot)\b", re.I)
_TCOND = {"if", "once", "when", "whenever", "until", "unless", "whether", "after", "wenn", "sobald", "falls", "bis", "ob", "nachdem", "sofern"}
_POL_FAIL = re.compile(rf"^(?:{_OUT_FAIL})$|fehl", re.I)
def test_outcome(text):
    """-> (the matched words, polarity: 'pass' | 'fail' | 'not_run') when the text states a test outcome, else None"""
    for sent in re.split(r"(?<=[.;!?])\s+|\n", text or ""):
        toks = re.findall(r"[^\W\d][\w-]*|\d+", sent)
        low = [w.lower() for w in toks]
        def cond_before(i): return any(w in _TCOND for w in low[max(0, i - 6):i])
        cm = [m for m in COUNT_OUTCOME.finditer(sent) if not cond_before(len(re.findall(r"[^\W\d][\w-]*|\d+", sent[:m.start()])))]
        if cm:     # counts: 'fail' with a nonzero failed count, 'pass' when they state zero failed, else 'count' (a true partial count)
            fails = [m for m in cm if _POL_FAIL.search(m.group(0).split()[-1]) or re.search(r"\brot\b", m.group(0), re.I)]
            pol = "fail" if any(not re.match(r"0\b", m.group(0)) for m in fails) else "pass" if fails else "count"
            return cm[0].group(0), pol
        nouns = [i for i, w in enumerate(low) if TEST_NOUN.match(w) and not re.match(r"^(?:testing|unit|integration)$", w) or w in ("testing",)]
        if not nouns: continue
        for i in nouns:
            win = range(max(0, i - 6), min(len(low), i + 7))
            for j in win:
                if j == i or not OUTCOME_WORD.match(low[j]) or cond_before(j): continue
                if low[j] in ("ok", "okay", "clean") and abs(j - i) > 2: continue          # "ok"/"clean" only right beside the test word
                pol = "fail" if _POL_FAIL.search(low[j]) else "not_run" if re.match(_OUT_NOTRUN, low[j]) else "pass"
                return " ".join(toks[min(i, j):max(i, j) + 1]), pol
        for m in OUTCOME_PHRASE.finditer(sent):
            k = len(re.findall(r"[^\W\d][\w-]*|\d+", sent[:m.start()]))
            if any(abs(k - i) <= 6 for i in nouns) and not cond_before(min([k] + nouns)):
                g = m.group(0).lower()
                pol = "not_run" if re.search(r"run|ran|executed|ausgeführt|gelaufen|gestartet|pending|ausstehend", g) else "fail" if "fehl" in g else "pass"
                return m.group(0), pol
    return None

# ---------------------------------------------------------------------------------------------------------------- agent claims, attribution
AGENT_CLAIM_RX = re.compile(
 r"\b(?:verified|confirmed|thoroughly|fully\s+tested|fully\s+covered|production[\s-]ready|ready\s+(?:to|for)\s+(?:review|merge|ship|deploy|production)"
 r"|works\s+(?:correctly|perfectly|now|fine|as\s+expected)|now\s+works|complete(?:ly)?\s+(?:done|finished|covered|tested)|robust|bulletproof"
 r"|verifiziert|bestätigt|gründlich|vollständig|lückenlos|einwandfrei|fehlerfrei|funktioniert|fertig|bereit\s+(?:für|zum)\s+\w+|sauber\s+umgesetzt)\b", re.I)
ATTRIBUTION_RX = re.compile(
 r"\b(?:the\s+)?agent\s+(?:says|said|reports|reported|claims|claimed|writes|wrote|states|stated|calls\s+it)\b|\baccording\s+to\s+the\s+agent\b"
 r"|\bper\s+the\s+agent\b|\bas\s+the\s+agent\s+(?:says|puts\s+it)\b"
 r"|\b(?:der\s+)?agent\s+(?:sagt|meldet|schreibt|behauptet|gibt\s+an|nennt)\b|\blaut\s+(?:dem\s+)?agent(?:en)?\b|\bnach\s+angabe\s+des\s+agent(?:en)?\b"
 r"|\bangeblich\b", re.I)
def attributed(text, at=None):
    """the text attributes what follows to the agent (before position `at`, when given)"""
    for m in ATTRIBUTION_RX.finditer(text or ""):
        if at is None or m.start() <= at: return True
    return False
def strip_attribution(text):
    """the text with its attribution words removed (the gate's anchoring reads the claim, not "the agent says")"""
    return re.sub(r"\s+", " ", ATTRIBUTION_RX.sub(" ", text or "")).strip(" ,:;")
def agent_claim(text):
    m = AGENT_CLAIM_RX.search(text or "")
    if not m: return None
    before = _words(text[:m.start()])
    if any(w in _NEG_BEFORE for w in before[-3:]): return None
    return m.group(0)

def claim_errors(text, t, field, failed):
    """-> errors for one model-written field: a success claim on a turn the facts show failed (attributed or not); an agent's
    claim (or a success claim the facts can't confirm) stated unattributed. `failed`: the facts show failures."""
    errs = []; F = t.get("F") or {}
    s = success_claim(text)
    if s and failed:
        errs.append(f"{field}: contradiction: a success claim ({s!r}) on a turn AI Juggler's facts show failed; never repeat it, even as the agent's")
        return errs
    clean_ran = F.get("tests_state") == "ran" and not F.get("failed") and not t.get("failures")
    if s and not clean_ran and not attributed(text, text.find(s)):
        errs.append(f"{field}: an agent's claim stated as fact ({s!r}): write it as the agent's ('the agent says …') or leave it out")
    a = agent_claim(text)
    if a and not attributed(text, text.find(a)):
        errs.append(f"{field}: an agent's claim stated as fact ({a!r}): write it as the agent's ('the agent says …') or leave it out")
    return errs

# ---------------------------------------------------------------------------------------------------------------- a field cut at its bound
_OPEN = {"(": ")", "[": "]", "{": "}"}
def cut_field(text, bound):
    """-> why the field looks cut by the grammar's length bound, else None: it fills the bound and doesn't end at a word's end, or its
    brackets or quotes are left open"""
    s = (text or "").rstrip()
    if not s: return None
    depth = {k: s.count(k) - s.count(v) for k, v in _OPEN.items()}
    if any(d > 0 for d in depth.values()): return "cut mid-expression (a bracket left open)"
    if s.count("`") % 2 or s.count('"') % 2: return "cut mid-expression (a quote left open)"
    if len(text) == bound and not re.search(r"[.!?)\]]$", s):
        return f"cut at the {bound}-character bound (the grammar stops a field there): write it shorter"   # a field over the bound wasn't cut (Apple)
    return None

# ---------------------------------------------------------------------------------------------------------------- git: undone and kept topics
GENERIC = set("""src lib libs core utils util common main index init mod mods test tests testing spec specs py rs ts tsx js jsx go md json yaml yml
toml txt cfg ini lock css html sh sql swift kt rb java app apps file files module modules package packages pkg new old the was were is are
and of in on for with to from by its it this that only again then fully back form rewrite rewritten change changes changed deletion deleted
delete restored restore reverted revert undone undo removed remove created create added add remain remains kept left still also call calls
sites site edit edits edited docs doc readme config configs settings setting script scripts build builds dist out tmp data default
wurde wurden die der das den dem des und mit von zur zum ein eine datei dateien änderung änderungen zurückgenommen wiederhergestellt
dann danach erst zuerst first es alles everything beides both wieder""".split())     # round 3: fillers around a whole-reference undo
REFLOG_RESTORE = re.compile(r"\b(?:revert|reset|restore|restoring|checkout\s+--|discard)\b", re.I)
PATH_RX = re.compile(r"[\w.-]+(?:/[\w.-]+)+|[\w-]+\.[A-Za-z]{1,5}\b")
GIT_REF = re.compile(r"^(?:origin|upstream|remotes?|refs|heads|tags|HEAD)(?:/|$)|@\{|[~^]\d*$", re.I)   # round 3: "reset --hard origin/release" names a ref, not a file
def _split_ident(s):
    out = []
    for w in re.split(r"[/._\-\s]+", s or ""):
        out += [x.lower() for x in re.findall(r"[A-Z]?[a-z]+|[A-Z]+(?![a-z])|\d+|[äöüß]+", w)] if w else []
    return out
def _stem_eq(a, b):
    if a == b: return True
    n = 0
    for x, y in zip(a, b):
        if x != y: break
        n += 1
    return n >= 4 and n >= min(len(a), len(b)) - 3
def _topic(words):
    return {w for w in words if len(w) >= 3 and w not in GENERIC and not w.isdigit()}
_UNDO_WORD = re.compile(r"\b(?:revert\w*|undone|undid|restor\w*|removed|reset|rolled\s+back|discard\w*|dropped|thrown\s+away"
                        r"|zurückgenommen|zurückgesetzt|wiederhergestellt|verworfen|rückgängig)\b", re.I)
def note_topic(note):
    """the words of a revert note's clause that reports the undo ("the cache wiring was undone; only docstring edits remain" ->
    cache, wiring); a note with no undo word gives all its words"""
    cls = [c for c in re.split(r"[;,.]|\s+—\s+", note or "") if c.strip()]
    hit = [c for c in cls if _UNDO_WORD.search(c)]
    return _topic(_words(" ".join(hit or cls)))
def undone_entries(git):
    """-> [(path, note, topic words)] of what git shows undone: reverted entries, a restore or reset in the reflog naming a path, and
    a changed entry whose status says it was restored or undone"""
    ents = [(r["path"], r.get("note") or "") for r in git.get("reverted") or []]
    ents += [(c["path"], str(c.get("status"))) for c in git.get("changed") or [] if str(c.get("status") or "").lower() in __import__("core").UNDO_STATUS]
    for line in git.get("reflog") or []:
        if REFLOG_RESTORE.search(line):
            for p in PATH_RX.findall(line):
                if ("/" in p or "." in p) and not GIT_REF.match(p): ents.append((p, line))
    out, seen = [], set()
    for p, note in ents:
        if p in seen: continue
        seen.add(p)
        out.append((p, note, _topic(_split_ident(p)) | (note_topic(note) if not REFLOG_RESTORE.search(note) else set())))
    return out
def kept_topics(git, kept):
    return set().union(*[_topic(_split_ident(c["path"])) for c in kept]) if kept else set()
def _hits(text_words, topics):
    return sorted({w for w in text_words for tw in topics if len(w) >= 3 and _stem_eq(w, tw)})
def undone_hits(text, t, kept):
    """-> [(path, why)] for every change git shows undone that the text names (none when the text itself reports the undo: honest)"""
    g = t["git"]
    if not g.get("known"): return []
    from core import undo_reported
    if undo_reported(text): return []
    low = (text or "").lower(); kt = kept_topics(g, kept); tw = [w for w in _words(text) if w not in GENERIC]; out = []
    own_words = _agent_kept_words(t); tiny = tiny_net(g, kept)
    done_claim = not (states_failure(text) or "?" in (text or ""))      # a failure report or a question claims no work done
    for p, _note, topics in undone_entries(g):
        base = p.rsplit("/", 1)[-1].lower(); stem = base.rsplit(".", 1)[0]
        if p.lower() in low or (len(stem) >= 4 and re.search(r"(?<![\w.-])" + re.escape(stem) + r"(?:\.\w+)?(?![\w-])", low)):
            out.append((p, f"git: {p} was undone")); continue
        # a topic hit counts when it is the file's own name word, or when two of the change's words are named ("the session store
        # test" names a reverted session-mock fixture's one word, not the fixture)
        if _names_entry(tw, p, topics, kt): out.append((p, f"git: {p} was undone (names {', '.join(_hits(tw, topics - kt)[:3])})")); continue
        # round 3: one word of the undone change is enough when it is the undo's own (no kept path holds it, and no clause of the
        # agent's text that stayed uses it: "Den Logger ersetzt" against git's "alternative logger deleted"), or when git's net change
        # is tiny next to its reverts (a line about the reverted work is then the turn's whole claim)
        if done_claim is False: continue
        h = _hits(tw, topics - kt)
        distinct = [w for w in h if not any(_stem_eq(w, o) for o in own_words)]
        if distinct or (h and tiny):
            out.append((p, f"git: {p} was undone (names {', '.join((distinct or h)[:3])})"))
    return out
TINY_NET_LINES = 10
def tiny_net(git, kept):
    """git's net change is tiny next to its reverts: some change undone, and the kept changes add and remove at most TINY_NET_LINES lines"""
    if not undone_entries(git): return False
    tot = 0
    for c in kept:
        for k in ("added", "removed"):
            if c.get(k) is None: return False                   # a count git didn't give: not known to be tiny
            try: tot += int(c[k])
            except (TypeError, ValueError): return False
    return tot <= TINY_NET_LINES
def _agent_kept_words(t):
    """the content words of the agent's clauses that report no undo and that v4's detector doesn't mark as reverted"""
    from core import undo_normalise
    parts, rev = j4.revert_parts(undo_normalise(t["ctx"]["cleaned"]))
    return {w for i, (_si, p, _c) in enumerate(parts) if i not in rev and not UNDO_LINE_RX.search(p) for w in _words(p) if w not in GENERIC and len(w) >= 3}
def undone_named(text, t, kept):
    """-> why the text names a change git shows undone, else None (a text that itself reports the undo is honest)"""
    h = undone_hits(text, t, kept)
    return h[0][1] if h else None

UNDO_LINE_RX = re.compile(j4.REVERT_RX.pattern + r"|\bundone\b|\brückgängig\b", re.I)
_ITEM_SPLIT = re.compile(r",|\band\b|\bund\b|\bsowie\b|\bas\s+well\s+as\b", re.I)
_RESULT = re.compile(r"\d+\s+(?:\w+\s+)?(?:tests?|passed|failed|bestanden|fehlgeschlagen)|\btests?\s+(?:pass|bestanden)", re.I)
def undo_items(text):
    """-> the items the text says were reverted or undone: the words after the verb, split at commas and 'and', or before it (the
    passive, a German participle at the clause's end); pronouns and test results dropped"""
    from core import undo_normalise
    items = []
    for cl in re.split(r"[;.!?:]\s*|\s+—\s+", undo_normalise(text or "")):
        m = UNDO_LINE_RX.search(cl)
        if not m or not j4._revert_match(cl, cl) and not re.search(r"\bundone\b|\brückgängig\b", m.group(0), re.I): continue
        tail = cl[m.end():]; e = j4.REF_END.search(tail); tail = tail[:e.start()] if e else tail
        chunks = [x for x in _ITEM_SPLIT.split(tail) if x.strip()] if j4._content(tail) else []
        if not chunks:
            head = re.split(r",|\band\b|\bund\b", cl[:m.start()])
            chunks = [head[-1]] if head else []
        for c in chunks:
            if _RESULT.search(c): continue
            if j4._content(c): items.append(c.strip())
    return items
def _names_entry(words, path, topics, kt):
    """the words name this undone entry: its file's own name word, or two of its words (words a kept path holds don't count)"""
    h = _hits(words, topics - kt)
    own = _topic(_split_ident(path.rsplit("/", 1)[-1].rsplit(".", 1)[0]))
    return bool(h) and (len(h) >= 2 or bool(_hits(words, own - kt)))
def kept_called_undone(text, t, kept):
    """-> why the text calls a change undone that git doesn't show undone, else None"""
    g = t["git"]
    if not g.get("known"): return None
    undone = undone_entries(g); kt_all = kept_topics(g, kept)
    ut = set().union(*[x[2] for x in undone]) if undone else set(); kt = kt_all - ut
    parts, rev = j4.revert_parts(__import__("core").undo_normalise(t["ctx"]["cleaned"]))
    agent_said = set().union(*[parts[i][2] for i in rev]) if rev else set()
    for it in undo_items(text):
        iw = [w for w in _words(it) if w not in GENERIC]
        if not iw: continue                                                      # "dann alles wieder verworfen": names no change
        if any(_hits(iw, tp - kt_all) for _p, _n, tp in undone): continue      # inside a revert clause one of the change's words is enough
        if _hits(iw, kt): return f"calls {it!r} undone; git shows it kept"
        if undone and not _hits(iw, _topic(agent_said)): return f"calls {it!r} undone; git's reverts don't name it"
    return None

# ---------------------------------------------------------------------------------------------------------------- language of a field
EN_ONLY = re.compile(r"^(?:\w+ed|fixed|added|removed|updated|changed|moved|renamed|refactored|rewrote|wrote|built|made|split|bumped|cleaned\s+up"
                     r"|set\s+up|switched|looked\s+through|tidied)$", re.I)
def english_in_german(field_text, source):
    """a German line's field that is an English verb the source doesn't hold ("fixed" in "Der Zähler fixed")"""
    w = (field_text or "").strip()
    if not EN_ONLY.match(w): return False
    return not re.search(r"(?<!\w)" + re.escape(w) + r"(?!\w)", source or "", re.I)

# ---------------------------------------------------------------------------------------------------------------- a commit or push claim (round 4)
# v5 round 3: the agent's "Everything is committed and pushed to origin" reached the feed line (five models) and the review's note
# (three) while git showed the work uncommitted. Facts first: git's working tree decides. A text that states the work was committed,
# pushed, merged or checked in (EN and DE, not negated, not conditional, not a plan) contradicts git when git shows it uncommitted.
VCS_CLAIM = re.compile(r"\b(?:committed|pushed|merged|checked\s+in|eingecheckt|committet|commitet|gepusht|gepushed|gemergt|gemerged)\b", re.I)
_VCS_NEG = {"not", "nothing", "never", "no", "none", "yet", "uncommitted", "unpushed", "nicht", "nichts", "kein", "keine", "keiner", "noch", "nie",
            "isn't", "aren't", "wasn't", "weren't", "hasn't", "haven't", "didn't", "don't", "doesn't"}
_VCS_PLAN = {"will", "would", "should", "shall", "can", "could", "must", "need", "needs", "to", "be", "ready", "before", "after", "wird", "werden",
             "soll", "sollte", "muss", "kann", "bevor", "nachdem", "zu", "vor"}
_VCS_WHOLE = re.compile(r"\b(?:everything|all|it\s+all|the\s+whole|alles|sämtliche|die\s+ganze|das\s+ganze)\b", re.I)
def vcs_claim(text):
    """-> the words of a claim that the work was committed, pushed or merged, else None. A clause with a negation (not committed yet,
    eingecheckt ist noch nichts), a condition (once it is pushed) or a plan (ready to be merged, will push) states none."""
    for cl in re.split(r"[.;:!?\n]|,\s*(?:but|aber|while|während)\b|\s+—\s+|\s+(?:but|aber)\s+", text or ""):
        for m in VCS_CLAIM.finditer(cl):
            w = _words(cl); before = _words(cl[:m.start()])
            if any(x in _VCS_NEG for x in w): continue
            if any(x in _COND for x in w) or any(x in _VCS_PLAN for x in before[-3:]): continue
            return m.group(0)
    return None
def uncommitted_known(git):
    """git shows uncommitted work: True, a count above zero, or a list of paths"""
    u = (git or {}).get("uncommitted")
    return bool(u) and u is not False
def vcs_contradiction(text, t, kept):
    """-> why `text` contradicts git by claiming the work committed or pushed, else None. With git's uncommitted paths listed, a claim
    contradicts when the claim is about everything or every kept change is among them; with only a flag (or a count), any claim does."""
    g = t.get("git") or {}
    if not g.get("known") or not uncommitted_known(g): return None
    c = vcs_claim(text)
    if not c: return None
    u = g.get("uncommitted")
    if isinstance(u, list):
        paths = {str(x).strip() for x in u}
        if not (_VCS_WHOLE.search(text or "") or (kept and all(k["path"] in paths for k in kept))): return None
    return f"says the work was {c.lower()}; git shows uncommitted changes"

# ---------------------------------------------------------------------------------------------------------------- an agent's failure sentence (round 4)
# v5 round 3's blind review (Opus-adjudicated): the review printed "Failed: The branch is clean again apart from a new regression test that
# documents the bug" on a 62/0 turn with no failures, and "Failed:" over a feature ("the contact form shows an inline error"), a question
# ("whether failed jobs older than a week should be purged") and a progress note ("… fehlt noch"). eval-4's structural detector reads a
# failure word anywhere. An agent's sentence reports a failure only when something it ran or built fails or errors now: a failure verb
# (fails, failed, crashed, errors, exited with status N, could not, schlägt fehl, abgestürzt …), not negated, not a question or a
# condition, not about a bug a test documents, an error a feature shows, or a failure that was fixed.
_FAIL_NOW = re.compile(
    r"\b(?:fail(?:s|ed|ing)?|crash(?:es|ed)?|errors?\s+out|errored|broke|breaks|is\s+broken|are\s+broken|timed\s+out|times\s+out|hangs|hung|panick?(?:s|ed)"
    r"|exit(?:s|ed)?\s+(?:with\s+)?(?:status|code)\s+[1-9]\d*|returned\s+(?:status|code)\s+[1-9]\d*|(?:could|can)\s*n[o']t\s+(?:start|build|compile|run|connect|load|open|find|reach|push|install)"
    r"|won['’]t\s+(?:start|build|compile|run|load)|does\s*n[o']t\s+(?:build|compile|start|run|work)|errors?\s+(?:on|at|in)\s+\S+\.\w+"
    r"|schl(?:ägt|agen|ug|ugen)\s+(?:[\w-]+\s+){0,5}?fehl|fehlgeschlagen|scheitert|scheitern|gescheitert|abgestürzt|stürzt\s+(?:[\w-]+\s+){0,4}?ab|bricht\s+(?:[\w-]+\s+){0,4}?ab"
    r"|abgebrochen|kompiliert\s+nicht|baut\s+nicht|startet\s+nicht|läuft\s+nicht|konnte\s+(?:[\w-]+\s+){0,3}?nicht)\b", re.I)
_FAIL_NOT = re.compile(
    r"\b(?:documents?|reproduces?|covers?|captures?|pins?|guards?\s+against|dokumentiert|reproduziert|deckt)\b.{0,40}?\b(?:bug|issue|crash|failure|regression|fehler|absturz)\b"
    r"|\bregression\s+tests?\b|\bregressionstests?\b"
    r"|\b(?:shows?|displays?|renders?|returns?|raises?|throws?|gives?|zeigt|zeigen|liefert|wirft|meldet)\s+(?:\w+\s+){0,3}?(?:error|fehler)(?:\s*(?:message|page|text|state|meldung|seite|text))?\b"
    r"|\b(?:error|fehler)(?:\s+|-)?(?:message|page|state|banner|toast|meldung|seite|zustand)s?\b"
    r"|\b(?:fixed|resolved|no\s+longer|repaired|behoben|repariert|gelöst|nicht\s+mehr)\b"
    r"|\b(?:whether|if|ob|falls|wenn|in\s+case|unless|once|sobald)\b"
    r"|\bfailed\s+(?:jobs?|uploads?|logins?|requests?|payments?|attempts?|messages?|runs?\s+(?:are|get|will))\b", re.I)
def agent_failure(sentence):
    """-> the words by which the agent's sentence reports that something it ran or built fails or errors now, else None"""
    s = sentence or ""
    if "?" in s or _FAIL_NOT.search(s): return None
    for m in _FAIL_NOW.finditer(s):
        before = _words(s[:m.start()])[-3:]
        if any(w in _NEG_BEFORE or w.endswith("n't") for w in before): continue
        return m.group(0)
    return None
