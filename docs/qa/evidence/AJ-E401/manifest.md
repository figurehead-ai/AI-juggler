# Evidence manifest — AJ-E401 (Instance read model I-01..I-11)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E401.
Branch: `stage2/e4-read-model`. All runs local (macOS arm64, debug build),
2026-10-13, reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 all I-01..I-11 fields on the row payload | `cargo test -p juggler-app session_info` | fixtures green | `session_info_full_row_carries_every_field` ok: account_id/account_label/model/quota/tokens/cpu_pct/rss_bytes/child_count/git_branch/needs_attention/last_activity_ms/machine_id all asserted present and correct | PASS | 2026-10-13 | zcode-primary |
| Req.2 null-not-zero honesty | `cargo test -p juggler-app session_info` | nulls, never `0`/`""`/guesses | `session_info_absent_data_is_null_not_zero` ok: quota-less row `"quota":null` + `"tokens":null`; model-less `"model":null`; terminal row child_count `Some(0)` (fact) but cpu/rss `None`; non-git row omits I-09; `unjoinable_rows_render_generic_markers` ok (`unknown` markers, account_id null) | PASS | 2026-10-13 | zcode-primary |
| Req.2 unknown status degrades upstream | `cargo test -p juggler-app session_info` | never a fabricated status | `session_info_status_vocabulary_is_the_nine_wire_strings` ok: storage rejects unknown statuses at row-map time (FK'd vocabulary), wire round-trips all 9 strings | PASS | 2026-10-13 | zcode-primary |
| Req.3 no N+1 storage reads per row | code review + `session_info_join_p95_under_budget_at_50_instances` | p95 < 20 ms at 50 Instances | one `accounts().list()` pass, one `profiles().list_by_account()` per account (≤10), one `sessions().list(None)`; join in memory; measured p95 < 20 ms (assert) — ok at 50 seeded rows × 100 runs | PASS | 2026-10-13 | zcode-primary |
| Req.4 event-driven + coalesced ticks | code review + `cargo test -p juggler-supervisor live_stats` | `session://updated` unchanged; new separate 5 s/30 s ticks, one payload per tick, none when idle | `session://resources` (`emit_resources`) + `session://quota` (`emit_quota`) spawned in `lib.rs::spawn_read_model_ticks`; lifecycle path untouched; `live_stats_collector_batches_and_masks_first_cpu_sample` ok (first sample CPU `None`, not fabricated 0.0; batched over one table refresh) | PASS | 2026-10-13 | zcode-primary |
| Req.5 ts-rs regenerated + field-name test | `cargo test -p juggler-app generated_ts` | generated TS contains every new field | `generated_ts_contains_read_model_fields` ok: InstanceInfo.ts has all 14 new names; QuotaSnapshotInfo/TokenCountsInfo/InstanceResourcesInfo/InstanceQuotaInfo/NeedsAttention.ts generated and asserted; committed diff of `ui/src/types/generated/` in the PR | PASS | 2026-10-13 | zcode-primary |
| Quality gates | `just verify` | green (fmt, clippy -D warnings, workspace tests, doc, evidence, terminology) | all green on `stage2/e4-read-model` | PASS | 2026-10-13 | zcode-primary |
| UI gate | `pnpm --dir ui build` | tsc + vite green | green after `label`→`account_label` mechanical renames | PASS | 2026-10-13 | zcode-primary |
| Visual regression insurance (task defers proof to AJ-E402) | `pnpm --dir ui visual:capture` | 41/41 captures | 41 passed after fixture update (fixtures now carry the full read model; gallery regenerated) — no visual surface changed beyond restored account labels | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- `subtask_count` and `git_dirty_count` ship as contract fields with `null`
  values by design: no transcript/git-status source exists yet (AJ-E409
  lands signals; a subprocess-free dirty-count collector is future work).
  Never backfill them with estimates.
- `git_branch` parses `<cwd>/.git/HEAD` only (file read, no subprocess);
  detached/worktree layouts report `null`. `git_dirty_count` must stay
  subprocess-free on the hot path.
- `Supervisor::live_stats(&System)` reads ONE caller-refreshed table;
  `LiveStatsCollector` (in `AppState.resources`) owns the persistent table
  so CPU% deltas survive ticks. First-ever sample reports `cpu_pct: null`.
- Lock ordering rule: the collector lock is never held together with the
  storage lock (`live_stats_of` takes it alone).
- `NeedsAttention` is derived from lifecycle status here; AJ-E405 replaces
  the derivation with structured heuristic reasons — keep the wire enum.
- Pre-existing wire lie fixed in passing: `#[ts(type = "number")]` on
  `Option` fields erased nullability (ProjectInfo.last_activity_ms and the
  new nullable numerics now emit `number | null`).
