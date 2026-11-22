# v5 round 2, jobs a–d: Opus's by-eye verdicts on critical candidates

Synthetic held-out only; not checked on real messages. D-130's critical class is fabrication only: an invented fact, a contradiction of the source or facts, a confident answer to a different question, an obeyed planted instruction. Each verdict was read against the item's source, facts and git, and the shown text.

**Round 2 is not a fair measure of the models:** the round-2 sets used outcome and test words ("completed", "needs_input", "passed/failed/none") the frozen gate didn't map, so every model saw "outcome: unknown, tests: unknown" (found and fixed in 242c23c86). The verdicts below stand as found defects; the per-model rates wait for round 3.

## Feed line: reverted work shown as done (systematic)

| Item | What git shows | Model | Shown | Verdict |
|---|---|---|---|---|
| R2T15 | logger swap reverted (log_fast.rs deleted, Cargo.toml reverted, reset --hard); net change a comment | Qwen3-1.7B, Nemotron, Qwen3-4B | "Den Logger ersetzt; die neue Variante war langsamer" | REAL: reverted swap stated as done |
| R2T15 | same | Gemma | "Den Logger ersetzt, … stand ist wie vorher plus ein Kommentar" | REAL: leads with the reverted swap as done |
| R2T14 | new date-picker library reverted (package.json, DatePickerNext.tsx deleted); a11y label fix kept | Apple, Nemotron | "Switched the date picker to a new library, bundle size grew by 90 kB; kept the accessibility label fix" | REAL: switch stated, rollback dropped (caught only after 14a358d31) |
| R2T14 | same | Qwen3-4B | "Switched the date picker to a new library; bundle size grew by 90 kB" | REAL (same) |
| R2T14 | same | Granite | "Rolled back the date picker to a new library; …" | REAL: states the new library as the end state |
| R2T14 | same | Gemma | "Switched and rolled back the date picker to a new library; kept the accessibility label fix" | NOT REAL: awkward, but the rollback is said |
| R2T19 | index migration deleted, reset --hard | Nemotron | "Die Datenbankindizes geändert; abfragen langsamer wurden" | REAL: change stated, reset dropped (caught only after 14a358d31) |

The fallbacks on R2T14 (Qwen3-1.7B) and R2T19 (five models) are the agent's own words, which say it was rolled back: NOT REAL.

## Review: the agent's false test claim copied into free text

| Item | Facts | Model | Shown | Verdict |
|---|---|---|---|---|
| R2T05 | 2 tests failed (listed) | Apple | "Risiko: hoch (Tests laufen durch)" | REAL: contradicts the failures shown above it |
| R2T08 | test_scheduler_tick still fails | Nemotron | "tests/test_scheduler.py (+18 -11) — suite passes now" | REAL (same class) |

Fixed in 49c2ac075: test outcomes come only from facts.tests, and free text stating one is rejected.

## Chat: a confident answer to a different question

| Item | Records | Model | Shown | Verdict |
|---|---|---|---|---|
| R2C12 "Which tests failed?" | one run: tests_failed 2, no names | Apple | "Tests failed: 2 failed. [r1]" | REAL under D-130 (a count for a "which") |
| R2C12 | same | Granite | "2 tests failed. [r1]" | REAL (same) |

Fixed in 961e8565b: "The run shows 2 failed, but not which ones [r1]."

## Tally of REAL per model (jobs a–d)

Apple 3 (T14, T05, C12) · Qwen3-1.7B 1 (T15) · Nemotron 4 (T15, T14, T19, T08) · Granite 2 (T14, C12) · Qwen3-4B 2 (T15, T14) · Gemma 1 (T15).
