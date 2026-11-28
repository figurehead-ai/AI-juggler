# v11 watch round (AJ-E3830, D-149 amendment 3), Apple on-device model

Sonnet run; Opus to verify; no verdict recorded. Run 2026-11-26 by a Sonnet agent, per `harness/v11/v11-runbook.md`; set `heldout/v11/watch-v11.json` (60 requests, sha256 208c9c485e00c7478ab1baf29214a42ed53eccfeeff3711167ebf10b9ce78141). App drafting binary `watch_round` built from commit b44d02707 (not rebuilt). Machine: macOS 27.0.1 (26A434), Apple M5 Ultra (Mac17,15), 96 GB. Selftest `selftest_v11.py` 41/41 (no model, honest labeller and liar labeller each: 0 unsupported claimed, exact 53 of 67, nearest 40 of 41); `--check` clean (60 watch items, 0 unknown gold conditions, 0 unlisted languages); `replay_mismatches`: empty. One run only. Nothing committed; no verdict or builtin.json change.

## Bars

| Bar | Score | Observed |
|---|---|---|
| 1. Unsupported claimed == 0 (hard bar, first) | 0 of 22 unsupported requests | holds |
| 2. Exact on supported, ADR-0036 default for an unsaid lifetime, >= 90 % | 25 of 38 = 65.79 % (strict 22 of 38 = 57.89 %) | below 90 % |
| 3. Nearest offered on unsupported, >= 90 % | 21 of 22 = 95.45 % (equal to gold 19 of 22 = 86.36 %) | holds |
| Watches written by the job == 0 | 0 | holds |

Other counters: planted-order mismatches 0; quiet hours broken unasked 0; asked (picker) 0; critical reaching the user 0; omissions 11 (7 false_unsupported, 4 no_watch).

## Latency

32 of 60 decided by code; model called on 28 (labelling): 16 passed first attempt, 12 after a retry, 0 fell back. First attempt median 2105 ms, p95 3598 ms; item total median 3459 ms, p95 10107 ms. Whole run 128.9 s wall, 52 model calls. Helper CPU 3.19 s; probe RSS 131 to 149 MB.

## Unsupported claimed (0)

None.

## Exact misses on supported requests (13 with the ADR default; 3 more only strict)

Shown as unsupported though gold is supported (false_unsupported, 7):
- R11-W07 "Ping me if the Flag parsing session sits idle for 45 minutes" -> "I can't watch for 'sits'. Nearest: session Flag parsing - idle_for - today." Gold: session s-pine, idle_for 45, today.
- R11-W12 "Egal was sonst läuft, informiere mich, wenn in pilzfinder eine neue Sitzung startet" -> "I can't watch for 'Egal was sonst läuft'. Nearest: (the form asks) - starts - today." Gold: project p-pilz, starts, today.
- R11-W19 "Von allen meinen Sitzungen: sag mir nur Bescheid, wenn die Sensordaten-Sitzung fehlschlägt" -> "I can't watch for 'nur Bescheid, wenn die Sensordaten-Sitzung'. Nearest: (the form asks) - fails - today." Gold: session s-wetter, fails, today.
- R11-W24 "Whatever else is running, let me know when the Axis labels session ends" -> "I can't watch for 'Whatever else is running'. Nearest: session Axis labels - ends - once." Gold: session s-tide, ends, today.
- R11-W28 "Nur einmal: melde dich, wenn die Fahrplan-Import-Sitzung fertig ist" -> "I can't watch for 'Fahrplan-Import-Sitzung'. Nearest: (the form asks) - finishes - today." Gold: session s-bahn, finishes, once.
- R11-W40 "Permanently watch for new sessions starting in quiz-engine" -> "I can't watch for 'new sessions starting'. Nearest: (the form asks) - finishes - today." Gold: project p-quiz, starts, until_removed.
- R11-W56 "Sag mir, wenn die Terminvergabe-Sitzung fertig ist" -> "I can't watch for 'Terminvergabe-Sitzung'. Nearest: (the form asks) - finishes - today." Gold: session s-dach, finishes, today.

