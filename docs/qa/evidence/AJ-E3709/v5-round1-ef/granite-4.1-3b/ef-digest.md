# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 0, after a retry 0, fallback 29.
Shown model items 2; critical reaching the user 0 ({}); critical caught on 24 items ({'outcome_against_facts': 109, 'invented': 8, 'wrong_agent': 15, 'anchoring': 44, 'contradiction': 5}); omissions 30 ({'must_surface_missed': 82}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 3.1 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 5507 | yes |
| latency p95, ms | <= 20000 | 6421 | yes |

- AI items shown: 2
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 3.1

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 1, after a retry 2, fallback 11.
Shown model items 19; critical reaching the user 1 ({'quiet_flagged': 1}); critical caught on 6 items ({'anchoring': 12, 'contradiction': 6, 'outcome_against_facts': 11}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 5.26 | NO |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 13 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 12 | NO |
| items passing without fallback, % | >= 80.0 | 65.5 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 5130.5 | yes |
| latency p95, ms | <= 20000 | 6303 | yes |

- AI items shown: 19
- quiet agents flagged: 13
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 65.5

