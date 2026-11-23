# Evidence manifest — AJ-E1554

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1554, the rail's rows hold still on
hover, and every group says quietly what more there is, one click from it
(P1). Builds on AJ-E2327, AJ-E1550's tokens (PAT-SURF) and AJ-E1551's
layout-shift check. Status: **DONE (0.5.1)**, 2026-11-23.

Commits: d17f49342 (a session row's fixed slots; the branch count),
cd8e8d9b7 (an App header's counts and slots, the group footer, folded
Inactive apps' summary, the sign-in and not-added rows, the hover check).

No captures: the slots are proved by measuring the built rail in Chromium
(`layout-shift.spec.ts`, numbers in [hover-hold-still.json](hover-hold-still.json))
and the counts by `rail-more.test.tsx` against generic fixtures.

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | Every rail row type has fixed slots (gutter: dot, or the grip in its place on hover; title; end: state word, swapped for the tools on hover and focus); actions never overlap the title or move the dot | PASS | `RowShell` (rail layout): gutter and end are one-cell grids holding both layers, swapped by opacity only; the ⋯ moved from over the dot into the end's tools. App header: the grip over its icon (the icon gives way), its ↗ in a slot of its own; the "+n recent" that appeared on hover moved to the footer. Sign-in row: dot in the gutter, selected by a fill (no accent bar). Not-added row: unchanged slots, its Add a fill. `layout-shift.spec.ts` "a rail row and an App header hold still under the pointer and the keyboard" (dark and light: no part's box moved at rest, hover or focus, CLS 0); `rail-more.test.tsx` R1 cases (session row, header, sign-in and not-added rows) |
| R2 | An App header shows a compact status cluster, non-zero only, tabular, with tooltips: working, needs you (amber), done to review, subagents running (branch + n), replacing the bare count | PASS | `HeadCounts` (`rail-more.tsx`) from `headCounts` (`lib/rail-counts.ts`); `rail-more.test.tsx` "counts from the rows it holds, non-zero only, in order…", "the header draws its counts in place of the bare count" |
| R3 | A session row shows a branch icon + n when it runs subagents, opening its Subagents tab | PASS | `RowSubagents` (always shown when n > 0, beside the title, outside the hover swap so it stays clickable); `place-board.test.tsx` "every tool's session says how many it runs…" |
| R4 | An App group with more than shown has a quiet footer ("+12 more · 3 recent") opening that App's page filtered | PASS | `GroupFooter`/`RailFooter`: "+n more" opens the App's Sessions on the Open now filter (`lib/list-presets.ts`, read by `RelatedList`), in place where there is no App page or for Latest activity's quiet ones; "n recent" shows them in place; no footer when nothing more. `rail-more.test.tsx` R4 cases; `rail-apps.test.ts` and the rail matrix (the footer line) |
| R5 | Folded Inactive apps carry a summary ("10 apps · 48 sessions"); a folded Needs you keeps its count | PASS | `SectionHead` `summary`, `idleSummary`; `rail-more.test.tsx` "say what they hold"; Needs you's count unchanged (`rail-sections.test.tsx` passes) |
| R6 | Every count opens exactly those items, by id, from the cached counts (no new scan); AJ-E1550's tokens throughout | PASS | each count keeps its items (`HeadCount.items`); a click opens the next of them by id (`nextOf`), subagents on the Subagents tab; the numbers come from the rail's own rows (`subtask_count`, the needs list). Hover is a tone (`hover:bg-hover`), no borders on the counts, grip, pin, ↗ or Add. `rail-more.test.tsx` "each is a button… opens the next of exactly those, by id", "\"+n more\" opens its App's page on Sessions, its list on Open now" |

## Usability answers, checked on the built rail

1. Rows stay put under the pointer and the map hints at what lies beyond: the slots hold still (measured), and each group's counts and footer say what more there is.
2. Counts are small, muted, non-zero only, each with a tooltip saying what it counts ("2 working here: each click opens the next"); the footer says what opening it shows ("Opens Work's page on Open now: all 17 open").
3. The rail stays calm (open sessions only); each count is a door to exactly those items.
4. Zero values are hidden; a group with nothing more has no footer; folded Inactive apps are one summary line.
5. None.
6. Avoided: actions over the title (they take the state word's box), a dot that moves (the grip shares its gutter), a bare "2" (every count is named), subagents invisible (a branch count on the row and the header).

## Checks run

vitest: the whole UI suite (593 files, 4065 tests) before the last commit, then the rail, row, place, related and sign-in tests (412) after it; `tsc -b --noEmit`; `scripts/ui_lint.py` (no new findings); `scripts/private_data_lint.py`; `scripts/terminology_lint.sh` (one finding, not in these files); `layout-shift.spec.ts` through `scripts/heavy.sh` (6 passed, 2026-11-23).

## Left for later

- The screenshot baselines that draw the rail (`regression.spec.ts` rail-order, `places-gallery`, `inactive-apps-gallery`) change with this layout and need the visual gate's re-render; their test steps were updated, not run here.
- The View's "Subagents" switch no longer changes the rows (R3 shows the count always); it can go from the View menu.
- The project lens's rows (`WorkflowRail` `BoardRow`) moved their ⋯ off the dot into the row's end beside ↗ (over its App marks, never the title), but are not in the hover measurement.
