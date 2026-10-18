# Evidence manifest — AJ-E406 (Right Inspector shell: 6 tabs × 5 selection types)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E406.
Branch: `stage2/e4-inspector-shell`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 atomic selection + header contract | `pnpm --dir ui test` (inspector-shell.test) | type icon, name, I-04 chip, primary action | 5/5 selection kinds render the header (icon `title` = type, name, Instance carries the Running state chip; Stop is the live Instance primary action); Instance/Project from the rail, Account from the Accounts screen glyph, Provider from Overview quota cards | PASS | 2026-10-13 | zcode-primary |
| Req.2 six tabs for all five types (D-06) | same | tabs present, never hidden/disabled | exact tab labels asserted for all five kinds (Overview · Usage · History · Quality / Performance · Processes · Actions); pending tabs render the honest `Unavailable` + owning-task empty, never blank | PASS | 2026-10-13 | zcode-primary |
| Req.3 per-type tab persistence | same | `aij.inspector.tab.<type>` restored on reselection | persistence test: Instance set to Usage → Project shows Overview → Instance returns to Usage; storage key asserted | PASS | 2026-10-13 | zcode-primary |
| Req.4 fleet-at-a-glance empty state | same + capture | counts by state, lowest-quota Account, machine gauges; never blank | `FleetAtAGlance` renders state tallies, lowest-quota card (`% left` + tier tag, `Quota: no data` honest), machine CPU/RAM sums labelled measured-over-N (or `—`); captures `06-inspector/empty-*` | PASS | 2026-10-13 | zcode-primary |
| Req.5 real-source footers + stale rule | same | `Updated Ns ago · source: <source>` per tab | footer test: Overview → `source: supervisor`, Usage → `source: usage-engine`; updated-ago derives from the E403 freshness map (stale chips apply where data exists) | PASS | 2026-10-13 | zcode-primary |
| Req.6 keyboard + focus | same | Cmd/Ctrl+1..6; header→tabs→content; visible rings | keydown test switches tabs (Cmd+3 → History, Ctrl+6 → Actions) and moves focus to the tab; arrow-key roving in the tablist; `focus-visible` rings on tabs; panels `aria-labelledby` their tabs (WCAG 2.4.6 test) | PASS | 2026-10-13 | zcode-primary |
| Req.7 fleet-shaped | same | machine chip on remote selections; read-only remote Actions | machine chip test (`studio-desk` renders); with one local machine the chip is absent by design; remote Actions read-only lands with Fleet control (D-13) | PASS | 2026-10-13 | zcode-primary |
| Terminal never unmounts (gate) | same | TerminalView persists across tab switches | gate test: the terminal element stays in the DOM under `hidden` panels across Usage/History switches — no remount, no focus steal | PASS | 2026-10-13 | zcode-primary |
| Visual regression | `pnpm --dir ui visual:capture` | all green; axe 0 critical/serious both themes | 65/65 passed incl. 8 new `06-inspector/` captures (empty, Instance/Overview, Account/Usage honest-empty, Project/Processes × 1440x900 + 1280x800); honest-empty rail captures updated to the fleet summary; contrast fixes the axe gate forced: fleet/HonestEmpty text muted (faint ≈3.5:1 fails), gauge values explicit ink | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green (30 test binaries); 71/71 component tests; terminology gate caught a legacy "profiles" string in the new code — fixed to "launch settings" | PASS | 2026-10-13 | zcode-primary |

## Recorded flake (AJ-E806 rule: fix, never weaken)

The E405 attention-corpus test (real-process fixtures) flaked **twice**
under cold parallel workspace load — its 5 s spawn/Started deadline
starved while every crate compiled concurrently. Fix: deadlines widened
to 20 s (spawn/exit) and the dogfood budget to 90 s — timing thresholds
that guard behaviour, not the classification itself (windows, debounce,
signatures all unchanged). Three consecutive fully-loaded
`cargo test --workspace` runs green after the fix. The corpus's
precision/recall gates are untouched.

## Notes for the next agent

- `RightInspector.tsx` is the ONLY inspector shell; content tasks
  (AJ-E407/408/409/410) fill tab panels via `InspectorPanel` — never a
  new shell. Panels mount once per selection and toggle `hidden`
  (pre-fetched content model; the terminal survives tab switches by
  construction).
- Selection kinds live in App (`Selection` union): account + provider
  are new; login terminals keep their dedicated panel. Session
  (conversation) selections render honest empties until AJ-E407.
- Tab sources (`TAB_SOURCES`) are the footer contract — keep them
  honest as tabs gain real data.
- The fleet-at-a-glance machine gauges are honest SUMS over live
  instances (labelled `measured over N live instances`); per-machine
  rollups generalise when Fleet lands.
