# Evidence manifest — AJ-E1551

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1551 (nothing moves after it appears: every column, header, rail and badge has its place from the first paint; layout shift near zero). 2026-11-23, builder claude-opus.

## What moved, and the fix

Each case is fixed the same way: what decides whether something shows, and how much room it takes, comes from what is already known on the first frame (D-103.6's kept shapes, the lists last read), and while the read runs it is a skeleton at its final size through AJ-E1552's one loading state (`LoadingState`: "Still loading…" when slow, Retry past its deadline).

| Where (R1, R2) | Before | Fix |
|---|---|---|
| A session's page, the subagent tree (the maintainer's report) | The column was added only after the subagent lists were read, pushing the conversation over | `ConversationSwitcher` decides the column from what's known (the lists last read for that session, `switcher-lists.tsx`'s cache; else the rail's or a heard subagent count, `knownHasSubagents`) and draws it as a skeleton at the list's 12 rem width while the lists are read |
| The rail, Needs you (the App lens's board and the project lens) | Needs you mounted when the needs read answered, pushing Open now down about 350 px | `RailNeedsHeld` (board) and `NeedsYouSection` (project lens) hold last time's height (`RAIL_NEEDS_PLACE`, kept per data folder) as a skeleton until the needs are known |
| Home, Your apps | A row's need mark arrived with the needs and, wrapping, made its row 17 px taller, pushing the rows under it and Runway down | Each open App's row keeps last time's height until the needs are known (`aij.home.open-row:<key>`), and the card its height (`HOME_APPS_PLACE`) |

Already holding before this task (checked): the session page's side column (a fixed 340 px grid column from the first paint), Home's Needs you now (D-103.6), the switch and Runway cards (part of the same render), the header's counts (right-aligned; a count appearing moves nothing to its right).

## The check

`ui/tests/visual/layout-shift.spec.ts`, in the a11y gate's shards (`scripts/a11y_gate.sh`): Chromium's `layout-shift` entries summed from the first paint, **counting shifts right after a click too** (the browser's own CLS forgives those, the eye doesn't); budget 0.01 (about 0; a shift a person notices is about 0.05 and up). Opening Home (the second launch's first frame, as a user opens the app each day: a first run has nothing kept yet), opening two sessions from the rail, switching every tab of a session's page, and Home again from a session; both themes, 1280×800. Its numbers: [layout-shift.json](docs/qa/evidence/AJ-E1551/layout-shift.json).

| Moment (R3) | Before | After |
|---|---|---|
| Opening Home, a returning launch | 0.0743 (Your apps' rows, Runway, the rail's Open now rows) | 0.0051 (one row's words settling inside its held height) |
| Opening Home, the very first run (nothing kept yet) | 0.1533 | not gated: nothing is known before the first read |
| Opening a session (sess-1, sess-2) | measured after the tree's fix only | 0 |
| Switching a session's tabs (Pools, Usage, Processes) | measured after the tree's fix only | 0 |
| Home again from a session | 0 | 0 |

The preview's needs come from its stand-in's first tick (`ipc/notices.ts` `standInNeedsKnown`), so in the preview they always arrive after the first paint: the check measures the worst case. A unit test holds the tree's reserve (`conversation-switcher.test.tsx`: the skeleton on the first frame, the list in its place when the read answers).

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — vitest `pages/session/conversation-switcher.test.tsx` (the subagent column as a skeleton on the first frame, decided from what is known) (de193a190); Playwright `ui/tests/visual/layout-shift.spec.ts` "opening a session and switching its tabs hold still" (both themes, 3 runs each).
R2: PASS — the sweep's cases (the rail's Needs you, Home's Your apps) held in place: Playwright `layout-shift.spec.ts` "opening Home holds still" and "a rail row, an App header and a Project lens row hold still…" (both themes, 3 runs each) (b36feea73).
R3: PASS — `ui/tests/visual/layout-shift.spec.ts` (budget 0.01): its three checks, both themes, `--repeat-each=3` on the host through `scripts/heavy.sh`, 18 of 18 passed (2026-11-23).

### Usability answers, matched against the built screens

1. What they aimed at stays where it is: matches (the shift check, R3).
2. Nothing to explain: matches (no new words).
3. A skeleton at the final size on the first frame: matches (the subagent column's skeleton at 12 rem; Needs you and Your apps keep last time's height).
4. Decided from cached data, never a late fetch: matches (`knownHasSubagents`, the kept heights); the very first run with nothing kept is not gated (above).
5. None: matches.
6. A layout that jumps: matches on Home, a session and its tabs; every other screen by eye in PL1 B2: pending critique.
