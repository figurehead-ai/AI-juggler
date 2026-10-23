# Evidence manifest — AJ-E1508

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1508 (copy shared settings into a new account). Status: **DONE-EVIDENCED**. 2026-10-22, claude-opus.

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
| Req.1 Add account offers MCP servers, permissions, plugins and the status line from an account the user picks, each listed and ticked by default, for fresh accounts and copies | Fresh accounts: `account_settings` `a_new_account_gets_the_usual_settings_and_never_a_credential_history_or_cache`, `codex_homes_copy_their_approval_sandbox_and_status_line`, `the_offers_before_the_account_exists_are_the_ones_it_gets`; vitest `fresh-account-settings` (10). Copies, and any account later: every account's "Copy settings from…" (vitest `account-settings-sheet`, 3). Gallery `83-account-settings` | PASS |
| Req.2 Credentials, history and caches never copied (a guardrail test lists the files) | The same test lists every file in the new account's folder after the copy: exactly its marker plus `settings.json` and `.claude.json`. No `.credentials.json`, `history.jsonl`, `projects/`, `todos/` or `statsig/`, and no secret's text in any file. The Codex test does the same: only `config.toml` is new, with no `auth.json` or `sessions/`. | PASS |
| Each copy holds exactly what was ticked | vitest "copies what's ticked once the account is made"; the Codex test (`model`, not picked, isn't copied) | PASS |
| Never over the account's own; undone only when unchanged | `config_edit` tests (4); `a_setting_the_new_account_has_is_left_as_it_is` | PASS |
| The switch's offers stay as they were | `a_switch_still_offers_no_account_settings` | PASS |
| Visual: the Add account section, collapsed and open | gallery `83-account-settings` (collapsed, open, nothing to copy, the sign-in after it), both themes, axe 0 critical/serious; `71-add-account` shows the line | PASS |

## After the reviews

- **H1, secrets:**
  - A setting with a secret written anywhere in it is never copied, and its text is never kept. The check reads word by word: a token-shaped or base64-shaped word, a URL with a password, `Bearer`/`Basic` and a value, a header or flag that names a secret and its value, and `-u user:password`.
  - A `$VARIABLE` is a reference, not a secret, and names match by their parts, so `--author` isn't `auth`.
  - Only the permission rules that hold a secret stay behind, and the line says how many.
  - Undo records keep fingerprints, never values.
- **M1, the fail-safe status line:** under AJ-E1530's fail-safe line, the user's own status line is unknown here, and the item says so. A stale backup beside it is never offered.
- **M2, undo after a re-save:** undo compares values by fingerprint, so a tool re-saving the file in its own layout doesn't block it.
- **M3, M4 and Enter:** Create (the button and Enter alike) waits for what the account offers. A change of account never carries the old ticks over, and the line shows even when the accounts arrive late.
- **M5, what a setting holds:** each setting says what it holds ("4 allowed, 1 denied"). A mode that turns checks off is offered, never ticked.
- **Tests:** `account_settings` (11), `config_edit` (8), `holds_secret` (2), vitest `fresh-account-settings` (10) and `account-settings-sheet` (3).

## On the real setup (`3ecae165`, 2026-10-22)

A Dev build of this branch, walked by QA through the read-only Dev bridge. The full record is
walk batch 27 in `docs/qa/evidence/walks/2026-10-21-real-readonly.md` on
`claude/qa-real-walk-clean` (`e5d08b2a`). The setups were brought in where they are, with
redacted labels. Nothing was copied, and no account was made.

- **Six Claude Code accounts**, each against a new account and against each of the other five
  (36 checks): no offer at all. Their files hold nothing to copy, and nothing was invented:
  - no `permissions`, `statusLine`, `enabledPlugins` or MCP servers;
  - two `settings.json` hold one unrelated key, one is empty, and three are absent;
  - the empty `settings.json` gives no permissions offer.
- **The Codex account** (one home only), against a new account: 13 offers.
  - Its one enabled MCP server is offered as "copy", ticked; its second, `enabled = false`, isn't
    offered.
  - Its 12 plugins are offered as "Needs you", unticked, each with
    "install it here with `codex plugin add …`".
- **The guardrail:**
  - No offer's text carries a secret, or any text found in the source's rules.
  - No offer names a credential, history, session or cache path.
  - The offered server's `env` values are paths and short settings, none secret-shaped.
- **Snapshots:** the real accounts had no path added or removed, and no settings file changed.
  Only the vendors' own writes changed mtimes, at the same size.
- **Not exercised here: the real setup has no permissions and no status lines.** The per-rule
  secret check, the left-out count, the `bypassPermissions` offer that's never ticked, and the
  status-line cases (the user's own; AI Juggler's wrapper or fail-safe as "can't be copied …
  usage capture") rest on the unit and integration tests (`holds_secret`, `account_settings`).

## Found while writing the guide

- Settings › Context packs & secrets titled its undo list "Set up by switches", though it also lists rules applies (AJ-E1507) and settings copies. It's now "Set up by AI Juggler" and says what it holds.

## Follow-ups

- **Plugins** are listed with their install command ("Needs you"), not installed. Installing one fetches it from its marketplace, and copying the plugin folder would carry absolute paths that point into the other account.
- **The regression baselines** for Add account and the account menu change, and are re-rendered in the pinned container.
