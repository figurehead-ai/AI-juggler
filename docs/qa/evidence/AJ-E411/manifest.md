# Evidence manifest — AJ-E411 (Subscription-expiry warning hook, D-12)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E411.
Branch: `stage2/e4-renewal-expiry`. Local runs (macOS arm64) 2026-10-14,
reviewer zcode-primary. Rust suites run in CI (verify job); component tests
in the ui job.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 field + additive migration | `cargo test -p juggler-core --test storage_roundtrip` (`account_renewal_date_roundtrip_and_default_none` + version asserts) | additive, serialisation-compatible | schema v8 `ALTER TABLE accounts ADD COLUMN renewal_date TEXT` (nullable); v4 DB upgrades to v8 exactly once on open (asserted, plus reopen no-op); rows stored without the field load as `None`; set/clear round-trips `2026-11-02` | PASS | 2026-10-14 | zcode-primary |
| Req.2 unknown ≠ guessed | `cargo test -p juggler-core --lib renewal` (`renewal_no_date_is_unknown_never_guessed`) + UI test | no date → `Expiry: unknown`, nothing on the row | `renewal_label(None) == None` asserted; malformed strings also `None` (honest rejection); inspector renders `Expiry: unknown` (component test); Accounts row renders no badge for unknown (capture + row-visibility unit test); NO inference path exists anywhere (no plan-tier/usage inputs to the label fn by construction) | PASS | 2026-10-14 | zcode-primary |
| Req.3 thresholds | same Rust suite + `pnpm --dir ui test` (renewal) | exact copy at every boundary | Rust fixed-clock tests: >14d → `Renews <date>` (date only); 14d and 8d → `Renews in N days`; 7d and 1d → warning `Renews in N days` (row-visible); 0d and −1d → danger `Renewal date passed`; TS mirror asserts the identical strings — 10/10 Rust + UI boundary suite green | PASS | 2026-10-14 | zcode-primary |
| Req.4 editor + Actions entry | `pnpm --dir ui test` (renewal) | labelled date input + Clear + helper; `Edit renewal date...` from Actions | Account Overview hosts `Renewal date (optional)` (`<label for>` + `input[type=date]` asserted), Clear button, helper `Entered by you - Juggler does not read billing`; Account Actions offers `Edit renewal date...` which dispatches `aij:edit-renewal` → inspector switches to Overview and the editor opens the picker (listeners asserted) | PASS | 2026-10-14 | zcode-primary |
| Req.5 Needs-me tie-break | UI tests (renewal › Needs-me) | ≤7d only; below the four live states; never outranks | equal-severity peers: the row on an expiring account (5d) sorts first (asserted); a `failed` agent still outranks it (asserted); tiers outside ≤7d never participate (`renewalRowVisible` false at 8d, asserted) | PASS | 2026-10-14 | zcode-primary |
| Req.6 research hook + D-12 note | `cargo test -p juggler-core --lib renewal` (`renewal_source_seam_returns_unknown_today`) + doc review | documented seam returning `Unknown`; research note names the gate | `RenewalSource` trait + `UnknownRenewalSource` (`Ok(None)` always) shipped and tested; `docs/research/subscription-expiry-sources.md` created with the D-12 gate, candidate audit, and new-task revisit trigger | PASS | 2026-10-14 | zcode-primary |
| Req.7 local dates + no correction | Rust suite (`renewal_calendar_dates_parse_strictly`, `renewal_yesterday_is_danger_and_never_corrected`) + TS local-date math | strict `YYYY-MM-DD`, leap-aware; past dates warn immediately | parser rejects `2026-02-29` (non-leap), `2026-13-01`, datetime suffixes; accepts `2028-02-29`; −1d renders danger `Renewal date passed` (never "corrected"); TS side computes against the LOCAL calendar date (`getFullYear/getMonth/getDate`, no timezone conversion) | PASS | 2026-10-14 | zcode-primary |
| Command validation | `cargo test -p juggler-app --test renewal` | invalid input rejected, never repaired | `account_set_renewal` rejects `2026/11/02`, `11-02-2026`, `2026-13-01`, `2026-02-30`, `not-a-date`, datetime suffixes with the `YYYY-MM-DD` message; valid set/clear round-trips through storage; unknown account errors naming the id — 2/2 | PASS | 2026-10-14 | zcode-primary |
| Once-per-day announce | UI test (renewal › announcement) | polite, once per local day, not every tick | `RenewalAnnouncer` announces `Work Max renewal: Renews in 3 days` on the crossing render, stays silent on same-day re-renders (asserted), and never announces far-future accounts; `aria-live="polite"` + `sr-only` | PASS | 2026-10-14 | zcode-primary |
| Performance | by construction | zero new polling; <1ms/Account | the label is computed on the existing accounts refresh + on edit (no new tick/thread/watcher — code-reviewed); the pure fn is O(1) date arithmetic (10 boundary tests run in 0.00s) | PASS | 2026-10-14 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | every threshold state + editor + row chip | 86/86 incl. new `11-renewal/` (account date-only / countdown / warning amber / passed danger / `Expiry: unknown`, editor 1440×900 + 1280×800, Accounts row chip); dates pinned to T0 so tiers are deterministic; a11y 3/3 | PASS | 2026-10-14 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings`, workspace tests, docs, evidence-check, terminology); 119/119 component (111 prior + 8 new); `pnpm --dir ui build` green | PASS | 2026-10-14 | zcode-primary |

## Notes for the next agent

- The threshold logic lives twice **by design**: `renewal.rs` (Rust, the
  canonical seam + research hook) and `src/lib/renewal.ts` (the UI
  mirror, computed against the LOCAL calendar date — the reason the
  label is not computed backend-side). Both boundary suites must stay
  in lockstep; change one, change both, in the same PR.
- Needs-me tie-break: `sortSessions(…, accounts)` — accounts thread
  App → WorkflowRail → InstanceBoard. Expiry ranks only among
  equal-severity peers (after `severityRank || snoozed || waiting`),
  so it can never outrank a blocked agent.
- `AppError::Validation { subject, message }` is new (first user-input
  validation surface) — reuse it for future editor validation instead
  of ad-hoc string errors.
- AJ-E412 (running-model telemetry) is the last E4 task; its
  `model_source` work touches the same InstanceInfo wire shape.
