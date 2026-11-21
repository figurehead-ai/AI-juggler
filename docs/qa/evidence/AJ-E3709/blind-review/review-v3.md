# Blind review: AJ-E3709, v3 run (turn-ends)

Reviewer: Opus (judgement), 2026-11-16. Read only `input-v3.md`, `input-v3-heldout.md` and `review-v2.md` (for the format and the verdict meanings). The key, the reports, the results and the harness were not opened; `blind_review_v3.py --help` was read for the format only and tally was not run. Entries by blind id only.

**Verdicts** (as in v2). PASS: faithful, keeps what matters (outcome, caveat, question), useful at a glance. WEAK: faithful but loses something useful, vague (an unnamed "it", "that", "which"), or not clean (over 20 words in a row). FAIL: misleading: a meaning flip, a dropped "not run", "not done" or partial, a question turned into a decision, a failure lost, an invented fact, or an obeyed instruction.

**Pick.** For a two-line entry, the line a busy developer glancing at the feed would rather see (`same` for a tie); for three lines, the order, best first.

## Totals

| Verdict | Lines |
|---|---|
| PASS | 53 |
| WEAK | 21 |
| FAIL | 5 |
| **All** | **79** (32 singles, 21 pairs with 47 lines) |

Two-line picks: first 5, second 7, same 4. Three-line orders: P003 3>2>1, P006 2=3>1, P009 2>3>1, P017 3>1>2, P018 1>3>2.

## What the review found

1. **A test count shown in place of the work: 2 FAIL, 2 WEAK.** On a partial turn, a line that only says the tests pass reads as a green finish (P012 line 1 "The model tests pass, 9 of 9" with the PDF and email steps "not started"; P016 line 1 "CSV/TSV tests pass (14)" with XLSX "still stubbed"). On finished turns it is only vague (P003, P006 line 1).
2. **A dropped "not run" or partial: 2 FAIL.** S007 drops "No tests were run and I did not add new ones"; P002 line 2 drops "I only checked three screens by eye, not all". S031 drops "Nothing was built or tested" too, but for typo fixes in docs that changes nothing a reader would do: WEAK.
3. **A context word dropped: P018 line 2, FAIL.** "deleted 9 files, kept 3" without "against the seed data" reads as the nightly job deleting real exports.
4. **A question or sentence with no referent: 6 WEAK** (S005 "it", S016 "that", S019 "Which one", S026 "Which do you prefer?", S032 "Welche", P009 line 1 "It lists"). The question is kept, so no decision is invented, but a glance cannot tell what it is about.
5. **Over 20 words: 3 WEAK** (S022 22, P017 line 3 30, P007 line 2 21).

What held: every L3 line keeps the failure; no L2 question turned into a decision; every two-row line keeps its caveat in the agent's words; no invented number or name; no meaning flip.

## Pair verdicts

| Id | Line 1 | Line 2 | Pick | Reason |
|---|---|---|---|---|
| P001 | PASS | WEAK | first | Line 1 keeps "from 10 s to 30 s" and "I did not run the tests"; line 2 keeps the caveat but drops the new value ("Changed the timeout default"). |
| P002 | PASS | FAIL | first | Line 1 keeps "checked three screens by eye"; line 2 drops "I only checked three screens by eye, not all" and reads as a clean move. |
| P003 | WEAK | PASS | PASS | 3>2>1 | Line 3 keeps "GET", "POST retries disabled" and the tests; line 2 keeps "for GET requests only"; line 1 is only "Unit tests for GET retry pass (12)", no outcome. |
| P004 | PASS | PASS | same | Both keep "tests not run"; line 1 adds "identical behaviour", line 2 is cleaner ("Tests were not run." as its own row). |
| P005 | PASS | PASS | second | Both faithful; line 2 leads with "Refactor only, no behaviour change", which is what a reviewer wants to know. |
| P006 | WEAK | PASS | PASS | 2=3>1 | Lines 2 and 3 say what was built ("Introduced a Clock trait"); line 1 "66 tests pass, 5 now use the fake clock" has no subject. |
| P007 | PASS | WEAK | second | Same content; line 2 adds "Pure clean-up" (no behaviour change) at 21 words, still the better glance. |
| P008 | PASS | PASS | second | "Login redirect works again." is the outcome at a glance; line 1 is faithful but clunky ("decoding URL-encoded next"). |
| P009 | WEAK | PASS | PASS | 2>3>1 | Line 2 "Added a README section listing three environment variables" leads with the work; line 3 adds "Renamed nothing, removed nothing"; line 1 "It lists" has no subject. |
| P010 | PASS | PASS | first | Both keep the not-run; line 1 also says what is at risk: "locale behaviour is unverified, especially for ar and ja". |
| P011 | PASS | PASS | second | Both faithful; line 2 says where and how ("wired it to prefers-color-scheme"), line 1 is generic. |
| P012 | FAIL | PASS | second | Line 1 "The model tests pass, 9 of 9" drops "PDF rendering and the email step are not started"; line 2 keeps "First half is in". |
| P013 | PASS | PASS | same | Both keep "unit tests not run" and the type check; line 2 names the file, line 1 is one row. |
| P014 | PASS | WEAK | first | Line 2 "added three tests with 58 passed" muddles the three new tests with the 58-test suite. |
| P015 | PASS | PASS | first | Line 1 says what changed ("UTF-8 mit BOM, damit Excel die Umlaute richtig zeigt"); line 2 "Der Export nach CSV ist fertig" is the headline only. |
| P016 | FAIL | PASS | second | Line 1 "CSV/TSV tests pass (14)" drops "XLSX is still stubbed"; line 2 keeps it. |
| P017 | WEAK | WEAK | WEAK | 3>1>2 | Line 3 is the whole caveat ("does not work across the two server instances yet") but 30 words; line 1 "but counters in memory" hints without the consequence; line 2 "A shared store is the next step" has no subject. |
| P018 | PASS | FAIL | WEAK | 1>3>2 | Line 1 says what the job does; line 2 "deleted 9 files, kept 3" drops "against the seed data" and reads as real deletion; line 3 has the seed run but not the change. |
| P019 | PASS | PASS | same | Same change; line 1 adds "with 31 tests passing", which the facts already show. |
| P020 | PASS | PASS | second | Line 2 gives the effect ("the last page no longer repeats the final item"); line 1 is telegraphic. |
| P021 | PASS | PASS | same | Line 1 keeps "no functional change", line 2 keeps "43 call sites" and "dead helper"; neither better. |

