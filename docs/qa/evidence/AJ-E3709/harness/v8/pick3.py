#!/usr/bin/env python3
"""pick3.py: harness v8's feed line, design (A): v7's pick from code's top three, plus the step-count fix (AJ-E3831; the coordinator's
v8 calls). v7's pair 016: "The new column and the backfill script, 5 passed" put a step count where a test count goes. A count the
message or the facts use for steps, tasks, parts or "N of M" is never shown as a test count: the line then carries no count."""
import os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__)); H = os.path.dirname(HERE)
for p in (os.path.join(H, "v6"), os.path.join(H, "v7"), HERE):
    if p not in sys.path: sys.path.insert(0, p)
import core6                                                        # noqa: E402,F401
from core import register                                           # noqa: E402
import turns                                                        # noqa: E402
import pick2                                                        # noqa: E402

PROMPT_VERSION = 8
NUM_WORDS = {"one": 1, "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7, "eight": 8, "nine": 9, "ten": 10, "eleven": 11, "twelve": 12,
             "eins": 1, "zwei": 2, "drei": 3, "vier": 4, "fünf": 5, "sechs": 6, "sieben": 7, "acht": 8, "neun": 9, "zehn": 10}
N = r"(\d+|" + "|".join(NUM_WORDS) + r")"
STEPS = re.compile(r"\b" + N + r"\s+(?:of\s+(?:the\s+)?" + N + r"\s+)?(?:steps?|tasks?|parts?|items?|phases?|stages?|migrations?|todos?|points?|schritte?n?|aufgaben?|teile?n?)\b"
                   r"|\b(?:step|task|part|phase|stage|schritt|aufgabe)\s+" + N + r"\b|\b" + N + r"\s+(?:of|von)\s+(?:the\s+|den\s+)?" + N + r"\b", re.I)
def num(x): return int(x) if x.isdigit() else NUM_WORDS.get(x.lower())
def step_counts(t):
    """the numbers the message or the facts use for steps, tasks, parts, or as "N of M" """
    out = {num(g) for m in STEPS.finditer(t["ctx"]["cleaned"]) for g in m.groups() if g}
    part = (t["F"] or {}).get("partial")
    if part: out |= set(part)
    return {n for n in out if n is not None}
class FeedPickSteps(pick2.FeedPickTop):
    name, letter, title = "feed_pick3", "a8", "feed line (A): pick from code's top three, no step count as a test count"
    def fields_of(self, t, chs, pick):
        fields, why = super().fields_of(t, chs, pick)
        if fields is None: return fields, why
        m = re.match(r"(\d+) passed(?:, (\d+) failed)?$", fields.get("result") or "")
        if m and ({int(m[1])} | ({int(m[2])} if m[2] else set())) & step_counts(t):
            fields = dict(fields, result="none", _notes=fields["_notes"] + ["the test count equals a step count the message uses: left out"])
        return fields, None
    def write_prompts(self):
        return pick2.FeedPickTop.write_prompts(self).replace("Harness v7, job (a7)", "Harness v8, job (a8)").replace("prompt version 7", "prompt version 8").replace(
            "`selftest_v7.py --write-prompts`", "`selftest_v8.py --write-prompts`").replace("job (a7) feed_pick2", "job (a8) feed_pick3")
FEED_PICK3 = register(FeedPickSteps())
