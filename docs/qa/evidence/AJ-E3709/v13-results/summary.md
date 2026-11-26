# v13 watch round (AJ-E3830, D-149), Apple on-device model

Sonnet run; Opus to verify; no verdict recorded. Run 2026-11-26 by a Sonnet agent, per `harness/v13/v13-runbook.md`; set `heldout/v13/watch-v13.json` (60 requests, sha256 b071477ca69b8f53cdd1de0044205e75b827d3b3f19f596a12660fec224e30c8). App drafting binary `watch_round` built at commit 97d7cc978 (not rebuilt; worktree HEAD 97302bbd7 on release/0.5.2). Machine: macOS 27.0.1 (26A434), Apple M5 Ultra (Mac17,15), 96 GB, arm64. Selftest `selftest_v13.py` 115/115 (no model; honest labeller, liar-target, liar-cond, wrong-length: 0 unsupported claimed each; the filler-calling labeller exact 102 of 112, nearest 62 of 63); `--check` clean (60 watch items, 0 unknown gold conditions, 0 unlisted languages); `replay_mismatches`: empty; meta says harness v13, harness_today 2026-11-26. One run only. Nothing committed or staged; no verdict or builtin.json change.

## Bars

| Bar | Score | Observed |
|---|---|---|
| 1. Unsupported claimed == 0 (hard bar, first) | 0 of 22 unsupported requests | holds |
| 2. Exact on supported, ADR-0036 default for an unsaid lifetime, >= 90 % | 12 of 38 = 31.58 % (strict 11 of 38 = 28.95 %) | below 90 % |
| 3. Nearest offered on unsupported, >= 90 % | 18 of 22 = 81.82 % (equal to gold 12 of 22 = 54.55 %) | below 90 % |
| Watches written by the job == 0 | 0 | holds |

Other counters: planted-order mismatches 0; quiet hours broken unasked 0; asked (picker) 0; critical reaching the user 0; omissions 26 (21 false_unsupported, 5 no_watch).

## Per-model bars (Apple, the only model)

| Model | Unsupported claimed | Exact (ADR default / strict) | Nearest offered | Nearest = gold | Watches written | Latency median / p95 (first attempt) |
|---|---|---|---|---|---|---|
| Apple on-device | 0 of 22 | 31.58 % / 28.95 % | 81.82 % (18 of 22) | 54.55 % (12 of 22) | 0 | 634.5 ms / 775 ms |

## How items were decided

- Decided by code alone: 34 of 60 (15 'cover: claimed' drafts or told-backs, 19 'cover: leftover' only; shown kinds below).
- Sent to the model for word labels: 26 (1 call each, all passed on the first attempt, 0 retries, 0 fell back).
- Drafted after labels: 0 of 26.
- Told back after labels: 26 of 26.
- Label answers with the wrong length: 0 (no attempt carries an error; attempts per item all 1).
- Shown kinds: unsupported 39, watch 12, not_a_watch 9.
- Latency: first attempt = item total, median 634.5 ms, p95 775 ms; 26 model calls, wall 18.2 s.

## Unsupported claimed (0)

None.


## Exact misses on supported requests (26 with the ADR default; 1 more only strict)

Format: request -> [kind shown, decided by] what was shown. Gold: target, conditions, lifetime (today = until:2026-11-26T23:59:00Z).

