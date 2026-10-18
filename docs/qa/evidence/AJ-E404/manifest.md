# Evidence manifest — AJ-E404 (needs-me sort, filters, search, snooze)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E404.
Branch: `stage2/e4-sort-filter`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 Needs-me default order | `pnpm --dir ui test` (sorting.test) | exact rendered order on the named 8-row fixture | `[s1 Failed, s2 QuotaBlocked, s3 wait 4m, s4 wait 1m, s5 Stuck?, s7 Running, s6 Running(snoozed), s8 Finished]` asserted verbatim; all 7 sort modes in the menu | PASS | 2026-10-13 | zcode-primary |
| Req.1b severity property | same | invariant holds for EVERY adjacent pair on a seeded-random 200-row fixture | property test green: severity non-decreasing; within equal severity snoozed never precede unsnoozed | PASS | 2026-10-13 | zcode-primary |
| Req.2 Quota risk + `no data` group | same | lowest % left first; unknowns group last | `s2` (0% left) renders first; `no data` group header present with the no-quota row last; menu label `Quota risk (quota only)` asserted (burn rate lands with AJ-E409) | PASS | 2026-10-13 | zcode-primary |
| Req.3 chips + dropdowns ANDed; machine dimension | same | counts announced; AND across groups | chips carry counts (`Needs me, 5 items` aria-labels asserted), `aria-pressed` toggles; Provider/Account/Project dropdowns AND with chips; `machine` filters in the model (D-13, one machine today) | PASS | 2026-10-13 | zcode-primary |
| Req.4 search ⌘K/Esc + empty state | same + visual | match label/project/branch/model/account; `No instances match` + `Clear filters` | branch (`feat/x`) and model (`opus`) matches asserted; Esc clears and returns focus to the input; empty state renders Clear filters (capture `empty-search-1440x900.png`); ⌘K/Ctrl+K dispatches `aij:focus-search` from anywhere | PASS | 2026-10-13 | zcode-primary |
| Req.5 group-by rollups | same + visual | `N running - M needs you - lowest quota X%`; needs order inside groups | group-by Project headers carry rollups; needs order asserted inside each group (capture `group-by-project-1440x900.png`) | PASS | 2026-10-13 | zcode-primary |
| Req.6 snooze/mute | same + visual | sink below peers, never disappear; `snoozed Nm` label; badge counts unsnoozed only | `Snooze 15m / 1h / Until reset` (disabled without a known reset) + `Mute this instance` on the row context menu (⋯ button + right-click); `snoozed 12m` renders (capture); muted sinks below its waiting peer, stays above lower severities; `unsnoozedNeedsCount` excludes muted+snoozed; Instances-tab badge `!N` | PASS | 2026-10-13 | zcode-primary |
| Req.7 virtualisation; caps deleted | same | window past 50; `slice(0,6/8/15)` gone | 120 rows render a ~20-row window + spacers covering all 120 (window math asserted); `grep slice(0,` in WorkflowRail → only the windowing slice; legacy Live/Recent/15-cap lists deleted | PASS | 2026-10-13 | zcode-primary |
| Perf budget | same | sort+filter 200 rows < 16 ms | measured inside the test: < 16 ms asserted (single pass) | PASS | 2026-10-13 | zcode-primary |
| No regression to E402/E403 | full suite | snapshots + honesty copy | 55/55 component tests (incl. row honesty regression guard); 57/57 visual (prior matrices, densities, axe both themes, overflow gate) | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green; typecheck+bundle green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Pure logic lives in `ui/src/lib/railview.ts` (sort/filter/group/snooze +
  `severityRank`); the board (`InstanceBoard` in WorkflowRail) only wires
  it. New sorts/filters belong in the lib + tests, not the component.
- The WaitingUser live-binary dogfood moved to AJ-E405 (emission does not
  exist yet): E404 asserted the event→row-1 path via the mocked bridge.
- The `machine` filter key exists (D-13) but no machine dropdown renders
  while only `this-mac` exists.
- Virtualisation windows the flat list past 50 rows (grouped views render
  fully); row heights: comfortable 70 px / compact 42 px — keep
  `ROW_HEIGHT` in sync with the CSS if rows change.
- Two a11y fixes shipped with this task: `.attn-badge` (contrast-safe
  needs-you badge, both themes) and the in-bounds badge position — the
  overflow gate caught the tab widening; keep both gates green.
