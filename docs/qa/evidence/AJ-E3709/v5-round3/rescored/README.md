Harness v5 round 3 (jobs a–d) RE-SCORED under round 4's rules (fixes b46d722a9 through a672806cd, then e13a38ee0, 1782dda5b, 97a1f3780 from the adjudicated blind review), 2026-11-18. No model was re-run, and the blind packets and keys were not rebuilt.

**This is a floor, not a measurement of the fixed harness:**
- A draft the new rules refuse has no recorded retry, so its replay shows the fallback.
- A Chat question that code now routes to the look-up path or the model's pick has no recorded phrasing, so it shows the looked-up value or code's listing.
- A live run retries both.
- The fixes were shaped by this set: Opus read every round-3 critical candidate, the Chat omissions and a sample of fallbacks while fixing. The re-score is therefore not blind, and round 4's fresh set is the measure.

Synthetic held-out only; not checked on real messages. Script flags are candidates, and Opus confirms each one (../verdicts-opus.md).

Command, per model:

    python3 harness/v5/report_v5.py v5-round3/<model>.json --corpus turns=heldout/v5r3/turns-v5r3.json --corpus chat=heldout/v5r3/chat-v5r3.json --md v5-round3/rescored/report-<model>.md --candidates v5-round3/rescored/candidates-<model> > v5-round3/rescored/report-<model>.json

The table shows critical reaching the user (shown model items with a critical candidate, as a % of shown model items) and omissions (as a % of shown items; for Chat and attention, of all items), before -> after:

| Job | Model | Critical reaching (script) | Omissions % | Fallbacks (of calls) |
|---|---|---|---|---|
| feed_line | apple | 1 (10.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 17 -> 17 of 27 |
| feed_line | qwen3-1.7b-q8 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 17 -> 13 of 27 |
| feed_line | nemotron-3-nano-4b | 1 (7.69 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 14 -> 14 of 27 |
| feed_line | granite-4.1-3b | 1 (7.14 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 13 -> 9 of 27 |
| feed_line | qwen3-4b-instruct-2507 | 1 (8.33 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 15 -> 14 of 27 |
| feed_line | gemma-4-e2b | 1 (6.67 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 12 -> 12 of 27 |
| review | apple | 1 (3.33 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 17 -> 12 of 50 |
| review | qwen3-1.7b-q8 | 1 (4.76 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 29 -> 24 of 50 |
| review | nemotron-3-nano-4b | 1 (2.78 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 11 -> 11 of 50 |
| review | granite-4.1-3b | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 20 -> 18 of 50 |
| review | qwen3-4b-instruct-2507 | 0 (0.0 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 12 -> 14 of 50 |
| review | gemma-4-e2b | 1 (4.17 %) -> 0 (0.0 %) | 0.0 -> 0.0 | 25 -> 23 of 50 |
| attention | apple | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 3 -> 2 of 31 |
| attention | qwen3-1.7b-q8 | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 4 -> 4 of 31 |
| attention | nemotron-3-nano-4b | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 4 -> 4 of 31 |
| attention | granite-4.1-3b | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 7 -> 5 of 31 |
| attention | qwen3-4b-instruct-2507 | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 0 -> 0 of 31 |
| attention | gemma-4-e2b | 0 (0.0 %) -> 0 (0.0 %) | 22.0 -> 22.0 | 2 -> 2 of 31 |
| chat | apple | 1 (4.76 %) -> 0 (0.0 %) | 42.0 -> 20.0 | 6 -> 9 of 36 |
| chat | qwen3-1.7b-q8 | 1 (4.76 %) -> 0 (0.0 %) | 42.0 -> 20.0 | 10 -> 13 of 36 |
| chat | nemotron-3-nano-4b | 1 (6.25 %) -> 0 (0.0 %) | 46.0 -> 22.0 | 7 -> 10 of 36 |
| chat | granite-4.1-3b | 1 (5.26 %) -> 0 (0.0 %) | 44.0 -> 22.0 | 10 -> 13 of 36 |
| chat | qwen3-4b-instruct-2507 | 1 (5.56 %) -> 0 (0.0 %) | 42.0 -> 20.0 | 6 -> 9 of 36 |
| chat | gemma-4-e2b | 1 (5.56 %) -> 0 (0.0 %) | 40.0 -> 18.0 | 7 -> 10 of 36 |

How to read it:
- **Every REAL critical of round 3 is refused under the new rules.** R3T48's commit/push claim (five feed lines and three review notes) now falls back to A's excerpt, which carries "git: the changes are not committed". Gemma's Italian R3T27 now shows the code-only review.
- **No new candidate appeared.** Opus read the 13 model texts that newly reach the user (R3T13/14/32/33/35 feed lines and R3T21/22/38 Apple review fields): none is REAL. Two are minor: Apple's risk reason "not run", and Apple's note on R3T38 that copies the agent's question.
- **Attention is unchanged at 11/30 missed.** All 11 are gold-vs-ADR (L1 turns per D-117); see ../verdicts-opus.md.
- **Chat omissions fell to 18–22 %.** Of what remains, 6 items (12 %) are questions in French, Spanish, Dutch or Italian (gold-vs-ADR: Chat's lexicon is EN and DE). Another 4 are known code refusals left unfixed: R3C12 (a "why" held in a record's text), R3C50 (a count of records), R3C48 and R3C49 (lexicon window, kind word). On Nemotron and Granite, R3C10 also remains.
- **Chat fallbacks rose by 3 per model.** That is the floor effect: the newly found records have no recorded phrasing.

- **After the adjudicated blind review** (e13a38ee0 "Failed:" only from facts or a failure of this turn; 1782dda5b claims vs facts on both paths): every adjudicated AI-side critical is refused or gets no call. The review's fallbacks shift by -2 to +2 per model: drafts with a check or done claim are now refused, and A's review lines no longer invent a failure.
- **A alone, the non-AI default:** see A-baseline.md (critical items 0 in every job).