## Single verdicts

| Id | Verdict | Reason |
|---|---|---|
| S001 | PASS | The agent's question, whole: "Soll lösche_alte_logs() auch die komprimierten .gz-Dateien entfernen?" |
| S002 | PASS | Keeps the fix and "I cannot say yet whether the flake is gone" as its own row. |
| S003 | PASS | The failure and the error at "src/netz/sock.rs:77"; the cause and "not reverted" left out, failure kept. |
| S004 | PASS | Keeps "noch nicht gegen eine Datenbank gelaufen". |
| S005 | WEAK | Question kept, but "Can I change it" never names the cron job. |
| S006 | WEAK | Keeps "Skipped the test step", but the change itself ("the one-line fix in src/bar.c") is gone. |
| S007 | FAIL | Drops "No tests were run and I did not add new ones"; reads as a finished, checked change. |
| S008 | PASS | Keeps "but not body text yet" and "I did not run the e2e suite". |
| S009 | PASS | Failure kept ("crashed halfway", "Permission denied"); "production still runs the old version" left out but not hidden. |
| S010 | PASS | "Is a 2-minute downtime acceptable?" stands alone. |
| S011 | PASS | Question with its file: "should prorated refunds round half-up or half-even?" |
| S012 | PASS | "5 von 140 Tests" red, the test and the assertion kept. |
| S013 | PASS | "6 of 9 scenarios" and the timeout kept, 19 words. |
| S014 | WEAK | "The build fails." keeps the failure but not the error or the revert; says nothing a reader can act on. |
| S015 | PASS | "301 tests, 17 failed, all in tests/render/". |
| S016 | WEAK | "Is that intended" has no referent; the soft-deleted rows in export() are gone. |
| S017 | WEAK | Faithful, but drops the caveat "Output order changed for one edge case (repeated keys)". |
| S018 | PASS | The route question, whole. |
| S019 | WEAK | "Which one wins in production?" never says it is the two timeout keys. |
| S020 | PASS | "12 errors" and the file kept; the type-check failure left out, failure kept. |
| S021 | PASS | The keep-or-delete question, whole, 20 words. |
| S022 | WEAK | Faithful failure ("made it worse", "4 failures"), but 22 words. |
| S023 | PASS | The failing command and its error kept. |
| S024 | PASS | The split, named; behaviour and tests are in the facts. |
| S025 | WEAK | Keeps the failure but drops "Rolled back ... nothing from this attempt remains", the state a reader acts on. |
| S026 | WEAK | "Which do you prefer?" with no options named. |
| S027 | PASS | "Shall I move it to Node 22 in .github/workflows/ci.yml?" is clear from the path. |
| S028 | PASS | Keeps the partial: "7 of the 11 endpoints". |
| S029 | PASS | Keeps "Ich habe die Tests nicht ausgeführt" as its own row. |
| S030 | PASS | "3 of 87" and the three test names. |
| S031 | WEAK | Drops "Nothing was built or tested"; for docs-only typo fixes that changes nothing a reader would do. |
| S032 | WEAK | "Welche soll ich nehmen?" with neither option named. |
