#!/usr/bin/env python3
"""chat.py: harness v5's job (d), Chat v1's answer, facts first (D-130 lever 1). Design: ../../v5-design.md.

  code   reads what the question asks for from its own words (an English and German lexicon, ATTR_LEXICON), mapped to the records'
         typed fields (duration_s, tests_passed, tests_failed, files_changed, started_at, ended_at, status, branch, cost_usd, author,
         path); finds the question's subject among the records (their kind, name and lead label, then their links); and looks the
         field up. No record at the best subject match holds the field: "I don't know" and the found rows, with NO model call (the
         near-miss v4 kept answering: the session's 48 minutes for "how long did the test suite take").
  model  only phrases the found value(s), citing their record ids. Code checks the phrased value equals the field (numbers, durations,
         times, costs, statuses and names, read by v4's classifier), that every found record is cited and no other, and the frozen
         gate's invented-number check. A question the lexicon can't read ("what happened in…") takes v4's Chat path (the claims, the
         nearby-question guard, the hard checks), and the model's asks_about is a hint only: when it names a typed field no matched
         record holds, code says "I don't know"."""
import datetime, json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from core import Job, register, register_item_kind, j4, gate, gate_v4, checks, fit, PROMPT_VERSION, prepare_v5  # noqa: E402

# ======================================================================================================================
# The item
# ======================================================================================================================
def is_chat(raw): return bool(raw.get("question")) and isinstance(raw.get("records"), list)
def norm_chat(raw):
    recs = []; removed = []
    for r in raw["records"]:
        txt = r.get("text") or ""; ctx = prepare_v5(txt) if txt.strip() else {"cleaned": "", "removed": []}
        removed += ctx["removed"]
        recs.append({"id": str(r.get("id")), "kind": r.get("kind") or "record", "fields": dict(r.get("fields") or {}), "text": ctx["cleaned"],
                     "raw_text": txt})
    return {"id": raw.get("id"), "kind": "chat", "raw": raw, "question": raw["question"], "records": recs, "removed": removed,
            "gold": raw.get("gold") or {}, "origin": raw.get("origin") or "", "lang": gate.language(raw["question"]) or "en"}
register_item_kind("chat", is_chat, norm_chat)

# ======================================================================================================================
# What the question asks for: the lexicon (English and German; first match wins; a cue's span is removed before the subject is read)
# ======================================================================================================================
ATTR_LEXICON = [   # (fields, kind, pattern, cue words: the words of the cue, dropped before the subject is read). Specific before general.
 # round 3: "which tests failed" asks for names, not a count (a count answered it in round 2); its record field is a list of names
 (("failed_tests",), "names", r"\b(?:which|what)\s+(?:of\s+the\s+)?(?:[\w.-]+\s+)?tests?\s+(?:have\s+|had\s+|did\s+|are\s+|were\s+|is\s+|was\s+)?(?:fail|failed|failing|broke|break|red)\b"
                             r"|\bwelche[nrs]?\s+(?:[\w.-]+\s+)?tests?\b.{0,50}?\b(?:fehlgeschlagen|gescheitert|rot|fehl|kaputt)\b|\bnames?\s+of\s+the\s+(?:failing|failed)\s+tests?\b",
  "tests test failed fail failing broke break red fehlgeschlagen gescheitert rot fehl kaputt names name which welche welcher welches welchen"),
 (("passed_tests",), "names", r"\b(?:which|what)\s+(?:of\s+the\s+)?(?:[\w.-]+\s+)?tests?\s+(?:have\s+|had\s+|did\s+|are\s+|were\s+)?(?:pass|passed|passing|green)\b"
                             r"|\bwelche[nrs]?\s+(?:[\w.-]+\s+)?tests?\b.{0,50}?\b(?:bestanden|grün|durchgelaufen)\b",
  "tests test passed pass passing green bestanden grün durchgelaufen which welche welcher welches welchen"),
 (("changed_files",), "names", r"\b(?:which|what)\s+files\s+(?:[\w.-]+\s+){0,4}?(?:changed|change|touched|modified)\b|\bwelche\s+dateien\s+(?:[\w.-]+\s+){0,4}?(?:geändert|angefasst|bearbeitet)\b",
  "files file changed change touched modified dateien datei geändert angefasst bearbeitet which welche wurden were did has have hat haben"),
 (("tests_failed",), "count", r"how many (?:of the )?tests? (?:have |had |did |are )?(?:failed|fail|failing)|wie viele tests? (?:[\w.-]+ ){0,6}?(?:fehlgeschlagen|gescheitert|rot)\b|failing tests|fehlgeschlagene tests",
  "tests failed fail failing fehlgeschlagen fehlgeschlagene gescheitert rot"),
 (("tests_passed",), "count", r"how many (?:of the )?tests? (?:have |had |did |are )?(?:passed|pass|passing|green)|wie viele tests? (?:[\w.-]+ ){0,6}?(?:bestanden|durchgelaufen|grün)\b",
  "tests passed pass passing green bestanden durchgelaufen grün"),
 (("tests_passed", "tests_failed"), "tests", r"did (?:the |all |all the )?(?:[\w.-]+ )?tests? pass|(?:were|are) (?:all )?(?:the )?tests? (?:green|passing|ok)|test results?|"
                                             r"(?:sind|waren) (?:die |alle )?tests? (?:[\w.-]+ ){0,6}?(?:durch|grün|bestanden|ok)\b|haben (?:die |alle )?tests? bestanden|testergebnis(?:se)?",
  "tests pass passed passing green ok result results durch grün bestanden testergebnis testergebnisse"),
 (("files_changed",), "count", r"how many files|wie viele dateien|number of (?:changed )?files|anzahl (?:der )?(?:geänderten )?dateien",
  "files file number changed change touched anzahl dateien geänderten geändert"),
 (("cost_usd",), "cost", r"how much did .{0,60}?(?:cost|spend)|\bcost(?:s|ed)?\b|\bspen[dt]\b|was hat .{0,60}?gekostet|\bkosten\b|\bgekostet\b",
  "cost costs costed spend spent kosten gekostet usd dollars"),
 (("ended_at",), "when", r"how long ago|wie lange (?:[\w.-]+ ){0,4}?her\b|when did .{0,80}?\b(?:finish|end|complete|stop|wrap up|get done)|what time did .{0,80}?\b(?:finish|end)|"
                         r"wann (?:[\w.-]+ ){0,6}?(?:fertig|beendet|geendet|abgeschlossen|aufgehört|zu ende)|wann (?:endete|endeten|beendete|schloss)\b|\bend(?:ed)? at\b|\bendzeit\b",
  "ago finish finished end ended complete completed stop stopped wrap up get done her fertig beendet geendet abgeschlossen aufgehört ende endzeit geworden endete endeten beendete"),
 (("started_at",), "when", r"when did .{0,80}?\b(?:start|begin|kick off)|what time did .{0,80}?\bstart|wann (?:[\w.-]+ ){0,6}?(?:gestartet|begonnen|angefangen|losgegangen|los)\b|"
                           r"wann (?:begann|begannen|startete|starteten|fing)\b|\bstart(?:ed)? at\b|\bstartzeit\b",
  "start started begin began kick off gestartet begonnen angefangen losgegangen los startzeit begann begannen startete starteten fing an"),
 (("duration_s",), "duration", r"how long\b|wie lange\b|how much time|wie viel zeit|\bduration\b|\bdauer\b|how many (?:seconds|minutes|hours)|wie viele (?:sekunden|minuten|stunden)",
  "duration dauer seconds minutes hours sekunden minuten stunden zeit last lasted runtime laufzeit elapsed"),
 (("branch",), "string", r"(?:which|what|on what|on which) branch|(?:auf )?welche[mrns]? (?:branch|zweig)", "branch zweig"),
 (("author",), "string", r"\bwho\b|\bwer\b|\bauthor\b|\bautor(?:in)?\b", "author autor autorin wrote written made authored geschrieben gemacht committed eingecheckt pushed gepusht"),
 (("path",), "string", r"\bwhere\b|(?:which|what) (?:file|path)\b|\bwo\b|welche[rs]? (?:datei|pfad)", "file path datei pfad located live lives liegt"),
 (("status",), "status", r"did .{0,80}?\b(?:finish|succeed|complete|fail|pass)\b|is .{0,80}?\b(?:done|finished|still running|running|stuck|waiting|complete)\b|"
                         r"what(?:'s| is) the (?:status|state)|how did .{0,80}?\b(?:end|go)\b|\bstatus\b|ist .{0,80}?\b(?:fertig|durch|abgeschlossen|fehlgeschlagen)\b|"
                         r"läuft .{0,60}?\bnoch\b|wie ist .{0,80}?\b(?:ausgegangen|gelaufen)\b|\bzustand\b",
  "finish finished succeed succeeded complete completed fail failed pass done running stuck waiting status state end go went "
  "fertig durch abgeschlossen fehlgeschlagen läuft ausgegangen gelaufen zustand"),
]
ATTR_LEXICON = [(f, k, re.compile(rx, re.I), frozenset(cw.split())) for f, k, rx, cw in ATTR_LEXICON]
FIELD_KIND = {f: k for fs, k, _r, _c in ATTR_LEXICON for f in fs}
# "when" is two attributes (round 2: an end time answered "when did it start"): the question's own verb decides, the last start or end
# word wins ("when did the end-to-end suite start" asks the start), English and German
WHEN_START = re.compile(r"\b(?:start|started|starts|starting|begin|began|begun|begins|kick(?:ed)?\s+off|launch(?:ed)?|gestartet|begonnen|angefangen"
                        r"|losgegangen|los|begann|startete|fing|startzeit|beginn)\b", re.I)
