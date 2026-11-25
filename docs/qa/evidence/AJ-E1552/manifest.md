# Evidence manifest — AJ-E1552

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1552 (nothing loads forever: one loading primitive, a deadline on every resource, an empty state wherever there is a skeleton). 2026-11-23, reviewer claude-opus.

## Root cause of the stuck card (written before the fix)

**The card waited on an App whose model never comes.** An Account card lists its Apps from two reads that use two different rules for which Apps are "the user's":

- **The Account's ids.** `SubscriptionInfo.apps` comes from `subscriptions::with_apps` (`crates/juggler-app/src/features/subscriptions/mod.rs:605-626`), which runs over `apps::list_impl` → `users_apps`. `users_apps` keeps a desktop App when `App::is_users` is true (`crates/juggler-core/src/storage/entities.rs:507-513`), and that includes any desktop App that merely has a `folder_id`.
- **The App models.** A desktop App's model comes from the places list (`useKnownPlaces` → `places::commands::known`), which keeps only places whose book entry says `users` (`crates/juggler-app/src/features/places/commands.rs:304-310`). An app seen running and linked to the Account's folder, but not added, is not on it (D-84, AJ-E2327's "Running, not added").

So on the fresh install, the Account named a desktop App id that the places list never returned. `appsAwaited` (`ui/src/lib/account-items.ts:81-84`) treated "a non-CLI id with no model" as "still being read", with no end. The card drew `AppRowPending` for it for ever (`ui/src/screens/accounts/AccountCard.tsx:166`, `:289-293`). Nothing in it had a deadline, and an Account with no App at all had no empty state either: its list was simply empty.

Two more ways a skeleton could wait for ever, found on the way:

- **A shared read that never answers.** `lib/shared-read.ts` `store().load()` returned the pending read to every later caller (`if (pending) return pending`), so one hung read held every later load on it for ever.
- **The shell's first read.** `shell/use-app-data.ts` `refresh` awaited five lists with no deadline, and Home's skeleton (`loading = !shell.loaded`) waits on it.

## Fix

1. **One loading primitive** (`ui/src/lib/loading.ts`, `ui/src/components/LoadingState.tsx`):
   - The states are loading, ready, empty, error and stale (`loadStateOf`).
   - `withDeadline` fails a read with `TookTooLong`, 12 s by default.
   - `useLoadDeadline` gives a derived loading state "Still loading…" after 3 s and an error after the deadline. A Retry restarts the clock, and the cause is logged.
   - `LoadingState` draws its skeleton only in the loading state, "Still loading…" once a read is slow, "Couldn't load <what>." with Retry on error, its own empty state when empty, and else what was read.
   - `useResource` gains `state`, `slow`, `timeoutMs` and `isEmpty`, and every read it makes has the deadline, with failures logged.
   - A shared read (`sharedRead`) now has the deadline too. A hung read fails, is logged, and no longer holds later loads. `failure()` and `useSharedReadState` give its state.
   - The shell's first read runs under the deadline. When it misses, Home leaves its skeleton and the "service stopped answering" path takes over.
2. **The Account card** (`AccountCard.tsx`):
   - Once the App models are read, an id with no model is no longer waited on (`appsAwaited`, `shownAppIds`).
   - The Apps row is a `LoadingState`: the pending row only while the models are read; "Still loading…" at 3 s; "Couldn't load this account's apps." with Retry at 12 s (Retry re-reads the places, the stored Apps and the accounts); and the empty state **"No app uses this account yet · Add its app"**. Add its app calls the existing Add entry point, `addAppFor(tool)` (`lib/found-tools.ts`, the shell's `ADD_APP` flow).
   - The count comes from the backend's list at once, and from what is shown once the Apps are read.
3. **Backend.** `ipc::blocking`, which every command that touches storage, files or processes runs through, times each call and logs one over its 2 s budget, with its time (`ipc::BUDGET`, `over_budget`).

## Proof

One line per requirement (2026-11-23, the legacy sites' pass):

| Req | Proof | Result |
|---|---|---|
| R1 root cause (file:line) and one primitive; a skeleton only in loading | Root cause above; `src/lib/account-apps-pending.test.tsx`. Primitive: `src/lib/loading.ts`, `src/components/LoadingState.tsx` (now with an optional `failed` sentence for a read that knows why), `src/lib/paged-state.ts` (a paged list's state). `src/lib/loading.test.tsx` (7 tests). `tests/bundle/skeleton-sites.test.ts`: 35 of the 36 legacy sites moved onto `LoadingState` (32) or into `BY_DESIGN` with why (AppIcon, PoolPanel's lazy fallbacks, ProjectRow's skeleton definition); `LEGACY` holds one site. Commits 0c15b8067, 7634defc0, ba85388d9, 8604b38ca, ae838fcab, f48480213, 1485381e5, cb59f86b4, abe784d9e, ecd06dce5, 5bbf32575, 612077dab, 15b5afdf4, cd31f2f55. | PASS |
| R2 deadline: Still loading at 3 s, error with Retry at the timeout, cause logged | Every moved site: `useReadState`/`useLoadDeadline`/`usePagedState`/`useResource`, reads that used to swallow failures now surface them (watch catalogue, session pools, terminal kept, conversation feed `reread`, pool rules, canvas, task counts, pools, models catalog). Each area's state test checks "Still loading…" after 3 s and the error's Retry reading again. SwitchHeader's check (`Checking`) is on `LoadingState` too: `components/switch/switch-checking.test.tsx`. | PASS |
| R3 empty state everywhere there is a skeleton; the Account card's words and Add | Account card: `account-apps-pending.test.tsx`. Every moved site has its own empty words (e.g. "No ended watches in the last 30 days.", "In no pool now. Add it to one from a pool's diagram…", "Nothing needs you right now.", "This conversation isn't there any more. Find it in History…"); a failed read is never shown as empty (Needs action is no longer "all caught up" on a failure). Single records that can't be empty (a thread, an open task, the pool setup, the models list) say so in their tests. SwitchHeader's check (`Checking`) is on `LoadingState` too: `components/switch/switch-checking.test.tsx`. | PASS |
| R4 lint/test: no skeleton outside the primitive; per-site empty and error tests; the Playwright sweep | Ratchet `skeleton-sites.test.ts` (3 tests). Per-area state tests: `components/watches/watches-loading.test.tsx`, `pages/messages/messages-loading.test.tsx`, `pages/session/session-loading.test.tsx`, `components/components-loading.test.tsx`, `pages/pool/pool-loading.test.tsx`, `pages/tasks/tasks-loading.test.tsx`, `screens/tasks/tasks-board-loading.test.tsx`, `screens/pools/pools-loading.test.tsx`, `pages/subagents/subagents-loading.test.tsx`, `screens/screens-loading.test.tsx`, `screens/processes/processes-loading.test.tsx`, `screens/projects/projects-loading.test.tsx`, `screens/settings/ai/models-loading.test.tsx` (shared checks in `src/test/loading-kit.tsx`). Sweep: `tests/visual/skeleton-sweep.spec.ts` (passed earlier). Full vitest: 589 files, 4015 tests pass. SwitchHeader: `components/switch/switch-checking.test.tsx` (skeleton while reading and Still loading at 3 s, error with Retry, its own empty words, none once answered); `LEGACY` in `skeleton-sites.test.ts` is empty. | PASS |
| R5 backend: every query within its budget, over-budget logged | `ipc.rs` `a_command_over_its_budget_is_named`; the UI's deadline ends every wait. | PASS |
| R6 the card's cramped wrapping | AJ-E1550's pass, not this item. | N/A (AJ-E1550) |

## Not done here

- **The two rules for "the user's App" still differ** (`App::is_users` and the places' `users`). The card no longer waits on the difference, but a desktop App seen running and linked to a folder is still counted in `SubscriptionInfo.apps` and shown nowhere. One-line fix for the owner of `apps`/places (AJ-E2327's area), in `crates/juggler-app/src/features/apps/mod.rs` `users_apps`: keep a desktop App only when it was added, named or kept alive (`.filter(|app| app.kind != "desktop" || app.added || app.label.is_some() || app.keep_alive)`), so both lists agree. That changes D-84's lists, so it is that owner's call.
- **The last legacy site moved** (2026-11-23): `ui/src/components/switch/SwitchHeader.tsx`'s check is on `LoadingState` with its state test; `LEGACY` is empty.
- The card's cramped wrapping is AJ-E1550's.
