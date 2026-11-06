# Evidence manifest — AJ-E2018

Task: AJ-E2018 "UI: split bridge.ts, keep preview out of production, a
shell registry" (E20). Status: **DONE-EVIDENCED** (agent self-check,
2026-11-06, release/0.4.1). Commits: ce09603f8 (R1), a1f2f95b3 (R2),
ee32c11fc (R3, R4).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `lib/bridge.ts` 4,729 → 69 lines and no `invoke`: it keeps `EVENTS`, `hasLiveBackend`, `refused`, `errorText`, `signedIn`. Its calls became 32 modules in `lib/ipc/` (accounts, launch, instances, settings, cleanup, kill, fleet, link, figurehead, updates, backup, …; each ≤ 275 lines). `lib/machine.ts`'s four calls and listener moved to `ipc/this-mac.ts`, `ResumeButton.tsx`'s `session_resume` to `ipc/launch.ts`, `StepBodies.tsx`'s `switch://progress` listener to `ipc/switch-preflight.ts`: outside `lib/ipc/` nothing imports `ipc/transport` or `@tauri-apps/api`. 415 importers rewritten by a codemod (types straight from `types/generated`, no barrel); the 11 `vi.mock` of bridge retargeted by hand. `ipc_audit.py` OK. |
| R2 | PASS | `src/preview/` plus the mock data that lived in `bridge.ts` and `lib/ipc` (fixtures, preflight/target/graph/validation tables, scan report, OS access, update, Cloud, Figurehead and pairing mocks, slots, names, switch examples, subagent preview) form one module, `preview/hooks.ts`, reached only through `lib/preview-hooks.ts` (`previewHooks(): PreviewHooks`, `loadPreview()` before the first render and only when no live backend answers). Every `previewHooks()` call checked to run only in a no-backend branch or a helper only those call. **Build grep** (`pnpm build`): before, "Weekend build", the preflight's "Fix the checkout flow" and the live-preview PNG were in `bridge-*.js`, in the first load; after, only in `hooks-*.js`, which no chunk imports statically (one guarded dynamic import). Browser smoke of the production build: as the app (`__TAURI_INTERNALS__` set) `hooks-*.js` is never requested; as the plain-browser preview it loads and Home renders, no console errors. The `window.__AIJ_*__` seams and `sessionStorage` flags are unchanged; vitest loads the data before each file's first test, after its imports, so a `vi.hoisted` `__AIJ_FIXTURE__` still seeds it. |
| R3 | PASS | `App.tsx` 1,132 → 143 lines. Its state is in `shell/use-rail`, `use-navigation`, `use-app-data`, `use-dialogs`, composed by `use-shell` (`Shell`). Registries: `shell/screens.tsx` (header screens), `shell/pages.tsx` (item pages), `shell/Overlays.tsx`, inspector tab bodies (`INSPECTOR_TABS` in `RightInspector.tsx`, tab lists in `pages/registry.ts`), Home's cards (`screens/home-cards.tsx`; `Home.tsx` 579 → 467), Settings' panes (`PANES`/`PANE_BODIES`, already). |
| R4 | PASS | `lib/use-resource.ts` (`{ data, error, loading, reload }`, key-based, `pollMs`), unit test `use-resource.test.tsx` (4 cases), adopted in 7 reads: Home's ledger state, the header's and Settings' paired Macs, tool capabilities (two hooks), Welcome's installs, a switch's disk space. |

**No visible change.** vitest 2,059 pass (2,055 before + 4 new). The
production preview build's DOM (frozen clock, Playwright's Chromium driven
by a script, not the gallery) equals the pre-split build's on Home,
Accounts, Usage, Projects, Pools, Tasks and History, and on three rail
pages except React's generated `useId` values (lazy chunks resolve in a
different order) and a toast's wall-clock animation offset. Visual
baselines were not run locally (the container gate rewrites the gallery).

**Bundle** (`dist/assets`, before = 5e817a71c built the same way):

| | Before | After |
|---|---|---|
| Total | 2,950,322 B (212 files) | 2,945,771 B (193 files) |
| JS | 2,841,237 B | 2,836,686 B |
| First load (index.html script + modulepreloads) | 1,524,908 B | 1,478,146 B |

The `lib/ipc` modules the first window loads ship as one `bridge` chunk
(`STARTUP_IPC` in `vite.config.ts`): a chunk per module cost ~17 KB of
cross-chunk imports.

**Gates.** `tsc --noEmit` clean; `pnpm lint` OK (2,309 → 2,287 baselined
findings; every rule's total equal or lower — the baseline followed code
that moved between files); `debt_baseline.py` UI entries OK (knip exports
247 → 245, types 38 → 33, jscpd UI lines 1,746 → 1,715; six unused command
wrappers that `import * as bridge` had hidden from knip now count, offset
by exports made file-local); `size_budget.py` (bridge.ts and App.tsx off
the list, Home.tsx capped at 467, WorkflowRail 835); `ipc_audit.py` OK;
`tool_literal_lint.py` (bridge.ts's 42 allowed literals gone, none in
`lib/ipc`); `pnpm build` OK. Failures left in those scripts are Rust files
other sessions are changing (`codex.rs` jscpd, `juggler-floor/models.rs`
size, `usage_history/settled.rs` literal).

## Handover

- About 100 reads in ~90 files still use the hand-written
  `let alive = true; void read().then(...)` pattern (`grep -rl "let alive
  = true" ui/src`). Move them to `useResource` when a screen is touched;
  the ones that also patch the state locally (EditorCard, OutsideSearch,
  CleanupStatus' countdown, event-fed reads like `useCleanupCount`) need
  `reload` or a small reducer, not a straight swap.
- Small inline mock branches (`if (!hasBackend) { … }`) stay in the
  `lib/ipc` modules as logic over the seams; the data they read is in
  `src/preview/`.
- Six command wrappers have no caller (`accountForgetIdentity`,
  `accountSetCleanup`, `appQuitPlan`, `environmentScan`,
  `figureheadRefresh`, `vaultEnterCode`): wire them up, or drop them and
  list their commands in `ipc_audit.allow`.
- `STARTUP_IPC` in `vite.config.ts` lists by name the ipc modules the
  first window loads; a new one the first window loads belongs there.