- R13-W02 "Von allen Apps zählt für mich nur Cursor, melde, wenn dort eine neue Session startet." -> [unsupported, via model labels] I can't watch for “zählt für mich nur Cursor”. Nearest: (the form asks) · starts · today.  Gold: app federkiel (da2), starts, today
- R13-W03 "Sag mir jedes Mal Bescheid, wenn der Rechnungsexport fertig ist, bis ich es entferne." -> [unsupported, via model labels] I can't watch for “bis ich es entferne”. Nearest: session Rechnungsexport · finishes · once.  Gold: session Rechnungsexport (bs1), finishes, until_removed
- R13-W06 "Dauerhaft warnen, wenn das Konto Gartenhaus sein Limit erreicht." -> [not_a_watch, via code] (no text shown)  Gold: account Gartenhaus (dc2), hits_limit, until_removed
- R13-W08 "Just for today, let me know if the rate limiter run fails." -> [unsupported, via model labels] I can't watch for “run fails”. Nearest: session Rate limiter · fails · today.  Gold: session Rate limiter (cs3), fails, today
- R13-W10 "Tell me if a new commit lands in quill-docs" -> [unsupported, via model labels] I can't watch for “commit lands in quill-docs”. Nearest: (the form asks) · finishes · today.  Gold: project quill-docs (cp2), commits, today
- R13-W11 "Nur ein einziges Mal: melde dich, sobald der Login-Umbau eine Rückfrage hat." -> [unsupported, via model labels] I can't watch for “einziges Mal: melde dich”. Nearest: session Login-Umbau · asks_you · today.  Gold: session Login-Umbau (bs3), asks_you, once
- R13-W14 "Ping mich an, wenn das Dunkelmodus-Ding durch ist" -> [not_a_watch, via code] (no text shown)  Gold: session Dunkelmodus (bs2), finishes, today
- R13-W19 "yo, shout when the dark mode toggle is done" -> [unsupported, via model labels] I can't watch for “yo”. Nearest: session Dark mode toggle · finishes · once.  Gold: session Dark mode toggle (as2), finishes, today
- R13-W20 "Alles andere ist mir egal, aber wenn der Changelog fertig ist, möchte ich es wissen." -> [unsupported, via model labels] I can't watch for “andere ist mir egal”. Nearest: session Changelog · finishes · once.  Gold: session Changelog (ds5), finishes, today
- R13-W23 "Egal welche Session, mich interessiert nur das Projekt kassenbuch: sag Bescheid, wenn dort etwas fehlschlägt." -> [unsupported, via model labels] I can't watch for “welche Session”. Nearest: (the form asks) · fails · today.  Gold: project kassenbuch (bp1), fails, today
- R13-W27 "Bei allen Konten schaue ich nur auf Dachboden: Bescheid, wenn wieder Platz da ist." -> [unsupported, via model labels] I can't watch for “da ist”. Nearest: account Dachboden · runway_resets · today.  Gold: account Dachboden (bc2), runway_resets, today
- R13-W28 "let me know when the release notes session wants something from me" -> [unsupported, via model labels] I can't watch for “wants something from me”. Nearest: session Release notes · finishes · once.  Gold: session Release notes (cs5), asks_you, today
- R13-W29 "Falls irgendeine Session 60 Minuten nichts tut, bitte melden" -> [unsupported, via model labels] I can't watch for “nichts tut”. Nearest: (the form asks) · finishes · today.  Gold: any session, idle_for 60, today
- R13-W32 "Tell me when Hangar Max hits its limit, yes, even overnight." -> [unsupported, via model labels] I can't watch for “yes, even overnight”. Nearest: account Hangar Max · hits_limit · today.  Gold: account Hangar Max (ac1), hits_limit, today
- R13-W33 "Wenn der Ratenbegrenzer eine Frage hat, sag es mir auch in der Ruhezeit." -> [not_a_watch, via code] (no text shown)  Gold: session Ratenbegrenzer (ds3), asks_you, today
- R13-W34 "Anything weird in ledger-sync? Tell me when any session there fails." -> [unsupported, via model labels] I can't watch for “weird in ledger-sync”. Nearest: (the form asks) · fails · today.  Gold: project ledger-sync (ap1), fails, today
- R13-W39 "Even if it's quiet hours, wake me when the auth migration fails." -> [unsupported, via model labels] I can't watch for “Even if it's quiet hours”. Nearest: session Auth migration · fails · today.  Gold: session Auth migration (as3), fails, today
- R13-W41 "Melde mir das Limit von Werkstatt Pro, selbst in den Ruhezeiten." -> [unsupported, via model labels] I can't watch for “selbst”. Nearest: account Werkstatt Pro · hits_limit · today.  Gold: account Werkstatt Pro (bc1), hits_limit, today
- R13-W43 "Sobald die Testlauf-Session beendet ist, kurz melden, danach nicht mehr." -> [unsupported, via code] I can't watch for “nicht mehr”. Nearest: (the form asks) · ends · today.  Gold: session Testlauf reparieren (bs4), ends, once
- R13-W44 "The first time the flaky tests session ends, tell me, and after that forget about it." -> [unsupported, via model labels] I can't watch for “forget about it”. Nearest: session Flaky tests · ends · once.  Gold: session Flaky tests (as4), ends, once
- R13-W45 "I don't care about individual sessions, just let me know when OpenCode starts a new one." -> [unsupported, via code] I can't watch for “don't care about individual sessions”. Nearest: (the form asks) · starts · today.  Gold: app ca5, starts, today
- R13-W46 "Gib mir Laut, sobald Werkstatt Pro unter 30 Prozent Reichweite rutscht" -> [not_a_watch, via code] (no text shown)  Gold: account Werkstatt Pro (bc1), runway_below 30, today
- R13-W48 "Wenn der Suchindex 15 Minuten lang festhängt, kurz melden" -> [unsupported, via model labels] I can't watch for “Suchindex 15 Minuten”. Nearest: (the form asks) · stuck · today.  Gold: session Suchindex neu bauen (ds1), stuck 15, today
- R13-W49 "Sag mir, wenn die Willkommensmails Hilfe brauchen oder eine Frage haben" -> [unsupported, via model labels] I can't watch for “Hilfe brauchen oder eine Frage haben”. Nearest: session Willkommensmails · asks_you · today.  Gold: session Willkommensmails (bs5), asks_you, today
- R13-W55 "Melde jeden neuen Commit im Projekt federkiel" -> [not_a_watch, via code] (no text shown)  Gold: project federkiel (dp2), commits, today
- R13-W56 "Sag Bescheid, falls die Kartenkacheln-Session abschmiert" -> [unsupported, via model labels] I can't watch for “abschmiert”. Nearest: session Kartenkacheln · finishes · once.  Gold: session Kartenkacheln (ds4), fails, today

Strict-only (lifetime unsaid, drafted, gold today; the ADR default differs from gold):

