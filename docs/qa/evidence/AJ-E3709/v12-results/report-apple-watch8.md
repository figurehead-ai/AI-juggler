# Harness v12 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and the v8 calls (2026-11-26): watches: 0 unsupported claimed, then >= 90 % exact; the feed line (A or B) on only if preferred (60 % of decided pairs, pooled); B's fabrication <= 1 %. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## watch12 / watch8

Items 60; decided by code 30; model called 30: first attempt 3, after a retry 27, fallback 0.
Shown model items 30; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 25 ({'false_unsupported': 19, 'no_watch': 6}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| unsupported claimed (a hard bar, ahead of exact) | == 0 | 0 | yes |
| exact rule matches on supported requests, an unsaid lifetime by ADR-0036's default, % (D-149: >= 90) | >= 90.0 | 31.58 | NO |
| exact rule matches on supported requests, strict, % (reported) | >= None | 28.95 | n/a |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 95.45 | yes |
| watches written by the job (only the user's click adds one) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |

- exact rule matches: 11 of 38
- exact with ADR-0036's default for an unsaid lifetime: 12 of 38
- an unsaid lifetime whose ADR default differs from gold: 1
- unsupported claimed: 0
- nearest offered: 21 of 22
- nearest equal to gold: 17 of 22
- asked (picker): 0
- exact with defaults, English/German: 12 of 38
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 0
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: none

