# Harness v11 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and the v8 calls (2026-11-26): watches: 0 unsupported claimed, then >= 90 % exact; the feed line (A or B) on only if preferred (60 % of decided pairs, pooled); B's fabrication <= 1 %. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## watch11 / watch7

Items 60; decided by code 32; model called 28: first attempt 16, after a retry 12, fallback 0.
Shown model items 28; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 11 ({'false_unsupported': 7, 'no_watch': 4}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| unsupported claimed (a hard bar, ahead of exact) | == 0 | 0 | yes |
| exact rule matches on supported requests, an unsaid lifetime by ADR-0036's default, % (D-149: >= 90) | >= 90.0 | 65.79 | NO |
| exact rule matches on supported requests, strict, % (reported) | >= None | 57.89 | n/a |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 95.45 | yes |
| watches written by the job (only the user's click adds one) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |

- exact rule matches: 22 of 38
- exact with ADR-0036's default for an unsaid lifetime: 25 of 38
- an unsaid lifetime whose ADR default differs from gold: 3
- unsupported claimed: 0
- nearest offered: 21 of 22
- nearest equal to gold: 19 of 22
- asked (picker): 0
- exact with defaults, English/German: 25 of 38
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 0
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: none

