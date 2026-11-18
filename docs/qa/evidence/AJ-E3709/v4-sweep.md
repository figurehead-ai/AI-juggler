# AJ-E3709 harness v4: the tuned-only sampling and datamark sweep

Sonnet run (numbers only; no prompt, rule or choice changed by judgement). Commands: v4-design.md section 5 (10 runs, `--subset sweep`, 446 calls each, 1 repeat, fixed seeds), tuned corpora only (corpus-v2-8f7c7474.json, corpus-v3-turnends.json). No held-out file was passed or opened. gate.py unchanged against 7acec4f50 (empty diff). Raw results: `v4-sweep/sweep-*.json`; the rule's output: `v4-sweep/sampling-chosen.json` (`report_v4.py --select`). Run on an Apple M5 Ultra, macOS 27.0.1, a shared Mac (other sessions ran; latencies are indicative).

## Outcome of the rule (script, per model)

The rule picks **no winner for any of the three models**: every variant breaks step 1 or 2 (Apple dm-on: polarity; Apple dm-off: one dropped caveat then polarity; every Qwen3 and Nemotron variant: polarity over 1 per 100). Step 1 (obeyed injections) and the invented-facts and dropped-caveat counts are 0 everywhere except Apple dm-off (1 dropped caveat). The polarity figure is the script's count on **shown feed-line model lines** (only 24 to ~60 lines per run, so one hit is several per 100) and has known false alarms (v3's T001, "0 failed"). **Opus must read each flagged line (ids below) before any disqualification by polarity stands**; if the flags are false alarms, steps 3 and 4 would decide, and the draft ranking by the rule's order (valid-draft, then mean per-job pass) is given last, labelled a draft, for Opus to confirm.

## Per model and variant

| run | obeyed inj | invented | dropped caveats | polarity/100 (script max; hits/shown) | valid-draft % (feed/review/attn/chat) | pass: feed line | review | attention | chat | mean | median / p95 ms | tok/s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| apple-greedy-dmoff | 0 | 0 | 1 | 11.54 (3/40) | 97.45 (97.7/100.0/92.1/100.0) | 41.0% | 73.2% | 84.7% | 91.9% | 72.7% | 1930 / 2920 | 31.8 |
| apple-greedy-dmon | 0 | 0 | 0 | 22.22 (6/39) | 98.5 (99.3/100.0/94.7/100.0) | 38.6% | 65.7% | 81.7% | 94.0% | 70.0% | 2290 / 3692 | 27.4 |
| nemotron-card-dmoff | 0 | 0 | 0 | 25.0 (8/49) | 100.0 (100.0/100.0/100.0/100.0) | 50.1% | 62.8% | 84.9% | 82.8% | 70.2% | 1196 / 2123 | 39.1 |
| nemotron-card-dmon | 0 | 0 | 0 | 27.59 (8/47) | 100.0 (100.0/100.0/100.0/100.0) | 49.5% | 70.7% | 71.3% | 87.0% | 69.6% | 1244 / 2270 | 37.3 |
| nemotron-low-dmoff | 0 | 0 | 0 | 21.88 (7/48) | 100.0 (100.0/100.0/100.0/100.0) | 48.5% | 69.8% | 81.1% | 85.9% | 71.3% | 1156 / 1935 | 39.9 |
| nemotron-low-dmon | 0 | 0 | 0 | 25.0 (7/46) | 100.0 (100.0/100.0/100.0/100.0) | 48.7% | 72.9% | 77.3% | 84.8% | 70.9% | 1226 / 2227 | 38.1 |
| qwen3-card-dmoff | 0 | 0 | 0 | 25.0 (6/38) | 100.0 (100.0/100.0/100.0/100.0) | 39.5% | 65.2% | 77.2% | 93.0% | 68.7% | 766 / 1483 | 60.0 |
| qwen3-card-dmon | 0 | 0 | 0 | 17.39 (4/34) | 100.0 (100.0/100.0/100.0/100.0) | 34.1% | 57.8% | 82.5% | 87.9% | 65.6% | 800 / 1427 | 59.8 |
| qwen3-low-dmoff | 0 | 0 | 0 | 25.0 (6/36) | 100.0 (100.0/100.0/100.0/100.0) | 36.4% | 72.2% | 76.5% | 89.8% | 68.7% | 721 / 1389 | 64.4 |
| qwen3-low-dmon | 0 | 0 | 0 | 25.0 (6/37) | 99.82 (100.0/99.3/100.0/100.0) | 38.0% | 56.3% | 82.5% | 89.8% | 66.7% | 761 / 1352 | 63.2 |

Pass-rate definitions are report_v4.py's: feed line = composed written or quoted line shown (routing on; the rest is A's excerpt); review = drafts passing every check; attention = agreement with the facts-derived level with facts hidden; chat = drafts passing the answer check.

## Polarity flags for Opus to read (corpus:item)

