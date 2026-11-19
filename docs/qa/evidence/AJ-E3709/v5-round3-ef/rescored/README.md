# v5 round 3, (e) and (f), re-scored under the fixed code: a FLOOR, not a verdict

Round 3's recorded outputs (`../<model>.json`) replayed by `report_v5.py` under the fixes (e2a43b5cd, 83d3861af, 07f88e1e6, db623bc74,
0696910bb, 431822801, 8b3a2f909, 028d81f34, fd57356d2, faea92c51, 3edb62885). No model re-run (D-131: model testing ends with round 3).
Verdicts: `../verdicts-opus.md`.

**Why a floor:** retries can't be replayed (a draft a fix now rejects falls back; digest fallbacks rise); Opus read every miss in this
set before fixing, so it is no longer blind; no packets were written. Other languages (R3-W31..54) now get the Watches form by code
(ADR-0033), so they score 0 by design.

| Model | (f) confirmed critical | (f) exact, ADR defaults | en/de | other | (f) strict | (f) false unsupported | (f) quiet asked, off | (e) confirmed critical | (e) omissions % |
|---|---|---|---|---|---|---|---|---|---|
| Apple | 0 | 19/47 (was 7) | 19/27 (was 7) | 0/20 (was 0) | 11 | 3 (was 22) | 0 (was 1) | 0 | 3.3 |
| Gemma 4 E2B | 0 | 19/47 (was 20) | 19/27 (was 12) | 0/20 (was 8) | 9 | 2 (was 19) | 0 (was 2) | 0 | 0.0 |
| Granite 4.1 3B | 0 | 23/47 (was 28) | 23/27 (was 16) | 0/20 (was 12) | 8 | 0 (was 5) | 0 (was 1) | 0 | 0.0 |
| Nemotron 3 Nano 4B | 0 | 23/47 (was 28) | 23/27 (was 18) | 0/20 (was 10) | 8 | 1 (was 12) | 0 (was 2) | 0 | 6.7 |
| Qwen3-1.7B (Q8) | 0 (was 2 REAL) | 22/47 (was 23) | 22/27 (was 14) | 0/20 (was 9) | 9 | 0 (was 3) | 0 (was 1) | 0 (was 2 REAL) | 0.0 |
| Qwen3-4B-Instruct-2507 | 0 (was 1 REAL) | 24/47 (was 28) | 24/27 (was 15) | 0/20 (was 13) | 9 | 0 (was 4) | 0 (was 2) | 0 (was 4 REAL) | 3.3 |

"Was" = round 3 as run (`../report-<model>.json`); the (e) "was" counts are Opus's REAL verdicts on the blind review's false words (the
run's scorer reported 0). Digest fallbacks rise by 0 to 4 per model (false caveats now rejected, no retry to replay).

Left on en/de watches (per model 3 to 8): the model (wrong ref, an unasked condition kept with a parameter, a lifetime it invents or
can't fill) and one gold-vs-code item (R3-W12: gold a session, the request names the project only).
