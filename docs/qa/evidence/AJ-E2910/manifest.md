# Evidence manifest — AJ-E2910

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2910, Home:
filter chips, the r / h / s / e keys, and Mark all finished as reviewed.
Checked again on 2026-11-06 (release/0.4.1); the vitest files below pass;
the Rust test runs in the release preflight.
Status: **DONE, 4 gaps open** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | `home-chips.test.tsx` "offers chips only when two kinds or more are there" (none for one kind; "All 3", "Asks you 1", "Sign-in 1"; "To review 2") and "says a limit as its rows do…". **Missing:** the requirement names the chip **Limits n**, but it now says **Limit reached n** (0.4 design review item 69, `docs/qa/0.4-walks/design.md`). The task text needs an amendment, or the label goes back. |
| R2 | GAP | `home-chips.test.tsx` "puts each need under its chip, and the rest under All only" covers asks, stuck, review, sign-in and This Mac (All only); `homeChips(…, { asks, review })` covers agents' asks and tasks waiting on review. **Missing:** no test maps a quota need to Limits, or failed, heavy and left-running needs to All only (`chipOf` does it in code). |
| R3 | GAP | "narrows the list and says what it hides, never remembered" asserts "2 more under All" and that clicking it shows All again. Playwright "AJ-E2910: no chip changes the rail…" asserts the rail's text is identical after each chip. **Missing:** no test shows the choice is forgotten on a new visit (no remount), and none checks the bell and the Dock badge stay the same. |
| R4 | GAP | `app-keys.test.tsx` holds the four keys in the registry, never firing while typing ("holds for every shortcut…"), and on the ? sheet ("lists exactly the registered shortcuts…"). **Missing:** no test presses r, h, s or e on a focused Home row to check what each opens (the answer, Continue on another account…, the snooze choices, archive with Undo), or that a key that doesn't apply does nothing while the row's ⋯ says why. |
| R5 | PASS | `home-chips.test.tsx` "Mark all N reviewed covers the whole list": 3 shown, the button says "Mark all 7 reviewed" and calls once. `crates/juggler-app/tests/triage_lifecycle.rs` `mark_all_reviewed_marks_every_ended_one_or_none_and_undoes_in_one`: one call marks every ended one, none when one is live, and one call undoes them. Playwright "AJ-E2910…" clicks it and sees the toast; `marked-all-1440x900-dark.png` shows "Marked 3 reviewed." with Undo. |
| R6 | PASS | The chips use the shared `Segmented` (`HomeNeeds.tsx`); `controls.test.tsx` "is a radio group with one tab stop that the arrows move" proves the one tab stop and the arrows; `home-chips.test.tsx` picks a chip by its `radio` role. |

## The first part (2026-11-05)

### AJ-E2910 evidence: Home's chips, keys and Mark all

- vitest `src/components/home-chips.test.tsx` — pass (2026-11-05).
- Playwright "AJ-E2910: no chip changes the rail; Mark all and its toast;
  at 1440": the rail's text is identical after each chip; Mark all shows
  its toast with Undo — pass.
- Gallery `99-home-chips`: all, narrowed, Mark all's toast; 1024×700,
  1280×800 and 1440×900, dark and light; axe 0 critical/serious; critique
  stamped.
- Verdict: PASS.
