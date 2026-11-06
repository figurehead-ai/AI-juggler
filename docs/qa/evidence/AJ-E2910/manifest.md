# Evidence manifest — AJ-E2910

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2910, Home:
filter chips, the r / h / s / e keys, and Mark all finished as reviewed.
Checked again on 2026-11-06 (release/0.4.1); the vitest files below pass;
the Rust test runs in the release preflight.
Status: **DONE, 1 gap open** (agent self-check, 2026-11-06; R1–R3 closed
the same day, and R4's keys; R4's "its ⋯ says why" for e on a live row
waits on a design call).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `home-chips.test.tsx` "offers chips only when two kinds or more are there" (none for one kind; "All 3", "Asks you 1", "Sign-in 1"; "To review 2") and "says a limit as its rows do…". **Closed (2026-11-06), as amended ("Limit reached n", design review 69):** `components/home-chips.test.tsx` "says a limit as its rows do, and Continue one way" ("Limit reached 1") and "R2: …" ("Limit reached 2" beside "All 7" and "Asks you 1"). |
| R2 | PASS | `home-chips.test.tsx` "puts each need under its chip, and the rest under All only" covers asks, stuck, review, sign-in and This Mac (All only); `homeChips(…, { asks, review })` covers agents' asks and tasks waiting on review. **Closed (2026-11-06):** `home-chips.test.tsx` "R2: a limit and an account almost out of room are under Limit reached; failed, This Mac, heavy work and leftovers under All only". |
| R3 | PASS | "narrows the list and says what it hides, never remembered" asserts "2 more under All" and that clicking it shows All again. Playwright "AJ-E2910: no chip changes the rail…" asserts the rail's text is identical after each chip. **Closed (2026-11-06):** `home-chips.test.tsx` "R3: the chip chosen is forgotten on a new visit, and the bell's count never follows it": Sign-in chosen, the bell still 2; Home mounted again opens on All with every need. The chip is the card's own state and writes nothing to the model, so the badge (the model's count, AJ-E2502 R4) can't follow it either. |
| R4 | GAP | `app-keys.test.tsx` holds the four keys in the registry, never firing while typing ("holds for every shortcut…"), and on the ? sheet ("lists exactly the registered shortcuts…"). **Closed (2026-11-06):** `ui/src/screens/home.test.tsx` "AJ-E2910 R4 — r, h, s and e on the focused row": r opens the asking row's page; h opens its page (at Continue in another app…) only with a conversation, does nothing without one, and that row's ⋯ greys Continue with "It has no conversation to carry over yet"; s opens the row's menu and Snooze for an hour snoozes it; e archives an ended row with Undo and does nothing on a live one. **Missing (a design call):** on a live row e does nothing, but its ⋯ doesn't say why: a live session's menu leaves Archive out rather than greying it with a reason. Showing a greyed Archive on live sessions changes every session menu; left for the maintainer. |
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

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.