WHEN_END = re.compile(r"\b(?:end|ended|ends|finish|finished|finishes|complete|completed|stop|stopped|done|wrap(?:ped)?\s+up|ago|fertig|beendet"
                      r"|geendet|endete|abgeschlossen|aufgehört|ende|endzeit|her)\b", re.I)
def when_split(question):
    """-> 'started_at', 'ended_at', or None when the question's words name neither (or a start and an end at the same place)"""
    q = re.sub(r"\b[\w.]*-[\w.-]*\b", " ", question or "")          # a hyphenated name ("end-to-end", "ibis-mobile") holds no verb
    s = [m.end() for m in WHEN_START.finditer(q)]; e = [m.end() for m in WHEN_END.finditer(q)]
    if not s and not e: return None
    return "started_at" if (max(s) if s else -1) > (max(e) if e else -1) else "ended_at"
def part_when(text):
    """what a claim states about time: 'ended_at' (an end word, or a duration: 'after 33 min'), 'started_at', or None"""
    if WHEN_END.search(text or "") or j4._durations(text or "")[0]: return "ended_at"
    if WHEN_START.search(text or ""): return "started_at"
    return None
def classify(question):
    """-> (fields tuple, kind, cue words) or (None, None, frozenset()) when the question's own words name no typed field"""
    for fs, k, rx, cw in ATTR_LEXICON:
        if rx.search(question or ""):
            if k == "when": fs = (when_split(question) or fs[0],)
            return fs, k, cw
    return None, None, frozenset()
HINT_FIELD = {"duration": ("duration_s",), "cost": ("cost_usd",), "who": ("author",), "where": ("path",)}   # the model's asks_about, a hint only
def hint_fields(asks, question):
    """the typed field(s) the model's asks_about (after the question's words override it) points to; 'when' by the question's verb"""
    if asks == "when": return (when_split(question),) if when_split(question) else ("started_at", "ended_at")
    return HINT_FIELD.get(asks)

# ======================================================================================================================
# The subject: the question's words left after the cue, against each record's identity (kind, id, name-like fields, lead label) and
# links (session, project, branch…). Strict on purpose: a missed subject is an "I don't know" (an omission), a wrong one an answer to a
# different question (critical).
# ======================================================================================================================
STOP = set("""happen happened happens happening change changed changes went go goes doing make made
how what which when where who whom whose why did does do done is are was were be been being has have had the a an in on of for
to at by with from into it its this that these those my me i we our us you your take took taken takes taking run ran runs get got gets
now right today yesterday tonight last latest recent recently current currently please tell show can could would will much many long time
times there here any all so far total overall whole entire just also still ever again about as and or but not no yes s t
wie was welche welcher welches welchen welchem wann wo wer wem wen hat haben hatte hatten ist sind war waren wurde wurden werden wird der die
das den dem des ein eine einen einem einer im in am an auf von für mit bei zu zum zur es mir mich ich du dir uns hier da dauern dauerte
gedauert gebraucht braucht lange viele viel noch jetzt heute gestern letzte letzten letzter letztes bitte mal eigentlich insgesamt gesamt
denn schon so nun und oder aber nicht kein keine
hold holds held contain contains contained belong belongs sit sits live lives land landed
liegt liegen lag lagen steht stehen stand gehört gehören enthält enthalten geändert geänderte geänderten geänderter geändertes gemacht
änderung änderungen""".split())   # round 3: the question's frame verbs
SYN = [{"session", "sessions", "sitzung", "sitzungen"}, {"test", "tests", "testsuite", "testsuites", "suite", "suites", "testlauf", "testläufe"},
       {"build", "builds", "kompilierung"}, {"lint", "linter", "linting"}, {"deploy", "deployment", "deployments", "rollout", "bereitstellung"},
       {"branch", "branches", "zweig"}, {"file", "files", "datei", "dateien"}, {"agent", "agents"}, {"commit", "commits"},
       {"migration", "migrations", "migrationen"},
       {"config", "configs", "configuration", "konfiguration", "konfigurationen", "konfig", "settings", "einstellungen"},   # round 3: EN/DE nouns
       {"docs", "doc", "documentation", "dokumentation", "doku"}, {"task", "tasks", "aufgabe", "aufgaben"}, {"run", "runs", "lauf", "läufe"}]
