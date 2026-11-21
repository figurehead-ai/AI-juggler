# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r3 / digest

Items 30; decided by code 6; model called 24: first attempt 20, after a retry 0, fallback 4.
Shown model items 43; critical reaching the user 0 ({}); critical caught on 4 items ({'anchoring': 9, 'invented': 4, 'acting': 3}); omissions 1 ({'must_surface_missed': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 3.33 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 91.5 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 1055.0 | yes |
| latency p95, ms | <= 20000 | 4111 | yes |

- AI items shown: 43
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 91.5

## watch5r3 / watch

Items 60; decided by code 0; model called 60: first attempt 48, after a retry 3, fallback 9.
Shown model items 51; critical reaching the user 0 ({}); critical caught on 7 items ({'wrong_target': 15, 'invented': 2}); omissions 11 ({'no_watch': 6, 'false_unsupported': 5}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 19.15 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 59.57 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 71.43 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 735.0 | yes |
| latency p95, ms | <= 4000 | 2337 | yes |

- exact rule matches: 9 of 47
- exact with ADR-0036's default for an unsaid lifetime: 28 of 47
- an unsaid lifetime whose ADR default differs from gold: 19
- unsupported claimed: 0
- nearest offered: 5 of 7
- nearest equal to gold: 5 of 7
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 1
- unknown gold condition types: {'quand la session se termine': 1, 'cuando la sesión termine': 1, 'quando la sessione termina': 1}

