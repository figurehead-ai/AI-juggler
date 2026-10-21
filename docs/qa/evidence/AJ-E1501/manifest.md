# Evidence manifest — AJ-E1501

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1501 (manage accounts: rename, remove, set default). Status: **DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## What changed

1. **The account card has a menu**, the rail's row-menu pattern: the ⋯ button in the card's header, or a right-click on it. The menu is now one shared component (`ui/src/components/RowMenu.tsx`), used by the rail's attention menu too. It takes focus, moves with the arrow keys, closes on Escape, Tab or a click outside, and gives focus back. Opened from the keyboard, it opens under its button (it used to open at the window's corner).
2. **Rename** changes the label, and the plan when one is given. Every view reads the label from storage, so the rail, Usage, New session and the inspector follow without a restart. The labels a project's imported history keeps are renamed too, so "accounts used here" and the New session pick don't lose the account. A name another account has, in any case, is refused in words.
3. **Remove forgets an account; nothing is deleted.**
   - An account used where it is (ADR-0018): its folder stays exactly as it is. Asking for the Trash is refused.
   - A copy, or a fresh account: the sheet asks whether to keep its folder (the default) or move it to the Trash. The Trash is `NSFileManager trashItem` (the `trash` crate, `NsFileManager` method, never Finder scripting). Only a folder inside AI Juggler's own `accounts/` is ever sent there. If the Trash refuses, nothing is removed.
   - Refused while the account has open sessions (starting, running, waiting for you, stuck, or waiting for its limit), naming them by branch or folder.
   - The row stays in the database (`removed_at`, schema v30), so its sessions keep a name: "Mine (removed account)". It leaves every list (accounts, launch settings, switch targets, usage reads) and starts nothing.
   - A backup still withholds a removed account's environment values (the redaction pass covers every account).
4. **Defaults.** "Make the default" sets the global default (an app setting). A project's Actions tab picks its own ("New sessions here start with"). New session and New project preselect the project's default, then the global one, then the rule as before (last used here, then headroom), and say "this project's default" or "your default". Removing an account clears it as a default everywhere.
5. **Audit rows.** Every rename, plan change, removal and default change is a row in `account_audit` (who, what, when), readable with `account_audit`.
6. **Errors in the app's own words.** A new `AppError::Refused` reaches the screen as written (no "invalid account:" prefix).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 rename everywhere, no restart | Rust `manage_accounts :: rename_changes_the_label_everywhere_and_refuses_a_duplicate`: the accounts list, a session's label and a project's imported history follow; plan set; < 200 ms. Playwright `account-manage-gallery` "rename — a duplicate refused, then saved" (Accounts, then Usage) and "the rail by account after a rename", both themes. | PASS |
| Req.1 duplicate refused in words | Same Rust test ("max (PERSONAL)" vs "Max (personal)"; a blank name). vitest `account-manage.test.tsx` "rename saves the new name and plan…". Gallery `rename-duplicate-*`. | PASS |
| Req.2 in place: folder untouched | Rust `removing_an_account_used_where_it_is_never_touches_its_folder`: every path's size and mtime unchanged (the folder's own entry too); the Trash is never called; asking for it is refused. vitest and gallery `remove-in-place-*`. | PASS |
| Req.2 a copy: keep or Trash | Rust `removing_a_copy_keeps_its_folder_or_moves_it_to_the_trash` (kept unchanged; a refusing Trash removes nothing; moved, not deleted) and `a_copy_outside_ai_jugglers_data_is_never_sent_to_the_trash`. The real Trash: `move_to_trash_uses_the_macos_trash` (ignored by default; run by hand on this Mac: the folder left its place and was found under `~/.Trash/` by name). vitest and gallery `remove-copy-*`. | PASS |
| Req.3 refused while live, named | Rust `an_account_with_open_sessions_is_not_removed_and_they_are_named` ("«Busy» has 2 sessions open: api, web. Stop them first, then remove the account"; a finished session doesn't count). vitest and gallery `remove-refused-*`. | PASS |
| Req.3 history kept, marked | Rust `removing_an_account_used_where_it_is…`: the session row stays, "Mine (removed account)", and a launch on it is refused. Core `account_manage_storage :: a_removed_account_leaves_the_list_but_keeps_its_row`. | PASS |
| Req.4 defaults | Rust `defaults_preselect_and_follow_a_removal` (global, per project, cleared by a removal, a removed account refused). vitest "the project's own default wins, then the global default, then the last used". Gallery `default-*`. | PASS |
| Req.5 audit rows | Rust tests above check each row's action, detail, actor `user` and time; core `audit_rows_are_kept_in_order`. | PASS |
| Budget < 200 ms, no rescan | Rust tests time rename, remove and set-default (each < 200 ms in a debug build); no action reads a tool's folder. | PASS |
| Backups | Rust `a_backup_still_withholds_a_removed_accounts_environment_values`. | PASS |
| Schema v30 | `storage_roundtrip` (v4 → v30 and v12 → v30, once). The Dev build on a throwaway HOME made a v30 database (`removed_at`, `account_audit`). | PASS |

Gallery: `docs/qa/evidence/_gallery/75-account-manage/` (14 captures, axe 0 critical/serious on each state).

## Walked by hand

- **Browser preview** (1280×800 and 1024×768, dark): opened the menu from the keyboard (it opens under its button, Rename… focused), renamed to an existing name (refused in words), renamed back, made Personal the default (the chip, the notice), New session preselected it with "your default"; the project's Actions tab set Work Max for that project and New session from its tile preselected Work Max with "this project's default"; removed a fresh account keeping its folder ("Removed Personal. Its folder was kept.").
- **The Dev build** on a throwaway HOME (`env -i HOME=… PATH=<stand-ins>`): it started, made a v30 database, and quit cleanly. Its window can't be clicked from an agent session on this Mac (no Accessibility grant), so the clicks were walked in the preview against the same UI.

## Not covered here

- Clicking through the real window (above). The commands behind it are the ones the Rust tests call.
- A removed copy's Keychain sign-in is left in the Keychain (nothing is deleted); AI Juggler doesn't tidy it.
