# Evidence manifest — AJ-E606

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E606.
Branch: `stage2/e605-e606-batch`. 2026-10-16, reviewer zcode-primary.
Core evidence: `crates/juggler-core/tests/quota_windows.rs` (11 tests,
5 proptest properties × 256 cases) + `docs/qa/gate-M3.md` (output +
module sizes + the §Q8 timeline terminal dump).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Append-only §Q8 stores, additive migration, UNIQUE(account,msg_uuid) | next V-number, no existing-row semantics change | schema V18 (usage_events + quota_samples + reset_queue + idx); storage_roundtrip v4→v18 and v12→v18 upgrade tests pass (24/24) | PASS | 2026-10-16 | zcode-primary |
| §Q3 window maths: event sets (t−W, t], never buckets; Measured verbatim + clamped slope; Computed % only with user cap | property tests ≥256 cases | boundary prop × 256 green (rolling 5h/weekly identical predicate; anchored phase matched); monotone fold × 256; unknown-cap ⇒ no % × 256; interpolation clamped [0,100] | PASS | 2026-10-16 | zcode-primary |
| Subscription clock separate; Unknown without user date; never inferred | isolation property + D-12 semantics | fuzz × 256: window events never disturb the clock; no-date/malformed ⇒ Unknown; days math via E411 CalendarDate (composed, not duplicated) | PASS | 2026-10-16 | zcode-primary |
| §Q8 machine Unknown→Learning→Tracking⇄Stale→Exhausted→(reset)→Tracking; Unknown/Learning never 0% | timeline fixture + dump | state_machine_fixture_timeline green; terminal dump in gate-M3.md; type-enforced Option percentage | PASS | 2026-10-16 | zcode-primary |
| Pure maths, injected clock; dedupe by uuid before summing | no harness/DB/wall time in maths; resumed-file fixture | pure fns over &[UsageEvent] + now_ms/today params; store dedupe test: replayed file inserts 1/4, fold 40 not 80; uuid-less events never false-dedupe | PASS | 2026-10-16 | zcode-primary |
| UsageSnapshot populated from the engine at insert sites; no third representation | every default site migrated | engine_usage_snapshot is the single window producer; 3 sites wired (launch_session, desktop_launch_arc, session_handover_impl); pre-ingest honest zero (window None) | PASS | 2026-10-16 | zcode-primary |
| Clock-skew ±5 min never negative headroom; perf budget | prop + 100k fold | skew prop × 256 (all percentages finite ∈[0,100]); 100k-event fold green (≤100ms release-gated loose bound, AJ-E806; measured ~1.2ms release dev-box) | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core/src/quota.rs` (new, 659 lines) — UsageEvent,
  QuotaSample, WindowKind, window/phase bounds, WindowBurn fold,
  QuotaCap (settings-encoded), Measured interpolation + slope,
  estimated resets, SubscriptionClock (composing renewal.rs),
  QuotaState machine, evaluate_window/evaluate_account.
- `crates/juggler-core/src/storage/usage.rs` (new, 286 lines) +
  storage/mod.rs — V18 migration; UsageRepo (batched INSERT OR IGNORE
  dedupe, tail fetch, samples, reset-queue); cap/anchor settings keys.
- `crates/juggler-core/tests/quota_windows.rs` (new, 439 lines) — the
  property suite + timeline + dedupe + perf tests.
- `crates/juggler-app/src/commands.rs` — engine_usage_snapshot +
  epoch_today/civil_from_days; three insert sites migrated to the
  engine query.
- `docs/qa/gate-M3.md` (new) — M3 evidence section per EPICS.md:95.
