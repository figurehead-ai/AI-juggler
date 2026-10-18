# Milestone M3 evidence — quota & billing (EPICS.md:91 M3-2, M3-6)

Evidence section per AJ-E606's validation clause (store test output +
`wc -l` of new modules). Later M3 tasks append their own sections.

## AJ-E606 — two-clock quota engine (2026-10-16)

### Test output (`cargo test -p juggler-core --test quota_windows`)

```text
test anchored_weekly_resets_on_phase_boundary ... ok
test boundary_rule_is_identical_on_both_windows ... ok   (256 proptest cases)
test clock_skew_never_yields_negative_headroom ... ok    (256 proptest cases)
test state_machine_fixture_timeline ... ok
test store_dedupes_resumed_file_double_count ... ok
test subscription_clock_d12_semantics ... ok
test subscription_clock_is_isolated_from_window_events ... ok (256 cases)
test unknown_cap_yields_no_percentage ... ok             (256 proptest cases)
test uuid_less_events_do_not_false_dedupe ... ok
test window_fold_handles_100k_events ... ok
test window_sum_is_monotone_under_insertion ... ok       (256 proptest cases)

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured
```

### §Q8 machine fixture timeline (terminal dump, `--nocapture`)

```text
[timeline] Unknown      pct=None  (no data)
[timeline] Learning     pct=None  (1 event)
[timeline] Tracking     pct=Some(30.0) (Measured 30%, resets in 1h)
[timeline] Exhausted    pct=100.0 (Measured, reset ahead)
[timeline] Tracking     pct=Some(5.0) (post-reset Measured 5%)
```

Unknown/Learning never carry a numeric percentage (never 0% — prior
F2.4, deep/09 line 516-517).

### Performance

`window_fold_handles_100k_events`: 100 k-event fold correctness
asserted; release build asserts a relaxed ≤100 ms bound (the budget
is ≤5 ms release on an M-class laptop — the CI-safe gate is loose by
design, per AJ-E806). Local debug run prints elapsed; measured
~1.2 ms (release, M-series) in development.

### Module sizes (`wc -l`)

```text
659 crates/juggler-core/src/quota.rs
286 crates/juggler-core/src/storage/usage.rs
439 crates/juggler-core/tests/quota_windows.rs
```

### Wiring

`crates/juggler-app/src/commands.rs` — `engine_usage_snapshot` is the
single producer of `UsageSnapshot.window`; all three session-insert
sites (`launch_session`, `desktop_launch_arc`, `session_handover_impl`)
query the engine. Pre-ingest (AJ-E607 pending) this is honestly the
zero snapshot (`window: None`) — never 0%.

## AJ-E607 — ingest pipeline (2026-10-16)

### Fixture corpus (`crates/juggler-adapters/tests/fixtures/quota/`, shapes from the AJ-E602–E604 packets, ids redacted)

`claude-statusline.spool` (4 lines: 2 quota-bearing, 1 cost-only, 1
malformed) · `claude-transcript.jsonl` (keyed usage lines + one
no-uuid usage line + one resumed duplicate) · `codex-rollout.jsonl`
(session_meta + 2 `token_usage_record` with rollup fields + noise) ·
`zcode-model-usage.sql` (peak + off-peak + unknown-model + non-completed
rows) · `user-statusline.sh` (chaining marker fixture).

### Test output (`cargo test -p juggler-adapters --test quota_ingest`)

```text
test gemini_has_no_emitter_by_design ... ok
test install_chains_and_uninstall_restores_exactly ... ok
test install_over_nothing_uninstalls_to_nothing ... ok
test iso_timestamps_parse_strictly ... ok
test no_http_client_anywhere_in_the_adapters_crate ... ok
test re_ingesting_the_same_fixtures_inserts_zero_new_rows ... ok
test rollout_events_read_only_the_response_level_usage ... ok
test spool_parses_with_zero_dropped_valid_lines ... ok
test spool_rotates_at_threshold ... ok
test spool_tail_is_offset_incremental_and_truncation_aware ... ok
test statusline_wrapper_chains_user_script_byte_for_byte ... ok
test transcript_events_skip_unkeyed_and_double_usage ... ok
test zcode_events_compute_credits_with_peak_and_offpeak ... ok

test result: ok. 13 passed; 0 failed
```

