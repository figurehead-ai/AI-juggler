# v7 round on Apple's on-device model (2026-11-26, Sonnet 5.5 run; Opus to verify)

Runbook `../harness/v7/v7-runbook.md`, followed unchanged: probe built with the system swiftc, `runner_v7.py --check` clean (watch 60 items, turns 50 items), one job at a time, each set judged once. `replay_mismatches` empty on both. Nothing written to the app; no verdict flipped.

| Job | Score | Bar (D-149) | Verdict | Latency (item total, median / p95) |
|---|---|---|---|---|
| watch3 (AJ-E3830) | 77.1 % exact with ADR-0036 lifetime default (37/48); strict 56.3 % (27/48) | >= 90 % | FAIL | 574 ms / 1552 ms (code decided 52 of 60; the model ran on 8) |
| feed_pick2 (AJ-E3831) | AI preferred 33.3 % of decided pairs (8 AI, 16 other, 2 same of 26) | >= 60 % | FAIL | 1109 ms / 3283 ms (code decided 20 of 50; the model ran on 30; 4 fell back) |

Other watch3 rows: unsupported claimed 5 (bar 0, FAIL); nearest option offered on unsupported 3 of 10 = 30 % (bar 90 %, FAIL); watches written by the job 0 (holds). Feed safety rows (D-130): critical reaching the user 0, omissions 0, hidden questions or failures 0, model text on a turn that needs you 0 (all hold). The blind pairs were judged by one reviewer (Sonnet), not pooled with a second judge, and Opus's re-judge has not been done.

## Worst 5, watch3 (script flags: candidates, Opus confirms by eye)
1. V7-W05 "tell me when all sessions finish, except the retry logic one" -> drafted "Tell you when 'any session' finishes. Until midnight today." (exclusion lost, unsupported claimed; also the lifetime was invented).
2. V7-W19 "for every project except otter-billing, tell me if it fails" -> "Tell you when 'any session' fails. Until midnight today." (same).
3. V7-W43 "tell me when the webhook retries session uses more than 50k tokens" -> "Tell you when 'Webhook retries' hits a usage limit." (a token count claimed as a usage limit; nearest offered, but presented as supported).
4. V7-W14 "alert me if anything sits idle for 40 minutes, from now on" -> stuck 40 where gold is idle_for 40 (the idle/stuck swap v7 meant to remove).
5. V7-W23 "tell me when reed-sessions finishes" -> project p-reed where gold is project p-reed-sessions (wrong target). Also W15, W26, W41, W57: the form shown instead of a draft on supported requests; 10 lifetime defaults differ from gold (e.g. W13 "always tell me if anything fails" drafted today).

## Worst 5, feed_pick2 (pairs the blind reviewer preferred the plain side; the key was opened after the calls)
1. Pair 016 (V7T16, paused schema upgrade, five steps, two done): AI line "The new column and the backfill script, 5 passed" (invents a test count that is the step count); plain line "I got the first two of the five steps of the ibex-sync schema upgrade in..." preferred.
2. Pair 011 (V7T11, lock-free queue dropped): AI line "I dropped the change with git checkout, 6 passed" (loses why: benchmarks 12 percent worse); plain preferred.
3. Pair 008 (V7T08): AI line "Added aria-required on them, 31 passed" (fragment, dangling "them"); plain preferred.
4. Pair 004 (V7T04): AI line "Regenerated the fixtures; tests were not run" (a side detail over the date-parser replacement); plain preferred.
5. Pair 018 (V7T18, read-only explanation): AI line "Explained how the egret-scheduler picks the next job; tests were not run" (drops the answer, adds a test note for a question); plain preferred. Also 002, 010, 014, 015, 019, 020, 024: tests-not-run or fragment picks over the point.
Opus's re-judge of the same pairs is pending; the packet, review and key are in `blind-review/` and `review-feed_pick2.json`.

Files: `v7-apple-*.json` (raw), `report-apple-*.{md,json}`, `candidates/`, `blind-review/` (packet and key), `review-feed_pick2.json`.
