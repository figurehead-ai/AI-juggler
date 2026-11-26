# v10 watch round (AJ-E3830, D-149 amendment 2), Apple on-device model

Run 2026-11-26 by a Sonnet agent, per `harness/v10/v10-runbook.md`; set `heldout/v10/watch-v10.json` (60 requests, sha256 3f862f0a...). Machine: macOS 27.0.1, M5 Ultra. Selftests 31/31 (v10), 27/27, 45/45, 62/62, 70/70 (v9..v6); `--check` clean; `replay_mismatches`: empty. Opus verifies before any verdict is recorded; no verdict flipped here.

## Bars

| Bar | Score | Result |
|---|---|---|
| 1. Unsupported claimed == 0 (hard bar, first) | 3 of 18 unsupported requests | FAIL |
| 2. Exact on supported, ADR-0036 default for an unsaid lifetime, >= 90 % | 36 of 42 = 85.71 % (strict 32 of 42 = 76.19 %) | FAIL |
| 3. Nearest offered on unsupported, >= 90 % | 13 of 18 = 72.22 % (equal to gold 11 of 18) | FAIL |
| Watches written by the job == 0 | 0 | PASS |

Overall: FAIL on bars 1, 2, 3.

## Latency

57 of 60 decided by code; model called on 3 (2 passed first attempt, 1 fell back). First attempt median 1070 ms, p95 1445 ms; item total median 1070 ms, p95 3362 ms. Whole run 6.2 s wall, 5 calls.

## Unsupported claimed (3)

1. R10-W03 "Warne mich bei Fehlern, aber nur bei solchen, die nicht von der Wetterstation kommen" -> drafted: "Ich sage dir Bescheid, wenn "Wetterstation-Sync" fehlschlägt. Bis heute Mitternacht." (target Wetterstation-Sync session, fails, today; the exclusion became the target). Gold: unsupported, nearest "wenn eine Sitzung fehlschlägt".
2. R10-W40 "Tell me when the payment flow finishes unless it ran for under an hour" -> "Tell you when "Payment flow" finishes. Once." (the "unless" condition dropped). Gold: unsupported, nearest "when the checkout-api session finishes".
3. R10-W60 "Ping me if billing-sync has made 8 failed retries in a row" -> "Tell you when "billing-sync" fails. Until midnight today." (project target, count of 8 retries dropped). Gold: unsupported, nearest "when billing-sync fails".

## Exact misses on supported requests (6 with the ADR default; 4 more only strict) 

With the ADR default (unsaid lifetime -> today), the 6 misses:
- R10-W20 "From now on, tell me when the ingest worker asks me anything" -> unsupported: "I can't watch for "anything ... Ingest worker". I can tell you when a session asks you." Gold: session s-ing, asks, until_removed.
- R10-W59 "Every time the landing page copy session finishes, let me know" -> unsupported: "I can't watch for "Every ... Landing page copy". I can tell you when a session finishes." Gold: session s-lan, finishes, until_removed.
- R10-W27 "Tell me if any session looks stuck for half an hour" -> form: "I couldn't set that up from your words. New watch... opens the form." Gold: any session, stuck 30, today.
- R10-W05 "Wake me up, even during quiet hours, if checkout-api fails" -> "Tell you when "checkout-api" fails. Until midnight today. Even during quiet hours." (target project p-checko0). Gold: session s-chk, fails, even in quiet hours.
- R10-W21 "Of all my sessions, ping me when checkout-api fails" -> "Tell you when "checkout-api" fails. Until midnight today." (target project p-checko0). Gold: session s-chk, fails.
- R10-W54 "Any time the billing-sync session asks me something, tell me" -> "Tell you when "billing-sync" asks you. Until you remove it." (target project p-billin1, lifetime always). Gold: session s-bill, asks, default today.

Strict-only (lifetime unsaid, drafted "Once", gold today; the ADR default counts these as matching only where code drafts today; four items differ from default): R10-W04 "Tell me when the embedding refresh is done" -> "Tell you when "Embedding refresh" finishes. Once."; R10-W07 "Tell me when the storybook cleanup session closes" -> "... ends. Once."; R10-W43 "Across all my apps, tell me when the scan deskew session finishes" -> "... finishes. Once."; R10-W46 "Tell me when the scan deskew session finishes, but hold it until quiet hours end" -> "... finishes. Once." In all four the gold lifetime is until midnight today and the request says nothing of a lifetime, yet the draft says "Once".

Note for Opus (observation, not verified): four unsaid lifetimes were drafted "Once", and three of the exact misses are project-vs-session target confusion. Over-refusals R10-W20 and R10-W59 (broad words "anything", "Every") are the v9 pattern again.

## Nearest not offered (5 of 18 unsupported)

The 3 unsupported-claimed items above (R10-W03, W40, W60) drafted a watch instead; plus R10-W11 "Tell me when the dashboard filters session logs grow past 500 megabytes" and R10-W58 "Notify me about failures, but only the ones that aren't timeouts" opened the form ("I couldn't set that up from your words").

## Files

`v10-apple-watch6.json` (raw), `report-apple-watch6.json` / `.md`, `candidates-apple-watch6.json`.