KIND_WORDS = set().union(*SYN[:10]) | {"task", "tasks", "aufgabe", "aufgaben"}   # words that name a kind of record: matched on its kind or name only
SYN_OF = {w: g for g in SYN for w in g}
IDENT_KEYS = ("name", "title", "label", "step", "job", "task", "suite", "workflow", "target", "type", "tool", "check")
LINK_KEYS = ("session", "parent", "project", "repo", "app", "agent", "branch", "pool", "turn")
def toks(s): return [w for w in re.findall(r"[^\W_]+", unicodedata.normalize("NFKC", str(s or "")).lower()) if w]
def wmatch(a, b):
    if a == b or j4._wmatch(a, b): return True
    g = SYN_OF.get(a); return bool(g) and b in g
def _has_all(keys, pool): return all(any(wmatch(k, p) for p in pool) for k in keys)
LEAD = re.compile(r"^\s*([^:.;!?\n]{1,60}?):\s")
LABEL_WORDS = 8               # a record's text this short is its label (round 2's sets: "export session", "checkout-fix", "Limits angepasst")
VALUE_KEYS = ("path", "branch")   # string values that name the work (a branch "fix/timeout-guard", a path "config/limits.toml")
def text_label(r):
    """the record's own text as its label: the whole text when it is short, else its first clause, capped (round 3: the found list
    showed placeholders such as 'session r1' for records named only by their text)"""
    t = re.sub(r"\s+", " ", r.get("text") or "").strip()
    if not t: return ""
    if len(toks(t)) <= LABEL_WORDS: return t.rstrip(".")
    first = re.split(r"(?<=[.;!?])\s|\s+—\s+", t)[0]
    return gate._cap(first.rstrip("."), 60)
def record_label(r):
    f = r["fields"]
    for k in IDENT_KEYS[:3]:
        if isinstance(f.get(k), str) and f[k].strip(): return f[k].strip()
    m = LEAD.match(r["text"] or "")
    if m: return m[1].strip()
    return text_label(r) or f"{r['kind']} {r['id']}"
def record_tokens(r):
    """-> (ident, link, label): the record's identity (kind, id, name-like fields, lead label), its links, and the words that name its
    work: a short text (its label), the start of a long one, and its path and branch values (round 3)"""
    f = r["fields"]; m = LEAD.match(r["text"] or "")
    ident = set(toks(r["kind"])) | set(toks(r["id"])) | {w for k in IDENT_KEYS if isinstance(f.get(k), str) for w in toks(f[k])} | (set(toks(m[1])) if m else set())
    link = {w for k in LINK_KEYS if f.get(k) not in (None, "") for w in toks(k) + toks(f[k])}
    tw = toks(r["text"]); label = set(tw if len(tw) <= LABEL_WORDS else tw[:6]) | {w for k in VALUE_KEYS if isinstance(f.get(k), str) for w in toks(f[k])}
    return ident, link, label
def subject_alternatives(question, cue=frozenset()):
    """the question's words less the cue's and the stop words -> [key-word lists], one per subject ("the ochre relay and slate
    importer sessions" gives two)"""
    alts = []
    for chunk in re.split(r"\s*(?:,|;|\band\b|\bor\b|\bund\b|\boder\b|\bvs\.?)\s*", question or "", flags=re.I):
        keys = [w for w in toks(chunk) if w not in STOP and w not in cue and w not in QUANTIFIERS and len(w) > 1 and not w.isdigit()]
        if keys: alts.append(list(dict.fromkeys(keys)))
    return alts
QUOTED = re.compile(r"[\"'“”„‘’«»‚]([^\"'“”„‘’«»‚]{2,60})[\"'“”„‘’«»‚]")
def quoted_words(question):
    """the words the question puts in quotes: a name ('import-test' names a record; its 'test' is no kind)"""
    return {w for m in QUOTED.finditer(question or "") for w in toks(m[1])}
def match(records, keys, names=frozenset()):
    """-> [(score, record)] of the records that hold every key; score = keys held by the identity alone. A word that names a kind of
    record (session, test, build, commit…) must be the record's kind or name: a session's text that mentions "the test suite" is not
    the test suite's record. Any other key may also be in the record's label words (its short text, the start of a long one, its path
    or branch: round 3, records named only by their text were "about nothing" in round 2)."""
    out = []
    for r in records:
        ident, link, label = record_tokens(r)
        kind_word = lambda k: k not in names and (k in KIND_WORDS or bool(SYN_OF.get(k, set()) & KIND_WORDS))
        if all(any(wmatch(k, p) for p in (ident | link if kind_word(k) else ident | link | label)) for k in keys):
            out.append((sum(1 for k in keys if any(wmatch(k, p) for p in ident)), r))
    return out
QUANTIFIERS = {"both", "beide", "beiden", "each", "every", "jede", "jeder", "jeden", "jedes"}
PLURAL_CUES = {"sessions", "sitzungen", "both", "beide", "beiden", "all", "alle", "allen", "each", "every", "jede", "jeder", "jeden", "runs", "läufe",
               "records", "einträge"}
def has_field(r, fields): return any(r["fields"].get(f) not in (None, "") for f in fields)
NAMES_FIELDS = {"failed_tests": ("failed_tests", "failing_tests", "failures"), "passed_tests": ("passed_tests",), "changed_files": ("changed_files", "files", "paths")}
COUNT_OF = {"failed_tests": "tests_failed", "passed_tests": "tests_passed", "changed_files": "files_changed"}
for _f in [x for v in NAMES_FIELDS.values() for x in v]: FIELD_KIND.setdefault(_f, "names")
COUNT_ONLY = {"en": {"tests_failed": "“{label}” shows {n} failed, but not which ones [{id}].", "tests_passed": "“{label}” shows {n} passed, but not which ones [{id}].",
                     "files_changed": "“{label}” shows {n} files changed, but not which ones [{id}].",
                     "zero_tests_failed": "“{label}” shows no failed tests [{id}].", "zero_tests_passed": "“{label}” shows no passed tests [{id}].",
                     "zero_files_changed": "“{label}” shows no files changed [{id}]."},
              "de": {"tests_failed": "„{label}“ zeigt {n} fehlgeschlagen, aber nicht welche [{id}].", "tests_passed": "„{label}“ zeigt {n} bestanden, aber nicht welche [{id}].",
                     "files_changed": "„{label}“ zeigt {n} geänderte Dateien, aber nicht welche [{id}].",
                     "zero_tests_failed": "„{label}“ zeigt keine fehlgeschlagenen Tests [{id}].", "zero_tests_passed": "„{label}“ zeigt keine bestandenen Tests [{id}].",
                     "zero_files_changed": "„{label}“ zeigt keine geänderten Dateien [{id}]."}}
def count_only(item, found):
    """round 3: "which tests failed" with records that hold only a count: say the count and that the record doesn't say which, by code
    (round 2: a model answered "2 tests failed", a count for a which-question). Zero is a full answer: none."""
    lang = "de" if item.get("lang") == "de" else "en"; W = COUNT_ONLY[lang]; lines = []; zero = True
    for r, vals in found:
        for f, v in vals.items():
            n = int(v); zero = zero and n == 0
            lines.append((W["zero_" + f] if n == 0 else W[f]).format(label=record_label(r), n=n, id=r["id"]))
    return {"ok": True, "kind": "answer" if zero else "count_only", "shown": "\n".join(lines), "refs": [r["id"] for r, _v in found], "model_text": [],
            "errors": [], "notes": ["a which-question; the records hold only a count (no call)"], "found": [r["id"] for r, _v in found],
            "values": [{"id": r["id"], **vals} for r, vals in found]}
