# Evidence manifest — AJ-E607

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E607.
Branch: `stage2/e607-batch`. Gate section: `docs/qa/gate-M3.md` §AJ-E607.
2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Statusline capture: managed pass-through, chains user statusline; uninstall restores exactly | fixture-proven (deep/09 §Q9 line 523-524) | install/uninstall round-trip restores the statusLine value byte-for-byte (incl. a `padding` field); wrapper output byte-identical to the user script on the same stdin; idempotent re-install is a no-op; backup removed on uninstall | PASS | 2026-10-16 | zcode-primary |
| Per-source adapters emit graded UsageEvents | statusline ⇒ Measured samples; transcript ⇒ Measured tokens; rollout ⇒ Measured tokens; `ZCode` ⇒ Computed credits; Gemini per E605 ceiling | spool→Measured quota samples (rate_limits only); transcript uuid-keyed Measured tokens; rollout response-level only (rollups never re-count); `ZCode` read-only SQLite, credits = published formula (peak 83.28855 exact, off-peak ×0.5 exact), unknown model ⇒ no credits + Estimated; Gemini emitter ABSENT by typed decision | PASS | 2026-10-16 | zcode-primary |
| Dedupe + idempotent re-tail | re-tail inserts zero; resumed-file contract | store-level: double ingest → COUNT identical (duplicate uuid absorbed); pipeline-level: second pass with no changes inserts 0 rows; truncation/rotation → full reparse + dedupe absorbs | PASS | 2026-10-16 | zcode-primary |
| Single-writer + passive | no Provider process control; `ZCode` SQLite read-only (WAL-safe); no undocumented endpoint | one pipeline thread, one storage lock per batch; `ZCode` opened SQLITE_OPEN_READ_ONLY; only managed-dir writes (the statusline wrapper + its backup); no-HTTP grep gate test-pinned in CI | PASS | 2026-10-16 | zcode-primary |
| Source health observable | last_event_at + parse-error count per source; stale demotes never deletes | SourceHealth persisted (KV blob) + `usage_source_health` command; E606 Stale demotion owns the >10 min rule | PASS | 2026-10-16 | zcode-primary |
| Spool rotation + managed-dir-only | 10 MB rotate; no data leaves the machine | sparse-file rotation test (previous generation kept, live spool fresh); spool lives only at `<managed>/quota-spool/` | PASS | 2026-10-16 | zcode-primary |
| Fixture corpus 0 dropped valid lines | per-source captured shapes | 13/13 adapters tests + 1/1 pipeline integration test green (see gate-M3 §AJ-E607 for output) | PASS | 2026-10-16 | zcode-primary |
| Live dogfood ±5% vs /usage (SPEC M3 gate) | one real session ≥20 min | PENDING — requires the authenticated Claude login (gate-E5 open item); recorded in gate-M3 with the fixture gates passing meanwhile | PENDING | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-adapters/src/quota_ingest.rs` (new) — statusline
  wrapper render/install/uninstall (backup + exact restore), spool
  tail (offset, truncation-aware, partial-line safe) + 10 MB
  rotation, ISO-timestamp parser, transcript/rollout line adapters,
  `ZCode` read-only SQLite adapter (formula + peak/off-peak), Gemini
  typed absence.
- `crates/juggler-app/src/usage_pipeline.rs` (new) — the single-writer
  pipeline thread (1 s spool cadence; 4 s file scans; 15 s `ZCode`),
  persisted offsets+health, `source_health()` + the
  `usage_source_health` command (commands.rs, lib.rs registration).
- `crates/juggler-adapters/tests/fixtures/quota/` (5 fixtures) +
  `tests/quota_ingest.rs` (13 tests); `crates/juggler-app/tests/usage_pipeline.rs`
  (integration).
- `docs/qa/gate-M3.md` §AJ-E607 (output, idempotency proofs,
  no-HTTP, chaining; the pending-live dogfood note).
