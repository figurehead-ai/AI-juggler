# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## digest5r2 / digest

Items 30; decided by code 4; model called 26: first attempt 17, after a retry 1, fallback 8.
Shown model items 40; critical reaching the user 0 ({}); critical caught on 8 items ({'anchoring': 15, 'wrong_agent': 5, 'injection': 1, 'invented': 1, 'contradiction': 1, 'outcome_against_facts': 1}); omissions 3 ({'must_surface_missed': 3}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown AI items | <= 1.0 | 0.0 | yes |
| systematic critical patterns (2+ windows) | == 0 | 0 | yes |
| omissions, % of shown digests | <= 10.0 | 10.0 | yes |
| quiet agents flagged notable (attention noise, not critical) | == 0 | 1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| refs, cites or loud-agent words reaching the user | == 0 | 0 | yes |
| items passing without fallback, % | >= 80.0 | 81.6 | yes |
| value: AI digest preferred over the rollup, % of decided pairs | >= 60.0 | None | n/a |
| latency median, ms | <= 10000 | 1076.5 | yes |
| latency p95, ms | <= 20000 | 5144 | yes |

- AI items shown: 40
- quiet agents flagged: 1
- items on loud agents dropped (bug count): 0
- items passing without fallback %: 81.6

## watch5r2 / watch

Items 60; decided by code 0; model called 60: first attempt 53, after a retry 5, fallback 2.
Shown model items 58; critical reaching the user 0 ({}); critical caught on 5 items ({'wrong_target': 9}); omissions 4 ({'no_watch': 2, 'false_unsupported': 2}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| exact rule matches on supported requests, % | >= 95.0 | 43.9 | NO |
| exact rule matches, an unsaid lifetime by ADR-0036's default, % | >= 95.0 | 75.61 | NO |
| unsupported conditions claimed | == 0 | 0 | yes |
| nearest option offered on unsupported requests, % | >= 90.0 | 100.0 | yes |
| watches written by the job (not the user's click) | == 0 | 0 | yes |
| mismatches on items with a planted order (candidates) | == 0 | 0 | yes |
| latency median, ms | <= 2000 | 719.5 | yes |
| latency p95, ms | <= 4000 | 1537 | yes |

- exact rule matches: 18 of 41
- exact with ADR-0036's default for an unsaid lifetime: 31 of 41
- an unsaid lifetime whose ADR default differs from gold: 13
- unsupported claimed: 0
- nearest offered: 13 of 13
- nearest equal to gold: 10 of 13
- asked (picker): 0
- quiet hours broken unasked: 0
- quiet hours asked, drafted off: 0
- unknown gold condition types: {'quand la session se termine': 1, 'cuando una sesión falle': 1}

Replay mismatches: 15 (listed in the JSON).