def lookup(item, fields):
    """-> (found [(record, {field: value})], why). Found is empty when no record at the best subject match holds the field."""
    cue = classify(item["question"])[2] | {w for f in fields for _fs, _k, _r, cw in ATTR_LEXICON if f in _fs for w in cw}
    alts = subject_alternatives(item["question"], cue); recs = item["records"]
    if not alts:
        hold = [r for r in recs if has_field(r, fields)]
        return ([(r, {f: r["fields"][f] for f in fields if r["fields"].get(f) not in (None, "")}) for r in hold], "no subject words: the one record that holds it") \
            if len(hold) == 1 else ([], "no subject words, and not exactly one record holds the field")
    found = []
    for keys in alts:
        m = match(recs, keys, quoted_words(item["question"]))
        if not m: return [], f"no record is about {' '.join(keys)!r}"
        best = max(s for s, _r in m); top = [r for s, r in m if s == best]
        hold = [r for r in top if has_field(r, fields)]
        if not hold: return [], f"the records about {' '.join(keys)!r} hold no {'/'.join(fields)} (a nearby record may: not the answer)"
        if len(hold) > 3: return [], f"{len(hold)} records match {' '.join(keys)!r}: too many to answer"
        if len(hold) > 1 and not (set(toks(item["question"])) & PLURAL_CUES):
            same = len({record_label(r).lower() for r in hold}) == 1
            return [(r, {f: r["fields"][f] for f in fields if r["fields"].get(f) not in (None, "")}) for r in hold], ("same name: " if same else "ambiguous: ") + " ".join(keys)
        for r in hold:
            if all(r is not x for x, _v in found): found.append((r, {f: r["fields"][f] for f in fields if r["fields"].get(f) not in (None, "")}))
    return found, "found"

# ======================================================================================================================
# Values: shown by code, checked in the model's words
# ======================================================================================================================
def fmt_duration(s):
    s = float(s)
    if s < 90: return f"{round(s)} s"
    if s < 3 * 3600: return f"{round(s / 60)} min"
    return f"{round(s / 3600, 1):g} h"
def fmt_when(v):
    try:
        d = datetime.datetime.fromisoformat(str(v).replace("Z", "+00:00"))
        tz = " UTC" if d.utcoffset() == datetime.timedelta(0) else ""
        return d.strftime("%Y-%m-%d %H:%M") + tz
    except ValueError: return str(v)
def fmt_value(field, v):
    k = FIELD_KIND.get(field, "string")
    if isinstance(v, (list, tuple)): return ", ".join(str(x) for x in v)
    if k == "duration": return fmt_duration(v)
    if k == "when": return fmt_when(v)
    if k == "cost": return f"${float(v):.2f}"
    if field == "tests_passed": return f"{int(v)} passed"
    if field == "tests_failed": return f"{int(v)} failed"
    if k == "count": return str(int(v)) if isinstance(v, (int, float)) or str(v).isdigit() else str(v)
    return str(v)
def display(vals): return ", ".join(fmt_value(f, v) for f, v in vals.items())
FIELD_WORDS = {"duration_s": "duration", "tests_passed": "tests", "tests_failed": "tests", "files_changed": "files changed", "started_at": "started",
               "ended_at": "ended", "status": "status", "branch": "branch", "cost_usd": "cost", "author": "author", "path": "path"}
def found_lines(found):
    return [f"[{r['id']}] {r['kind']} \"{record_label(r)}\" | " + "; ".join(f"{FIELD_WORDS.get(f, f)}: {fmt_value(f, v)}" for f, v in vals.items())
            for r, vals in found]
def _nums(s): return checks.num_forms(s or "")
def value_errors(answer, found, item):
    """-> errors: the phrased value(s) must equal the looked-up field(s); no other value of that kind; every found record's value there"""
    errs = []; vals = [(f, v) for _r, vs in found for f, v in vs.items()]; kinds = {FIELD_KIND.get(f, "string") for f, _v in vals}
    labels = " ".join([item["question"]] + [record_label(r) + " " + r["id"] for r, _v in found])
    if kinds & {"count", "tests"} or any(f in ("tests_passed", "tests_failed", "files_changed") for f, _v in vals):
        exp = {str(int(v)) for f, v in vals if FIELD_KIND.get(f) in ("count", "tests") and str(v).lstrip("-").isdigit()}
        got = {n for n in _nums(answer) if n not in _nums(labels)}
        extra = got - exp
        if extra: errs.append(f"value: the answer says {sorted(extra)}, the records hold {sorted(exp)}")
        need = {str(int(v)) for f, v in vals if f != "tests_failed" or int(v) > 0}
        miss = need - got
        if miss and not (miss <= {"0"}): errs.append(f"value: the answer leaves out {sorted(miss)} from the records")
        failed = sum(int(v) for f, v in vals if f == "tests_failed")
        vw = gate.verdict_words_v3(j4.STATUS_NEG.sub(" ", answer))
        if failed > 0 and not (vw["fail"] or vw["nsucc"]): errs.append(f"value: {failed} failed, the answer carries no failure")
        if failed == 0 and any(f == "tests_failed" for f, _v in vals) and vw["fail"]: errs.append("value: 0 failed, the answer says something failed")
        if failed > 0 and gate.UNIVERSAL.search(answer) and not vw["fail"]: errs.append("value: the answer says all passed; tests failed")
    for f, v in vals:
        k = FIELD_KIND.get(f, "string")
        if k == "duration":
            got = j4._durations(answer)[0]; want = float(v); disp = fmt_value(f, v)
            tol = 0.5 if disp.endswith(" s") else 30 if disp.endswith(" min") else 180
            if not any(abs(g - want) <= tol for g in got): errs.append(f"value: the answer's duration is not {disp}")
            others = [g for g in got if not any(abs(g - float(v2)) <= tol for f2, v2 in vals if FIELD_KIND.get(f2) == "duration")]
            if others: errs.append(f"value: a duration no record holds ({others[0]:g} s)")
        elif k == "when":
            exp = j4._whens(fmt_value(f, v)); got = j4._whens(answer)
            allexp = set().union(*(j4._whens(fmt_value(f2, v2)) for f2, v2 in vals if FIELD_KIND.get(f2) == "when"))
            if not got & exp: errs.append(f"value: the answer's time is not {fmt_value(f, v)}")
            if got - allexp: errs.append(f"value: a time no record holds ({sorted(got - allexp)[0]})")
            if {x for x in exp if x.startswith("t:")} - got and any(x.startswith("t:") for x in got): errs.append("value: the clock time differs")
        elif k == "cost":
            got = j4._costs(answer)
            if not any(abs(g - float(v)) < 0.005 for g in got): errs.append(f"value: the answer's cost is not {fmt_value(f, v)}")
            if any(all(abs(g - float(v2)) >= 0.005 for f2, v2 in vals if f2 == "cost_usd") for g in got): errs.append("value: a cost no record holds")
        elif k == "status":
            want = j4._statuses(str(v)) or {str(v).lower()}; got = j4._statuses(answer)
            allw = set().union(*((j4._statuses(str(v2)) or {str(v2).lower()}) for f2, v2 in vals if f2 == "status"))
            if not (want & got or str(v).lower() in answer.lower()): errs.append(f"value: the answer doesn't say {v}")
            if got - allw: errs.append(f"value: the answer says {sorted(got - allw)}, the record says {v}")
        elif k == "string":
            if str(v).lower() not in answer.lower(): errs.append(f"value: the answer doesn't name {v}")
        elif k == "names":
            for x in (v if isinstance(v, (list, tuple)) else re.split(r"\s*[,;]\s*", str(v))):
                if str(x).strip() and str(x).lower() not in answer.lower(): errs.append(f"value: the answer doesn't name {x}")
    return errs

