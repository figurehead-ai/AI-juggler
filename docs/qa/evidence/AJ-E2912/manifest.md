# AJ-E2912 evidence: debug mode

- `crates/juggler-app/tests/bug_report_replica.rs`, with debug mode on: a
  launch, a switch that lands and a switch that fails, through the app's
  own commands on a replica data folder.
- [debug-log.txt](debug-log.txt): the log as written. Each step is a flow,
  a step, and numbers and closed words (sizes, counts, durations, the
  mechanism, the fidelity, the stage); the failure carries the tool's own
  words with paths scrubbed (`[path]`).
- The test holds that no line carries the conversation's marked text or
  its folder; `crates/juggler-handover/tests/debug_steps.rs` holds the same
  for the switch pipeline on its own (Req. 5).
- Bounds, the seven days and off-keeps-the-log: unit tests in
  `features/app/diagnostics/debug_log.rs`.
- Every line is now scrubbed before it's kept (home folder, user name,
  paths outside home, links, keys), not only when shown.
- The section's captures: gallery `99-debug-mode` (both themes, 1280 and
  1024, axe clean, critique stamped).
- Verdict: PASS.
