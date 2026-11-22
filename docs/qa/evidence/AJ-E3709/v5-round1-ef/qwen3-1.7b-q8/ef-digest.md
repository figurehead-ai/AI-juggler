# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 0, after a retry 0, fallback 29.
Shown model items 0; critical reaching the user 0 ({}); critical caught on 14 items ({'outcome_against_facts': 45, 'anchoring': 24, 'wrong_agent': 32}); omissions 30 ({'must_surface_missed': 82}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 0.0 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 11850 | NO |
| latency p95, ms | <= 20000 | 12626 | yes |

- AI items shown: 0
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 0.0

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 2, after a retry 0, fallback 12.
Shown model items 6; critical reaching the user 0 ({}); critical caught on 3 items ({'outcome_against_facts': 3, 'anchoring': 5, 'invented': 4}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 12 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 12 | NO |
| items passing without fallback, % | >= 80.0 | 54.5 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 12062.0 | NO |
| latency p95, ms | <= 20000 | 12353 | yes |

- AI items shown: 6
- quiet agents flagged: 12
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 54.5

