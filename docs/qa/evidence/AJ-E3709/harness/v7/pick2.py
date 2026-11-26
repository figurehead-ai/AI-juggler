#!/usr/bin/env python3
"""pick2.py: harness v7's feed line, the pick from code's top three (AJ-E3831 R1, D-149's amendment of 2026-11-26).

v6 round: the line was preferred in 34 % of pooled blind pairs against 60 %. The losers: the model picked a file diff ("Changed
Cargo.lock (+140 -128), 61 passed") or a side detail ("Deleted the old cron script…") over the turn's point, and the plain excerpt
read better. v7: code ranks the turn's checked choices (v6's, every v5 check still applied) and the model picks among the top three:

  first   what the turn came to: an outcome ("now", "works", "passes", "instead", "so that", "jetzt", "funktioniert") or a decision
          ("decided", "chose", "went with", "kept", "entschieden");
  then    the change itself: an action of substance (added, fixed, replaced, implemented, moved, switched, rewrote, …);
  last    side details: clean-ups, deletions, bumps, formatting, lint, typos, comments, lock files; a problem the turn started from;
  out     git's files (a diff is never the line when the agent said anything), unless nothing else is left.
"""
import os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); V6 = os.path.join(os.path.dirname(HERE), "v6")
for p in (V6, HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402
from core6 import j4, MAX_RETRIES                                   # noqa: E402
from core import register                                           # noqa: E402
import turns                                                        # noqa: E402
import pick as P6                                                   # noqa: E402  (v6: choices, the checks, the render)

PROMPT_VERSION = 7
TOP = 3
OUTCOME = re.compile(r"\b(?:now|works?|working|passes|pass(?:ing)?|green|instead|so\s+that|which\s+means|no\s+longer|again|faster|"
                     r"jetzt|funktioniert|läuft|wieder|schneller|nicht\s+mehr)\b", re.I)
DECISION = re.compile(r"\b(?:decided|chose|chosen|went\s+with|settled\s+on|opted|kept\s+(?:the|it|them)\s+\w+\s+(?:as|instead)|entschieden|gewählt|beibehalten)\b", re.I)
SUBSTANCE = {"added", "fixed", "replaced", "implemented", "moved", "switched", "rewrote", "built", "created", "introduced", "migrated",
             "ported", "reworked", "restructured", "split", "extracted", "converted", "wired", "enabled", "cached", "guarded", "handled",
             "renamed", "refactored", "batched", "validated", "unified", "corrected", "patched",
             "umgebaut", "hinzugefügt", "behoben", "ersetzt", "umgestellt", "eingebaut", "verschoben", "implementiert", "repariert"}
SIDE = re.compile(r"\b(?:deleted|removed|cleaned|tidied|bumped|formatted|reformatted|lint\w*|typos?|comments?|whitespace|renamed\s+(?:a\s+)?variables?|"
                  r"lock\s?files?|cargo\.lock|package-lock|pnpm-lock|yarn\.lock|old\s+\w+\s+script|gelöscht|entfernt|aufgeräumt|formatiert)\b", re.I)
PROBLEM = re.compile(r"\bkept\s+\w+ing\b|\b(?:was|were|used\s+to|had\s+been|before)\b.*\b(?:fail\w*|crash\w*|drop\w*|chok\w*|hang\w*|slow|broke\w*|timing\s+out|wrong)\b", re.I)
def rank(t, chs):
    """-> the choices best first, git's files only when nothing else is left"""
    words = [x for x in chs if x["kind"] != "git"]
    if not words: return chs[:TOP]
    def score(i, x):
        txt = x["clause"] if "clause" in x else x["words"]
        s = 0
        if OUTCOME.search(txt): s += 3
        if DECISION.search(txt): s += 3
        if x["kind"] == "action" and x["verb"] in SUBSTANCE: s += 2
        elif x["kind"] == "action": s += 1
        if SIDE.search(txt): s -= 3
        if PROBLEM.search(txt) and not OUTCOME.search(txt): s -= 2
        return (-s, i)
    return [x for _k, x in sorted(((score(i, x), x) for i, x in enumerate(words)), key=lambda kx: kx[0])][:TOP]
def renumber(chs):
    return [dict(x, n=str(i)) for i, x in enumerate(chs, 1)]

class FeedPickTop(P6.FeedPick):
    name, letter, title = "feed_pick2", "a7", "feed line (pick from code's top three)"
    def __init__(self): super().__init__(); self._sys = {}
    def system(self, mark=True):
        if mark in self._sys: return self._sys[mark]
        rules = [r for r in P6.RULES if not r.startswith("Pick a git file")]
        rules.insert(0, "AI Juggler has put the CHOICES in its order: what the turn came to (an outcome or a decision) first, the change itself next. Pick the one a busy developer most needs; usually the first.")
        exs = []
        for i, ex in enumerate(EXAMPLES, 1):
            t = P6.example_turn(ex); chs = renumber(rank(t, P6.choices(t)))
            exs.append(f"Example {i}\n" + P6.prompt_text(t, chs, f"v7ex{i}", mark).replace("\n\n" + P6.TASK, "") + "\nOUTPUT: " + j4.to_grammar_json(ex["out"]))
        s = P6.ROLE.replace("the agent's own clauses and the files git says changed", "the agent's own clauses, best first") + "\n\nRules:\n" + \
            "\n".join("- " + r for r in rules) + "\n\n" + P6.FIELDS + "\n\n" + "\n\n".join(exs)
        self._sys[mark] = s; return s
    def route(self, t):
        r = super().route(t)
        return r
    def run(self, t, setting, ask, mark=True, retries=MAX_RETRIES):
        r = self.route(t)
        if r: return self.decided(t, *r), "code"
        chs = renumber(rank(t, P6.choices(t)))
        note_text = ""
        for k in range(1 + retries):
            req = self.request(t, chs, mark, note_text)
            out, a = ask("pick", req, k)
            errs = []
            if out.get("refused") or out.get("error"): errs.append("engine: refused or failed")
            elif out.get("finish") == "length": errs.append("shape: cut at the token limit")
            got, hard = core6.parse(req["schema"], out.get("text")) if not errs else (None, [])
            errs += hard; res = None
            if got is not None:
                fields, why = self.fields_of(t, chs, got)
                if fields is None: errs.append(f"pick: {why}")
                else:
                    notes = fields.pop("_notes"); x = fields.pop("_choice")
                    res = turns.FEED.evaluate(t, setting, fields, out.get("finish", "stop"), False)
                    errs += res.get("errors") or []
                    if res.get("ok"):
                        res = dict(res, pick={"fact": x["n"], "kind": x["kind"], "verb": got.get("verb"), "rank": int(x["n"])}, fields=fields,
                                   notes=(res.get("notes") or []) + notes)
            a["errors"] = errs[:12]; a["ok"] = bool(res and res.get("ok") and not errs)
            if a["ok"]: return res, k
            note_text = core6.note(out.get("text"), errs or ["the answer was not valid"], "the fact and the verb")
        return self.fallback(t, setting), "fallback"
    def write_prompts(self):
        ex = EXAMPLES[0]; t = P6.example_turn(ex); chs = renumber(rank(t, P6.choices(t)))
        return turns.prompt_doc(self, self.system(True), P6.prompt_text(t, chs, "doc", True), P6.pick_schema(chs)).replace(
            "Harness v5, job (a7)", "Harness v7, job (a7)").replace("prompt version 5", f"prompt version {PROMPT_VERSION}").replace(
            "`selftest_v5.py --write-prompts`", "`selftest_v7.py --write-prompts`")
EXAMPLES = [  # fresh, fictional (trees; in no corpus: selftest_v7 checks)
 {"label": "the outcome first; the clean-up and the problem last",
  "message": "The larch importer kept choking on empty rows. I added a skip for empty rows in importer/larch.py, so imports of the big sheets now finish. I also deleted the old larch cron script.",
  "facts": {"outcome": "success", "level": "L1", "tests": {"state": "ran", "passed": 12, "failed": 0}},
  "git": {"changed": [{"path": "importer/larch.py", "added": 14, "removed": 3, "status": "M"}, {"path": "Cargo.lock", "added": 140, "removed": 128, "status": "M"}], "reverted": [], "reflog": []},
  "out": {"fact": "1", "verb": "none"}},
 {"label": "a decision",
  "message": "Compared two ways to page the rowan feed and went with cursor paging, since offsets drift when rows arrive.",
  "facts": {"outcome": "success", "level": "L1", "tests": {"state": "unknown"}}, "git": None,
  "out": {"fact": "1", "verb": "none"}},
 {"label": "German: the change itself",
  "message": "Ich habe den Export der Eiben-Liste umgebaut. Die alten Kommentare habe ich entfernt.",
  "facts": {"outcome": "success", "level": "L1", "tests": {"state": "unknown"}}, "git": None,
  "out": {"fact": "1", "verb": "umgebaut"}},
]
FEED_PICK2 = register(FeedPickTop())
