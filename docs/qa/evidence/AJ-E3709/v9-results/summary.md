# v9 watch round (AJ-E3830, D-149 amendment 2), Apple on-device model

Run 2026-11-26 by a Sonnet agent, per `harness/v9/v9-runbook.md`; set `heldout/v9/watch-v9.json` (60 requests, sha256 6679fde3...). Machine: macOS 27.0.1, M5 Ultra. `replay_mismatches`: empty. Opus verifies before any verdict is recorded; no verdict flipped here.

## Bars

| Bar | Score | Result |
|---|---|---|
| 1. Unsupported claimed == 0 (hard bar, first) | 0 of 28 unsupported requests | PASS |
| 2. Exact on supported, ADR-0036 default for an unsaid lifetime, >= 90 % | 22 of 32 = 68.75 % (strict 15 of 32 = 46.88 %) | FAIL |
| 3. Nearest offered on unsupported, >= 90 % | 23 of 25 due = 92.0 % (equal to gold 20 of 25 = 80 %) | PASS |
| Watches written by the job == 0 | 0 | PASS |
| Unsupported conditions claimed == 0 | 0 | PASS |

Overall: FAIL on bar 2 only. 8 false-unsupported (supported requests told back as unsupported) and 1 supported request answered not_a_watch also count against it.

## Latency

58 of 60 decided by code with no model call. Model called on 2 items (1 passed first attempt, 1 fell back after 3 attempts). First attempt median 1471 ms, p95 2012 ms; item total median 2378 ms, p95 2743 ms. Whole run 5.7 s wall, 4 calls.

## Unsupported claimed

None (0).

## The 5 worst other items

The pattern: a lifetime the user said in words was not read, so the draft falls to "until midnight today"; and one wrong parameter.

1. V9-W28 "out of every session I have open, let me know when the Pattern export one asks me something" -> drafted: "Tell you when "Pattern export" asks you and waits 1 minutes. Until midnight today." (asks_you with an invented 1-minute wait; gold has no param)
2. V9-W01 "Melde dich, wenn Karten mischen etwas fragt, dauerhaft" -> "Ich sage dir Bescheid, wenn "Karten mischen" dich etwas fragt. Bis heute Mitternacht." (gold: until_removed; "dauerhaft" ignored)
3. V9-W32 "warn me when Harbor Team drops below 30% runway, and keep that on permanently" -> "Tell you when "Harbor Team" runs below 30 %. Until midnight today." (gold: until_removed)
4. V9-W03 "page me during quiet hours too when Harbor Team goes below 15 percent, permanently" -> "Tell you when "Harbor Team" runs below 15 %. Until midnight today. Even during quiet hours." (gold: until_removed)
5. V9-W30 "Wenn Cursor sein Limit erreicht, gib mir einmal Bescheid" -> "Ich sage dir Bescheid, wenn "Cursor" ein Nutzungslimit erreicht. Bis heute Mitternacht." (gold: once; "einmal" ignored)

Also: V9-W40, W56, W58 are the same "once" miss; V9-W41 "whatever else is going on, I need to know the moment Till rounding finishes" -> not_a_watch (gold: finishes on a session). 8 supported requests with a broad word beside the named entity were told back as unsupported, e.g. V9-W02 "Bei all meinen Projekten: melde mir bitte, wenn gartenplaner scheitert, für heute" -> "Darauf kann ich nicht achten: "all ... gartenplaner"" (also W18, W21, W24, W25, W36, W37, W55): over-refusal, safe but not exact.

Nearest not offered on 2 unsupported items: V9-W17 "I want to know about every new session, leave out OpenCode" (not_a_watch) and V9-W47 "warn me when cache keys uses more than 1.5 million tokens today" (form opened).

## Files

`v9-apple-watch5.json` (results), `report-apple-watch5.json`, `report-apple-watch5.md`, `candidates-apple-watch5.json`.
