# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 17, after a retry 0, fallback 9.
Shown model items 37; critical reaching the user 1 ({'quiet_flagged': 1}); critical caught on 8 items ({'anchoring': 15, 'wrong_agent': 5, 'outcome_against_facts': 7, 'invented': 1, 'contradiction': 1}); omissions 6 ({'must_surface_missed': 6}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 2.7 | NO |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 20.0 | NO |
| quiet agents flagged | == 0 | 1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 75.5 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 1099.0 | yes |
| latency p95, ms | <= 20000 | 5144 | yes |

- AI items shown: 37
- quiet agents flagged: 1
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 75.5

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 47, after a retry 7, fallback 6.
Shown model items 54; critical reaching the user 1 ({'unsupported_claimed': 1}); critical caught on 8 items ({'wrong_target': 46}); omissions 9 ({'no_watch': 3, 'false_unsupported': 6}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 31.71 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 70.73 | NO |
| unsupported conditions claimed | == 0 | 1 | NO |
| nearest option offered on unsupported requests, % | >= 90.0 | 69.23 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 733.0 | yes |
| latency p95, ms | <= 4000 | 2088 | yes |

- exact rule matches: 13 of 41
- exact with ADR-0036's default for an unsaid lifetime: 29 of 41
- an unsaid lifetime whose ADR default differs from gold: 16
- unsupported claimed: 1
- nearest offered: 9 of 13
- nearest equal to gold: 4 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

