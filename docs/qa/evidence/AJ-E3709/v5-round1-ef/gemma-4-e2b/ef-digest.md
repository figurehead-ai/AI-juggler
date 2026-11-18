# Harness v5 report: local model (llama.cpp, Metal) via local_probe (gemma-4-e2b, sampling low, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 0, after a retry 1, fallback 28.
Shown model items 3; critical reaching the user 0 ({}); critical caught on 19 items ({'outcome_against_facts': 77, 'invented': 43, 'anchoring': 30, 'wrong_agent': 26}); omissions 30 ({'must_surface_missed': 82}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 6.0 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 16260 | NO |
| latency p95, ms | <= 20000 | 20751 | NO |

- AI items shown: 3
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 6.0

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 2, after a retry 1, fallback 11.
Shown model items 17; critical reaching the user 3 ({'quiet_flagged': 3}); critical caught on 5 items ({'anchoring': 4, 'outcome_against_facts': 7, 'wrong_agent': 2}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 17.65 | NO |
| systematic critical patterns (2+ windows) | == 0 | 1 | NO |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 14 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 11 | NO |
| items passing without fallback, % | >= 80.0 | 63.0 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 14304.0 | NO |
| latency p95, ms | <= 20000 | 20236 | NO |

- AI items shown: 17
- quiet agents flagged: 14
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 63.0

