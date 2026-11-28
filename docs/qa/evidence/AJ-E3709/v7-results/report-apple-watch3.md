# Harness v7 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and its amendment (2026-11-26): watches >= 90 % exact; the feed line on only if preferred (60 % of decided pairs, pooled); D-130's safety bars unchanged. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## watch7 / watch3

Items 60; decided by code 52; model called 8: first attempt 5, after a retry 0, fallback 3.
Shown model items 5; critical reaching the user 5 ({'unsupported_claimed': 5}); critical caught on 0 items ({}); omissions 4 ({'no_watch': 4}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, an unsaid lifetime by ADR-0036's default, % (D-149: >= 90) | >= 90.0 | 77.08 | NO |
| exact rule matches on supported requests, strict, % (reported) | >= None | 56.25 | n/a |
| unsupported conditions claimed | == 0 | 5 | NO |
| nearest option offered on unsupported requests, % | >= 90.0 | 30.0 | NO |
| watches written by the job (only the user's click adds one) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |

- exact rule matches: 27 of 48
- exact with ADR-0036's default for an unsaid lifetime: 37 of 48
- an unsaid lifetime whose ADR default differs from gold: 10
- unsupported claimed: 5
- nearest offered: 3 of 10
- nearest equal to gold: 3 of 10
- asked (picker): 0
- exact with defaults, English/German: 37 of 48
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 0
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 2
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: none