# ======================================================================================================================
# The look-up path's prompt (new in v5) and examples (fresh names; selftest_v5 checks the overlap)
# ======================================================================================================================
PHRASE_SCHEMA = lambda ids: j4.O("ChatPhrase", [
    ("answer", j4.S(200, 1, "One or two short sentences answering the QUESTION with the FOUND values, each copied exactly. At most 200 characters.")),
    ("cites", j4.A(j4.E(ids, "A FOUND record id."), len(ids), len(ids), "Every FOUND record id, each once."))])
PHRASE_INSTR = ("You are AI Juggler's Chat. AI Juggler watches a developer's coding agents. The developer asked a QUESTION, and AI Juggler "
  "has already looked up the answer: the FOUND lines are exact values from its records. You only write the answer in plain words.\n\n"
  "Rules:\n- Use the FOUND values exactly as written (48 min stays 48 min; 31 passed stays 31 passed). Never add a number, name, time or "
  "state that is not in the FOUND lines or the QUESTION.\n- Say which record each value belongs to, by its name in quotes.\n"
  "- Answer only the QUESTION. Never say you did or will do anything.\n- Cite every FOUND record id.\n"
  "- Write in the question's language.\n\nFields, in this order:\n- answer: one or two short sentences, at most 200 characters.\n"
  "- cites: every FOUND record id.")
PHRASE_EX = [
 {"question": "How long did the plover sync session take?", "found": [({"id": "s-41", "kind": "session", "fields": {"name": "plover sync"}, "text": ""}, {"duration_s": 2280})],
  "out": {"answer": "The plover sync session took 38 min.", "cites": ["s-41"]}},
 {"question": "Wie viele Tests sind in der Sitzung tern export fehlgeschlagen?", "found": [({"id": "s-7", "kind": "session", "fields": {"name": "tern export"}, "text": ""}, {"tests_failed": 2})],
  "out": {"answer": "In der Sitzung tern export sind 2 Tests fehlgeschlagen.", "cites": ["s-7"]}},
 {"question": "Did the tests pass in the auk cache and lark ui sessions?",
  "found": [({"id": "s-2", "kind": "session", "fields": {"name": "auk cache"}, "text": ""}, {"tests_passed": 19, "tests_failed": 0}),
            ({"id": "s-5", "kind": "session", "fields": {"name": "lark ui"}, "text": ""}, {"tests_passed": 7, "tests_failed": 1})],
  "out": {"answer": "In auk cache 19 passed and 0 failed; in lark ui 7 passed and 1 failed.", "cites": ["s-2", "s-5"]}},
 {"question": "Which branch is the heron api session on?", "found": [({"id": "s-9", "kind": "session", "fields": {"name": "heron api"}, "text": ""}, {"branch": "feat/heron-paging"})],
  "out": {"answer": "The heron api session is on feat/heron-paging.", "cites": ["s-9"]}},
]
def phrase_prompt(question, found): return "QUESTION (from the developer): " + question + "\n\nFOUND (looked up by AI Juggler; exact values):\n" + "\n".join(found_lines(found)) + \
    "\n\nWrite the answer from the FOUND values, copied exactly, and cite every FOUND id."
def phrase_system():
    ex = [f"Example {i}\n{phrase_prompt(e['question'], e['found'])}\nOUTPUT: {j4.to_grammar_json(e['out'])}" for i, e in enumerate(PHRASE_EX, 1)]
    return PHRASE_INSTR + "\n\n" + "\n\n".join(ex)

# ======================================================================================================================
# The job
# ======================================================================================================================
SCOPE_RX = re.compile(r"\b(?:write|code|regex|function|implement|refactor|translate|poem|joke|recipe|weather|schreib\w*|funktion|programmier\w*|übersetz\w*|witz|rezept|wetter)\b", re.I)
def fields_text(r, hide=(), cap=4):
    """a record's field values, less those of the asked kind (`hide`): a near miss's value never stands beside "I don't know"
    (round 1: the session's 48 min under "how long did the test suite take")"""
    vs = [f"{FIELD_WORDS.get(k, k)}: {fmt_value(k, v)}" for k, v in r["fields"].items()
          if v not in (None, "") and k not in IDENT_KEYS[:3] and FIELD_KIND.get(k, "string") not in hide and k not in hide]
    return "; ".join(vs[:cap])
def rows_text(item, ids=None, hide=()):
    """the found rows: each record's own label (its name, or its text: round 3) and its other field values, never a placeholder"""
    return "\n".join(f"[{r['id']}] {r['kind']} \"{record_label(r)}\"" + (f" | {fields_text(r, hide)}" if fields_text(r, hide) else "")
                     for r in item["records"] if ids is None or r["id"] in ids)
def asked_kinds(fields):
    """the kinds (and fields) a question asks: their values are hidden from the found rows"""
    return tuple({FIELD_KIND.get(f, "string") for f in fields or ()} | set(fields or ()) | ({"count", "tests"} if {"names"} & {FIELD_KIND.get(f) for f in fields or ()} else set()))
KIND_PLURAL = {"en": {"session": "sessions", "test_run": "test runs", "record": "records"}, "de": {"session": "Sitzungen", "test_run": "Testläufe", "record": "Einträge"}}
NUM_WORD = {"en": {2: "Two", 3: "Three"}, "de": {2: "Zwei", 3: "Drei"}}
def ambiguous(item, found, why):
    """two or three records at the same best subject match: code says so, with each one's value, instead of the model listing both
    values as one answer (round 2). No model call."""
    lang = "de" if item.get("lang") == "de" else "en"; n = len(found)
    kinds = {r["kind"] for r, _v in found}; kind = kinds.pop() if len(kinds) == 1 else "record"
    plural = KIND_PLURAL[lang].get(kind) or (kind + "s" if lang == "en" else "Einträge")
    labels = {record_label(r).lower() for r, _v in found}
    keys = [k for k in why.split(":", 1)[1].strip().split() if not any(wmatch(k, w) for w in toks(kind) + ["record", "records"])]
    m = re.search(r"[\w.]*" + r"[\s_-]+".join(re.escape(k) for k in keys) + r"[\w.]*", item["question"], re.I) if keys else None
    name = m.group(0) if m else " ".join(keys)
    if lang == "de": head = f"{NUM_WORD['de'].get(n, n)} {plural} " + ("heißen" if len(labels) == 1 else "passen zu") + f" „{name}“:"
    else: head = f"{NUM_WORD['en'].get(n, n)} {plural} " + ("are called" if len(labels) == 1 else "match") + f" “{name}”:"
    return {"ok": True, "kind": "ambiguous", "shown": head + "\n" + "\n".join(found_lines(found)), "refs": [r["id"] for r, _v in found],
            "model_text": [], "errors": [], "notes": [why + " (no call)"], "found": [r["id"] for r, _v in found]}
