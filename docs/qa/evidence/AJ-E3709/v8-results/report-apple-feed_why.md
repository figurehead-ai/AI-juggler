# Harness v8 report: Apple on-device Foundation Model via apple_probe.swift (apple, retries sampled)

Criteria: D-149 and the v8 calls (2026-11-26): watches: 0 unsupported claimed, then >= 90 % exact; the feed line (A or B) on only if preferred (60 % of decided pairs, pooled); B's fabrication <= 1 %. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## turns8 / feed_why

Items 50; decided by code 29; model called 21: first attempt 13, after a retry 3, fallback 5.
Shown model items 16; critical reaching the user 0 ({}); critical caught on 5 items ({'contradiction': 15}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| fabrication (D-130 critical), % of shown model half-sentences | <= 1.0 | 0.0 | yes |
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

