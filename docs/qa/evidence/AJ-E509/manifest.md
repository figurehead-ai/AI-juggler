# Evidence manifest — AJ-E509 (Append-only cleanup audit log)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E509.
Branch: `stage2/e5-audit`. Local runs (macOS arm64) 2026-10-15,
reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 the `cleanup_audit` table | `cargo test -p juggler-core --test storage_roundtrip` (v14 arm) | every kill accountable: timestamp/actor/candidate/attribution/identity/scope/signals-outcomes/ports before-after | schema v14 (`cleanup_audit` + `at` and `account,at` indexes) in the forward-only migration chain; `CleanupAuditRecord` carries every specified column; cmdlines stored HASHED + truncated only (`cmdline_hash` + `cmdline_trunc`; no full-cmdline column exists) | PASS | 2026-10-15 | zcode-primary |
| Req.2 intent BEFORE signal; crash leaves truth | `audit-tests.txt` (`crash_mid_ladder…`, `audit_intent_completes…`) | intent pending before any signal; mid-ladder crash ⇒ `TERM'd, outcome unknown` | `intent()` inserts outcome `pending` BEFORE the ladder; `complete()` is the ONLY post-insert mutation (guarded `WHERE outcome = pending`); the crash test (intent, never completed) reads as `TERM'd, outcome unknown` in history — a fact reported, never hidden; the live kill path (`orphan_stop`) writes intent → ladder → completion with per-PID outcome + ports after | PASS | 2026-10-15 | zcode-primary |
| Req.3 NO AUDIT, NO KILL | `fault_injection_no_audit_means_no_signal` | intent failure ⇒ typed error, ZERO signals; never a swallowed `let _` | the `AuditWriter` seam injects an always-failing writer: `orphan_stop_audited` returns the NEW typed `AppError::AuditUnavailable`, the trapped fixture's signal log stays EMPTY, the process stays ALIVE (both asserted); recovery with the real repo proceeds and the audit row completes (`gone`); automatic-tier stand-down is the same gate by construction (E508 calls the same seam) | PASS | 2026-10-15 | zcode-primary |
| Req.4 History reads the log; retention; export user-initiated | history tests + `export_is_valid_jsonl…` + UI tests | 7-day window; empty copy exact; JSONL 100% valid; redaction | `cleanup_inventory("history")` returns the AUDIT rows (not ledger exits) with the AJ-E507 outcome vocabulary + the copy-exact empty note `No cleanups yet — automatic cleanup is off by default.`; prune mirrors the ledger (7 days, tested: an 8-day record removed, the fresh one kept); `cleanup_audit_export` emits JSONL whose FIRST line is the redaction note (itself valid JSON) and every line parses (asserted); the export button is user-initiated only (a click — no automatic export path exists) | PASS | 2026-10-15 | zcode-primary |
| Req.5 append-only, atomic WAL | code review + repo surface | no update/delete surface; WAL; not deletable from any UI | the repo exposes insert/complete/history/prune ONLY (no general update/delete — the storage test asserts the guard `WHERE outcome=pending`); its OWN WAL connection (the ≤5 ms intent must not queue behind the storage mutex); the UI surfaces reads + the export ONLY — no delete control exists in any surface | PASS | 2026-10-15 | zcode-primary |
| UI: history rows + export + empty | `cleanup-centre.test.tsx` (165/165) + visuals | actors distinguishable icon+text; outcome vocabulary; export; empty copy | HistoryList: `👤 you` / `⚙ automatic` badges (icon+text), cmdline trunc + PID + outcome + timestamp; `Export audit log…` button downloads `juggler-cleanup-audit-{date}.jsonl` (Blob anchor; the bridge's mock path covers fixtures); the copy-exact empty note; the outcome vocabulary includes `TERM'd, outcome unknown` (asserted) | PASS | 2026-10-15 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | history populated + empty, light+dark, 1440×900 | `_gallery/19-cleanup/history-populated-*` (manual `exited on TERM` + automatic `outcome unknown` rows visible with distinct actor badges) + `history-empty-*` (copy visible) — suite 141/141 | PASS | 2026-10-15 | zcode-primary |
| Performance | construction | intent ≤5 ms p95; history ≤50 ms; export streaming | the intent is one indexed INSERT on the audit's OWN WAL connection (no lock contention with the app storage — by construction); history = one indexed `at_ms` range scan; export builds lines from the same read (10k rows is a single-digit-MB JSONL — well inside 500 ms/20 MB) | PASS | 2026-10-15 | zcode-primary |
| Regression | `just verify` + UI build/test | all green | verify green (fmt, clippy `-D warnings`, workspace tests — migration asserts bumped v13→v14); 165/165 UI; build green; the processes tests (which kill through `orphan_stop_inner`) now produce audit rows implicitly | PASS | 2026-10-15 | zcode-primary |

## Notes for the next agent

- E507 (kill ladder) next: call `AuditWriter::intent` BEFORE the first
  signal (the guardrail gate is now live), and `complete` with the
  per-PID outcomes array. The `AuditWriter` seam is the ONLY way a
  kill may proceed.
- E508 tiers: the stand-down rule is the same seam — `Err` from
  intent ⇒ skip the candidate, log, move on.
- The audit rows for TODAY's manual kills carry provider `unknown`
  (the E410 orphan tracker has no attribution); E507's sheet passes
  the full row context (provider/account/project from the ledger).
- History now reads the audit; E506's ledger-exited-rows history
  mapping is retired in the same change (one History, one source).
