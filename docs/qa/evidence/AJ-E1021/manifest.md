# Evidence manifest — AJ-E1021

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1021 (Triage and
Overview fold into Home, a D-25 amendment). Status: **DONE-EVIDENCED**.
2026-10-20, reviewer claude-opus.

## The evidence for folding

| Triage / Overview had | Home already had | Now |
|---|---|---|
| Triage › Needs you (waiting, stuck, limit, failed) | Needs you (same statuses, plus the attention flag), one sentence and one action each | Home; parked sessions excluded |
| Triage › Running | Running | Home |
| Triage › Parked until a reset (Resume when due) | nothing — a parked session showed as "hit its limit" | Home › Parked until a reset |
| Triage › Finished — not reviewed (Mark reviewed) | Pick up where you left off (3 most recent ended, reviewed or not) | Home › Pick up where you left off: ended this week, not reviewed; 3 then Show all; Mark reviewed |
| Triage › Sort (waiting, project, quota risk) | — | the rail's Sort (needs, quota risk, project, age) |
| Triage › J/K, Enter, A | — | Home, same keys, named in one line |
| Overview › Live sessions cards | Running | Home |
| Overview › Quota windows | Runway | Home |
| Overview › "Machine OK", "Fleet 1" | — | dropped: fixed values, not measurements |
| Overview › ledger-degraded banner | — | Home, only while degraded |

Both screens sat under More, outside D-25's structure (Home, Accounts,
History, Settings; Cleanup and Fleet when relevant).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 parked | vitest `home.test.tsx`: a parked session isn't under Needs you; its section says "resets in 2h 13m" or "reset reached"; Resume calls `parked_resume` and refreshes | PASS |
| Req.2 to review | vitest: `toReview` (ended this week, not reviewed or archived); three then Show all; Mark reviewed calls `instance_mark_reviewed` and refreshes | PASS |
| Req.3 keyboard | vitest: J and ArrowDown move, A marks the focused ended session reviewed, K back, Enter opens; the keys line; a11y "triage on Home — axe 0 and keyboard only" (J then K, both themes) | PASS |
| Req.4 banner | capture `17-ledger/degraded-banner-*` on Home, copy exact | PASS |
| Req.5 More | `home-gallery.spec.ts` More lists Usage, Fleet, Cleanup; the regression gate no longer has Triage or Overview (their 16 baselines removed) | PASS |
| Budget | vitest: 50 sessions render well inside 300 ms | PASS |
| Full suites | vitest 470/470; Playwright 919/919; a11y gate 940 states, 0 critical/serious; visual gate 65/65 (re-rendered; only Triage's and Overview's 16 baselines removed, the rest unchanged within threshold) | PASS |

Gallery: `55-triage/home-populated-*` and `home-empty-*` (1440×900 and
1024×680, both themes and densities).

## Not covered here

- The usage-statistics schema keeps the `overview` screen value (the
  shared contract); the app no longer sends it.
- `SessionCard` (the Overview card variant) has no screen now; its
  honesty tests still run. Retiring it is a follow-up.
- The tool inspector (selection kind "provider") was opened only from
  Overview's quota windows; it stays for Accounts to link to once a
  second tool is set up.
