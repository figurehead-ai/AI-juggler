# Evidence manifest — AJ-E1602

Task: `docs/tasks/E16-more-tools.md` AJ-E1602 (OpenCode CLI). Status:
**DONE-EVIDENCED** on fixtures and stub binaries, built from OpenCode's
source; the live run on a spare account is the maintainer's
(AJ-E1612, `docs/MAINTAINER-STEPS.md` step 16). 2026-10-21, reviewer
claude-opus.

## Basis

OpenCode's source at the tags `v1.19.3` and `v2.1.2`
(`https://github.com/anomalyco/opencode`), read as text on 2026-10-21;
nothing was installed or run, and no real OpenCode folder was read.

- Paths: every state folder comes from the XDG roots —
  `packages/core/src/global.ts` (v1), `packages/util/src/global-roots.ts`
  (v2: `XDG_DATA_HOME`/`XDG_CONFIG_HOME`/`XDG_STATE_HOME`/`XDG_CACHE_HOME`
  + `opencode`). v2 uses `OPENCODE_CONFIG_DIR` instead of the config
  root (`packages/util/src/global.ts`); v1 loads it after the global one.
- Database: `$XDG_DATA_HOME/opencode/opencode.db` on the shipped
  channels, `OPENCODE_DB` overrides (`packages/core/src/database/
  database.ts` v1; `packages/cli/src/database-path.ts` v2), WAL mode.
- Tables: v1 `session` + `message` (JSON `data`: `role`, `modelID`,
  `providerID`, `cost`, `tokens{input,output,reasoning,cache{read,write}}`,
  `time{created,completed}` — `packages/schema/src/v1/session.ts`);
  v2 `session_v2` + `session_message` (`type='assistant'`, `model{id,
  providerID}`, `cost`, `tokens`, `time` — `packages/schema/src/
  session-message.ts`, `token-usage.ts`). Sign-ins: `auth.json` (v1),
  `credential` (both), Console `account`/`account_state`, share secrets
  in `session_share`.
- CLI: v1 TUI `--session/-s`, `--continue/-c`, `--prompt`, `--model`
  (`packages/opencode/src/cli/cmd/tui.ts`), `run [message..]` with `--`
  joined into the message (`cmd/run.ts`); v2 root `--session`,
  `--continue`, `--prompt`, a `directory` positional and no `--model`;
  `run` has `--model` (`packages/cli/src/commands/commands.ts`).
