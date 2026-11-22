# Harness v5 report: local model (llama.cpp, Metal) via local_probe (gemma-4-e2b, sampling low, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 20, after a retry 2, fallback 4.
Shown model items 43; critical reaching the user 0 ({}); critical caught on 3 items ({'wrong_agent': 12}); omissions 3 ({'must_surface_missed': 3}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 10.0 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 3 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 82.7 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 2719.5 | yes |
| latency p95, ms | <= 20000 | 16064 | yes |

- AI items shown: 43
- quiet agents flagged: 3
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 82.7

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 56, after a retry 0, fallback 4.
Shown model items 56; critical reaching the user 0 ({}); critical caught on 3 items ({'wrong_target': 6, 'invented': 3}); omissions 16 ({'no_watch': 4, 'false_unsupported': 12}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 36.59 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 56.1 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 100.0 | yes |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1815.5 | yes |
| latency p95, ms | <= 4000 | 4301 | NO |

- exact rule matches: 15 of 41
- exact with ADR-0036's default for an unsaid lifetime: 23 of 41
- an unsaid lifetime whose ADR default differs from gold: 8
- unsupported claimed: 0
- nearest offered: 13 of 13
- nearest equal to gold: 10 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

Replay mismatches: 21 (listed in the JSON).

