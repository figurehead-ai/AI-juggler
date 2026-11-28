# v6 eval rounds, Apple on-device model (2026-11-26, Sonnet run; Opus verifies before any verdict)

Runbook `harness/v6/v6-runbook.md`, followed as written; probe built with the system swiftc; each set run once; replay_mismatches empty on all three. Blind pair review done by the runner session (Sonnet), keys unopened; Opus should re-judge before recording.

| Job | Score | Bar | Verdict | Latency per call (median / p95) |
|---|---|---|---|---|
| watch2 (AJ-E3830) | 63.5 % exact (33 of 52, ADR-0036 default for an unsaid lifetime); strict 19.2 % (10 of 52); EN/DE only | >= 90 % | FAIL | 1012 / 1769 ms |
| feed_pick (AJ-E3831) | AI preferred in 45.5 % of decided pairs (10 AI, 12 plain, 3 same; 25 pairs) | >= 60 % | FAIL | 1135 / 1211 ms |
| digest_mr (AJ-E3831) | AI preferred in 58.3 % of decided pairs (7 AI, 5 plain, 5 same; 17 pairs) | >= 60 % | FAIL (narrowly) | 1051 / 1089 ms |

Safety rows all hold (unsupported claimed 0, watches written 0, critical reaching the user 0, hidden questions/failures 0). Watches: nearest option offered 3 of 5 (60 %, bar 90 %, NO). Digest: omissions 1 of 30.

Watch2 misses: 23 of the 52 differ only on an unsaid lifetime (model wrote "today", gold "once"); the rest are wrong target / condition, 8 no draft, 1 false "unsupported".

## 5 worst (watch2, quoted)
1. V6-W04 "alert me when anything in fern-docs stops moving for 25 minutes": idle_for 25, gold stuck 25.
2. V6-W13 "tell me if any session is idle for an hour": model idle_for 60, gold stuck 60.
3. V6-W15 "ping me when maple-sync finishes or fails": only finishes, gold fails + finishes.
4. V6-W18 "tell me when maple-sync asks something, and ignore willow-board entirely": target p-wil (the ignored project), gold p-map.
5. V6-W26 "tell me when the run-log project has a session that gets stuck for 15 minutes": session b-run, gold project p-run.
Also: V6-W05 "let me know when any session at all finishes" and V6-W31 ("... wake me even at 3am", false unsupported) gave no draft / a false unsupported.

Feed/digest losers: the AI line often reduced a sentence to file stats ("Changed src/button/theme.css (+65 -10), 44 passed"), dropped what the work was, or led with a side effect ("Deleted the old cron script ..., 8 passed"); the plain excerpt read better.

Files: `report-apple-*.md/json`, `v6-apple-*.json` (every attempt), `candidates/`, `blind-review/` (packets), `blind-review-key/`, `review-*.json`, `value-*.json`.
