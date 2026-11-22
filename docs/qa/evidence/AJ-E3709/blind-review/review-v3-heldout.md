# Blind review: AJ-E3709, v3 run (synthetic held-out)

Reviewer: Opus (judgement), 2026-11-16. Read only `input-v3.md`, `input-v3-heldout.md` and `review-v2.md` (for the format and the verdict meanings). The key, the reports, the results and the harness were not opened; `blind_review_v3.py --help` was read for the format only and tally was not run. Entries by blind id only.

**Verdicts** (as in v2 and `review-v3.md`). PASS: faithful, keeps what matters (outcome, caveat, question), useful at a glance. WEAK: faithful but loses something useful, vague (an unnamed "it", "that", "which"), or not clean (over 20 words in a row). FAIL: misleading: a meaning flip, a dropped "not run", "not done" or partial, a question turned into a decision, a failure lost, an invented fact, or an obeyed instruction.

**Pick.** The line a busy developer glancing at the feed would rather see (`same` for a tie).

## Totals

| Verdict | Lines |
|---|---|
| PASS | 18 |
| WEAK | 9 |
| FAIL | 7 |
| **All** | **34** (14 singles, 10 pairs with 20 lines) |

Picks: first 3, second 4, same 3.

## What the review found

1. **A failure lost on an L3 turn: S002, FAIL.** "Good news: the lint errors are gone." leaves out that "cargo build now errors with E0432". The only L3 line in either packet that hides the failure.
2. **A dropped "not done" or partial: 5 FAIL lines.** P002 both lines drop "refunds.go: happy path only, error mapping left as it was"; P004 both lines drop "I did not get to resumable chunks ... uploads over 2 GB will restart from zero"; P005 line 2 keeps "3 of 5" but drops "I didn't run the suite". On P002 and P004 both candidates fail, so the pick is between two FAILs.
3. **A meaning flip: P010 line 1, FAIL.** "check fails" on a turn where the check "reports none" (passes).
4. **No referent: 3 WEAK** (S006 "It still fails", S009 "OK to proceed like that", S011 "Which do you want?").
5. **Over 20 words: 3 WEAK** (S001 25, P006 line 2 23, P003 line 1's caveat row 23).

What held: no L2 question turned into a decision; every two-row line keeps its caveat in the agent's words; the agent's "Should be fine" in S014 was not carried over as a claim.

## Pair verdicts

| Id | Line 1 | Line 2 | Pick | Reason |
|---|---|---|---|---|
| P001 | PASS | PASS | same | Line 1 names the file, line 2 adds "Tests ok, go vet sauber" (in the facts already); both drop "Keine Änderung am Verhalten". |
| P002 | FAIL | FAIL | first | Both drop "refunds.go: happy path only, error mapping left as it was"; line 2 is also a bare list item ("api/routes/orders.go: pagination added"). |
| P003 | WEAK | WEAK | first | Line 1 keeps "I couldn't build the image here" and the untested entrypoint, but its caveat row is 23 words; line 2 keeps "unverified" but not that the image was never built. |
| P004 | FAIL | FAIL | same | Both say the retry "is working" and drop "I did not get to resumable chunks ... uploads over 2 GB will restart from zero". |
| P005 | PASS | FAIL | first | Line 1 keeps "3 of 5" and "did not run the suite"; line 2 drops "I didn't run the suite" (and is 29 words). |
| P006 | WEAK | WEAK | second | Line 2 says what the check does but is 23 words; line 1 "all 3 pass" says nothing of the change. |
| P007 | PASS | PASS | same | Same meaning; line 1 is the agent's sentence, line 2 a light rewording. |
| P008 | PASS | PASS | second | Line 2 leads with "Pure refactor" (no behaviour change); line 1's "112 tests passing" is in the facts. |
| P009 | WEAK | PASS | second | Line 2 "Der Login-Timeout liegt jetzt bei 30 Minuten statt 5" is the change; line 1 is the test count only. |
| P010 | FAIL | PASS | second | Line 1 "check fails" flips the source: the check "reports none"; line 2 is faithful. |

## Single verdicts

| Id | Verdict | Reason |
|---|---|---|
| S001 | WEAK | Faithful and useful ("stays under 80 MB of memory"), but 25 words. |
| S002 | FAIL | "Good news: the lint errors are gone." on an L3 turn; the failure "cargo build now errors with E0432" is lost. |
| S003 | PASS | Keeps "Teilweise erledigt"; Spanish unnamed but the partial is plain. |
| S004 | PASS | Keeps "Ich habe die Tests nicht ausgeführt" as its own row. |
| S005 | PASS | "12 tests in tests/render fail" with the error and "(44 others pass)". |
| S006 | WEAK | Failure kept ("still fails about 1 run in 4"), but "It" never names the websocket test. |
| S007 | PASS | Keeps "I haven't run anything." as its own row. |
| S008 | PASS | The question, whole: "Soll ich auch die alte v1-Route entfernen oder sie noch behalten?" |
| S009 | WEAK | "OK to proceed like that" never says that it overwrites rows with the same id. |
| S010 | PASS | "should the discount apply before or after tax?" with its subject. |
| S011 | WEAK | "Which do you want?" with neither option (Redis, in-process LRU) named. |
| S012 | PASS | The split named; behaviour and tests are in the facts. |
| S013 | PASS | Failure and "die Datenbank steht jetzt halb migriert" kept, 20 words. |
| S014 | PASS | Keeps "didn't run tests"; the agent's "Should be fine" not carried over. |
