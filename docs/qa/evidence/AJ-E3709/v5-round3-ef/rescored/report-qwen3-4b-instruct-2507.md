# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3-4b-instruct-2507, sampling card, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r3 / digest

Items 30; decided by code 6; model called 24: first attempt 12, after a retry 4, fallback 8.
Shown model items 37; critical reaching the user 0 ({}); critical caught on 8 items ({'anchoring': 17, 'outcome_against_facts': 7}); omissions 1 ({'must_surface_missed': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 3.33 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 78.7 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 3009.5 | yes |
| latency p95, ms | <= 20000 | 9522 | yes |

- AI items shown: 37
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 78.7

## watch5r3 / watch

Items 60; decided by code 0; model called 60: first attempt 33, after a retry 1, fallback 2.
Shown model items 34; critical reaching the user 0 ({}); critical caught on 5 items ({'invented': 1, 'wrong_target': 11}); omissions 22 ({'no_watch': 22}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 19.15 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 51.06 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 85.71 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 1251.0 | yes |
| latency p95, ms | <= 4000 | 3661 | yes |

- exact rule matches: 9 of 47
- exact with ADR-0036's default for an unsaid lifetime: 24 of 47
- an unsaid lifetime whose ADR default differs from gold: 15
- unsupported claimed: 0
- nearest offered: 6 of 7
- nearest equal to gold: 6 of 7
- asked (picker): 0
- exact with defaults, English/German: 24 of 27
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 20
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 5
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando la sesión termine': 1, 'quando la sessione termina': 1}

Replay mismatches: 33 (listed in the JSON).

