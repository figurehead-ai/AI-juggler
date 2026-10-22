# Evidence manifest — AJ-E1508

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1508 (copy shared settings into a new account). Status: **IN-PROGRESS**. 2026-10-22, claude-opus.

## What changed

1. **An account's own settings are inventory items** (`juggler-adapter` `inventory.rs`): its permissions (`ItemKind::Permissions`) and its status line (`ItemKind::StatusLine`), each a `Detail::Setting` with the config file and each key's value. The adapters read them key by key, so nothing else in the file is kept.
   - **Claude Code:** `settings.json` `permissions`, and the user's own `statusLine`. When AI Juggler's usage capture holds the status line, the user's original comes from the backup beside it, and the capture itself is never copied.
   - **Codex:** `config.toml` `approval_policy` and `sandbox_mode` (the permissions) and `tui.status_line`.
   - **Same tool only.** Another tool's permissions work differently, so the verdict says to set them in that tool.
2. **`config_edit`** (`juggler-adapters`) sets one setting in a JSON or TOML config and keeps every other byte: comments, order and spacing. It never sets a key that's already there. Its undo (`Undo::RemoveSetting`) takes a key out only while it still holds the value that was set.
3. **`features/account_settings`** works out what a new account, or one about to be made, could start with from another account of the same tool, and copies it. It uses the switch set-up's check, verdicts, record and undo (AJ-E1542), which now take a kind filter. **A switch still offers none of these settings:** its target keeps its own.
4. **Add account's fresh account** shows "Start with the same settings as …", ticked, naming the account. It opens to the list of what comes with it: the MCP servers, the permissions, the status line, and the plugins.
   - Each item that can be set up is ticked.
   - A plugin needs the tool's own install and shows the command.
   - "Never its sign-in, history or caches" is said.
   - The line is left out when the only account to copy from has nothing to copy.
   - Anything that couldn't be copied is shown before sign-in.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 Add account offers MCP servers, permissions, plugins and the status line from an account the user picks, each listed and ticked by default | `account_settings` `a_new_account_gets_the_usual_settings_and_never_a_credential_history_or_cache`, `codex_homes_copy_their_approval_sandbox_and_status_line`, `the_offers_before_the_account_exists_are_the_ones_it_gets`; vitest `fresh-account-settings` (5); gallery `83-account-settings` | PASS for fresh accounts |
| Req.2 Credentials, history and caches never copied (a guardrail test lists the files) | The same test lists every file in the new account's folder after the copy: exactly its marker plus `settings.json` and `.claude.json`. No `.credentials.json`, `history.jsonl`, `projects/`, `todos/` or `statsig/`, and no secret's text in any file. The Codex test does the same: only `config.toml` is new, with no `auth.json` or `sessions/`. | PASS |
| Each copy holds exactly what was ticked | vitest "copies what's ticked once the account is made"; the Codex test (`model`, not picked, isn't copied) | PASS |
| Never over the account's own; undone only when unchanged | `config_edit` tests (4); `a_setting_the_new_account_has_is_left_as_it_is` | PASS |
| The switch's offers stay as they were | `a_switch_still_offers_no_account_settings` | PASS |
| Visual: the Add account section, collapsed and open | gallery `83-account-settings` (collapsed, open, nothing to copy, the sign-in after it), both themes, axe 0 critical/serious; `71-add-account` shows the line | PASS |

## Left

- **Copies.** Req.1 also names an account brought in as a copy. A copied setup already brings its own folder's settings, so the offer is built for fresh accounts first. Bringing settings from a different account into a copy is still to come.
- **Plugins** are listed with their install command ("Needs you"), not installed. Installing a plugin fetches it from its marketplace, and copying the plugin folder would carry absolute paths that point into the other account.
- **The regression baselines** for Add account (`add-account-*`) change, because the line shows there. They're re-rendered in the pinned container with the PR.