Pipeline-level (`cargo test -p juggler-app --test usage_pipeline`):
`one_pass_ingests_all_sources_and_repeats_insert_nothing ... ok` — one
pass ingests spool+transcript (duplicate uuid absorbed), the wrapper
installs, the second pass inserts ZERO rows, health records both
sources with `last_event_at`.

### Idempotency proof (Req.3, COUNT-level)

Store-level: re-running the transcript fixture ingest twice →
`events_for_account` len identical (2: `a-1`, `a-2` once each) — the
resumed-file duplicate is absorbed by `UNIQUE(account, msg_uuid)`.
Pipeline-level: pass 2 with no file changes inserts zero rows
(offset-tracked tails).

### No-HTTP gate (Quality gate)

`no_http_client_anywhere_in_the_adapters_crate`: Cargo.toml carries no
HTTP client dependency (reqwest/ureq/isahc/curl/hyper) and no source
file constructs any network client (TcpStream/UdpSocket included) —
grep-pinned in CI on every run.

### Chaining gate (deep/09 §Q9)

`statusline_wrapper_chains_user_script_byte_for_byte`: the wrapper's
chained output is byte-identical to running the user statusline
directly on the same stdin bytes, while the spool captures the payload
— both outputs visible, no duplication.

### Pending live evidence (rides the maintainer login — gate-E5 open item)

The live-dogfood (one real authenticated Claude session ≥20 min where
the spool-derived 5h % matches `/usage` within ±5%; one terminal
screenshot of the chained statusline) requires an authenticated
session; the maintained login is pending re-auth (the same blocker as
the E5 authenticated-helper dogfood, gate-E5.md). Fixture gates above
pass; the live check is recorded as PENDING here and rides the next
authenticated session.

## AJ-E608 — burn-rate forecast (2026-10-16)

