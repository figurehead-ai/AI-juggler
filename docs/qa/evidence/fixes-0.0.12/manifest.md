# Evidence manifest — fixes for the maintainer's 0.0.12 reports

Not a task's evidence: one section per bug the maintainer reported while
dogfooding 0.0.12, with the fix's proof (branch `fix/maintainer-reports-2`).

## Screen Recording never turns to "Allowed" after System Settings

**Report.** In first run's "Computer access" step:
- the maintainer clicked Open System Settings for "See a live preview of
  each session" (Screen Recording) and allowed it there;
- the card never turned to "Allowed";
- Accessibility and Automation did.

**Cause** (`crates/juggler-app/src/os_access.rs`):
- The status comes from `CGPreflightScreenCaptureAccess`
  (`core_graphics::access::ScreenCaptureAccess.preflight()`).
- macOS answers that check once per process: a grant made in System
  Settings shows only after the app relaunches, which System Settings
  itself prompts for.
- Polling, which the step already did every 1.5 s, can't see it.

**Fix:**
- **The entry:** once AI Juggler has asked for Screen Recording and the
  OS still says no, it offers `reopen` with the words "Allowed it in System
  Settings? macOS needs AI Juggler to reopen to see it" (`preview_state`).
  Its status stays `not_allowed`: "Allowed" is shown only when the OS says
  so.
- **The card:** shows Open System Settings and **Reopen AI Juggler**.
  Reopen (`os_access_reopen`) relaunches the app (`AppHandle::restart`):
  - with sessions live, it first says how many and what reopening does to
    them, offering Reopen anyway or Not now;
  - a switch or a bring-in in progress makes it wait.

  This uses the updater's `busy_at`. AJ-E1503's quit plan is on
  release/0.1 only, and should replace it when that branch lands.
- **After the relaunch:** first run comes back at the access step (a
  one-time marker read at start).
- **The step re-checks** when the window regains focus, as well as on the
  timer.
- **Surfaces:** "Computer access" in first run is the only place this
  permission is shown on `main`; Settings has no Privacy entry for it yet.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the state machine | `cargo test -p juggler-app --lib -- os_access` (not asked → prompt; asked, no grant → `not_allowed` + `reopen` + the note; granted → `allowed`) | each case | 4 passed (with the existing 2) | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R2 the reopen asks first | same suite: `reopening_asks_first_when_sessions_are_live_and_waits_for_a_switch` (none → relaunch; 1/2 live → hold, can override; anyway → relaunch; switch/bring-in → hold, can't override) | each case | passed | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R3 the card's states | `pnpm --dir ui exec vitest run src/screens/first-run-access-reopen.test.tsx`: the note and Reopen, never Allowed; focus re-checks at once (sooner than the 1.5 s poll); Reopen marks the resume; live sessions said first, Reopen anyway / Not now; a switch makes it wait; the marker is read once | 6 pass; fail on the old step | 6 passed; on the old step 5 failed (the marker test is pure) | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R4 gallery and axe | `pnpm exec playwright test tests/visual/journey-first-run.spec.ts -g reopen`, dark and light, started by the resume marker → [reopen offered](docs/qa/evidence/_gallery/66-first-run/access-reopen-1280x800-dark.png), [live sessions said](docs/qa/evidence/_gallery/66-first-run/access-reopen-live-sessions-1280x800-dark.png) | axe 0 critical/serious | 2 passed | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R5 no TCC grant in tests | every test uses the bridge's seams or the pure functions; no test calls `ScreenCaptureAccess`, and no Allow was pressed | none | none | PASS | SHA_TBD | 2026-10-21 | claude-opus |

## Gates

GATES_TBD