COUNT_WORDS = {"en": {"files_changed": "files changed"}, "de": {"files_changed": "Dateien geändert"}}
SAME_NAME = {"en": ("{n} {plural} are called “{label}”: ", "started", "ended"), "de": ("{n} {plural} heißen „{label}“: ", "gestartet", "beendet")}
def _when_of(r, lang):
    W = SAME_NAME[lang]
    for f, word in (("started_at", W[1]), ("ended_at", W[2])):
        v = r["fields"].get(f)
        if v not in (None, ""): return f"{word} {fmt_when(v).split(' ')[0]}"
    return ""
def same_name(item, found):
    """round 3: two or three records equally named (a session run twice): each one's value with its record and its date, by code, as
    the answer ("20 min (started 2026-11-02) [r1]; 5 min (started 2026-11-09) [r2]")"""
    lang = "de" if item.get("lang") == "de" else "en"; n = len(found)
    kinds = {r["kind"] for r, _v in found}; kind = kinds.pop() if len(kinds) == 1 else "record"
    plural = KIND_PLURAL[lang].get(kind) or (kind + "s" if lang == "en" else "Einträge")
    head = SAME_NAME[lang][0].format(n=NUM_WORD[lang].get(n, n), plural=plural, label=record_label(found[0][0]))
    def val(vals):
        return ", ".join(f"{fmt_value(f, v)} {COUNT_WORDS[lang][f]}" if f in COUNT_WORDS[lang] else fmt_value(f, v) for f, v in vals.items())
    parts = [val(vals) + (f" ({_when_of(r, lang)})" if _when_of(r, lang) else "") + f" [{r['id']}]" for r, vals in found]
    return {"ok": True, "kind": "answer", "shown": head + "; ".join(parts) + ".", "refs": [r["id"] for r, _v in found], "model_text": [], "errors": [],
            "notes": ["equally named records: each value, by code (no call)"], "found": [r["id"] for r, _v in found],
            "values": [{"id": r["id"], **vals} for r, vals in found]}
PICK_SCHEMA = lambda ids: j4.O("ChatPick", [
    ("pick", j4.E(list(ids) + ["unclear"], "The one CANDIDATES record the QUESTION asks about, or unclear when its words don't tell them apart.")),
    ("answer", j4.S(200, 1, "One short sentence answering the QUESTION with that record's value, copied exactly, or none when unclear. At most 200 characters."))])
PICK_INSTR = ("You are AI Juggler's Chat. AI Juggler watches a developer's coding agents. The developer asked a QUESTION, and AI Juggler found "
  "more than one record that could be meant: the CANDIDATES lines, each with its own words and its exact value.\n\nRules:\n"
  "- Pick the one candidate the QUESTION's words are about. When its words fit more than one, or none, pick unclear: never guess.\n"
  "- Answer with the picked record's value exactly as written. Never add a number, name, time or state that is not in its line.\n"
  "- Answer only the QUESTION. Never say you did or will do anything.\n- Write in the question's language.\n\nFields, in this order:\n"
  "- pick: a candidate id, or unclear.\n- answer: one short sentence, at most 200 characters, or none when unclear.")
PICK_EX = [
 {"question": "Where is the changed lint rule?",
  "found": [({"id": "c-1", "kind": "commit", "fields": {"path": "lint/rules.yaml"}, "text": "tightened the lint rules"}, {"path": "lint/rules.yaml"}),
            ({"id": "c-2", "kind": "commit", "fields": {"path": "docs/lint.md"}, "text": "notes on linting"}, {"path": "docs/lint.md"})],
  "out": {"pick": "c-1", "answer": "The changed lint rule is in lint/rules.yaml."}},
 {"question": "How long did the plover session take?",
  "found": [({"id": "s-1", "kind": "session", "fields": {"name": "plover sync"}, "text": ""}, {"duration_s": 720}),
            ({"id": "s-2", "kind": "session", "fields": {"name": "plover backfill"}, "text": ""}, {"duration_s": 2400})],
  "out": {"pick": "unclear", "answer": "none"}},
]
def pick_prompt(question, found): return "QUESTION (from the developer): " + question + "\n\nCANDIDATES (found by AI Juggler; exact values):\n" + \
    "\n".join(found_lines(found)) + "\n\nPick the candidate the QUESTION is about, or unclear, and answer with its value."
def pick_system():
    ex = [f"Example {i}\n{pick_prompt(e['question'], e['found'])}\nOUTPUT: {j4.to_grammar_json(e['out'])}" for i, e in enumerate(PICK_EX, 1)]
    return PICK_INSTR + "\n\n" + "\n\n".join(ex)
def dont_know(item, why, fields=None):
    lead = j4.FIXED["dont_know"] if not item["records"] else j4.FIXED["not_sure"]
    hide = asked_kinds(fields) if fields else tuple(FIELD_KIND.values())        # the asked kind's values hidden; unknown ask: no values
    return {"ok": True, "kind": "dont_know", "shown": lead + ("\n" + rows_text(item, None, hide) if item["records"] else ""), "refs": [r["id"] for r in item["records"]],
            "model_text": [], "errors": [], "notes": [why], "found": []}
def v4_item(item):
    """the v4 Chat path's item: each record a row whose words are its text and its typed fields (code-rendered)"""
    rows = [{"ref": r["id"], "kind": r["kind"], "words": (r["text"] + (" | " if r["text"] and r["fields"] else "") +
             "; ".join(f"{k}: {fmt_value(k, v)}" for k, v in r["fields"].items() if v not in (None, ""))).strip()} for r in item["records"]]
    return {"id": item["id"], "question": item["question"], "rows": rows, "removed": item["removed"], "gold": item["gold"]}
