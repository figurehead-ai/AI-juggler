# Evidence manifest — AJ-E2012

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2012, a UI linter
and stricter TypeScript. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ui/.oxlintrc.json`: oxlint 1.89.0 with `oxlint-tsgolint` 7.0.2005 (both pinned exactly, released before 2026-10-23), type-aware: the typescript, react (hooks rules on, `exhaustive-deps` included), jsx-a11y and import plugins; `max-lines` 400, complexity 25; `no-restricted-imports` so only `lib/ipc` imports the Tauri core and event APIs. Run as `pnpm lint` (`scripts/ui_lint.py`) in CI's UI job, in preflight and in `just ui`; 1.3 s on this Mac. As amended in the task text: oxlint instead of typescript-eslint, because TypeScript 7 has no JavaScript API. Two rules that don't apply are off: `react-in-jsx-scope` (the automatic JSX runtime) and `no-underscore-dangle` (style). |
| R2 | PASS | `ui/tsconfig.json`: `noUncheckedIndexedAccess`, `noImplicitReturns`, `noImplicitOverride`, `verbatimModuleSyntax`; the 63 errors they raised fixed with no change in behaviour (28 files); `tsc` clean, vitest 2050/2050. |
| R3 | PASS | Today's 2,312 findings are the baseline (`ui/lint-baseline.json`, per file and rule): a count above it fails, a count below it fails until the baseline is lowered (`ui_lint.py --update`, which never raises it). The 19 files over `max-lines` are in it. The 8 hooks-rule findings are test stand-ins (`vi.mock`), not bugs; the 48 floating or misused promises are worked down with AJ-E2018's split of `bridge.ts`. |
