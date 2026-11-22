# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 7, after a retry 3, fallback 19.
Shown model items 17; critical reaching the user 0 ({}); critical caught on 5 items ({'outcome_against_facts': 2, 'wrong_agent': 3, 'anchoring': 5}); omissions 30 ({'must_surface_missed': 82}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 89.5 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 17776 | NO |
| latency p95, ms | <= 20000 | 18843 | yes |

- AI items shown: 17
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 89.5

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 3, after a retry 1, fallback 10.
Shown model items 10; critical reaching the user 2 ({'quiet_flagged': 2}); critical caught on 1 items ({'outcome_against_facts': 1}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 20.0 | NO |
| systematic critical patterns (2+ windows) | == 0 | 1 | NO |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 14 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 12 | NO |
| items passing without fallback, % | >= 80.0 | 90.9 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 17137.0 | NO |
| latency p95, ms | <= 20000 | 17555 | yes |

- AI items shown: 10
- quiet agents flagged: 14
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 90.9