class Chat(Job):
    name, letter, title, item_kind = "chat", "d", "Chat answer", "chat"
    omission_over_all_items = True
    def route(self, item):
        fs, kind, _cue = classify(item["question"])
        if fs and kind == "names":
            nf = NAMES_FIELDS.get(fs[0], fs)
            found, why = lookup(item, nf)
            if found: return ("pick" if why.startswith("ambiguous") else "lookup", nf, found, why)
            cf = (COUNT_OF[fs[0]],) if fs[0] in COUNT_OF else None
            if cf:
                cfound, cwhy = lookup(item, cf)
                if cfound and cwhy.startswith(("found", "no subject")): return ("count_only", cf, cfound, "which: only a count: " + cwhy)
            return ("lookup", nf, [], why)
        if fs:
            found, why = lookup(item, fs)
            return ("pick" if why.startswith("ambiguous") else "lookup", fs, found, why)
        alts = subject_alternatives(item["question"])
        if SCOPE_RX.search(item["question"]) and not any(match(item["records"], k) for k in alts): return ("scope", None, [], "not about AI Juggler's records")
        return ("open", None, [], "the lexicon names no typed field: v4's Chat path")
    def plan(self, item, setting, mark=True):
        path, fs, found, why = self.route(item)
        if path == "scope": return {"decided": {"ok": True, "kind": "scope", "shown": j4.FIXED["scope"], "refs": [], "model_text": [], "errors": [], "notes": [why], "found": []}}
        if path == "lookup" and not found: return {"decided": dont_know(item, why, fs)}
        if path == "count_only": return {"decided": count_only(item, found)}
        if path == "lookup" and why.startswith("same name"): return {"decided": same_name(item, found)}
        if path == "pick":
            ids = [r["id"] for r, _v in found]; schema = PICK_SCHEMA(ids); sys_t = pick_system(); src = item["question"] + "\n" + "\n".join(found_lines(found))
            schema = j4.with_script_guide(schema, src)
            def build(note):
                p = pick_prompt(item["question"], found) + note
                return {"job": "chat", "mode": "fields", "system": sys_t, "prompt": p, "schema": schema, "grammar": j4.gbnf(schema, src), "max_tokens": 200,
                        "est_tokens": j4.est_tokens(sys_t) + j4.est_tokens(p) + j4.est_tokens(json.dumps(schema)) + 200, "cut": False,
                        "refs": ids, "question": item["question"], "path": "pick"}
            return {"build": build, "schema": schema}
        if path == "lookup":
            ids = [r["id"] for r, _v in found]; schema = PHRASE_SCHEMA(ids); sys_t = phrase_system(); src = item["question"] + "\n" + "\n".join(found_lines(found))
            schema = j4.with_script_guide(schema, src)
            def build(note):
                p = phrase_prompt(item["question"], found) + note
                return {"job": "chat", "mode": "fields", "system": sys_t, "prompt": p, "schema": schema, "grammar": j4.gbnf(schema, src), "max_tokens": 200,
                        "est_tokens": j4.est_tokens(sys_t) + j4.est_tokens(p) + j4.est_tokens(json.dumps(schema)) + 200, "cut": False,
                        "refs": ids, "question": item["question"], "path": "lookup"}
            return {"build": build, "schema": schema}
        ci = v4_item(item)
        def build(note):
            sys_t, prompt, schema, grammar, mx, est, cut = j4.request("chat", chat_item=ci, mark=mark)
            return {"job": "chat", "mode": "fields", "system": sys_t, "prompt": prompt + note, "schema": schema, "grammar": grammar, "max_tokens": mx,
                    "est_tokens": est + j4.est_tokens(note), "cut": cut, "refs": [r["ref"] for r in ci["rows"]], "question": item["question"], "path": "open"}
        return {"build": build, "schema": j4.chat_schema([r["ref"] for r in ci["rows"]])}
    def evaluate(self, item, setting, fields, finish="stop", refused=False):
        path, fs, found, why = self.route(item)
        if path == "pick": return self._eval_pick(item, fields, found, why, finish)
        if path == "lookup": return self._eval_lookup(item, fields, found, finish)
        return self._eval_open(item, fields, finish)
    def _eval_pick(self, item, fields, found, why, finish):
        """round 3: several records match the subject and code can't tell them apart: the model picks one by the question's words, or
        unclear. Code checks the answer against the picked record alone (its value; no other candidate's) and cites it by code."""
        pick = fields.get("pick")
        if pick == "unclear": return dict(ambiguous(item, found, why), notes=["the model found the question unclear: the candidates, by code"])
        chosen = [(r, v) for r, v in found if r["id"] == pick]
        if not chosen: return {"ok": False, "kind": "rejected", "errors": [f"pick: {pick!r} is not a candidate"], "model_text": []}
        # the picked record alone is the source: another candidate's value is then "invented" or a wrong value (step 2, value_errors)
        res = self._eval_lookup(item, {"answer": fields.get("answer", ""), "cites": [pick]}, chosen, finish)
        if not res["ok"]: return res
        return dict(res, notes=res["notes"] + [f"picked {pick} of {[r['id'] for r, _v in found]}"])
    def _eval_lookup(self, item, fields, found, finish):
        if not found: return {"ok": False, "kind": "rejected", "errors": ["nothing was looked up: code says I don't know"], "model_text": []}
        ids = [r["id"] for r, _v in found]; ans = re.sub(r"\s*\[[^\]]{1,40}\]", "", j4._tidy(fields.get("answer", ""))).strip()
        errs = []
        if finish == "length": errs.append("cut at the token limit")
        if sorted(set(fields.get("cites") or [])) != sorted(ids): errs.append(f"cites: must be exactly {ids}")
        src = item["question"] + "\n" + "\n".join(found_lines(found))
        bad = j4.fields_foreign(fields, src)
        if bad: errs.append("a letter outside the question's and records' scripts: " + "".join(bad))
        ok2, inv = gate.step2_v3(src, ans, None)
        if not ok2: errs.append("invented: " + ", ".join(f"{c} {t}" for c, t in inv))
        for fam in sorted(gate.families_v3(ans) - gate.families_v3(src)): errs.append(f"words: a {fam} word the records don't hold")
        if j4.ACTING.search(ans): errs.append("an acting claim")
        errs += value_errors(ans, found, item)
        looked = {f for _r, v in found for f in v}
        if looked == {"started_at"} and part_when(ans) == "ended_at": errs.append("off the question: the start time was asked; the answer speaks of an end or a duration")
        if looked == {"ended_at"} and part_when(ans) == "started_at": errs.append("off the question: the end time was asked; the answer speaks of a start")
        if errs: return {"ok": False, "kind": "rejected", "errors": errs, "model_text": [], "found": [r["id"] for r, _v in found]}
        shown = ans.rstrip() + "".join(f" [{i}]" for i in ids)
        return {"ok": True, "kind": "answer", "shown": shown, "refs": ids, "model_text": [ans], "errors": [], "notes": ["looked up: " + ", ".join(fs for _r, v in found for fs in v)],
                "found": [r["id"] for r, _v in found], "values": [{"id": r["id"], **{f: v for f, v in vals.items()}} for r, vals in found]}
    def _eval_open(self, item, fields, finish):
        ci = v4_item(item); o = j4.chat_outcome(ci, fields)
        errs = list(o["errors"])
        if finish == "length": errs.append("cut at the token limit")
        if o["dropped"] and o["kind"] != "answer": errs += [f"off the question: {why}" for _i, why in o["dropped"]]
        hint = None
        if fields.get("covered") in ("yes", "partly"):
            asks = j4.chat_relevance(fields, ci)["asks"]; hint = hint_fields(asks, item["question"])
            if o["kind"] == "answer" and asks not in j4.FREE_ASKS:
                # round 2: each shown part's states must be the question's attribute, rechecked after code overrode asks_about
                for p in o.get("parts") or []:
                    st = p.get("states")
                    if st != asks: errs.append(f"{j4._tidy(p.get('text', ''))[:40]!r}: off the question: states {st}, the question asks {asks}")
                    if asks == "when" and when_split(item["question"]) == "started_at" and part_when(p.get("text", "")) == "ended_at":
                        errs.append(f"{j4._tidy(p.get('text', ''))[:40]!r}: off the question: the start was asked; it states an end or a duration")
                    if asks == "when" and when_split(item["question"]) == "ended_at" and part_when(p.get("text", "")) == "started_at" and not WHEN_END.search(p.get("text", "")):
                        errs.append(f"{j4._tidy(p.get('text', ''))[:40]!r}: off the question: the end was asked; it states a start")
        if not o["errors"] and hint and o["kind"] == "answer":
            found, why = lookup(item, hint)
            # no record at the subject holds the asked field (a start time for "when did it start"): I don't know and the found rows,
            # whatever the parts say (round 2: an end time and a duration answered a start question)
            if not found: return {**dont_know(item, f"the question asks {'/'.join(hint)}; {why}", hint), "notes": ["the asked field overrode a written answer"]}
            if not errs:
                ve = value_errors(o["text"], found, item)
                if ve: errs += ve
        if errs: return {"ok": False, "kind": "rejected", "errors": errs, "model_text": []}
        kind = {"answer": "answer", "scope": "scope"}.get(o["kind"], "dont_know")
        return {"ok": True, "kind": kind, "shown": o["text"] + ("\n" + rows_text(item, o["refs"], tuple(set(FIELD_KIND.values()))) if kind == "dont_know" else ""), "refs": o["refs"],
                "model_text": [p["text"] for p in o.get("parts") or []] if kind == "answer" else [], "errors": [], "notes": o["notes"], "found": []}
    def fallback(self, item, setting):
        path, fs, found, why = self.route(item)
        if path == "pick": return ambiguous(item, found, why)
        if path == "count_only": return count_only(item, found)
        if path == "lookup" and why.startswith("same name"): return same_name(item, found)
        if path == "lookup" and found:
            lines = [f"{record_label(r)} [{r['id']}]: {display(vals)}" for r, vals in found]
            return {"ok": True, "kind": "found_values", "shown": "\n".join(lines), "refs": [r["id"] for r, _v in found], "model_text": [], "errors": [],
                    "notes": ["every attempt failed its checks: the looked-up values, by code"], "found": [r["id"] for r, _v in found]}
        return {**dont_know(item, "every attempt failed its checks: the found rows", fs), "kind": "found_rows"}
    def alternative(self, item, setting):
        f = self.fallback(item, setting); return f["shown"]
    def write_prompts(self):
        e = PHRASE_EX[0]
        doc = ["# Harness v5, job (d) chat: prompt version " + str(PROMPT_VERSION), "",
               "Generated by `selftest_v5.py --write-prompts`. Two paths: the look-up path (new in v5) below; a question the lexicon can't read takes v4's "
               "Chat prompt unchanged (`../v4/prompts/chat.md`), with the retry note appended.", "",
               "## Look-up path: system prompt", "", "```text", phrase_system(), "```", "", "## Look-up path: prompt layout", "", "```text",
               phrase_prompt(e["question"], e["found"]), "```", "", "## Look-up path: fields", "", "```json",
               json.dumps(PHRASE_SCHEMA(["s-41"]), indent=1, ensure_ascii=False), "```", "", "## Look-up path: GBNF", "", "```gbnf",
               j4.gbnf(PHRASE_SCHEMA(["s-41"])).rstrip("\n"), "```", "",
               "## Pick path (round 3: several records match and code can't tell them apart): system prompt", "", "```text", pick_system(), "```", "",
               "## Pick path: fields", "", "```json", json.dumps(PICK_SCHEMA(["c-1", "c-2"]), indent=1, ensure_ascii=False), "```", "",
               "## The lexicon (code, not the model: what the question asks for)", "", "| Fields | Kind | Pattern |", "|---|---|---|"]
        doc += [f"| {', '.join(fs)} | {k} | `{rx.pattern.replace('|', '¦')}` |" for fs, k, rx, _cw in ATTR_LEXICON]
        return "\n".join(doc) + "\n"
