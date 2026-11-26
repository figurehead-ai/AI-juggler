# Harness v7 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and its amendment (2026-11-26): watches >= 90 % exact; the feed line on only if preferred (60 % of decided pairs, pooled); D-130's safety bars unchanged. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## turns7 / feed_pick2

Items 50; decided by code 20; model called 30: first attempt 26, after a retry 0, fallback 4.
Shown model items 26; critical reaching the user 0 ({}); critical caught on 3 items ({'outcome_or_caveat': 8}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

