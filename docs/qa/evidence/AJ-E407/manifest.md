# Evidence manifest — AJ-E407 (Instance + Session inspector content)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E407.
Branch: `stage2/e4-inspector-content`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 Instance Overview fields (§5.1) | `pnpm --dir ui test` (inspector-instance) | every field populated or honest-empty by `data-testid` | state+since chip, Provider/Account/Launch-settings/model chips, project (link) + cwd (copy), native session id (`not discovered yet` honest), quota bar, tokens+cost (`No data — arrives with statusline` exact copy), CPU/RAM/children; lineage section omitted (no Switch Graph — §5.1 empty rule, no fabricated edges) | PASS | 2026-10-13 | zcode-primary |
| Req.2 Instance Usage | same | breakdown real; charts honest | in/out/cache-read/cache-write table + `Measured` series tag on populated; exact `No usage recorded yet` / `No data — arrives with statusline` on bare; contribution card honest | PASS | 2026-10-13 | zcode-primary |
| Req.3 Instance History | same | honest empty + imported badge contract | `No history recorded yet` + timeline copy + the `imported` badge rendered in the contract line | PASS | 2026-10-13 | zcode-primary |
| Req.4 Instance Actions | same | disabled WITH reason, never hidden; shortcuts in tooltips | Focus terminal (real), Stop ⌘. (live), Resume/Relaunch (ended, real relaunch), Hand over ⌘⇧H (real: jumps + scrolls the section), Queue-at-reset (QuotaBlocked-only reason), Export/Archive/Delete disabled naming AJ-E202/E1007/guarded | PASS | 2026-10-13 | zcode-primary |
| Req.5 Session tabs (§5.2) | `pnpm --dir ui test` (inspector-session) | all six filled or honest | Overview: state, owning-Instance link, tokens aggregate, `Transcript not readable yet` exact copy (never zeroed); Usage per-turn honest + CSV disabled-with-reason; History `No lineage yet` (E2); Actions Resume ⌘R primary (real) + honest rows | PASS | 2026-10-13 | zcode-primary |
| Req.6 Ended immutability | same | `Ended HH:MM` stamp | ended fixture stamps `Ended HH:MM`; live fixture has none | PASS | 2026-10-13 | zcode-primary |
| Shortcuts (mocked bridge) | same | ⌘. → kill; ⌘⇧H → handover focus | predicates in `shortcuts.ts` shared by App + tests; ⌘. calls the mocked `terminalKill` with the instance id; ⌘⇧H switches the inspector to Overview; negative cases (shifted `.`) asserted | PASS | 2026-10-13 | zcode-primary |
| No shell/terminal regressions | full suites | E406 shell + terminal persistence intact | 86/86 component (71 prior incl. shell/terminal gates + 15 new); 69/69 visual (prior matrices + 5 new `07-inspector-content/` captures) | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green; typecheck+bundle green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Charts/per-turn/CSV/lineage are honest empties WITH owner labels —
  AJ-E606+ (usage engine), AJ-E2 (transcript indexing + Switch Graph),
  E2 (pack export). uPlot is deliberately not vendored yet (no data to
  plot); the Usage card structure is ready for it.
- The hand-over focus path is an event (`aij:focus-handover`): App's ⌘⇧H
  and the Actions row both dispatch it; RightInspector switches to
  Overview and the section scrolls. Extend, don't fork it.
- Actions are `ActionRow`s with stable `data-testid="action-<slug>"` —
  wire real handlers there as bridge commands land (Queue at reset →
  AJ-E1003, Export pack → AJ-E202, Archive/Delete → AJ-E1007).
- The live dogfood (hand-over → lineage in both History tabs) waits on
  the Switch Graph exactly like lineage rendering does (task handover
  note); both are E2-gated, recorded here.
