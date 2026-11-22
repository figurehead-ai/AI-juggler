# Harness v5 report: Apple on-device Foundation Model via apple_probe.swift (apple, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r3 / digest

Items 30; decided by code 6; model called 24: first attempt 16, after a retry 3, fallback 5.
Shown model items 36; critical reaching the user 0 ({}); critical caught on 4 items ({'outcome_against_facts': 2, 'anchoring': 15, 'wrong_agent': 3}); omissions 1 ({'must_surface_missed': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 3.33 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 83.7 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 4500.0 | yes |
| latency p95, ms | <= 20000 | 12025 | yes |

- AI items shown: 36
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 83.7

## watch5r3 / watch

Items 60; decided by code 0; model called 60: first attempt 31, after a retry 2, fallback 3.
Shown model items 33; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_target': 27, 'invented': 3}); omissions 26 ({'no_watch': 23, 'false_unsupported': 3}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 23.4 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 40.43 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 85.71 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 3175.5 | NO |
| latency p95, ms | <= 4000 | 9280 | NO |

- exact rule matches: 11 of 47
- exact with ADR-0036's default for an unsaid lifetime: 19 of 47
- an unsaid lifetime whose ADR default differs from gold: 8
- unsupported claimed: 0
- nearest offered: 6 of 7
- nearest equal to gold: 5 of 7
- asked (picker): 0
- exact with defaults, English/German: 19 of 27
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 20
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 5
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando la sesión termine': 1, 'quando la sessione termina': 1}

Replay mismatches: 35 (listed in the JSON).

