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
| R1 the state machine | `cargo test -p juggler-app --lib -- os_access` (not asked → prompt; asked, no grant → `not_allowed` + `reopen` + the note; granted → `allowed`) | each case | 4 passed (with the existing 2) | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |
| R2 the reopen asks first | `cargo test -p juggler-app --lib -- app_relaunch`: `reopening_asks_first_when_sessions_are_live_and_waits_for_a_switch` (none → relaunch; 1/2 live → hold, can override; anyway → relaunch; switch/bring-in → hold, can't override) | each case | passed | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |
| R3 the card's states | `pnpm --dir ui exec vitest run src/screens/first-run-access-reopen.test.tsx`: the note and Reopen, never Allowed; focus re-checks at once (sooner than the 1.5 s poll); Reopen marks the resume; live sessions said first, Reopen anyway / Not now; a switch makes it wait; the marker is read once | 6 pass; fail on the old step | 6 passed; on the old step 5 failed (the marker test is pure) | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |
| R4 gallery and axe | `pnpm exec playwright test tests/visual/journey-first-run.spec.ts -g reopen`, dark and light, started by the resume marker → [reopen offered](docs/qa/evidence/_gallery/66-first-run/access-reopen-1280x800-dark.png), [live sessions said](docs/qa/evidence/_gallery/66-first-run/access-reopen-live-sessions-1280x800-dark.png) | axe 0 critical/serious | 2 passed | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |
| R3b the pure state machine | `pnpm --dir ui exec vitest run src/lib/osAccess.test.ts`: not asked → Allow; asked, no grant → Reopen + System Settings, never Allowed; granted → Allowed; re-check on focus; resume marker | 6 pass | 6 passed | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |
| R5 no TCC grant in tests | every test uses the bridge's seams or the pure functions; no test calls `ScreenCaptureAccess`, and no Allow was pressed | none | none | PASS | b8207517, d966302b | 2026-10-21 | claude-opus |

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
  - `WindowReading.reset_basis` says where a reset came from (Measured
    or Calendar; the Estimated basis was dropped by P12 below);
  - a window with no percentage reaches the UI as `unmeasured`, never a
    known state;
  - `reset_basis` travels with every reset.
- **UI:** `ui/src/lib/quotaWindow.ts` is one rule for every surface: Home,
  the Accounts cards, Usage, the inspector's rings, the New session chip
  and the limit dialog.
  - "% left" and a filled bar come only from a real percentage.
  - Otherwise: "Not measured yet", the tokens used in the window, and a
    dashed empty bar.
  - A reset shows only when one is known (P12 below).
- **Not changed here:** "Nothing running" on the cards while sessions run
  outside AI Juggler is AJ-E1529 (release/0.1).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| A1 the backend never reports a known state without a sample | `cargo test -p juggler-app --test quota_display`: `use_without_a_sample_is_unmeasured_and_has_no_reset_time` (token events, no sample → `unmeasured`, no percentage, no badge, 12,000 tokens, no reset since P12); the Measured case has `reset_basis: measured` | pass | 6 passed | PASS | add9ffeb, 003680c5 | 2026-10-21 | claude-opus |
| A2 no "% left" on any surface | `pnpm --dir ui exec vitest run src/quota-unmeasured-surfaces.test.tsx`: one account with tokens and no sample on Home's Runway, the Accounts card (bars at 0%), Usage, the inspector's rings (no SVG ring) and the New session chip | no "% left" anywhere; fails on the old components | 4 passed; on the old components 3 failed (the 4th checks the new module) | PASS | add9ffeb | 2026-10-21 | claude-opus |
| A3 one runway, serde's payload | `ui/src/screens/Home.tsx` keeps no runway logic of its own; `runwayOf` moved into `components/AccountRunway.tsx` beside `runwayLines` (one session fallback, `lib/quotaWindow`'s `measured()`). `pnpm --dir ui exec vitest run src/quota-unmeasured-surfaces.test.tsx -t serde` (`state: "tracking"`, `percentage: null`, as main's backend sent it, on Home and Accounts). UI sweep of every `!== undefined` and `?? 0`: the update download showed "0%" and the live preview's alt text "captured 0s ago" when the value was unknown, both fixed; the rest are sort keys, counts hidden at zero, or non-wire values | no "% left"; fails on main's Home | passed; with main's `Home.tsx` swapped in it fails ("100% left") | PASS | 30e80a43 | 2026-10-21 | claude-opus |

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

### Which reading, and its "as of" (item B, QA's rules)

**Found by** QA, on 30 days of the real Codex account's `rate_limits`:
74,874 readings, analysed in the walk's Batch 5 on
`claude/qa-real-walk-be289a`.

**What the readings showed:**
- The newest line is not the newest value: a concurrent thread reports a
  point lower.
- A thread resumed days later reports its old window (seen up to 166 h
  later).
- `resets_at` jitters by up to 75 s inside one window.

**Fix (engine, for any tool's readings):**
- `quota::current_reading` takes the window with the latest `resets_at`
  (within an hour is one window), and the highest `used_pct` inside it.
- Once that reading's reset has passed with nothing newer, the window is
  `reset`: "Not measured since the reset", with no percentage and no reset
  time.
- Every measured window carries `measured_at_ms`, shown as "as of …".
- Credits name their source, sent by the backend.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| B4 the reading chosen | `cargo test -p juggler-core --test quota_readings` (a concurrent lower line; a resumed thread's old window, "88%, resets 10-24" against "1%, resets 10-27"; a 75 s jitter; no reset times) | each rule | 5 passed | PASS | 71ede0bc | 2026-10-21 | claude-opus |
| B5 nothing after the reset | same suite `after_the_reset_with_no_new_reading_nothing_is_known`; `cargo test -p juggler-app --test quota_display a_window_past_its_reset_with_no_new_reading_is_reset` (state `reset`, no percentage, no reset, the old as-of) | pass | passed | PASS | 71ede0bc | 2026-10-21 | claude-opus |
| B6 odd readings | `cargo test -p juggler-adapters --test quota_ingest codex_odd_readings_are_ignored_and_a_null_plan_is_fine` (`window_minutes` 0 and a null `primary` give no sample, no parse error; a null plan is fine). This pins existing behaviour | pass | passed | PASS | 71ede0bc | 2026-10-21 | claude-opus |
| B7 as-of and credits in words | `pnpm --dir ui exec vitest run src/lib/quotaWindow.test.ts` ("as of just now / 12 min ago / a time / a date", "36% left · as of 3 min ago", "Not measured since the reset", "Credits: 16.25 · reported by Codex, as of 5 min ago"); the pipeline test's credits `source` "Codex" and `measured_at_ms` | pass | 4 passed | PASS | 71ede0bc | 2026-10-21 | claude-opus |
| B8 by hand | the preview's Accounts screen with a measured weekly window, credits and a window past its reset | reads as above | "36% left · resets Sat 04:56 PM · as of 12 min ago"; "Credits: 16.25 · reported by Codex, as of 12 min ago"; "Not measured since the reset" | PASS | 71ede0bc | 2026-10-21 | claude-opus |

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
| M5 every surface agrees (P19) | `pnpm --dir ui exec vitest run src/machine-state-agreement.test.tsx`: one fixture per state (plenty, tight, pressure with the memory alert, unsampled); Needs you and its headline, This Mac, the rail footer (`MachineStatusLine`), the collapsed dot and "All clear" all from `lib/machine.machineStatus`. By hand in the preview (`?aij_machine=pressure&aij_alert=memory`): Needs you "Your Mac is under memory pressure", This Mac "Under memory pressure", footer "This Mac · Under memory pressure", no "healthy" | agree in every state | 4 passed; by hand as stated | PASS | dcb9bc8b | 2026-10-21 | claude-opus |
| P18 one "doesn't report" | `cargo test -p juggler-app --test codex_in_place` (the display's `commands::reports_limits` equals the provider list's `reports_limits` for every registered tool); vitest `home.test.tsx` (the reason from the backend's display, no "% left") | agree | passed | PASS | 20cd1ffd | 2026-10-21 | claude-opus |
| M4 one severity | `cargo test -p juggler-app --lib -- machine_health`: `this_mac_says_what_the_pressure_alert_says` ("Getting tight" before; "Under memory pressure" with the alert, the reason at pressure) | pass | passed | PASS | 97547c38 | 2026-10-21 | claude-opus |

## Usage blanked the whole window

**Report.** Clicking Usage on 0.0.12 showed a blank window, rail and header
gone. QA reproduced it on every real install with no fee set.

**Cause:**
- Every Rust `Option` in the cost payload arrives as `null`: the
  subscription view's `api_equivalent_usd`, `flat_fee_usd` and
  `value_ratio`, a row's `credits` and `usd.usd`, and `total_usd`.
- The hand-written TS types said `?: number`, and `CostMatrix` checked
  `!== undefined`, so `null.toFixed` threw.
- With no error boundary, the whole React tree unmounted.
- The preview mock sent `subscription: {}` (missing keys, not nulls), so
  no test saw it.

**Fix:**
- **One convention (rule I6, `docs/engineering/ipc-nulls.md`):** an
  `Option` is `null` on the wire, typed `T | null`, checked `!= null`.
- **Truthful bridge types:** the cost types plus `WindowDisplay`,
  `ForecastDisplay`, `QuotaDisplayData`, `RankedRowData`,
  `SwitchTargetData` and `CreditsDisplay` now mirror Rust. The compiler
  then found every unguarded site.
- **Mocks** send what serde sends, via the builders in `ui/src/lib/wire.ts`.
- **Unpriced model:** reads "No price for <model> yet", never $0.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| U1 null payloads render | `pnpm --dir ui exec vitest run src/usage-null-payload.test.tsx`: Usage and CostMatrix fed the maintainer's shape (no cost rows, an unpriced model, Codex credits null, value ratio null) | pass; fails on the old code | 3 passed; all 3 crashed on the old code | PASS | ed2e3833 | 2026-10-21 | claude-opus |
| U2 the convention is written down | `docs/engineering/ipc-nulls.md` (I6; to fold into the guide's C.4 when #101 lands); the lint is noted for AJ-E2012 | present | present | PASS | ed2e3833 | 2026-10-21 | claude-opus |

## A screen's error no longer blanks the window

**Fix:**
- **Boundaries:** `ScreenBoundary` wraps each screen (Home, Accounts,
  Usage, History, Settings, Fleet, Cleanup, This Mac, the inspector),
  plus a root boundary in `main.tsx`.
- **The panel** reads "This screen hit a problem", with Try again and
  Copy details. The rail and header keep working.
- **The error** goes to `crash_ui_error`: the screen's name, the message
  and the stack, scrubbed, and kept only if crash reports are on.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| E1 a throwing screen shows its panel | `pnpm --dir ui exec vitest run src/screen-boundary.test.tsx`: Usage throws `null.toFixed` → the panel; Home and the rail still there and working; the report carries the screen and the error only; Try again re-renders; Copy details copies the error | pass | 3 passed | PASS | 79f7d980 | 2026-10-21 | claude-opus |
| E2 the report respects the setting | `cargo test -p juggler-app --lib -- crash`: `a_screen_error_is_kept_like_a_panic_and_only_when_allowed` (scrubbed, kept only when crash reports are on) | pass | passed | PASS | 79f7d980 | 2026-10-21 | claude-opus |

## Tokens counted more than once (P3, P14)

**Found by** QA's oracle (`scripts/qa/real_truth.py` on
`claude/qa-real-walk-be289a`):
- Claude tokens were ×1.8–2.5 the transcripts' on every account in use.
- Codex tokens were exactly ×2.

**Causes:**
- **P3 (Claude):** Claude Code writes one transcript line per content
  block (thinking, text, each tool call). Each line has its own `uuid`, but
  all carry the same `message.id`, `requestId` and `usage`. The parser
  keyed events on the line `uuid`, so one response counted once per block.
  - Not the cause: the status-line spool carries no token events, and
    sidechain lines carry their own message ids.
- **P14 (Codex):** Codex's `input_tokens` includes `cached_input_tokens`,
  and its `output_tokens` includes `reasoning_output_tokens`. The four
  were stored as separate classes, which counted each token twice.
  - The inspector's rollout scan counted reasoning twice too.

**Fix:**
- **Claude events** are keyed `message.id/requestId`, falling back to the
  `uuid`. A response's blocks in one slice collapse to one event, each
  class taking its largest value.
- **Codex classes** are stored disjoint: input without the cached part,
  output without the reasoning. This follows Codex's own
  `non_cached_input()` and its total of input + output. Reasoning is priced
  at the output rate.
- **Existing installs:** a versioned one-time re-read reads every
  transcript and rollout still on disk from the start, and replaces its
  sessions' rows in the same pass.
  - Sessions whose files are gone keep their rows.
  - The re-read isn't counted as new use.
- **P15(a):** Codex usage now carries the model its `turn_context` names,
  carried across tail slices per file.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| T1 one response, one event | `cargo test -p juggler-adapters --test quota_ingest`: `one_response_written_as_several_content_blocks_counts_once` (`fixtures/quota/claude-transcript-content-blocks.jsonl`, shaped like the real lines, invented values: 3 + 2 blocks → 2 events, sums 10/362/24,800/890), `a_response_read_twice_is_stored_once` | pass; fails on the old parser | 22 passed | PASS | 84ff8f00 | 2026-10-21 | claude-opus |
| T2 Codex tokens once | same suite: `codex_tokens_are_counted_once` (classes sum to Codex's `total_tokens`); `cargo test -p juggler-app --lib rollout_scan` `the_quality_scan_counts_each_codex_token_once`; `cargo test -p juggler-core --test cost_ledger reasoning_tokens_are_priced_as_output` | pass | passed | PASS | 84ff8f00 | 2026-10-21 | claude-opus |
| T3 stored rows corrected once | `cargo test -p juggler-app --test usage_pipeline rows_counted_per_content_block_are_read_again_once` (0.0.12's per-line rows + offsets at the end → one pass leaves one row per response; a gone file's rows stay; the next pass changes nothing) | pass; fails on the old code | passed; on the old source it failed (5 rows) | PASS | 84ff8f00 | 2026-10-21 | claude-opus |
| T4 Codex model | `codex_usage_takes_the_model_its_turn_context_names` (adapters) and `codex_usage_keeps_its_turns_model_across_passes` (pipeline: a slice without its own `turn_context` keeps the file's model; tokens sum to 120) | pass; fails on the old code | passed; the pipeline case failed on the old source | PASS | 84ff8f00 | 2026-10-21 | claude-opus |

## Reset times only when known (P12)

**Found by** QA: with no vendor sample, every Claude window's `resets_at`
was the first usage event in the rolling window + 5 h or 7 d. Under
steady use, that is "now".

**Fix (D-06):** a reset time comes only from a measured source, or from an
anchored weekly schedule (`quota::known_reset`):
- a measured source is a status-line sample, or Codex's `rate_limits`;
- the anchored schedule is a settings key that nothing writes today.

Otherwise there is no reset time. `ResetBasis::Estimated` and the UI's
"resets about …, estimated" are gone.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| P12a no invented reset | `cargo test -p juggler-app --test quota_display use_without_a_sample_is_unmeasured_and_has_no_reset_time` (events over the last 30 min, no sample → `resets_at_ms` and `reset_basis` null on both windows); the measured case keeps the vendor's | pass; fails on the old engine | passed; the old engine returned an estimate here, so the null asserts fail (by reading; not re-run) | PASS | 003680c5 | 2026-10-21 | claude-opus |
| P12b every surface | `pnpm --dir ui exec vitest run src/quota-unmeasured-surfaces.test.tsx` ("Not measured yet · 12k tokens used", no "resets" on the card) | pass | 4 passed | PASS | 003680c5 | 2026-10-21 | claude-opus |

## Codex windows as Codex reports them (P13)

**Found by** QA: Codex was given a five-hour window, but the maintainer's
rollouts report only a weekly one (`primary` `window_minutes` 10080,
`secondary` null).

**Fix:**
- **Shown:** a Codex window appears when Codex reported it in the last
  week, as a Measured sample whose `window_minutes` names it.
- **Not reported:** a window Codex doesn't report is omitted, with "Codex
  reports no five-hour limit for this account".
- **Before any reading:** both windows say "Codex hasn't reported its
  limits yet — they show after you use Codex".
- The plan rule ("Pro has no five-hour limit") is gone: the reading
  decides.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| P13 windows from the reading | `cargo test -p juggler-app --test quota_display codex_shows_only_the_windows_it_reports` (no samples → both omitted with the reason; a weekly sample only → five-hour omitted "reports no five-hour limit", weekly 69 % measured) | pass; fails on the old plan rule | passed; the old rule gave a Plus account both windows with no reading (by reading; not re-run) | PASS | 74c35644 | 2026-10-21 | claude-opus |

## Prices for the models in use (P15)

**Found by** QA: the seed priced only `claude-opus-4` and
`claude-sonnet-4`, and 297 of 452 rows (Codex) had no model.

**Fix:**
- **Seed v3** carries the current Claude models and the GPT-5 family plus
  `gpt-5.3-codex`, from each vendor's own pricing page, accessed
  2026-10-21.
- **Record:** `docs/research/model-pricing.md` (AJ-E810, Verified-primary)
  records the prices and what isn't modelled.
- **Dated IDs:** a dated ID is priced through its dateless alias.
- **(a) Codex model:** read from `turn_context` (T4 above).
- **(c) Unknown model:** stays "No price for <model> yet" (U1).
- **(d) The pricing-update path:** written up as a follow-up in the
  research doc.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| P15 the models in use are priced | `cargo test -p juggler-core --test cost_ledger the_models_in_real_use_are_priced` (the six Claude IDs QA saw, the dated Haiku through its alias, gpt-5.5, gpt-5.3-codex; Opus 5.5 at 4/20/0.20/5 per MTok = $29.20 for 1M of each; an unknown model unpriced) | pass; fails on the old seed | passed; the old seed has none of these models (by reading; not re-run) | PASS | f1043f1a | 2026-10-21 | claude-opus |

## Gates

Local gates, run on this Mac (CARGO_TARGET_DIR shared):

| Gate | Command | Head | Result |
|---|---|---|---|
| verify | `just verify` (fmt, clippy with and without default features, every test, the leak checks, the release self-tests) | 8eafeb8c | EXIT 0, 142 suites ok |
| assertion-message change | `cargo test -p juggler-adapters --test quota_ingest`; `-p juggler-app --test usage_pipeline --test quota_display`; clippy `--tests` | 88d0bd6b | 23, 4 and 7 passed; clean |
| deny | `cargo deny check` (no dependency changes on this branch) | 66d10fd8 | advisories, bans, licenses, sources ok |
| vitest | `pnpm --dir ui exec vitest run`; `tsc --noEmit` | 71ede0bc | 590 passed (2 timing tests failed once at load average 43, then passed alone twice); tsc clean |
| a11y | `just a11y-gate` (axe on every gallery state, keyboard flows, zoom and motion) | 8eafeb8c | 682 passed; 0 critical, serious, moderate or minor: PASS |
| visual | `just visual-gate` (pinned Playwright container) | 8eafeb8c | 141 passed |
| by hand | the preview's Accounts screen (the B8 row) | 71ede0bc | as described |

**Two runs needed attention:**
- The first verify run stopped on clippy (`doc_markdown`, then `too_many_lines`); both are fixed.
- The `ports.rs` failures came from this session's preview server holding port 5173, which the test binds. With the preview stopped, it passes.
