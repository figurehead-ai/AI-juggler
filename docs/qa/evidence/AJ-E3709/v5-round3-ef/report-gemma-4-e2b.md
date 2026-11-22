# Harness v5 report: local model (llama.cpp, Metal) via local_probe (gemma-4-e2b, sampling low, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r3 / digest

Items 30; decided by code 6; model called 24: first attempt 17, after a retry 1, fallback 6.
Shown model items 40; critical reaching the user 0 ({}); critical caught on 3 items ({'anchoring': 14}); omissions 2 ({'must_surface_missed': 2}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 6.67 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 85.1 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 2747.5 | yes |
| latency p95, ms | <= 20000 | 13189 | yes |

- AI items shown: 40
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 85.1

## watch5r3 / watch

Items 60; decided by code 0; model called 60: first attempt 55, after a retry 0, fallback 5.
Shown model items 55; critical reaching the user 0 ({}); critical caught on 3 items ({'wrong_target': 6, 'invented': 3}); omissions 24 ({'no_watch': 5, 'false_unsupported': 19}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 14.89 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 42.55 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 100.0 | yes |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1764.0 | yes |
| latency p95, ms | <= 4000 | 5475 | NO |

- exact rule matches: 7 of 47
- exact with ADR-0036's default for an unsaid lifetime: 20 of 47
- an unsaid lifetime whose ADR default differs from gold: 13
- unsupported claimed: 0
- nearest offered: 7 of 7
- nearest equal to gold: 7 of 7
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 2
- unknown gold condition types: {'quand la session se termine': 1, 'cuando la sesión termine': 1, 'quando la sessione termina': 1}

