# Evidence manifest — AJ-E403 (density modes, legend, stale-data rule)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E403.
Branch: `stage2/e4-density-legend`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 `aij.density` persistence + default comfortable | `pnpm --dir ui test` (density.test) | persists; default comfortable; unknown values degrade | stored+read-back for all 3 modes; default comfortable; unknown `"cozy"` falls back without crashing | PASS | 2026-10-13 | zcode-primary |
| Req.1b legacy `aij.rail.collapsed` migrates, never competes | same | collapsed=1 → minimal, old key removed | migration test green: key removed, `aij.density=minimal` written; collapsed=0 → comfortable; no competing keys remain | PASS | 2026-10-13 | zcode-primary |
| Req.2 field presence per mode | same (3-row fixtures) | Comfortable all I-01..I-11; Compact I-01/02/04/05-bar/09/10+CPU-RAM text; Minimal disc+glyph+`!` count | Comfortable asserts provider/model/state/% left/tokens/branch/sparkline/cpu·ram; Compact asserts abbrev+state text+branch+cpu·ram, NO `% left` text and NO sparkline svg; Minimal asserts aria-labels carry full field text and `! 1` count renders | PASS | 2026-10-13 | zcode-primary |
| Req.3 I-10/I-04 text never hidden (Comfortable/Compact); Minimal full text | same | exact-copy assertions | Compact waiting row renders `Needs you` badge text; Minimal buttons carry full I-01..I-11 in `aria-label` + hover tooltip (capture `minimal-tooltip-1440x900.png`) | PASS | 2026-10-13 | zcode-primary |
| Req.4 segmented control + Cmd/Ctrl+D cycle; inspector unaffected | same (keyboard tests) | cycle order correct; web Cmd+D never swallowed | header segmented control (`aria-pressed`, Comfort/Compact/Minimal); keydown harness cycles comfortable→compact→minimal→comfortable; predicate test: Ctrl+D everywhere, Cmd+D only in-app, shift+D never; density prop reaches only the rail — panels/inspector-area render unchanged | PASS | 2026-10-13 | zcode-primary |
| Req.5 stale rule 2× interval, `stale - Nm` | same | 119 s fresh / 121 s stale at 60 s interval; integer minutes | `staleMinutes`: 119 s → null, 120 s → null (boundary = last fresh), 120.001 s+ → stale; caption `stale - 2m`; rendered chip asserted on an expired row and absent on a fresh one; ONE shared 15 s tick drives recomputation (no per-value timers) | PASS | 2026-10-13 | zcode-primary |
| Req.6 legend identical in its surfaces | `pnpm --dir ui visual:capture` + code review | 9 states + quota + sparkline semantics, identical content | one `IndicatorLegend` component rendered in rail-footer `?` popover, Overview header, and the All-clear empty state; sparkline upgraded to dual series (RAM solid / CPU outline) so the legend never describes what is not drawn (Inspector surface drops in with AJ-E406) | PASS | 2026-10-13 | zcode-primary |
| Visual: 3 densities × (1440x900, 1024x768) + tooltip + stale row | `pnpm --dir ui visual:capture` | all green; axe 0 critical | 52/52 passed (6 density captures, minimal hover tooltip, stale row `stale - 3m` asserted in DOM before capture, prior matrix, axe both themes, overflow gate); gallery `01-rail-density/` committed | PASS | 2026-10-13 | zcode-primary |
| Contrast on stale values (manual check) | code + token review | ≥4.5:1 or the chip-instead-of-dim fallback | fallback used by construction: values keep their full token colours (never dimmed); the `stale - Nm` warning-soft chip carries the meaning — the task's prescribed alternative | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green (fmt/clippy/tests/doc/evidence/terminology); 40/40 component tests (15 new + 25 E402 regression); UI typecheck+bundle green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Density state: `readDensity()`/`storeDensity()` in `ui/src/lib/density.ts`;
  App owns the state; the rail receives `density` (the legacy collapsed
  boolean is gone — migrated on first read).
- Terminal rows hide resource slots entirely (finished ≠ unavailable);
  active-but-unsampled keeps the honest `—`. The minimal rail lists ALL
  active statuses — waiting/stuck/quota-blocked included.
- Freshness: App tracks per-id `resources`/`quota` sampled-at (seeded at
  list load, bumped by each tick delta); `__AIJ_SAMPLED_AT__` forces it
  in fixtures. Staleness recomputes against a single 15 s `now` tick.
- AJ-E406 must drop `IndicatorLegend` into the Inspector Overview header
  (component is shared; surfaces today: rail footer `?`, Overview
  header, All-clear empty state).
- The density cycle shortcut is `isDensityShortcut(event, inApp)` —
  Ctrl+D everywhere, Cmd+D only inside Tauri; ⌘B toggles
  minimal↔comfortable (legacy muscle memory).
