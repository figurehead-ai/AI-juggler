# Evidence manifest — AJ-E402 (11-field indicator row + card variant)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E402.
Branch: `stage2/e4-indicator-row`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 I-01..I-11 render order, row + card | `pnpm --dir ui test` | 14 named fixtures green; exact strings | 25/25 passed: 9 states as icon+text chips (`Starting`…`Handed over`), quota bar `% left` + tier tag adjacent, tokens, children, branch, age, sparkline; card/row parity test asserts identical field text | PASS | 2026-10-13 | zcode-primary |
| Req.2 exact honesty copy | `pnpm --dir ui test` | `Quota: no data`, `default model`, `Unknown`; no `$`, no `0` fabrication | asserted verbatim: no-quota → `Quota: no data` + dashed bar; no pricing → tokens only, zero `$` occurrences in rendered text; non-git → branch omitted (query null); 0 children → badge hidden; never-observed tokens → no `tok` text; unsampled resources → `—`, not `0%` | PASS | 2026-10-13 | zcode-primary |
| Req.3 fabricated `percent={0}` rings replaced | `pnpm --dir ui build` + code review | no `percent={0}` anywhere | `QuotaRing` now takes `number \| null` (null = dashed honest ring); Overview renders per-live-Instance `QuotaBar`s, or the exact `Quota: no data` state when no feed exists | PASS | 2026-10-13 | zcode-primary |
| Req.4 card variant = same 11 fields | `pnpm --dir ui test` (parity test) | no number on the card the rail cannot explain | `SessionCard` shares `indicator.tsx` mapping + sparkline store with `SessionRow`; parity test asserts every field on both; card sparkline widened (64px, 5-min window) | PASS | 2026-10-13 | zcode-primary |
| Req.5 icon+text+colour states; `Unknown` fallback | `pnpm --dir ui test` | 9 states + unknown wire value | all 9 render icon+label+tone chips; unknown status renders `Unknown` chip with question glyph (never a neutral dot); unknown provider renders generic `Tool` mark | PASS | 2026-10-13 | zcode-primary |
| Quality gate: axe-core | `pnpm --dir ui visual:capture` (a11y.spec.ts) | 0 critical/serious on rail fixture page | axe wcag2a/2aa/21a/21aa clean in BOTH themes (D-16) | PASS | 2026-10-13 | zcode-primary |
| Layout gate: rows never overflow | same suite (no-overflow test, unpinned clock) | scrollWidth ≤ clientWidth per row | 0 overflows; provider abbrev/model/age/resource text yield (truncate) before chips/discs clip — verified against pathological durations (`wait 730d`) | PASS | 2026-10-13 | zcode-primary |
| Visual regression | `pnpm --dir ui visual:capture` | all captures green; determinism byte-identical | 44/44 passed (41 captures at 5 viewports × 2 themes incl. 1440x900 + 390x844, 2 axe, 1 overflow); gallery regenerated with the new row; fixtures cover running+quota, needs-you, quota-blocked, active no-data rows | PASS | 2026-10-13 | zcode-primary |
| Screen-reader pass | component test (aria-label) | every number reachable as text | row `aria-label` = full field summary (`Claude provider, Work Max, opus, Running, quota 58% left, computed, 19.1k tok, cpu 4%, ram 121 KB, 2 children, branch feat/read-model, …`); degrades honestly (`default model`, `Quota: no data`) | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green (fmt/clippy/tests/doc/evidence/terminology); UI typecheck+bundle green | PASS | 2026-10-13 | zcode-primary |
| CI round-trip (recorded failure → fix) | PR checks run 1 → run 2 | all 5 jobs green on final run | run 1: verify failed on Linux — the AJ-E401 collector test asserted `child_count == 0` for `sh -c sleep`, but Linux dash forks a child there (the read model counted honestly); fixed by spawning `/bin/sleep` directly (never has children, deterministic on every platform). Run 2: verify/ui/build-macos/build-linux/build-windows all pass | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Row layout: 4 lines per Comfortable row (~64px) — provider+disc+model+chip,
  suffix/meta (wait/resets + branch + age), quota bar, tokens+children+
  sparkline+resources. AJ-E403 owns squeezing this into 56/36px densities.
- The attention suffix (`wait Nm`, `resets HH:MM`) lives on the meta line
  per the wireframe — never inside the chip (it overflowed the rail).
- Sparkline history is UI-side (`rememberCpuSample`): appends measured
  `cpu_pct` samples from the 5 s `session://resources` tick (App merges
  deltas by id); first samples render the `—` placeholder until two exist.
- I-06 cost rendering is deliberately absent: no pricing source exists
  (full review F1.2). Tokens only — `$0.00` must never appear.
- Component harness: `pnpm --dir ui test` (Vitest + jsdom +
  @testing-library), also a CI ui-job step. Visual/a11y suite: `pnpm
  --dir ui visual:capture` (Playwright + @axe-core/playwright).
- The rail's `slice(0,6/8/15)` caps remain until AJ-E404 Req.7 (deleted
  together with virtualisation past 50 rows).
