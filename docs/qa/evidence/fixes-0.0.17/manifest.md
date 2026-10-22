# Evidence manifest: Codex compactions that name their model call (P21)

Not a task's evidence. This covers batch 6's item 2: Codex read ×1.015 over compressed sessions. QA's per-session comparison (walk batch 24, main 001c6ea4, read-only) found two things:
- the ×1.015 was the oracle's unit, not the backend's;
- one real double count, P21.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| The ×1.015 was the oracle's | QA: the backend's Codex rows are exactly the files' `token_usage_record` lines. In each of the 111 sessions that read high, backend − oracle = that session's compactions' own model calls (295,165,085 tokens in all, nothing left over). `token_count` never reports a compaction's call, so the old oracle, which counted `token_count` deltas, missed every one. The oracle now counts records by key: Codex all time ×1.00002, Account G 7 d ×1.00 | — | no counting change | INFO | — | 2026-10-22 | QA session |
| P21: a compaction naming another thread's call counts it once | Three real threads open with a `compacted` line whose `latest_token_usage_record` is another thread's record, already counted from that thread's rollout. The `token_count` after it was stored again as `…/compacted/…` (367,324 tokens). Now a `compacted` line that names its call (an object), or names none (`null`: a forked thread's copied summary, 36 real lines), opens no compaction row; only a `compacted` line without the key keeps P17's path. `quota_ingest`: `a_compaction_naming_another_threads_call_counts_it_once` (both files, either order), `a_compaction_naming_no_call_adds_nothing`, `a_mid_file_compaction_counts_its_call_once` (QA's mid-file shape). Fixture lines carry every key the real `compacted` payload has (values invented) | fails on main | the first two fail on main's code; all pass now | PASS | bab1095c | 2026-10-22 | claude-opus |
| Rows already stored are dropped | `REREAD_VERSION` 5. `usage_pipeline::a_compaction_counted_twice_by_an_older_build_is_read_again`: an older build's rows (the call, the same call again under the child's compaction) and its state at version 4. One pass leaves the call once | fails without either half | fails with main's parser, and with version 4; passes with both | PASS | bab1095c | 2026-10-22 | claude-opus |
| QA after | the same compare on this branch's head | Codex all time ≈ ×1.00; P21 rows gone | asked | PENDING | — | 2026-10-22 | QA session |

## Gates

| Gate | Head | Result |
|---|---|---|
| `just verify` (`CARGO_INCREMENTAL=0`) | 0c57ced3 | 150 test binaries, 1,204 passed, 0 failed; every step after the tests OK; nothing left running |
| `cargo deny check` | 0c57ced3 | advisories, bans, licenses, sources ok |
| `vitest`, visual, a11y | — | nothing under `ui/` changed |
