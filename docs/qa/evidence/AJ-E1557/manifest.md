# Evidence manifest — AJ-E1557

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1557, the rail is calm: no bullet
before a session, a state mark only when there is one, and a subagents
icon that reads as helpers (P1). Found by the maintainer on the 0.5.1
Nightly (2026-11-25): a dot before every session row read as bullets; the
subagents icon read as a git branch. Builds on AJ-E1554's fixed slots.
Status: **DONE (0.5.2)**, 2026-11-25.

Commits: 125574f72 (the marks, the helpers glyph, the tests and LST-8),
670da377b and eabc6ebff (96 gate baselines and their notes,
[rerender-0.5.2.md](../AJ-E803/rerender-0.5.2.md)), 02e2da1df (the rail's
gallery families re-captured, critiques stamped).

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | No leading dot on idle and ended rows; a state mark only for working, needs you, done to review and an error; alignment held | PASS | `hasStateMark` (`lib/attention.ts`) decides it once for every rail row: `StateMark` (`row/SessionRow.tsx`) draws nothing for an idle or ended row, so the App lens's rows (`SessionRow`), the Project and Account lenses' (`RailRow`), outside conversations' (`OutsideConversations`) and the folded rail's corner dot (`RailFolded`) all follow it. The gutter (`w-4`, AJ-E1554's slot) stays, empty, so every title starts in the same place and none moves when a mark appears. `rail-calm-marks.test.tsx` (12 cases: idle, ended, working, needs you, done to review, an error, in Comfortable and Compact: the mark only for the last four, the gutter and the title's place the same, the state's word always there); `regression.spec.ts` "status is never colour-only" (every rail row read by its word; an ended row drawn with no mark, working with its dot); `layout-shift.spec.ts` (the gutter, not the dot, held still under hover and focus) |
| R2 | The subagents icon is a helpers/agents glyph, legible at 12 px in both themes | PASS | `IconHelpers` (`components/icons.tsx`): a small cluster of agents, one in front of two, drawn in the set's style (24 grid, 1.75 stroke, round caps); a new icon for a new meaning (ICN-1: the git branch keeps meaning a git branch, Profiles' pair keeps meaning people). Used at 12 px by a session row's count (`RowSubagents`) and an App header's (`HeadCounts`). `rail-calm-marks.test.tsx` R2 (both counts draw `IconHelpers` at 12 px, never `IconGitBranch`); checked by eye at 3× zoom of the 12 px glyph in both themes on the re-captured 88-subagents captures (02e2da1df) and the gate baselines (eabc6ebff): three heads, the front one larger, a group, never a branch |

## Usability answers, checked on the built rail

1. A glance shows only what needs attention: idle and ended rows are their titles and a muted word; marks are left for working, needs you, done to review and an error. Match.
2. A row's state is its small mark (the live dot, its level's mark, ✓) or its trailing word; idle and ended rows have no leading mark; subagents are a cluster of agents and the number. Match.
3. The dot in every gutter goes; the gutter stays as the row's fixed slot (and Your order's grip). Match.
4. A row with no state worth a mark is just its title (and its muted word at the end). Match.
5. None. Match.
6. Watched for: rows jumping when a state appears (the gutter keeps its width: `rail-calm-marks.test.tsx`, `layout-shift.spec.ts`), and the icon at 12 px in both themes (opened in the captures). Match.

## Checks run

- vitest: the whole UI suite, 634 files, 4299 tests, pass; `tsc -b --noEmit` clean; `scripts/ui_lint.py` OK.
- `scripts/visual_gate.sh` (pinned container): the compare run on the tree passed 348/348 (every change under the threshold); `--update` on a clean export of HEAD 1b26459cf re-rendered them, and the 96 whose changed pixels are all in the rail were taken (the Project lens ones also catching up with 0.5.1 F77's mark), each diffed per region and opened (contact sheets of old and new rails side by side); `check_baseline_notes.py`: 96 changes, each explained.
- Gallery: `capture.spec.ts -g rail`, `rail-narrow`, `rail-projects`, `outside-gallery`, `subagents-gallery`, `places-gallery` on the host (96 passed); 81 changed captures diffed and opened, `critique_check.py --stamp` on the ten families, `critique_check.py` OK.
- `plain_words_lint.py`, `design_tokens_lint.py`, `private_data_lint.py`, `terminology_lint.sh`: OK on these files.

## Left

The Needs you rows keep their App's identity dot before the title (D-117 R8: the App's one colour, not a state); it is not an idle or ended row's mark, so it stays as specified.

## Amendment (2026-11-25, AJ-E1558)

Decided by the maintainer after this task closed: no bullet-like dot before any rail row, so the Needs you rows lose their App's colour dot too. Built in AJ-E1558 (4b76afc5c): `NeedRow` (`components/PlaceRows.tsx`) leads with nothing, its gutter kept empty like an idle row's, so no title moves; the need's own state mark (its level's mark, or review's ✓) stays before why it needs you; the App is named on the row's place line ("ai-juggler · Claude Code CLI · Work"), and a compact row, which has no place line, names it after why ("Asks you · Claude Code CLI"). Proof: `rail-calm-lenses.test.tsx` (the Needs you row has no `place-mark` and an empty gutter), `layout-shift.spec.ts` (the row's title, why and where hold still with no mark to measure); the rail's gate baselines and 01-rail's captures re-rendered with AJ-E1558 (its manifest, R2 and R5).
