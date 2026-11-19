# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r3 / digest

Items 30; decided by code 6; model called 24: first attempt 13, after a retry 1, fallback 10.
Shown model items 34; critical reaching the user 0 ({}); critical caught on 7 items ({'anchoring': 12, 'wrong_agent': 1, 'outcome_against_facts': 6}); omissions 2 ({'must_surface_missed': 2}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 6.67 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 72.3 | NO |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 4544.5 | yes |
| latency p95, ms | <= 20000 | 14602 | yes |

- AI items shown: 34
- quiet agents flagged: 0
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 72.3

## watch5r3 / watch

Items 60; decided by code 0; model called 60: first attempt 35, after a retry 1, fallback 0.
Shown model items 36; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 21 ({'false_unsupported': 1, 'no_watch': 20}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 17.02 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 48.94 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 85.71 | NO |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 2152.0 | NO |
| latency p95, ms | <= 4000 | 2669 | yes |

- exact rule matches: 8 of 47
- exact with ADR-0036's default for an unsaid lifetime: 23 of 47
- an unsaid lifetime whose ADR default differs from gold: 15
- unsupported claimed: 0
- nearest offered: 6 of 7
- nearest equal to gold: 6 of 7
- asked (picker): 0
- exact with defaults, English/German: 23 of 27
- exact with defaults, other languages (the form by design, ADR-0033): 0 of 20
- gold lifetimes beyond the draft's shape (a later date; counted with the defaults): 5
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando la sesión termine': 1, 'quando la sessione termina': 1}

Replay mismatches: 32 (listed in the JSON).

