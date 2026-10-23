# Evidence manifest — AJ-E1544

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1544, LiveStatsCollector in its own module (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

## What changed

- `LiveStatsCollector` and its helpers (`ProcessRow`, `ProcessTable`,
  `ProcessFacts`, `TreeSample`, `TreeState`, `push_ring_sample`,
  `CollectorWork`, `ProcessBrief`, `ObservedTree`, `ObservedRoot`,
  `observed_sums`, the process-table and tree-row helpers, and the
  listener-probe cadence) moved, unchanged, from
  `crates/juggler-supervisor/src/supervisor.rs` into
  `crates/juggler-supervisor/src/live_stats.rs`.
- `supervisor.rs` re-exports the moved public items, so every existing
  path (`juggler_supervisor::LiveStatsCollector`,
  `juggler_supervisor::supervisor::LiveStatsCollector`, …) still names the
  same type, and no caller or test changed.
- The collector reads a few of the supervisor's fields (its children,
  ledger, spawn contexts, event sender) and calls `publish_table`; those
  are now `pub(crate)` instead of private to the file. Nothing outside the
  crate can see them.
- The moved lines are byte-identical to the removed ones apart from the
  visibility words (checked with a line diff of the two).

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 the collector in its own module, behaviour unchanged | the move is pure (line diff); the supervisor and app suites unchanged and passing (`just verify`, which runs `load_budgets`); new module tests `live_stats::tests` (`the_old_path_names_the_same_collector`, `an_idle_supervisor_has_nothing_to_report`, `the_ring_keeps_five_minutes_and_at_most_its_capacity`) | PASS |
| Req.2 supervisor.rs's cap back down to its new size | `scripts/size_budget.allow`: 2962 → 1515; `size_budget.py` OK. `live_stats.rs` is 1341 counted lines, under the 1,500 budget, so it needs no cap | PASS |

## Notes

- supervisor.rs is 1515 counted lines (the budget counts up to its first
  inline test module), 15 over the plain budget. What is left before that
  point is the supervisor itself (spawning, the handles, the stop ladder);
  splitting it further is its own task, not this move.
- No user-visible change, so no CHANGELOG entry.
