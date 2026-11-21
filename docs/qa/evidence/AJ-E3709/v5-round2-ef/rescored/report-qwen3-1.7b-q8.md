# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 19, after a retry 0, fallback 7.
Shown model items 41; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_agent': 14, 'anchoring': 4, 'outcome_against_facts': 4, 'injection': 3}); omissions 4 ({'must_surface_missed': 4}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 13.33 | NO |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 78.8 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 2649.5 | yes |
| latency p95, ms | <= 20000 | 17118 | yes |

- AI items shown: 41
- quiet agents flagged: 1
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 78.8

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 43, after a retry 0, fallback 17.
Shown model items 43; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_target': 78}); omissions 17 ({'no_watch': 13, 'false_unsupported': 4}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 29.27 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 48.78 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 92.31 | yes |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1653.5 | yes |
| latency p95, ms | <= 4000 | 5347 | NO |

- exact rule matches: 12 of 41
- exact with ADR-0036's default for an unsaid lifetime: 20 of 41
- an unsaid lifetime whose ADR default differs from gold: 8
- unsupported claimed: 0
- nearest offered: 12 of 13
- nearest equal to gold: 9 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

Replay mismatches: 22 (listed in the JSON).

