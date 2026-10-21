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
  Reopen (`app_relaunch`, in its own `app_relaunch.rs`) relaunches the app
  (`AppHandle::restart`):
  - with sessions live, it first says how many and what reopening does to
    them, offering Reopen anyway or Not now;
  - a switch or a bring-in in progress makes it wait.

  This uses the updater's `busy_at`. AJ-E1503's quit plan is on
  release/0.1 only, and should replace it when that branch lands.
- **After the relaunch:** first run comes back at the access step (a
  one-time marker read at start).
- **The step re-checks** when the window regains focus, as well as on the
  timer.
- **The logic is a pure module,** `ui/src/lib/osAccess.ts`, so release/0.1's
  `OsAccessRows.tsx` can reuse it. First run's step only wires it up.
- **Surfaces:** "Computer access" in first run is the only place this
  permission is shown on `main`; Settings has no Privacy entry for it yet.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the state machine | `cargo test -p juggler-app --lib -- os_access` (not asked → prompt; asked, no grant → `not_allowed` + `reopen` + the note; granted → `allowed`) | each case | 4 passed (with the existing 2) | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R2 the reopen asks first | `cargo test -p juggler-app --lib -- app_relaunch`: `reopening_asks_first_when_sessions_are_live_and_waits_for_a_switch` (none → relaunch; 1/2 live → hold, can override; anyway → relaunch; switch/bring-in → hold, can't override) | each case | passed | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R3 the card's states | `pnpm --dir ui exec vitest run src/screens/first-run-access-reopen.test.tsx`: the note and Reopen, never Allowed; focus re-checks at once (sooner than the 1.5 s poll); Reopen marks the resume; live sessions said first, Reopen anyway / Not now; a switch makes it wait; the marker is read once | 6 pass; fail on the old step | 6 passed; on the old step 5 failed (the marker test is pure) | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R4 gallery and axe | `pnpm exec playwright test tests/visual/journey-first-run.spec.ts -g reopen`, dark and light, started by the resume marker → [reopen offered](docs/qa/evidence/_gallery/66-first-run/access-reopen-1280x800-dark.png), [live sessions said](docs/qa/evidence/_gallery/66-first-run/access-reopen-live-sessions-1280x800-dark.png) | axe 0 critical/serious | 2 passed | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R3b the pure state machine | `pnpm --dir ui exec vitest run src/lib/osAccess.test.ts`: not asked → Allow; asked, no grant → Reopen + System Settings, never Allowed; granted → Allowed; re-check on focus; resume marker | 6 pass | 6 passed | PASS | SHA_TBD | 2026-10-21 | claude-opus |
| R5 no TCC grant in tests | every test uses the bridge's seams or the pure functions; no test calls `ScreenCaptureAccess`, and no Allow was pressed | none | none | PASS | SHA_TBD | 2026-10-21 | claude-opus |

## Runway: "100% left" for every account in use (item A)

**Report.** Home's Runway, and every Accounts card's "5-hour window" and
"This week", read "100% left · resets …" with full bars for every Claude
account in active use. The reset times differed per account.

**Where it came from:**
- **The engine** (`juggler-core::quota::evaluate_window`): with two or
  more token events and no Measured sample, the window is `Tracking` with
  `percentage: None`. That is honest: a percentage needs a sample or a
  confirmed cap. The status-line spool, which gives samples, runs only for
  sessions AI Juggler launches.
- **The reset** ("resets Wed 03:07 PM"): with no sample it came from
  `estimated_reset`, the earliest use in the rolling window plus the
  window's width. Per account, that is when that account was first used,
  hence the differing week resets. It was shown as if the vendor had said
  it.
- **The UI**: `100 - (window.percentage ?? 0)` in
  `AccountRunway.windowLine` (and the ring's `pct ?? 100`) read a missing
  percentage as 0% used, so "100% left" and a full bar. The New session
  chip showed a red "blocked".

**Fix, at both ends:**
- **Backend:**
  - `WindowReading.reset_basis` is Measured, Calendar or Estimated;
  - a window with no percentage reaches the UI as `unmeasured`, never a
    known state;
  - `reset_basis` travels with every reset.
- **UI:** `ui/src/lib/quotaWindow.ts` is one rule for every surface: Home,
  the Accounts cards, Usage, the inspector's rings, the New session chip
  and the limit dialog.
  - "% left" and a filled bar come only from a real percentage.
  - Otherwise: "Not measured yet", the tokens used in the window, and a
    dashed empty bar.
  - An estimated reset says "about …, estimated".
- **Not changed here:** "Nothing running" on the cards while sessions run
  outside AI Juggler is AJ-E1529 (release/0.1).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| A1 the backend never reports a known state without a sample | `cargo test -p juggler-app --test quota_display`: `use_without_a_sample_is_unmeasured_and_its_reset_is_an_estimate` (token events, no sample → `unmeasured`, no percentage, no badge, 12,000 tokens, `reset_basis: estimated`); the Measured case has `reset_basis: measured` | pass | 6 passed | PASS | add9ffeb | 2026-10-21 | claude-opus |
| A2 no "% left" on any surface | `pnpm --dir ui exec vitest run src/quota-unmeasured-surfaces.test.tsx`: one account with tokens and no sample on Home's Runway, the Accounts card (bars at 0%), Usage, the inspector's rings (no SVG ring) and the New session chip | no "% left" anywhere; fails on the old components | 4 passed; on the old components 3 failed (the 4th checks the new module) | PASS | add9ffeb | 2026-10-21 | claude-opus |

## Codex limits from its own rollouts (item B)

**Finding.** Codex writes `rate_limits` on its rollout `event_msg`
`token_count` lines: `limit_id`, plus `primary` and `secondary` windows
`{used_percent, window_minutes, resets_at}` (secondary nullable), plus
`credits {has_credits, unlimited, balance}`. The shape was checked on the
maintainer's real rollouts by key names only. It is recorded in
`docs/research/codex.md` §5, graded local-observable (AJ-E601).

**Ingest:**
- `quota_ingest::codex_rate_limits` turns each window into a Measured
  sample, its pool taken from `window_minutes` (300 or 10080; any other
  width is skipped, never guessed);
- the newest `credits` snapshot is kept;
- the pipeline now keeps rollout samples (it dropped them before), and the
  account card shows the credit balance;
- Codex declares `USAGE_QUOTA_FEED`, so `reports_limits` is true;
- `auth.json` is never read.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| B1 parse | `cargo test -p juggler-adapters --test quota_ingest` (`fixtures/quota/codex-rollout-rate-limits.jsonl`: the real line's shape, invented values): primary + secondary + `resets_in_seconds`, newest credits, an odd width counted, null `rate_limits` skipped; seconds or ms | pass | 18 passed | PASS | 9dc5bdbd | 2026-10-21 | claude-opus |
| B2 end to end | `cargo test -p juggler-app --test usage_pipeline`: a Codex home with that rollout → stored samples (two 5-hour, one weekly) → the display measured (20 % and 40 %, `reset_basis: measured`) with credits 16.25 | pass | 2 passed | PASS | 9dc5bdbd | 2026-10-21 | claude-opus |
| B3 reports its limits | `cargo test -p juggler-app --test codex_in_place providers_capabilities_says_which_tools_report_their_limits` (Codex true); vitest `accounts-no-limits.test.tsx` (no "doesn't report" line for Codex; the line still shows for a tool that doesn't) | pass | passed | PASS | 9dc5bdbd | 2026-10-21 | claude-opus |

## Home under memory pressure (items 1–4)

**Report** (a screenshot of Home):
1. The global banner overlapped and clipped Home's headline.
2. The alert showed three times: the banner, the headline and Needs you.
3. The figures disagreed: the banner said 4.2 GB, Needs you 8.1 GB, and
   This Mac listed three Claude app rows.
4. The words disagreed: the alert said "under memory pressure", while This
   Mac said "Getting tight".

**Fix:**
1. **The banner** and the screen are one flex column. The banner takes its
   own space and pushes the screen down.
2. **Shown once on Home**, in Needs you. Home's headline only points there
   ("Your Mac needs a look"), and the banner keeps the alert for the other
   screens. `lib/machine.alertWords` is the one source of its words.
3. **One figure:** the alert names a tool AI Juggler didn't start as a
   whole, with its windows' total and their count ("The Claude app is
   using about 16 GB across 3 windows").
   - Ported unchanged from #85 (`claude/e14-menu-bar`), so #85 rebases
     clean: `ToolGroupInfo`, `by_tool`, `one_core`, `TreeInput.provider`,
     `provider_name`, `machine_alerts::candidates` and its tests, and the
     preview's `by_tool`.
   - New here: `subject_windows`.
4. **One severity:** while the memory-pressure alert is on, This Mac says
   "Under memory pressure" (`align_with_alerts`). "Getting tight" is the
   calmer state before it.

The "See what's using it" target is unchanged; it becomes a new page on
release/0.1.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| M1 the banner takes its own space | `pnpm exec playwright test tests/visual/overload-gallery.spec.ts` "the banner above a session", 1280 and 1024, dark and light: the banner's bottom ≤ the screen's top → [1024 dark](docs/qa/evidence/_gallery/74-overload/banner-above-session-1024x800-dark.png) | no overlap; axe 0 critical/serious | 10 passed | PASS | 97547c38 | 2026-10-21 | claude-opus |
| M2 once on Home | same spec "memory pressure": no banner on Home, the headline "Your Mac needs a look", the title once; vitest `this-mac.test.tsx` (Needs you once; the region hides machine alerts on Home and shows them elsewhere) | pass | passed (vitest 18) | PASS | 97547c38 | 2026-10-21 | claude-opus |
| M3 one figure | `cargo test -p juggler-app --lib -- machine_alerts`: `the_claude_app_is_named_by_all_its_windows_never_one` (#85) and `the_alert_says_how_many_windows_its_figure_sums` (3 windows, 16.1 GB; a session of ours has no count); vitest `alertSubject` → "The Claude app is using about 16 GB across 3 windows" | pass | 15 passed | PASS | 97547c38 | 2026-10-21 | claude-opus |
| M4 one severity | `cargo test -p juggler-app --lib -- machine_health`: `this_mac_says_what_the_pressure_alert_says` ("Getting tight" before; "Under memory pressure" with the alert, the reason at pressure) | pass | passed | PASS | 97547c38 | 2026-10-21 | claude-opus |

## Gates

GATES_TBD
