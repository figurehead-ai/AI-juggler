# Evidence manifest — AJ-E1602

Task: `docs/tasks/E16-more-tools.md` AJ-E1602 (OpenCode CLI). Status:
**DONE-EVIDENCED** on fixtures and stub binaries, built from OpenCode's
source; the live run on a spare account is the maintainer's
(AJ-E1612, `docs/internal/MAINTAINER-STEPS.md` step 16). First built
2026-10-21 (parked on `claude/e1602-opencode`), brought onto 0.2 and
finished 2026-10-24, reviewer claude-opus.

## Basis

OpenCode's source at the tags `v1.19.3` and `v2.1.2`
(`https://github.com/anomalyco/opencode`), read as text on 2026-10-21;
nothing was installed or run, and no real OpenCode folder was read.
Research: `docs/research/opencode.md`.

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
- Settings: `opencode.json` / `opencode.jsonc` with `mcp` (`type: local`
  with a `command` list and `environment`, or `type: remote` with `url`
  and `headers`), and `AGENTS.md` beside it (research §6).

## What changed on 0.2 (shared building blocks, ADR-0019 §2)

The parked branch predated the tools catalog and the conversation index;
its per-tool branches in `commands.rs` and the usage pipeline were
replaced by adapter calls any tool can use:

- `HarnessAdapter::stored_conversations` — conversations kept in one
  store (a database), keyed `<store>#<id>`, read again only once they
  changed; the conversation index reads them beside per-file ones.
- `HarnessAdapter::usage_since` — usage from a store read by time; the
  pipeline's one generic arm keeps the high-water mark.
- A running session's account is matched by any of its tool's pinned
  folders (OpenCode pins four), not by the first variable alone.
- `jsonc::parse` (JSON with comments as a value) and nested JSON keys in
  `config_edit` (a JSON Pointer, `/mcp/<name>`), for OpenCode's settings.
