# AJ-E2911 evidence: Report a bug

- `crates/juggler-app/tests/bug_report_replica.rs`: on a replica data
  folder built by the test (three Claude Code accounts, a four-turn
  conversation whose every turn and whose project folder carry a marked
  word), a switch through the app's own command fails because the target
  account's folder refuses the write. Report a bug is prepared from that
  failure as the failed step's button prepares it (`from: switch`, the
  screen's words, its detail and code).
- It holds: the parts come in order (what AI Juggler showed, the app and
  this computer, the last switch, then the logs); no part carries the
  marked word or a path outside home; leaving parts out removes them from
  what goes; the bug form's link fits.
- [report.txt](report.txt): the whole text Copy copies, as composed.
- [link.txt](link.txt): the link's length (1 970 characters, nothing cut)
  and each field's size.
- Found and fixed on the way: a tool's own error named the project's path
  outside home (a temporary folder); the shared scrubber now replaces a
  personal path outside home with `[path]` (`crash::scrub`, unit test
  `a_personal_path_outside_home_goes_and_a_system_one_stays`).
- The sheet's captures: gallery `99-report-bug` (both themes, 1280 and
  1024, axe clean, critique stamped).
- Verdict: PASS.