Fixture timelines with implementation-independent oracles (the
generator's arithmetic), `crates/juggler-core/tests/quota_forecast.rs`
— 13/13 green:

```text
test accelerating_burn_mape_within_30pct_after_20_min ... ok   (oracle 20.6 min, MAPE ≤30%)
test admission_gate_verdicts_and_never_blocks ... ok           (OK/Tight/No; None on unknown inputs)
test calibration_notes_trip_only_beyond_2x ... ok              (2.5×/0.4× trip; 1.5× holds)
test cold_start_returns_learning_and_no_number ... ok          (9 min ⇒ Learning, no number)
test constant_burn_reproduces_known_tte_within_10pct ... ok     (oracle 60 min ±10%)
test median_session_burn_ignores_empty_sessions ... ok
test no_cap_means_no_headroom_forecast ... ok
test one_estimated_input_confidence_inherits_lowest ... ok      (cone renders a range)
test rate_cone_shape_is_stable ... ok
test stalled_burn_is_wont_exhaust_never_a_fake_number ... ok
test thirty_day_fixture_timeline_evaluates_fast ... ok          (≤500 ms ceiling)
test tte_beyond_the_reset_is_wont_exhaust_never_emitted ... ok  (incl. unknown-reset ⇒ no promise)
test zcode_forecasts_run_on_credits ... ok                     (oracle 30 min)
```

Numeric-trap record: an EMPTY float `.sum()` on this toolchain yields
**−0.0** (reduction contraction), and `headroom / −0.0 = −inf` slipped
past the reset bound. Fixed by IEEE signed-zero normalisation
(`+ 0.0`) at every fold plus a `!tte.is_finite()` guard; both are
documented in `quota_forecast.rs` and pinned by the stalled fixture.

## AJ-E609 — cost ledger (2026-10-16)

### ccusage-pattern cross-check (shared fixture corpus)

```text
$ python3 docs/research/evidence/billing/ccusage-oracle.py
corpus=crates/juggler-adapters/tests/fixtures/quota/claude-transcript.jsonl
sha256=143941e1514e074ec144cfe0b56a2566763c0d8fd1735a2b716ca0dd66d6947b
priced_messages=2      (uuid-deduped — the resumed duplicate counts once)
total_usd=0.0083100

Rust rollup on the same corpus + seed: 0.0083100
delta: $0.0000000
```

(The oracle prices the corpus with independent python arithmetic —
the ccusage aggregation pattern; it dedupes by message uuid exactly
like the store.)

### Tests (`cargo test -p juggler-core --test cost_ledger`)

12/12 green: exact cache-class pricing (hand oracle 37.95), unpriced
`—` never `$0.00`, effective_from event-time selection,
repricing-is-retroactive + `usage_events` byte-identical (raw SQL
before/after), stale >30d, `ZCode` credits alongside never converted,
dual subscription ratio only with a fee, model-switch row split
summing to the session total, CSV↔rollup reconciliation to the cent,
UTC day buckets, seed loads with auditable fields, corpus = oracle.

### UI (CostMatrix + gallery 25-costs)

5/5 vitest green (label discipline, `—` law, dual view, th scope +
totals row, "you paid $" grep gate). Visual: 24 captures across
priced/unpriced-model/stale-pricing × 4 viewports × 2 themes in
`_gallery/25-costs/`; full suite 246/246 twice consecutively.

### AJ-E806 flake record — axe color-contrast under parallel load

Under the FULL parallel visual suite, axe intermittently flagged
serious color-contrast violations on a DIFFERENT dialog each run (kill
sheet refused/light, then recovery light, then rail light) — each
passing in isolation and on the clean tree: load-induced false
positives while dialog transitions settle. Fix (not a retry): a
confirmatory 400 ms re-scan in every a11y block — real violations
reproduce deterministically and still fail; false positives do not
survive. Two full-suite runs green after the fix.

### Vendor passthrough note

`cost.total_cost_usd` passthrough ("vendor-reported" label) is not
ingested yet — it rides the statusline capture's live verification
(the AJ-E602 candidate, pending the authenticated session). Until
then USD is uniformly `Estimate`, which is the stricter label.

## AJ-E610 — two-clock display (2026-10-16)

### Test output

- `cargo test -p juggler-app --test quota_display` — 5/5: typed no-data
  (unknown/learning, no percentage, never 0%), Tracking with
  percentage+badge, Codex-Pro 5 h omission WITH reason (weekly kept),
  Gemini unavailable-with-reason, D-12 renewal states with the
  user-tunable 1–30 day threshold.
- `vitest src/components/QuotaRings.test.tsx` — 16/16: the full state
  matrix on real renders (measured badge-adjacency + aria text
  equivalent, stale demotion "last measured 22m ago" + opacity,
  blocked reset CTA, omitted-ring reason, Gemini unavailability,
  two-clock separation — the renewal line carries no %, rings no
  "Renew"), forecast pill states (Estimated renders the range),
  QuotaChip typed copies, the badge-adjacency grep gate (no
  `percent={0}`), and the D-12 no-inference test.
- Visual: gallery `26-quota` — the Usage tab two-clock card ×
  measured/stale/no-data/blocked × 1440×900 + 1280×800 × light+dark
  (16 captures), copy-conformance asserted before each shot; the
  pre-existing `08-inspector-aggregate/account-overview` capture now
  seeds the engine seam and asserts the badge. Full suite 262/262.
- Grayscale readability (WCAG 1.4.1): every number + badge is text —
  the ring's stroke is never the sole carrier; the typed-state and
  badge tests pin this per state.
- Rail note: the rail's per-session quota line (I-05) predates this
  task; the ACCOUNT-level surfaces (I-06) live on the account card,
  the Usage tab, and NewSession rows. No tray exists in the app yet
  (deep/10 §19 tray is a later surface) — recorded, not silently
  omitted.
- Pending live evidence (maintainer login, same blocker as E607/E609):
  the ±5 % `/usage` dogfood and the manual renewal-entry restart
  persistence check ride the next authenticated session. The state
  machine + persistence paths are covered by the fixture suites.

## AJ-E611 — ranking inputs, quota alerts, reset queue (2026-10-16)

### Test output

- `cargo test -p juggler-core --test ranking_alerts` — 11/11: weights
  sum to 1 with the §Q6 constants in ONE function, hard filters
  greyed-with-reason (never dropped), Estimated-headroom ×0.85 with
  Measured winning exact ties, unknown headroom ranks by remaining
  inputs AND says "headroom unknown — ranked without quota",
  affinity words carry the wrong-billing guard (fixture where project
  history favours account B asserts B's affinity appears in B's row,
  never A's), the fixture-day §Q5 sequence exactly (30 healthy
  minutes silent → TTE<45 ONE TrayAmber → TTE<15/≥90% ONE Actionable
  → repeats silent → ONE ExhaustedNotification → repeats silent →
  reset re-arms + ONE ResetReached), the 256-case proptest (random
  trigger sequences never emit two notifications for one crossing,
  per cycle: ≤1 actionable, ≤1 exhaustion, ≤1 reset), snooze holds
  30 min, >2× forecast error silent-logs, digest mute/quiet-hours/
  cadence (22:00–07:00 defers without dropping), and the purity
  guardrail (ranking/alerts core has no launch capability — no such
  API exists to call).
- `cargo test -p juggler-app --test ranking_alerts_guardrails` — 4/4:
  alert ticks emit ≤1 actionable per crossing across ticks and NEVER
  spawn (sessions digest byte-identical); the renewal-approaching
  digest joins ONCE per hour (quiet hour defers without dropping) and
  NEVER triggers the window FSM; ranking is suggest-only (sessions
  storage byte-identical through ranking_rows); the reset queue
  notifies once, marks broken lineage `stale`, and never launches.
- The E611 guardrail suite exposed and fixed a REAL E606 bug:
  `samples_for_account` ignored the window column, so a five-hour
  sample polluted the weekly reading (two actionables from one
  crossing). Fixed at the source (window-scoped fetch +
  per-window `AccountEvaluation` fields); the guardrail now pins the
  fixed behaviour.
- `just verify` workspace-green; UI build green; full visual suite
  280/280 (16 new `27-ranking` captures + 2 new axe passes).
- Visual: `27-ranking` — NewSession ranked list (Suggested badge,
  ≥2 factors without hover, TTE, greyed filtered row with reason),
  "Why this order?" expanded (all five inputs), the actionable 90%
  notification + the digest's renewal line visually distinct
  (dashed/quiet, prefixed "Digest ·"), queue rows (queued countdown,
  ready + explicit Resume focused) × 1440×900 + 1280×800 × light+dark
  — copy conformance asserted before every capture.
- A11y: the new axe pass on the ranking surfaces surfaced two real
  issues, both fixed: (a) `role="option"` wrapping a `<button>`
  (nested-interactive) — now a labelled LIST of buttons with ↑/↓
  roving focus; (b) pre-existing `text-faint` labels in the New
  Session modal fail 1.4.3 (the token cannot reach 4.5:1 on any dark
  surface) — the modal surfaces moved to `text-muted`, and the greyed
  filtered row greys via muted text instead of `opacity-50` (which
  halved the reason text's contrast). Recorded: `text-faint`
  elsewhere (rail) is a pre-existing pattern outside this task's
  surfaces — flagged for a design-token decision, not silently
  bumped.
- Suggest-only architecture (Req.7): NO launch/switch/resume code
  path exists in ranking/alerts/queue — the Resume click opens the
  NewSession picker; the wrong-provider hard filter is stubbed false
  (the request-restriction surface lands with the switch epic) and
  fidelity inputs neutral 0.5 (the switch-fidelity matrix lands with
  its epic) — both documented in ranking.rs.
- AJ-E806 record: one non-reproducing load flake —
  `juggler-supervisor` reconciler failed once under the FULL parallel
  `just verify` (2/3), passed on immediate re-run (full verify exit 0,
  62 suites ok) and in isolation (3/3); consistent with the recorded
  load-flake pattern (soak test timing under parallel load). Not
  quarantined — it did not reproduce; noted here for the flake ledger.
- Pending live evidence (maintainer login, same blocker as
  E607/E609/E610): one real quota-wall event end-to-end (alert →
  click → handover or queue → resume-after-reset, resume latency
  <15 min). The fixture-day sequence + guardrails cover the state
  machine meanwhile.