- OpenCode's sign-in file is read only in `sign_in.rs`, by entry kind
  (`no_bare_tools.rs` enforces it).

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 discovery of v1 and v2 by version output (with the user's PATH), never claimed verified (no live run yet); a 3.x says it is newer; a missing binary is not found | `cargo test -p juggler-adapters --test opencode_adapter` (`versions_of_both_lines_parse`, `discovery_tells_the_lines_apart_and_never_claims_a_live_check`, `a_missing_binary_is_not_found`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.1 existing data folders found: the default `~/.local/share/opencode` and `$XDG_DATA_HOME/opencode` | `opencode_adapter::the_scan_finds_the_default_and_an_xdg_folder` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.2 isolation: `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `OPENCODE_CONFIG_DIR` each point at the account's own folder, pinned after Launch-settings overrides; `OPENCODE_DB`, `OPENCODE_TEST_HOME`, `OPENCODE_AUTH_CONTENT` and inherited provider keys removed; the default folder runs with all four unset | `cargo test -p juggler-adapters --test adapter_conformance` (`opencode_v1::*`, `opencode_v2::*`); `opencode_adapter::{each_account_gets_its_own_xdg_folders, the_default_folder_runs_as_a_plain_opencode, inherited_sign_ins_and_database_movers_never_reach_a_launch}` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.2 the isolation checklist passes for an OpenCode account (four pinned variables, four folders) and its shell line pins them | `cargo test -p juggler-app --test opencode_provider` (`an_opencode_account_is_isolated_in_four_folders_and_the_checklist_says_so`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 sessions read from `opencode.db`, read-only, for v1, v2 and a v1 database v2 upgraded in place; a closed database is opened `immutable` (no `-wal`/`-shm` appear) | `opencode_adapter::sessions_of_both_lines_and_of_an_upgraded_database`; kit `finds_and_resumes_sessions`, `labels_usage_honestly` (the folder's files compared before and after) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 its sessions are in the conversation list (top-level, not archived; a sub-agent's belongs to its parent), their folders become projects, and a running `opencode` is recognised (the TUI, `run`; never `serve`, `auth`, `mcp`…), its account matched by any pinned folder | `opencode_adapter::{sessions_are_listed_for_the_index_without_sub_agents_or_archived_ones, a_session_resumes_from_a_terminal_and_a_running_one_is_told_from_helpers}`; `opencode_provider::an_opencode_sessions_folder_becomes_a_project`; `juggler-app` lib `live::tests::a_session_belongs_to_the_account_its_pinned_folders_name` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 cost and tokens: completed assistant messages become usage events keyed `opencode:<message id>` (Measured tokens, OpenCode's price as `usd_est`, no credits); a running message waits; read once through the pipeline's store arm; the limit rings say OpenCode doesn't report its limits | `opencode_adapter::usage_reads_completed_assistant_messages_with_their_estimate`; kit `labels_usage_honestly` (through `usage_since`); `opencode_provider::usage_comes_from_the_database_once_and_quota_says_there_is_no_limit_to_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 resume and continue: `--session <id>` with the next message as `--prompt=<text>` (a leading "-" stays text); the copyable resume line `opencode --session <id>`; option-like ids refused; headless `run --format json --model=… -- <text>`; a Launch-settings model reaches the TUI as inline settings | `opencode_adapter::{launches_per_mode, a_session_resumes_from_a_terminal_and_a_running_one_is_told_from_helpers}`; kit `refuses_option_like_resume_ids` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.4 `opencode serve` only where the files fall short | not used: the database gives sessions, cost and tokens; no server is started | — | not needed | PASS | branch head | 2026-10-24 | claude-opus |
| Req.5 rules (`AGENTS.md`) and MCP join the sync: its spec names `AGENTS.md`; its servers (`mcp`, `local`/`remote`, `{env:NAME}` kept as a reference, a secret never kept) and its main rules are listed; a server from another tool is added beside the rest (every other byte kept) and undone only while unchanged; SSE can't come | `cargo test -p juggler-adapters --lib opencode_inventory`; `config_edit::tests::a_nested_json_value_is_set_inside_its_object_and_taken_out_again`; `jsonc::tests::json_with_comments_reads_as_a_value`; `opencode_adapter::rules_and_servers_are_read_from_the_accounts_own_settings_folder` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Bring-in, as Codex (ADR-0018): in place for the default folder only, nothing written; a copy keeps sessions (a consistent `VACUUM INTO` snapshot) and settings but never `auth.json`, `mcp-auth.json`, the database's sign-in tables, or a settings file with a key written in it (named in what stayed behind) | `opencode_adapter::{a_copy_keeps_sessions_and_settings_but_never_a_sign_in, a_settings_file_with_a_key_written_in_it_stays_behind_named, a_settings_only_copy_leaves_the_history, only_the_default_folder_is_used_where_it_is}`; kit `copies_no_credentials` (a canary in `auth.json`, `mcp-auth.json`, three tables and a settings file's `apiKey`); `opencode_provider::the_default_folder_is_used_where_it_is_and_never_written` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Sign-in probe: `auth.json` entry kinds only (`oauth`/`wellknown` → signed in, `api` → API key), read in `sign_in.rs`; the v2 `credential` table's active-row count; never a value | `opencode_adapter::{the_sign_in_probe_reads_kinds_never_values, a_v2_sign_in_in_the_database_counts}`; `juggler-app --test no_bare_tools` `no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| ADR-0019: OpenCode is data — its spec (name, install, sign-in words, group, `AGENTS.md`, terms) reaches every screen through the catalog; no tool-id literal outside the adapter | `opencode_adapter::the_spec_describes_opencode_as_data`; `juggler-app --test tools_catalog`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `python3 scripts/tool_literal_lint.py` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| It appears wherever tools appear: Add account's chooser, first run (its own group, its install state), its page under More › Tools; a version nobody tried yet says "not tested with AI Juggler yet" | `ui/tests/visual/more-tools-gallery.spec.ts` (run locally, 6 of 6; captures checked by eye, not committed: the central gate makes the baselines); vitest `inspector-aggregate.test.tsx` "says what's installed in plain words", `beta-matrix.test.tsx` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| It passes the kit (AJ-E1521) | `adapter_conformance` — 30 of 30 with OpenCode v1 and v2 | 30 pass | 30 pass | PASS | branch head | 2026-10-24 | claude-opus |
| Live: sign-ins separate across two homes, a session starts and resumes, usage is read, quota exhaustion seen, outside sessions listed, a server synced | the maintainer's checklist, `docs/internal/MAINTAINER-STEPS.md` step 16 | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |

## Not claimed (on the live checklist)

- No OpenCode binary ran. The `--prompt=` and `--model=` forms and `--`
  on 2.x's CLI parser, and the model through `OPENCODE_CONFIG_CONTENT`,
  are unverified.
- That OpenCode merges `config.json`, `opencode.json` and
  `opencode.jsonc` in that order is read from the source, not seen.
- Channel databases (`opencode-<channel>.db`, non-release builds) are
  not read.
- The XDG variables reach the tools the agent runs (`gh` looks for its
  settings in the account's `config/`): a known trade-off.
- A session's text isn't shown or searched (it has no transcript file:
  its messages are rows in the database). Switching from OpenCode is not
  offered; to it, only a briefing pack (no handover support registered).
- OpenCode Desktop is not covered (track-only, AJ-E1610).

## Redaction checklist

- [x] No credentials, tokens or keys (every sign-in in the fixtures is a made-up string)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
