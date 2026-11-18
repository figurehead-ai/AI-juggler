Harness v5 round 2 (jobs a-d) RE-SCORED under round 3's rules (v5-design.md §13), 2026-11-18. No model re-run; blind packets and keys not rebuilt.

THIS IS A FLOOR, not a measurement of the fixed harness: a draft the new rules refuse has no recorded retry, so its replay shows the fallback; a question code now routes to the look-up path has no recorded phrasing, so it shows the looked-up value by code. A live run retries both. Not blind: Opus read the round-2 items named in the fix brief. Synthetic held-out only; not checked on real messages. Script flags are candidates; Opus confirms.

Critical reaching the user (shown model items with a critical candidate, % of shown model items) and omissions (% of shown items; Chat and attention: of all items), before -> after:

| Job | Model | Critical reaching | Omissions % | Fallbacks (of calls) |
|---|---|---|---|---|
| feed_line | apple | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 8 -> 11 of 15 |
| feed_line | gemma-4-e2b | 1 (8.33 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 3 -> 6 of 15 |
| feed_line | granite-4.1-3b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 4 -> 8 of 15 |
| feed_line | nemotron-3-nano-4b | 1 (7.69 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 2 -> 9 of 15 |
| feed_line | qwen3-1.7b-q8 | 1 (16.67 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 9 -> 12 of 15 |
| feed_line | qwen3-4b-instruct-2507 | 1 (9.09 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 4 -> 10 of 15 |
| review | apple | 1 (11.11 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 41 -> 45 of 50 |
| review | gemma-4-e2b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 24 -> 28 of 50 |
| review | granite-4.1-3b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 21 -> 30 of 50 |
| review | nemotron-3-nano-4b | 1 (4.17 %) -> 0 (0.0 %) | 0.0 -> 5.88 | 26 -> 33 of 50 |
| review | qwen3-1.7b-q8 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 37 -> 41 of 50 |
| review | qwen3-4b-instruct-2507 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 5 -> 17 of 50 |
| attention | apple | 0 (0.0 %) -> 0 (0.0 %) | 6.0 -> 0.0 | 0 -> 3 of 18 |
| attention | gemma-4-e2b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 0 -> 0 of 18 |
| attention | granite-4.1-3b | 0 (0.0 %) -> 0 (0.0 %) | 2.0 -> 0.0 | 2 -> 3 of 18 |
| attention | nemotron-3-nano-4b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 0 -> 0 of 18 |
| attention | qwen3-1.7b-q8 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 0 -> 0 of 18 |
| attention | qwen3-4b-instruct-2507 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 1 -> 1 of 18 |
| chat | apple | 1 (6.67 %) -> 0 (0.0 %) | 37.25 -> 7.84 | 4 -> 4 of 25 |
| chat | gemma-4-e2b | 0 (0.0 %) -> 0 (0.0 %) | 39.22 -> 9.8 | 6 -> 6 of 25 |
| chat | granite-4.1-3b | 1 (9.09 %) -> 0 (0.0 %) | 45.1 -> 13.73 | 9 -> 9 of 25 |
| chat | nemotron-3-nano-4b | 0 (0.0 %) -> 0 (0.0 %) | 43.14 -> 13.73 | 7 -> 8 of 25 |
| chat | qwen3-1.7b-q8 | 0 (0.0 %) -> 0 (0.0 %) | 41.18 -> 11.76 | 10 -> 7 of 25 |
| chat | qwen3-4b-instruct-2507 | 0 (0.0 %) -> 0 (0.0 %) | 37.25 -> 7.84 | 4 -> 3 of 25 |

Attention, facts given, after: misses 0 of 32 for every model; false alarms apple 0/18, gemma-4-e2b 0/18, granite-4.1-3b 0/18, nemotron-3-nano-4b 0/18, qwen3-1.7b-q8 0/18, qwen3-4b-instruct-2507 0/18 (before: apple 3/18, granite 1/18).

What moved, and why (all generic; Opus to confirm by eye):
- Feed: the reverted-work lines are refused now, on R2T15 ("Den Logger ersetzt", 4 models) and also on R2T14 ("Switched the date picker to a new library", the agent "rolled it back"; git reverted package.json and DatePickerNext.tsx) and R2T19 ("Die Datenbankindizes geändert", the agent "zurückgesetzt"; git deleted the index migration): the same class, which round 2's detector missed. More feed fallbacks also come from the facts now reaching the frozen gate (outcome finished, tests not run on a "none" state): its outcome rule refuses a failure word with no resolution on a finished turn, and its caveat rule wants "tests not run".
- Review: the remaining fallbacks are mostly test outcomes in the risk reason (now code's to show) and "risk low with tests not run" (the facts now say so). Apple's 45 fallbacks are its engine error on one-path schemas (no text recorded to replay); the one-path field fixes it for the next run (probed).
- Chat: omissions fall to 7.8-13.7 %; most newly answered items show the looked-up value by code here (no phrasing recorded). R2C12 now says the count and that the record doesn't say which tests. R2C31 answers each duration with its record and date. Remaining omissions: open questions answered "I don't know" (R2C29, R2C30, R2C46), the total-tests question (R2C40), R2C44, R2C50 ("Verbrauch"), and R2C51 (the question carries an instruction).
- Replay mismatches are listed in each report JSON (expected: every outcome the new rules moved).
