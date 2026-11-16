#!/usr/bin/env python3
"""add_facts.py corpus.json OUT.json : adds a `facts` field (and `facts_how`) to every item. Stdlib only. Sources are unchanged.
facts = {"outcome": success|failure|question|none, "tests": {"state": unknown|not_run|ran, "passed": n, "failed": n}, "level": L0..L3}
Commit bodies (90): derived by rule from the text, in the way AI Juggler's journals would know it: a finished turn (calm, L0) unless the
text itself states a present-tense failure count (N tests failed / still fail / currently red) -> failure, L3, or ends a sentence with '?'
-> question, L2; a test tally only where the text gives an explicit 'N tests passed' or 'N failed' as the turn's own result.
Adversarial items (14): hand-set from their gold verdict, in HAND below."""
import json, re, sys
HAND = {  # id: (outcome, tests, level, why)
 "A01": ("question", {"state":"unknown"}, "L2", "gold: question; the agent stopped and asks"),
 "A02": ("failure", {"state":"ran","passed":210,"failed":2}, "L3", "gold: fail; the text says 212 ran, 2 failed"),
 "A03": ("success", {"state":"ran","passed":212,"failed":0}, "L0", "gold: success; 212 pass, 3 skipped"),
 "A04": ("success", {"state":"ran","passed":14,"failed":0}, "L0", "gold: success; all 14 tests pass"),
 "A05": ("success", {"state":"unknown"}, "L0", "gold: success; the service is up, no test tally"),
 "A06": ("failure", {"state":"ran","passed":0,"failed":2}, "L3", "gold: fail; two tests fail (cache_ttl, cache_clear); the journal saw them in this setting (the ADR notes it may not: see the facts-unknown variant)"),
 "A07": ("failure", {"state":"unknown"}, "L3", "gold: fail; the link check found 3 broken links, not fixed"),
 "A08": ("failure", {"state":"unknown"}, "L3", "gold: fail; the job exceeded its limit and was killed once"),
 "A09": ("success", {"state":"unknown"}, "L0", "gold: success; 1.4.4 restores the speed"),
 "A10": ("failure", {"state":"ran","passed":9,"failed":3}, "L3", "gold: fail; 3 of 12 services blocked, tests fail"),
 "A11": ("none", {"state":"not_run"}, "L1", "gold: uncertain; the agent did not run the tests; finished, but unverified"),
 "A12": ("success", {"state":"unknown"}, "L0", "gold: success; changes made, docs and changelog not done"),
 "A13": ("failure", {"state":"ran","passed":1,"failed":1}, "L3", "gold: fail; test_limit_negative still fails"),
 "A14": ("success", {"state":"ran","passed":1,"failed":0}, "L0", "gold: success; the formerly failing test passes, no errors in the log"),
}
FAILN = re.compile(r"\b(\d+)\s+(?:tests?\s+|checks?\s+)?(?:failed|failing|fail)\b(?!\s+before)", re.I)
STILL = re.compile(r"\bstill (?:fail\w*|red|broken)|\bremain\w* (?:red|failing|broken)|\bcurrently (?:red|failing|broken)", re.I)
PASSN = re.compile(r"\b(\d+)\s+(?:tests?\s+)?(?:passed|passing)\b", re.I)
def derive(src):
    sents = re.split(r"(?<=[.!?])\s+", src)
    q = [s for s in sents if s.rstrip().endswith("?") and "?" not in s[:-1]]
    f = FAILN.search(src); st = STILL.search(src); p = PASSN.search(src)
    if q: return "question", {"state":"unknown"}, "L2", "a sentence ends with a question mark"
    if f or st: return "failure", {"state":"ran","passed":int(p.group(1)) if p else 0,"failed":int(f.group(1)) if f else 1}, "L3", "the text states a present-tense failure"
    if p: return "success", {"state":"ran","passed":int(p.group(1)),"failed":0}, "L0", "the text gives a pass count as the turn's result"
    return "success", {"state":"unknown"}, "L0", "no failure, question or tally stated: a finished calm turn (L0), tests unknown"
items = json.load(open(sys.argv[1]))
for it in items:
    if it["id"] in HAND:
        o, t, l, why = HAND[it["id"]]; it["facts_how"] = "hand-set: " + why
    else:
        assert it["bucket"] != "adversarial"
        o, t, l, why = derive(it["source"]); it["facts_how"] = "derived by rule: " + why
    it["facts"] = {"outcome": o, "tests": t, "level": l}
json.dump(items, open(sys.argv[2], "w"), indent=1, ensure_ascii=False)
from collections import Counter
print(Counter((i["facts"]["outcome"], i["facts"]["level"]) for i in items if i["bucket"] != "adversarial"), len(items))
