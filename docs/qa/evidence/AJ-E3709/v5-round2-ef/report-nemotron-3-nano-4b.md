# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 17, after a retry 5, fallback 4.
Shown model items 38; critical reaching the user 1 ({'quiet_flagged': 1}); critical caught on 4 items ({'outcome_against_facts': 4, 'anchoring': 1, 'wrong_agent': 1}); omissions 8 ({'must_surface_missed': 8}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 2.63 | NO |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 26.67 | NO |
| quiet agents flagged | == 0 | 1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 82.6 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 3390.0 | yes |
| latency p95, ms | <= 20000 | 13613 | yes |

- AI items shown: 38
- quiet agents flagged: 1
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 82.6

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 51, after a retry 5, fallback 4.
Shown model items 56; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_target': 30, 'invented': 2}); omissions 14 ({'false_unsupported': 12, 'no_watch': 2}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 19.51 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 48.78 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 76.92 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 2247.5 | NO |
| latency p95, ms | <= 4000 | 5984 | NO |

- exact rule matches: 8 of 41
- exact with ADR-0036's default for an unsaid lifetime: 20 of 41
- an unsaid lifetime whose ADR default differs from gold: 12
- unsupported claimed: 0
- nearest offered: 10 of 13
- nearest equal to gold: 5 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

