# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3-4b-instruct-2507, sampling card, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5 / digest

Items 30; decided by code 1; model called 29: first attempt 7, after a retry 9, fallback 13.
Shown model items 37; critical reaching the user 0 ({}); critical caught on 17 items ({'outcome_against_facts': 17, 'anchoring': 22, 'wrong_agent': 12, 'invented': 2}); omissions 30 ({'must_surface_missed': 80}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 69.8 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 10982 | NO |
| latency p95, ms | <= 20000 | 14625 | yes |

- AI items shown: 37
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 69.8

## digest5b / digest

Items 14; decided by code 0; model called 14: first attempt 3, after a retry 8, fallback 3.
Shown model items 31; critical reaching the user 2 ({'quiet_flagged': 2}); critical caught on 4 items ({'outcome_against_facts': 5, 'anchoring': 7, 'contradiction': 1, 'wrong_agent': 2}); omissions 14 ({'must_surface_missed': 32}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 6.45 | NO |
| systematic critical patterns (2+ windows) | == 0 | 1 | NO |
| omissions, % of shown digests | <= 10.0 | 100.0 | NO |
| quiet agents flagged | == 0 | 11 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 9 | NO |
| items passing without fallback, % | >= 80.0 | 91.2 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 8359.0 | yes |
| latency p95, ms | <= 20000 | 11506 | yes |

- AI items shown: 31
- quiet agents flagged: 11
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 91.2

