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

| Requirement | Proof | Result |
|---|---|---|
| Req.1 root cause, file:line | Above. Unit test `src/lib/account-apps-pending.test.tsx` "once the Apps are read, an App with no model isn't waited on": with the old `appsAwaited` the card showed `account-apps-pending` for ever; now it shows the empty state. | PASS |
| Req.1 one primitive; a skeleton only in loading | `src/lib/loading.test.tsx` (7 tests): the states, `LoadingState` drawing its skeleton in loading only, `useResource`'s and a shared read's deadline. `tests/bundle/skeleton-sites.test.ts` (3 tests): every skeleton site is on `LoadingState`, a skeleton by design (listed with why), or a legacy site whose count may only fall. | PASS |
| Req.2 deadline: Still loading at 3 s, error with Retry at the timeout, cause logged | `loading.test.tsx` "useResource: loading, slow after 3 s, then an error at its deadline, logged; Retry loads again"; "a shared read … fails at its deadline, and the next load reads again"; `account-apps-pending.test.tsx` "Apps that never come: Still loading after 3 s, then an error with Retry after the deadline". | PASS |
| Req.3 empty state; the card's words and Add | `account-apps-pending.test.tsx` "an Account with no App: the empty state, and Add its app opens the tool's Add flow" (it dispatches `ADD_APP` with `claude-code`). | PASS |
| Req.4 per-site empty and error tests | The Account card: the two tests above. The legacy sites (37 files, listed in `skeleton-sites.test.ts` `LEGACY`) don't have them yet. They still draw their skeletons by their own rules, and each one gets its pair as it moves onto `LoadingState`. | PARTIAL |
| Req.4 Playwright sweep | `ui/tests/visual/skeleton-sweep.spec.ts`: Home, the 13 screens, Accounts with each filter (Out, Low) and tab, and every Settings pane. It fails if any skeleton is still on screen 15 s after opening. CI runs it in the regression config's `assertions` project. Local run (Chromium, through `scripts/heavy.sh`): passed. Every place settled within 512 ms, the slowest being Settings panes at about 0.5 s (their code loading) and Home, Tasks, History, Accounts, Usage and Processes at about 0.26 s. | PASS |
| Req.5 backend budget logged | `ipc.rs` test `a_command_over_its_budget_is_named`. No command has a hard timeout of its own: a long write (a scan, an install) mustn't report failure while it runs on. The screen's deadline is what ends a wait. | PASS (logging); the hard timeout is the UI's |

## Not done here

- **The two rules for "the user's App" still differ** (`App::is_users` and the places' `users`). The card no longer waits on the difference, but a desktop App seen running and linked to a folder is still counted in `SubscriptionInfo.apps` and shown nowhere. One-line fix for the owner of `apps`/places (AJ-E2327's area), in `crates/juggler-app/src/features/apps/mod.rs` `users_apps`: keep a desktop App only when it was added, named or kept alive (`.filter(|app| app.kind != "desktop" || app.added || app.label.is_some() || app.keep_alive)`), so both lists agree. That changes D-84's lists, so it is that owner's call.
- **The legacy skeleton sites** (`LEGACY` in `skeleton-sites.test.ts`) move onto `LoadingState` one by one, each with its empty-state and error-state tests. The ratchet stops new ones.
- The card's cramped wrapping is AJ-E1550's.
