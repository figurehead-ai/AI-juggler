# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## watch5 / watch

Items 60; decided by code 0; model called 60: first attempt 38, after a retry 10, fallback 12.
Shown model items 48; critical reaching the user 0 ({}); critical caught on 19 items ({'wrong_target': 42}); omissions 29 ({'false_unsupported': 19, 'no_watch': 10}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 4.76 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | None | n/a |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 2044.5 | NO |
| latency p95, ms | <= 4000 | 5749 | NO |

- exact rule matches: 2 of 42
- unsupported claimed: 0
- nearest offered: 0 of 0
- nearest equal to gold: 0 of 0
- asked (picker): 0
- unknown gold condition types: {'wenn die Sitzung fertig ist': 1, 'wenn eine Sitzung mich etwas fragt': 1, 'wenn mein Kontingent unter einen Prozentwert fällt': 1, 'when the session finishes': 4, 'when a session asks me a question': 2, 'when a session fails': 2, 'when my account allowance drops below a percentage': 1, 'when a session fails or asks me something': 1, 'tell me in the app when the billing session finishes': 1, 'when a session starts on that project': 1, 'when the account allowance drops below a percentage': 1, 'when sessions finish': 1, 'when a session is stuck for a number of minutes': 1}

