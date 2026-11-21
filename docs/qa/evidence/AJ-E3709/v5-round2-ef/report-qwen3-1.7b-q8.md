# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 14, after a retry 1, fallback 11.
Shown model items 34; critical reaching the user 1 ({'quiet_flagged': 1}); critical caught on 8 items ({'wrong_agent': 14, 'outcome_against_facts': 15, 'anchoring': 13}); omissions 7 ({'must_surface_missed': 7}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 2.94 | NO |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 23.33 | NO |
| quiet agents flagged | == 0 | 1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 65.4 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 3521.0 | yes |
| latency p95, ms | <= 20000 | 17118 | yes |

- AI items shown: 34
- quiet agents flagged: 1
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 65.4

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 32, after a retry 0, fallback 28.
Shown model items 32; critical reaching the user 0 ({}); critical caught on 21 items ({'wrong_target': 410, 'invented': 3}); omissions 25 ({'no_watch': 16, 'false_unsupported': 9}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 19.51 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 36.59 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 23.08 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1854.0 | yes |
| latency p95, ms | <= 4000 | 6067 | NO |

- exact rule matches: 8 of 41
- exact with ADR-0036's default for an unsaid lifetime: 15 of 41
- an unsaid lifetime whose ADR default differs from gold: 7
- unsupported claimed: 0
- nearest offered: 3 of 13
- nearest equal to gold: 2 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