Not read as a watch request (not_a_watch, 4):
- R11-W38 "Keep telling me whenever any session in sage-billing fails, permanently". Gold: project p-sage, fails, until_removed.
- R11-W43 "Tell me about any new session that starts anywhere". Gold: any, starts, today.
- R11-W49 "Ich möchte wissen, wenn in lagerhaus eine Sitzung endet". Gold: project p-lager, ends, today.
- R11-W50 "Across everything I run, I only care when the Timezone fix session finishes". Gold: session s-orbit, finishes, today.

Drafted a watch with the wrong rule (2):
- R11-W04 "Tell me when Cursor crashes and gets started again" -> "watch app Cursor - fails, starts - today". Gold: app app-cur, restarted, today.
- R11-W30 "Notify me when a sage-billing session ends and leaves processes running" -> "watch project sage-billing - ends, leaves_processes - today". Gold: project p-sage, leaves_processes only, today.

Strict-only (lifetime unsaid, drafted "once", gold today; 3 items where ADR default differs from gold):
- R11-W20 "Tell me when the Timezone fix session finishes, but hold it until quiet hours end" -> "session Timezone fix - finishes - once".
- R11-W47 "Let me know when the Search index session finishes" -> "session Search index - finishes - once".
- R11-W54 "Bescheid bitte auch nachts, wenn der Bestandsabgleich fertig ist" -> "session Bestandsabgleich - finishes - once - even during quiet hours".

## Nearest not offered or not equal to gold (3 of 22 unsupported)

Not offered (1): R11-W55 "Ich will wissen, wann Sitzungen fertig sind, nicht von Cursor" -> not read as a watch request (no text shown). Gold unsupported, nearest "finishes".

Offered but different from gold (2):
- R11-W23 "Melde dich, wenn mehr als vier Sitzungen gleichzeitig laufen" -> nearest "finishes". Gold nearest "starts".
- R11-W58 "Warn me when the Image resizer's logs go over 500 MB" -> "I can't watch for 'logs go over 500 MB'. Nearest: session Image resizer - finishes - once." Gold nearest "stuck".

Note for Opus (observation, not verified): all 11 omissions come from the model-labelling path or the not_a_watch path (code placed no word it should have); R11-W40's nearest "finishes" for a "starting" request and R11-W24/W28/W56 are labels naming a session word or compound as the unplaced span. The harness meta still prints `harness: v6` and `harness_today: 2026-11-18` (gold lifetimes use 2026-11-18).

## Files

`v11-apple-watch7.json` (raw), `report-apple-watch7.json` / `.md`, `candidates-apple-watch7.json`.

## Opus verification (2026-11-26)

I recomputed the bars from the raw results and the set, which was opened only after the run. They match:
- Unsupported claimed: 0. No gold-unsupported item is a watch.
- Exact: 25 of 38 with the ADR default, 22 strict.
- Nearest offered: 21 of 22. Every unsupported answer carries a nearest.
- Watches written: 0. `replay_mismatches` is empty.

**Verdict: FAIL on the exact bar (65.8 % against 90 %). Watches stay `not_yet`.** The hard bar and the nearest bar pass.

Every miss is code under-reading a supported request. None is the model making something supported. The causes:
1. **German "-Sitzung" compounds** (W19, W28, W56). "Terminvergabe-Sitzung" is one word, so the name "Terminvergabe" is never matched whole.
2. **Ways of asking that aren't recognised** (W38, W43, W49, W50, W55). "keep telling me", "tell me about any new …", "ich möchte/will wissen, wenn/wann", "I only care when". These were read as not a request.
3. **Context clauses that change nothing** (W12, W24). "whatever else is running", "egal was sonst läuft". They are told back by design.
4. **Paraphrases** (W07, W40). "sits idle", "new sessions starting".
5. **Composite conditions** (W04, W30). "crashes and gets started again" should be restarted, not fails plus starts. "ends and leaves processes running" should be leaves_processes alone.

v12 is not started; the coordinator decides.
