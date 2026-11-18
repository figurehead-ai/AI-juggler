# Harness v5 report: local model (llama.cpp, Metal) via local_probe (gemma-4-e2b, sampling low, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 17, after a retry 2, fallback 7.
Shown model items 40; critical reaching the user 3 ({'quiet_flagged': 3}); critical caught on 6 items ({'wrong_agent': 12, 'outcome_against_facts': 12, 'anchoring': 3}); omissions 7 ({'must_surface_missed': 7}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 7.5 | NO |
| systematic critical patterns (2+ windows) | == 0 | 1 | NO |
| omissions, % of shown digests | <= 10.0 | 23.33 | NO |
| quiet agents flagged | == 0 | 3 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 76.9 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 2873.5 | yes |
| latency p95, ms | <= 20000 | 16064 | yes |

- AI items shown: 40
- quiet agents flagged: 3
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 76.9

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 50, after a retry 3, fallback 7.
Shown model items 53; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_target': 24, 'invented': 4}); omissions 30 ({'false_unsupported': 25, 'no_watch': 5}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 9.76 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 24.39 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 46.15 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1820.0 | yes |
| latency p95, ms | <= 4000 | 5484 | NO |

- exact rule matches: 4 of 41
- exact with ADR-0036's default for an unsaid lifetime: 10 of 41
- an unsaid lifetime whose ADR default differs from gold: 6
- unsupported claimed: 0
- nearest offered: 6 of 13
- nearest equal to gold: 3 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

