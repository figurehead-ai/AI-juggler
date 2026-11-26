#!/usr/bin/env python3
"""watch9.py: harness v13's watch drafting (AJ-E3830): the app's own code at its v13 commit, with word-indexed labels.

v12 (`../../v12-results/summary.md`) held the hard bar (0 claimed) and the nearest bar (95.5 %), but the labels never verified: Apple's
model repeated the request word for word in about 30 of 84 span answers. v13 (the coordinator's calls, 2026-11-26):
  - word-indexed labels: code splits the words and places every one it can; the model sees all the words numbered and labels only those
    code couldn't place, an array of exactly that many slots (DynamicGenerationSchema minimumElements = maximumElements), so coverage
    holds by construction; code's slot stands for every word it placed; a wrong length is refused (never padded, never cut);
  - filler from the model is a second guard: never filler a number or a word for one, a time or date word, a word for how often or
    which, a name or a part of one, a condition's cue word, a negation, a unit or a word for code work (else told back, the nearest);
  - scope-neutral negations exempt only as closed exact phrases ("don't care about the rest", "quiet hours don't matter", "in der
    Ruhezeit bitte nicht" as a quiet-hours setting);
  - code fixes from v12's misses: the paraphrase map ("still ist", "durch ist", "zu Ende", "wieder Luft/Platz", "goes quiet"), asks
    ("buzz me", "shout", "meld dich", "say when"), lifetimes ("until I delete the watch", "bis ich es wieder lösche", "Einmalig",
    "heute noch", "the first time", "one heads-up is enough"), "immer noch" as still, dash and space name variants, compounds with
    -Agent and -Konto, a session over a project of its name said with dashes.
The harness clock is the round's own day (AIJ_HARNESS_TODAY, default 2026-11-26)."""
import os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), os.path.join(H, "v10"),
          os.path.join(H, "v11"), os.path.join(H, "v12"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import watch8 as W8                                                 # noqa: E402  (the clock, the base job)
from core import register                                           # noqa: E402

PROMPT_VERSION = 13
TODAY = W8.TODAY
class WatchWordLabels(W8.WatchVerifiedLabels):
    name, letter, title = "watch9", "f13", "watch drafting: the app's own code, word-indexed labels, a second guard on filler"
    def fake_draft(self, req, prompt, bad, k):
        """runner --fake: the label call calls every word it is asked about other (when bad, one slot short: a wrong length)"""
        if req.get("step") != "label": return {"target": "none"}
        m = re.search(r"LABEL \((\d+) words", prompt); n = int(m[1]) if m else 0
        return {"slots": ["other"] * (max(0, n - 1) if bad else n)}
    def write_prompts(self):
        return W8.WatchVerifiedLabels.write_prompts(self).replace("# Harness v12, job (f12) watch8: prompt version 12",
                                                                  "# Harness v13, job (f13) watch9: prompt version 13").replace(
            "`selftest_v12.py --write-prompts`", "`selftest_v13.py --write-prompts`")
WATCH9 = register(WatchWordLabels())