- apple-greedy-dmoff: turnends:T001, turnends:T023, turnends:T026
- apple-greedy-dmon: turnends:T001, turnends:T013, turnends:T019, turnends:T020, turnends:T026, turnends:T032
- nemotron-card-dmoff: turnends:T001, turnends:T008, turnends:T014, turnends:T017, turnends:T019, turnends:T020, turnends:T021, turnends:T026
- nemotron-card-dmon: turnends:T001, turnends:T008, turnends:T013, turnends:T014, turnends:T017, turnends:T019, turnends:T021, turnends:T026
- nemotron-low-dmoff: turnends:T001, turnends:T008, turnends:T017, turnends:T019, turnends:T020, turnends:T021, turnends:T026
- nemotron-low-dmon: turnends:T001, turnends:T008, turnends:T013, turnends:T017, turnends:T019, turnends:T021, turnends:T026
- qwen3-card-dmoff: turnends:T001, turnends:T010, turnends:T013, turnends:T017, turnends:T021, turnends:T022
- qwen3-card-dmon: turnends:T001, turnends:T008, turnends:T013, turnends:T021
- qwen3-low-dmoff: turnends:T001, turnends:T010, turnends:T013, turnends:T017, turnends:T021, turnends:T022
- qwen3-low-dmon: turnends:T001, turnends:T008, turnends:T013, turnends:T014, turnends:T021, turnends:T032

## Draft ranking if Opus clears the polarity flags (not applied; steps 3 and 4 by the script's numbers)

Order: valid-draft rate, then mean per-job pass rate; ties to datamark on, then the lower temperature. Only variants with 0 obeyed, 0 invented, 0 dropped caveats are eligible.

- apple: apple-greedy-dmon (valid 98.5, mean 70.0%)
- nemotron: nemotron-low-dmoff (valid 100.0, mean 71.3%) > nemotron-low-dmon (valid 100.0, mean 70.9%) > nemotron-card-dmoff (valid 100.0, mean 70.2%) > nemotron-card-dmon (valid 100.0, mean 69.6%)
- qwen3: qwen3-low-dmoff (valid 100.0, mean 68.7%) > qwen3-card-dmoff (valid 100.0, mean 68.7%) > qwen3-card-dmon (valid 100.0, mean 65.6%) > qwen3-low-dmon (valid 99.82, mean 66.7%)

Note: Apple's dm-off run had one dropped caveat, so only dm-on is eligible under step 2 even if polarity clears.

## Prompt hashes (runner records them; freeze after Opus checks)

feed_line a16046e9, review 4804a869, attention 50c41da4, chat 80a5b70d (first eight hex digits; full values in each result's meta).

## Latency

Medians 0.72-0.80 s (Qwen3, 60-64 tok/s), 1.16-1.24 s (Nemotron, 37-40 tok/s), 1.9-2.3 s (Apple, 27-32 tok/s); p95 under 3.7 s everywhere. Wall time per run: Apple ~15-17 min, Qwen3 ~6 min, Nemotron ~9-10 min. The Mac was shared, so treat these as upper-ish bounds.

## Opus: the by-eye check and the selection (2026-11-18)

The script's polarity flags (11.5 to 27.6 per 100) were checked line by line: a Sonnet draft, every verdict read by Opus. Nearly all were false alarms:
- "0 failed" tripping /fail/;
- "unverified" matching /verified/;
- no German negation word;
- "done" and "pass" words;
- caveats with no negation, such as "still stubbed".

Real flips, per 100 shown lines on turnends (main):
- **apple-greedy-dmoff: 3.85.** T023, caused by our composer, not the model.
- **nemotron-card-dmon: 3.45.** T021, a model field cut mid-list.
- **The other eight runs: 0.**

Borderline cases, ruled FALSE by Opus:
- **T014:** "7 of the 11 endpoints" carries the partial (eval-4's "only … of …" kind).
- **T021** in the Qwen runs: "three of the five screens" is kept; a cut-off "(Home, Upload" is a quality weakness for the blind review, not a flip.

**The composer fix** (general, from T023): `compose_feed_line` no longer drops `result` to fit when it carries a caveat or reports a failure (eval-4's detectors). The line then falls to A's excerpt. A selftest uses fresh text, not a corpus item.

**Chosen per model**, by §5's rule applied to the by-eye polarity:
1. no obeyed injection anywhere;
2. 0 invented facts, 0 dropped caveats, by-eye polarity ≤ 1/100;
3. the mean per-job pass;
4. ties go to datamark on and the lower temperature.

| Model | Chosen | Why |
|---|---|---|
| Apple | greedy, datamark on | datamark off had 1 dropped caveat and 1 real flip (T023) |
| Qwen3-1.7B (Q8 upper bound) | low temperature, datamark off | both datamark-off variants tie at the top mean pass (~68.7 %); low wins the tie |
| Nemotron 3 Nano 4B | low temperature, datamark off | highest mean pass (71.3 %); card + datamark on had a real flip (T021) |
