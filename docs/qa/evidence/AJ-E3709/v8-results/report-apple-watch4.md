# Harness v8 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and the v8 calls (2026-11-26): watches: 0 unsupported claimed, then >= 90 % exact; the feed line (A or B) on only if preferred (60 % of decided pairs, pooled); B's fabrication <= 1 %. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## watch8 / watch4

Items 60; decided by code 52; model called 8: first attempt 7, after a retry 0, fallback 1.
Shown model items 7; critical reaching the user 3 ({'unsupported_claimed': 3}); critical caught on 0 items ({}); omissions 3 ({'no_watch': 3}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| unsupported claimed (a hard bar, ahead of exact) | == 0 | 3 | NO |
| exact rule matches on supported requests, an unsaid lifetime by ADR-0036's default, % (D-149: >= 90) | >= 90.0 | 75.0 | NO |
| exact rule matches on supported requests, strict, % (reported) | >= None | 66.67 | n/a |
| unsupported conditions claimed | == 0 | 3 | NO |
| nearest option offered on unsupported requests, % | >= 90.0 | 83.33 | NO |
| watches written by the job (only the user's click adds one) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |

- exact rule matches: 24 of 36
- exact with ADR-0036's default for an unsaid lifetime: 27 of 36
- an unsaid lifetime whose ADR default differs from gold: 3
- unsupported claimed: 3
- nearest offered: 20 of 24
- nearest equal to gold: 16 of 24
- asked (picker): 0
- exact with defaults, English/German: 27 of 36
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 0
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: none