CHAT = register(Chat())

# ======================================================================================================================
# Chat items for the TUNED corpora (the fake run): every four turns of a corpus become one record set with typed fields, and five
# questions with known answers. Deterministic. Never used on a held-out file, which brings its own items.
# ======================================================================================================================
def chat_items_tuned(label, raw_items, per_set=4):
    turns = [x for x in raw_items if x.get("source") and not x.get("records")]; out = []
    for si in range(0, len(turns) - per_set + 1, per_set):
        grp = turns[si:si + per_set]; recs = []; n = si // per_set
        for j, it in enumerate(grp):
            F = gate.facts_norm(it.get("facts")); ctx = gate.prepare(it["source"])
            fields = {"name": j4.TITLES[(n + j) % len(j4.TITLES)], "status": j4.row_state(it.get("facts")) if F else None}
            if F and F["tests_state"] == "ran": fields.update(tests_passed=F["passed"], tests_failed=F["failed"])
            recs.append({"id": f"s-{label}-{n:02d}{j}", "kind": "session", "fields": {k: v for k, v in fields.items() if v is not None},
                         "text": gate._cap(gate_v4.excerpt_v4(ctx["cleaned"], it.get("facts"))[0].replace("\n", " "), 280)})
        a, b = recs[n % per_set], recs[(n + 1) % per_set]
        an, bn = a["fields"]["name"], b["fields"]["name"]; sid = f"{label}-C{n:02d}"
        qs = [(f"{sid}-happened", f"What happened in the {an} session?", {"attribute": None, "unanswerable": False, "cite": [a["id"]]}),
              (f"{sid}-passed", f"How many tests passed in the {bn} session?", {"attribute": "tests_passed", "unanswerable": "tests_passed" not in b["fields"],
                "cite": [b["id"]] if "tests_passed" in b["fields"] else [], "answer_must": [str(b["fields"]["tests_passed"])] if "tests_passed" in b["fields"] else []}),
              (f"{sid}-long", f"How long did the {an} session take?", {"attribute": "duration_s", "unanswerable": True, "near_miss": True, "cite": []})]
        if n % 3 == 2: qs.append((f"{sid}-scope", j4.OUT_OF_SCOPE_Q, {"attribute": None, "unanswerable": True, "out_of_scope": True, "cite": []}))
        else: qs.append((f"{sid}-status", f"Did the {bn} session finish?", {"attribute": "status", "unanswerable": "status" not in b["fields"], "cite": [b["id"]]}))
        for qid, q, gold in qs: out.append({"id": qid, "question": q, "records": recs, "gold": gold, "origin": "built from a tuned corpus (chat.chat_items_tuned)"})
    return out