- Version: v1 prints the bare version (yargs `.version`); v2 passes
  `OPENCODE_VERSION` to its CLI runtime (`packages/cli/src/index.ts`).

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 discovery of v1 and v2 by version output, never claimed verified (no live run yet); a 3.x says it is newer; a missing binary is not found | `cargo test -p juggler-adapters --test opencode_adapter` (`versions_of_both_lines_parse`, `discovery_tells_the_lines_apart_and_never_claims_a_live_check`, `a_missing_binary_is_not_found`) | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.1 existing data folders found: the default `~/.local/share/opencode` and `$XDG_DATA_HOME/opencode` | `opencode_adapter::the_scan_finds_the_default_and_an_xdg_folder` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.2 isolation: `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `OPENCODE_CONFIG_DIR` each point at the account's own folder, pinned after Launch-settings overrides; `OPENCODE_DB`, `OPENCODE_TEST_HOME`, `OPENCODE_AUTH_CONTENT` and inherited provider keys removed; the default folder runs with all four unset | `cargo test -p juggler-adapters --test adapter_conformance` (`opencode_v1::*`, `opencode_v2::*` — 12 checks); `opencode_adapter::{each_account_gets_its_own_xdg_folders, the_default_folder_runs_as_a_plain_opencode, inherited_sign_ins_and_database_movers_never_reach_a_launch}` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.2 the isolation checklist passes for an OpenCode account (four pinned variables, four folders) and its shell line pins them | `cargo test -p juggler-app --test opencode_provider` (`an_opencode_account_is_isolated_in_four_folders_and_the_checklist_says_so`) | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.3 sessions read from `opencode.db`, read-only, for v1, v2 and a v1 database v2 upgraded in place; a sub-agent's session is never the one linked; reading writes nothing even with OpenCode closed (an `immutable` open: no `-wal`/`-shm` appear) | `opencode_adapter::sessions_of_both_lines_and_of_an_upgraded_database`; kit `finds_and_resumes_sessions`, `labels_usage_honestly` (the folder's files are compared before and after) | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.3 cost and tokens: completed assistant messages become usage events keyed `opencode:<message id>` (Measured tokens, OpenCode's price as `usd_est`, no credits); a running message waits; read once through the pipeline; the limit rings say there is nothing to read | `opencode_adapter::usage_reads_completed_assistant_messages_with_their_estimate`; `opencode_provider::usage_comes_from_the_database_once_and_quota_says_there_is_no_limit_to_read` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.3 resume and continue: `--session <id>` with the next message as `--prompt=<text>` (a leading "-" stays text); option-like ids refused; headless `run --format json --model=… -- <text>`; a Launch-settings model reaches the TUI as inline settings (v2's TUI has no `--model`) | `opencode_adapter::launches_per_mode`; kit `refuses_option_like_resume_ids` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Bring-in, as Codex (ADR-0018): in place for the default folder only, nothing written; a copy keeps sessions (a consistent `VACUUM INTO` snapshot) and settings (`~/.config/opencode`, plugin packages excepted) but never `auth.json`, `mcp-auth.json` or the database's sign-in tables (emptied, then vacuumed); the original is untouched | `opencode_adapter::{a_copy_keeps_sessions_and_settings_but_never_a_sign_in, a_settings_only_copy_leaves_the_history, only_the_default_folder_is_used_where_it_is}`; kit `copies_no_credentials` (a canary in `auth.json`, `mcp-auth.json` and three tables: absent from every copied byte); `opencode_provider::the_default_folder_is_used_where_it_is_and_never_written` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Sign-in probe: `auth.json` entry kinds only (`oauth`/`wellknown` → signed in, `api` → API key), the v2 `credential` table's active-row count; never a value | `opencode_adapter::{the_sign_in_probe_reads_kinds_never_values, a_v2_sign_in_in_the_database_counts}` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| First run and Add account offer it: OpenCode folders are their own group with an install note when OpenCode isn't installed; Add account's chooser lists it (from the registry); one tool's words for an OpenCode-only list | vitest `first-run.test.tsx` "AJ-E1602 — OpenCode in first run and Add account" (4); `opencode_provider::opencode_is_a_listed_provider` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Projects from history: the folders OpenCode sessions ran in become projects | `opencode_provider::an_opencode_sessions_folder_becomes_a_project` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| It passes the kit (AJ-E1521) | `adapter_conformance` — 30 of 30 with OpenCode v1 and v2 | 30 pass | 30 pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.4 `opencode serve` only where the files fall short | not used: the database gives sessions, cost and tokens; no server is started | — | not needed | PASS | branch head | 2026-10-21 | claude-opus |
| Req.5 rules (`AGENTS.md`) and MCP join the sync | depends on AJ-E1507, not built | — | — | UNPROVEN | branch head | 2026-10-21 | claude-opus |
| Live: sign-ins separate across two homes, a session starts and resumes, usage is read, quota exhaustion seen | the maintainer's checklist, `docs/MAINTAINER-STEPS.md` step 16 | — | — | UNPROVEN | branch head | 2026-10-21 | claude-opus |

## Not claimed

- No OpenCode binary ran. The `--prompt=` and `--model=` forms and `--`
  on 2.x's CLI parser, and the model through `OPENCODE_CONFIG_CONTENT`,
  are on the live checklist.
- Channel databases (`opencode-<channel>.db`, non-release builds) are
  not read.
- The XDG variables reach the tools the agent runs (`gh` looks for its
  settings in the account's `config/`): a known trade-off, on the live
  checklist.
- Switching a session to or from OpenCode is not offered (no handover
  support registered).
- OpenCode Desktop is not covered (track-only, a separate task).

## Redaction checklist

- [x] No credentials, tokens or keys (every sign-in in the fixtures is a made-up string)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
