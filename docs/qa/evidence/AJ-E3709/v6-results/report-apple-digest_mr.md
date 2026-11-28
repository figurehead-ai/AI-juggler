# Harness v6 report: Apple on-device Foundation Model via apple_probe.swift (apple, first attempt greedy, retries sampled, datamark on)

Criteria: D-149 (2026-11-25): watches >= 90 % exact; feed line and digest on only if preferred; D-130's safety bars unchanged. Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest6 / digest_mr

Items 30; decided by code 13; model called 17: first attempt 17, after a retry 0, fallback 0.
Shown model items 25; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 1 ({'must_surface_missed': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 3.33 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 100.0 | yes |
| value: AI digest preferred over the rollup, % of decided pairs (D-149: on only if preferred; D-130's value bar) | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 1080 | yes |
| latency p95, ms | <= 20000 | 2153 | yes |

- AI items shown: 25
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 100.0

