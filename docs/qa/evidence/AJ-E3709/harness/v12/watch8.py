#!/usr/bin/env python3
"""watch8.py: harness v12's watch drafting (AJ-E3830, D-149 amendment 3 as corrected 2026-11-26): the app's own code, as v11's watch7,
now with **verified labels that may draft**.

v11 (`../../v11-results/summary.md`) held the hard bar (0 claimed) and the nearest bar (95.5 %) but drafted only what code placed word by
word: exact 65.8 %. v12 (the app at its v12 commit):
  - labels may draft once code verifies them all: the spans repeat the request exactly; no exclusion or other span; a word code can't
    place only in a target span, and a word of the one entity the target spans resolve to (a name in part also says its kind or two of
    its words; German compounds "-Sitzung", "-Session", "-Lauf" apart); the condition spans name exactly one condition (the cue table and
    its closed paraphrase map); every number in a parameter span whose unit that condition takes;
  - a negation guard independent of the labels: not/n't, nicht, kein*, except, außer, unless, es sei denn, without, ohne, but not, never,
    nie, other than, apart from (and ignore, skip, excluding) anywhere outside a name or a quiet-hours phrase: told back with the nearest;
  - code fixes from v11's misses: compound names, "keep telling me", "tell me about any new …", "ich möchte/will wissen", "I only care
    when", scope-neutral context phrases ("whatever else is running", "egal was sonst läuft"), "only"/"nur" as scope words, paraphrases
    ("sits idle", "new sessions starting"), composites ("crashes and gets started again" → restarted; "ends and leaves processes running"
    → leaves_processes).
The harness clock is the round's own day: AIJ_HARNESS_TODAY (default 2026-11-26, v12's set); the report's meta says harness v12."""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), os.path.join(H, "v8"), os.path.join(H, "v9"), os.path.join(H, "v10"),
          os.path.join(H, "v11"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6, watch2 as W2                                          # noqa: E402
from core import register                                           # noqa: E402
import watch7 as W7                                                 # noqa: E402

PROMPT_VERSION = 12
TODAY = os.environ.get("AIJ_HARNESS_TODAY", "2026-11-26")
core6.HARNESS_TODAY = TODAY; W2.HARNESS_TODAY = TODAY             # gold's "today" is this round's own day

class WatchVerifiedLabels(W7.WatchFullCover):
    name, letter, title = "watch8", "f12", "watch drafting: the app's own code, verified labels may draft, a negation guard"
    def write_prompts(self):
        return W7.WatchFullCover.write_prompts(self).replace("# Harness v11, job (f11) watch7: prompt version 11",
                                                             "# Harness v12, job (f12) watch8: prompt version 12").replace(
            "`selftest_v11.py --write-prompts`", "`selftest_v12.py --write-prompts`")
WATCH8 = register(WatchVerifiedLabels())
