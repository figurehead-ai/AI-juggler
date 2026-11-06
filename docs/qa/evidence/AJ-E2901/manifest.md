# AJ-E2901 evidence: the command palette

- vitest: `src/components/goto-palette.test.tsx`, `src/lib/palette-match.test.ts`,
  `palette-commands` and `palette-actions` tests — pass (2026-11-05).
- [timing.json](timing.json): 500 sessions, 200 projects, 20 accounts,
  30 apps: opens in 0.54 ms, a keystroke 2.7 ms p95 (budgets 50 and 16).
- Gallery `99-palette`: empty, typed, the preview, no matches, offline;
  1024×700, 1280×800 and 1440×900, dark and light; axe 0
  critical/serious; critique stamped.
- Found on the way: words typed straight after ⌘K were lost while the
  palette's code loaded; it's now fetched first after paint (App.tsx).
- Verdict: PASS.
