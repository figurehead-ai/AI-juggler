# Evidence manifest — AJ-E2907

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2907, a confirm
before deleting launch settings, and no delete while they're in use. Built
in 0.3.1 (D-60: the sessions stay in History) and 0.4. Checked again,
requirement by requirement, on 2026-11-06 (release/0.4.1). Status:
**DONE, 1 gap open** (agent self-check, 2026-11-06; R1, with its title
fixed, and R4 closed the same day; R3 is left).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The bin opens a confirm on the shared `Modal` with a red Delete: `ui/src/screens/accounts/delete-launch-settings.test.tsx` "the row's delete button opens the confirm, never deletes on the click" and "asks first…". **Closed (2026-11-06), with the words fixed:** the title now reads "Delete the launch settings “Deep work”?" (`DeleteLaunchSettings.tsx`). `ui/src/screens/accounts/delete-launch-settings.test.tsx` "R1: “Delete the launch settings “Deep work”?”, Cancel focused, Esc cancels, focus back on the bin", from the real `ProfileRow` (its ⋯ › Edit launch settings › the bin): a modal named so, Cancel focused, the red Delete, Esc closes without deleting and focus is back on the bin; Cancel does the same. The gallery's `accounts-delete-launch-settings-*` captures still show the old title until re-captured. |
| R2 | PASS | Q1 was ruled by D-60 (0.3.1): the sessions stay. `delete-launch-settings.test.tsx` "asks first and says the sessions made with them stay" asserts "Sessions already made with it stay in History."; `crates/juggler-app/tests/manage_accounts.rs` `deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones` asserts both sessions are still in storage after the delete (Rust: run in the release preflight). |
| R3 | PASS | **In the window, before the click:** the launch settings' bin is greyed while AI Juggler has a live session started with them, with "1 live session uses these launch settings — stop it first." (`ProfileRow` `liveWords`, the per-settings count `AccountCard` `liveByRecipe`). Proven by `screens/accounts/delete-launch-settings.test.tsx` "AJ-E2907 R3 — refused in the window before the click": the click opens nothing and deletes nothing. **In the backend:** `manage::launch_settings_remove_impl` refuses with the open sessions named (`tests/manage_accounts.rs` `deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones`). **No race:** a launch counts as under way from before its deleted-check until its session is recorded (`features::launch::in_flight`), and the delete refuses while one is, under the storage lock it marks with: `a_delete_never_slips_between_a_launch_and_its_session`, plus `in_flight::tests`. |
| R4 | PASS | `delete-launch-settings.test.tsx` "the row's delete button opens the confirm…" asserts `ProfileRow.tsx` uses `<ActionIcon name="delete" size={13} />`. **Closed (2026-11-06):** `delete-launch-settings.test.tsx` "R4: the bin is the app's delete icon and keeps its name": the rendered bin is the button named "Delete Deep work", with its icon. |
| R5 | PASS | `delete-launch-settings.test.tsx` "asks first and says the sessions made with them stay": after Delete, the toast "Deleted “Deep work”." shows; there is no Undo in `DeleteLaunchSettings.tsx`. |

Run 2026-11-06: `pnpm exec vitest run src/screens/accounts/delete-launch-settings.test.tsx` — pass.

## The first part (2026-11-05)

- `cargo test -p juggler-app --test manage_accounts`
  ("deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones":
  refused through the command while a session is open, naming it; the
  sessions stay in History) and vitest `delete-launch-settings.test.tsx`
  — pass (2026-11-05).
- Gallery `41-policy`: the confirm (`accounts-delete-launch-settings-*`)
  and the refusal while three sessions are open
  (`accounts-delete-launch-settings-refused-*`); 1280×800 and 1024×680,
  dark and light; critique updated and stamped.
- The preview now refuses as the backend does, from its own sessions
  (`lib/ipc/manage.ts` `deleteProfile`), so the refusal can be seen and
  captured without a live backend.
- Verdict: PASS.

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.
