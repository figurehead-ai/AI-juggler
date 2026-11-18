# Harness v5 report: Apple on-device Foundation Model via apple_probe.swift (apple, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 0, after a retry 0, fallback 29.
Shown model items 7; critical reaching the user 0 ({}); critical caught on 29 items ({'outcome_against_facts': 126, 'anchoring': 10, 'wrong_agent': 4}); omissions 30 ({'must_surface_missed': 82}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 13.2 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 16917 | NO |
| latency p95, ms | <= 20000 | 19751 | yes |

- AI items shown: 7
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 13.2

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 6, after a retry 4, fallback 4.
Shown model items 27; critical reaching the user 2 ({'quiet_flagged': 2}); critical caught on 3 items ({'outcome_against_facts': 6}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 7.41 | NO |
| systematic critical patterns (2+ windows) | == 0 | 1 | NO |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 14 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 12 | NO |
| items passing without fallback, % | >= 80.0 | 87.1 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 9483.0 | yes |
| latency p95, ms | <= 20000 | 16751 | yes |

- AI items shown: 27
- quiet agents flagged: 14
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 87.1