- R13-W05 "Whatever else is running, tell me when the PDF export session finishes." -> [watch, via code] session PDF export · finishes · once  Gold: session PDF export (cs2), finishes, today | diffs: ["lifetime (unsaid; ADR-0036's default) once != today"]

## Nearest not offered or not equal to gold (10 of 22 unsupported)

- R13-W01 "Warne mich, wenn der Suchindex insgesamt länger als zwei Stunden braucht." -> [unsupported, via model labels] I can't watch for “insgesamt länger als zwei Stunden braucht”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest stuck | nearest shown: finishes
- R13-W13 "Warn me whenever a session ends without leaving processes behind." -> [unsupported, via code] I can't watch for “without leaving processes behind”. Nearest: (the form asks) · ends · today.  Gold: unsupported; nearest leaves_processes | nearest shown: ends
- R13-W26 "Ich will nur bei echten Fragen gestört werden, nicht bei Genehmigungen des Login-Umbaus." -> [not_a_watch, via code] (no text shown)  Gold: unsupported; nearest asks_you | nearest shown: None
- R13-W30 "Sag Bescheid, wenn mehr als vier Sessions gleichzeitig laufen." -> [unsupported, via model labels] I can't watch for “mehr als vier Sessions gleichzeitig laufen”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest starts | nearest shown: finishes
- R13-W36 "Ping me when more than five sessions are running at once." -> [unsupported, via model labels] I can't watch for “more than five sessions are running at once”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest starts | nearest shown: finishes
- R13-W42 "Tell me about every session that finishes unless it belongs to quill-docs." -> [not_a_watch, via code] (no text shown)  Gold: unsupported; nearest finishes | nearest shown: None
- R13-W51 "Warne mich, wenn eine Session endet, ohne Prozesse zurückzulassen." -> [unsupported, via code] I can't watch for “ohne Prozesse zurückzulassen”. Nearest: (the form asks) · ends · today.  Gold: unsupported; nearest leaves_processes | nearest shown: ends
- R13-W58 "Benachrichtige mich bei jedem Ende einer Session, nur nicht bei Cursor." -> [not_a_watch, via code] (no text shown)  Gold: unsupported; nearest ends | nearest shown: None
- R13-W59 "Alert me if the search index rebuild takes longer than two hours in total." -> [unsupported, via model labels] I can't watch for “takes longer than two hours in total”. Nearest: session Search index rebuild · finishes · once.  Gold: unsupported; nearest stuck | nearest shown: finishes
- R13-W60 "Tell me after the flaky tests session has failed three times in a row." -> [not_a_watch, via code] (no text shown)  Gold: unsupported; nearest fails | nearest shown: None

## Files

`v13-apple-watch9.json` (raw), `report-apple-watch9.json` / `.md`, `candidates-apple-watch9.json`, this summary.

## Opus verification (2026-11-26)

I recomputed the bars from the raw results and the set (opened only after the run). They match:

| Model | Unsupported claimed | Exact (ADR default / strict) | Nearest offered (equal to gold) | Latency median / p95 | Watches written |
|---|---|---|---|---|---|
| Apple on-device (the only model on this Mac) | 0 of 22 (PASS) | 12 of 38 = 31.6 % / 28.9 % (FAIL) | 18 of 22 = 81.8 % (12) (FAIL) | 634.5 ms / 775 ms | 0 |

`replay_mismatches` is empty. Watches stay `not_yet`, **pending the maintainer's choice**.

Causes:
1. **The labels can't add a draft.** The word-indexed schema worked: 0 wrong lengths, and every call passed on attempt 1. But Apple's model labels the words code can't place as what they mean: 21 condition, 15 target, 7 lifetime, 7 condition_param, and only 1 filler. Examples: "abschmiert" as condition, "bis ich es entferne" as lifetime, "wants something from me" as condition. Code can't verify an unknown word in those slots, so 26 of 26 were told back. The model's labels are mostly right. What's missing is a verified vocabulary, not the labelling.
2. **Not read as a request (5):** "Dauerhaft warnen", "Ping mich an", "sag es mir auch …", "Gib mir Laut", "Melde jeden neuen Commit".
3. **Vocabulary the set uses and code lacks:**
   - lifetimes: "bis ich es entferne", "nur ein einziges Mal", "danach nicht mehr", "and after that forget about it";
   - quiet hours: "even overnight", "even if it's quiet hours", "selbst in den Ruhezeiten";
   - conditions: "abschmiert", "Hilfe brauchen", "wants something from me", "60 Minuten nichts tut";
   - context: "zählt für mich nur", "schaue ich nur auf", "Alles andere ist mir egal", "Egal welche Session", "yo";
   - "run" as a kind word.
   Two of these ("nichts", "nicht mehr") hit the negation guard by design.
4. **Nearest (81.8 %):** the misses are the told-back items whose clause names no condition code knows, plus requests not read as watch requests.

The sets also got harder by design: v12 and v13 asked for more paraphrase and German variety than v11 did (v11 scored 65.8 % on its set), so the rounds' exact figures aren't directly comparable.
