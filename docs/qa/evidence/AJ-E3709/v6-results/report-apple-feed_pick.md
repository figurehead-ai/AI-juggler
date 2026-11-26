# Harness v6 report: Apple on-device Foundation Model via apple_probe.swift (apple, first attempt greedy, retries sampled, datamark on)

Criteria: D-149 (2026-11-25): watches >= 90 % exact; feed line and digest on only if preferred; D-130's safety bars unchanged. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## turns6 / feed_pick

Items 50; decided by code 21; model called 29: first attempt 25, after a retry 0, fallback 4.
Shown model items 25; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs (D-149: on only if preferred; D-130's value bar) | >= 60.0 | None | n/a |

