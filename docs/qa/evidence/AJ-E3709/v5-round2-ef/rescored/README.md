# v5 round 2, (e) and (f), re-scored under the fixed code: a FLOOR, not a verdict

These are round 2's recorded model outputs (`../<model>.json`), replayed by `report_v5.py` under the fixed job code and scorer
(52408c2d6, 92d183827, 5edd62f0b). No model was re-run. Opus's verdicts on what reached the user in the run are in `../verdicts-opus.md`.

**Why it is a floor and not a pass:**
- **Retries can't be replayed.** Where a fix changes an attempt's errors, the retry the model would then have written doesn't exist.
  Every such item falls back (`replay_mismatches`: 15 to 22 per model across both jobs), so fallbacks are overstated.
- **The fixes were shaped by this set.** Opus read every miss in it. A clean verdict needs a fresh blind set, written without seeing
  these fixes, run on every model.
- **No packets.** This re-score wrote none (`--packets` not passed: report_v5 still writes the key into the packet folder; see the hand-back).

| Model | (e) critical reaching | (e) omissions | (e) fallbacks | (e) items without fallback % | (e) quiet flagged (noise) | (f) exact strict | (f) exact, ADR defaults | (f) false unsupported | (f) nearest offered (= gold) | (f) unsupported claimed |
|---|---|---|---|---|---|---|---|---|---|---|
| Apple | 0 (was 2) | 4 (13.3 %) (was 6) | 7 (was 12) | 76.7 | 2 | 13/41 (was 8) | 13/41 (was 8) | 15 (was 22) | 13/13 (9) (was 8, 4) | 0 |
| Gemma 4 E2B | 0 (was 3) | 3 (10.0 %) (was 7) | 4 (was 7) | 82.7 | 3 | 15/41 (was 4) | 23/41 (was 10) | 12 (was 25) | 13/13 (10) (was 6, 3) | 0 |
| Granite 4.1 3B | 0 (was 1) | 3 (10.0 %) (was 6) | 8 (was 9) | 81.6 | 1 | 18/41 (was 13) | 31/41 (was 29) | 2 (was 6) | 13/13 (10) (was 9, 4) | 0 (was 1) |
| Nemotron 3 Nano 4B | 0 (was 1) | 5 (16.7 %) (was 8) | 3 (was 4) | 84.8 | 1 | 15/41 (was 8) | 29/41 (was 20) | 5 (was 12) | 13/13 (8) (was 10, 5) | 0 |
| Qwen3-1.7B (Q8) | 0 (was 1) | 4 (13.3 %) (was 7) | 7 (was 11) | 78.8 | 1 | 12/41 (was 8) | 20/41 (was 15) | 4 (was 9) | 12/13 (9) (was 3, 2) | 0 |
| Qwen3-4B-Instruct-2507 | 0 (was 1) | 3 (10.0 %) (was 7) | 1 (was 2) | 96.2 | 1 | 21/41 (was 13) | 36/41 (was 25) | 1 (was 8) | 13/13 (10) (was 8, 7) | 0 |

"Was" is round 2 as scored at the run (`../report-<model>.json`). Asked (picker) stays 0 for every model. The `(e)` critical count is 0
because `quiet_flagged` is now noise: all 9 were true lines (see the verdicts).

**What still keeps each bar from holding (by-eye classes):**

- **(e) omissions left:** 12 of 22 are R2-D07 and R2-D21. Those agents write French and Dutch, so by ADR-0036 §1.5 rule 8 they get the
  rollup, and its excerpt drops their caveat. The rest are the model's: the rejected rows of an earlier turn (R2-D09), and empty or stale
  details (R2-D11, D13, D26).
- **(f) strict misses left:**
  - gold's lifetime "once" where ADR-0036's default for an unsaid lifetime is "today": 45 + 13;
  - a lifetime the draft can't express ("through Sunday", "Bis Freitag", "Hasta el viernes"): 15;
  - French, Spanish or Dutch requests the en/de cue net and covered() can't read: 33;
  - the model (wrong target, condition or lifetime): 46.

Files: `report-<model>.{json,md}` and `candidates-<model>/` (per job), from `report_v5.py RESULTS --corpus digest5r2=… --corpus
watch5r2=… --md … --candidates …`.
