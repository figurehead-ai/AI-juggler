# Evidence manifest — AJ-E2802 The rail's grouping switch and a real View menu

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2802, judged against the
requirements as amended 2026-11-06. The switch is `GroupSwitch` in
`ui/src/components/RailToolbar.tsx` (the shared `Segmented`), the View is
`ui/src/components/ViewPopover.tsx`, the stored view `ui/src/lib/rail-view.ts`;
built in 0.3 (b17fb5d80, shipped in ca2247e04) and finished on
`release/0.4.1`; re-checked against the code, its tests and the committed
gallery, 2026-11-06.

Run for this manifest (one command):
`cd ui && pnpm -s exec vitest run src/components/rail-group-switch.test.tsx src/components/rail-sections.test.tsx src/components/sorting.test.tsx src/components/rail-view-words.test.tsx src/lib/rail-model.test.ts src/app-keys.test.tsx src/components/place-board.test.tsx src/components/controls/controls.test.tsx`
→ 8 files, 101 tests passed. Playwright was not run here: its tests run in
the gallery gate, and the captures cited are the committed ones.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a segmented App · Project · Account · Pool switch above Needs you (`role="radiogroup"`, named "Group sessions by"), App first and the default; the View menu no longer has Group by (amended) | vitest `rail-group-switch.test.tsx`: "offers App, Project and Account; one click groups, kept per Mac"; Playwright `controls-gallery.spec.ts` "the rail's grouping switch, Pool offered" and "the View, as it opens" (asserts no `radio` and no `select` in the View); `_gallery/97-controls/rail-group-switch-1280x800-dark.png` | radiogroup, App first and checked; no Group by in the View | passes; the capture shows App · Project · Account · Pool under the search, above Needs you (spec runs in the gallery gate) | PASS | b17fb5d80 | 2026-11-06 | Claude (self) |
| R2 the choice remembered per Mac and restored at start | vitest `rail-group-switch.test.tsx`: "offers App, Project and Account; one click groups, kept per Mac" (writes `aij.rail.view.v1`); `lib/rail-model.test.ts`: "is read guarded: junk or a wrong shape is the defaults; Pool is kept as picked" (`parseView` restores the stored grouping) | stored on change, read back | both pass: Project is stored, and a stored `project` / `pool` is read back as picked | PASS | b17fb5d80 | 2026-11-06 | Claude (self) |
| R3 Pool shows once a pool of yours has members; a saved Pool with none shows App | vitest `rail-group-switch.test.tsx`: "offers Pool once a pool of yours has members (a project's own pool, or an empty one, doesn't count)", "shows App while Pool is kept with no pool of yours" | not shown, then shown; App shown | both pass | PASS | 3822aab2c | 2026-11-06 | Claude (self) |
| R4 the View has Show (seven lines, each with its hint: Inactive, Quiet ones, Ended today, Background runs, Subagents, Other Macs, Archived) and Only (App, Account, Project, each a submenu of choice lines, never a system select) (amended) | vitest `rail-group-switch.test.tsx`: "gives every Show line its hint (AJ-E2802 R4)" (one `menuitemcheckbox` per `SHOW_KEYS`, seven, each with its note), "is a menu: Show lines turn on and off, Only opens a submenu, Reset shows only when changed" (no `select`/`input`; Only › Account a `menuitemradio` submenu); `rail-view-words.test.tsx`: "says Subagents, and what Inactive, Quiet ones and Background runs are"; `_gallery/97-controls/view-changed-only-open-1280x800-dark.png` | seven hinted lines; Only as submenus | all pass; the capture shows Only › Account open with ✓ | PASS | 3822aab2c | 2026-11-06 | Claude (self) |
| R5 Reset in the menu's head only when the view differs, with Undo in a toast | vitest `rail-group-switch.test.tsx`: "is a menu: Show lines turn on and off, Only opens a submenu, Reset shows only when changed" (no Reset by default; Reset shown once changed; toast "The View is reset."; Undo restores); Playwright `controls-gallery.spec.ts` "the View, as it opens" / "the View, changed, Only › Account open"; `_gallery/97-controls/view-open-1280x800-dark.png` | Reset only when changed; Undo restores | passes | PASS | 3822aab2c | 2026-11-06 | Claude (self) |
| R6 ⇧V still opens the View; the switch is reached by Tab and moves with the arrows | vitest `app-keys.test.tsx`: "⌘1 goes to the first place and ⇧V opens the View"; `controls/controls.test.tsx`: "is a radio group with one tab stop that the arrows move"; `rail-group-switch.test.tsx`: "offers App, Project and Account; one click groups, kept per Mac" (ArrowRight moves it) | ⇧V opens; one tab stop; arrows move | all pass | PASS | b17fb5d80 | 2026-11-06 | Claude (self) |
| R7 the rail's sections, counts and Needs you unchanged by the move (one count, D-49) | vitest `rail-sections.test.tsx`: "keeps Needs you and its count whatever the grouping (AJ-E2802 R7)", "reads Needs you, then Open now, then the Also line"; `sorting.test.tsx` (AJ-E404's expectations on the one order) | same count and rows in App, Project, Account | all pass: count 2 and the same first rows in every grouping | PASS | 3c5dbab0a | 2026-11-06 | Claude (self) |
| Gate: the rail's tests | the vitest command above | pass | 101 / 101 pass | PASS | 3c5dbab0a | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-controls/rail-group-switch-*.png`,
`_gallery/97-controls/view-open-*.png`,
`_gallery/97-controls/view-changed-only-open-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
