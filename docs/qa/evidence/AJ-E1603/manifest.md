# Evidence manifest — AJ-E1603

Task: `docs/tasks/E16-more-tools.md` AJ-E1603 (Kimi Code CLI). Status:
**IN-PROGRESS** — Req.1, 2 and 4 evidenced on fixtures and stub binaries;
Req.3 (usage from `kimi web`) waits for the maintainer's ruling (below);
the live run on a spare account is the maintainer's (AJ-E1612,
`docs/internal/MAINTAINER-STEPS.md` step 16). 2026-10-24, reviewer
claude-opus.

## Basis

Research `docs/research/kimi.md` (Kimi Code docs, the MIT
`MoonshotAI/kimi-code` source at 2.3.0, the Kimi Terms of Service and the
Kimi Code Community Guidelines), read 2026-10-21. Nothing was installed or
run, and no real Kimi Code home was read.

- Home: `KIMI_CODE_HOME` (default `~/.kimi-code`) holds config,
  sign-ins, sessions, logs, its `AGENTS.md`, skills and `mcp.json`;
  `~/.agents/` stays shared across tools (`KD/configuration/
  data-locations.md`).
- Sign-ins: `credentials/<name>.json`, the membership's as
  `credentials/kimi-code.json` (`SRC/packages/oauth`, its storage test).
- Sessions: `session_index.jsonl` (`sessionId`, `sessionDir`, `workDir`)
  and `sessions/<workDirKey>/<id>/state.json` (title, `lastPrompt`,
  times) beside `agents/main/wire.jsonl`; resume `--session/-S <id>`;
  headless `-p … --output-format stream-json`; model `-m`.
- Terms: "Kimi Code subscriptions are for interactive use only"; never
  alter the client's identity (`KIMI_CODE_IDENTITY_*`).

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 discovery of `kimi` by path and version: the installer's `~/.kimi-code/bin/kimi` first, then PATH, Homebrew, npm; 2.x is Kimi Code, never claimed verified; the archived Python Kimi CLI (0.x, 1.x, same command name) is told apart and not offered | `cargo test -p juggler-adapters --test kimi_code_adapter` (`versions_parse_and_the_old_python_cli_is_told_apart`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.1 isolation: `KIMI_CODE_HOME` per account, pinned after Launch-settings overrides; the default `~/.kimi-code` runs with it unset; `KIMI_CODE_IDENTITY_*` never set | `adapter_conformance` (`kimi_code::*`); `kimi_code_adapter::each_account_has_its_own_home_and_the_default_runs_as_a_plain_kimi`; `kimi_provider::a_kimi_account_has_its_own_home_and_its_limits_say_they_arent_read` (the isolation checklist and the shell line) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.1 existing homes found: `~/.kimi-code`, `$KIMI_CODE_HOME`, `~/.kimi-code*`, never the old CLI's `~/.kimi` | `kimi_code_adapter::the_scan_finds_the_default_an_override_and_other_homes_but_not_the_old_cli` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.2 sessions from the documented layout and `session_index.jsonl`: listed (title, else the last prompt; folder; times, else the file's), found for a launch and resumed `--session <id>`; a running `kimi` recognised, helpers (`web`, `login`, `acp`…) not; tokens unknown (never zero) | `kimi_sessions` unit tests; `kimi_code_adapter::sessions_are_listed_resumed_and_told_from_helpers`; kit `finds_and_resumes_sessions`; `kimi_provider::its_sessions_are_listed_and_their_folders_become_projects` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 usage from `kimi web`'s usage endpoint, labelled experimental | not built: it needs the home's `server.token` (a bearer token) and a request that `kimi web` answers from Moonshot's servers; the Privacy page says AI Juggler reads no tokens and makes no requests to AI providers. Kimi Code claims no usage reader, and its limits say they aren't read | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |
| Req.4 interactive use only on a subscription: a membership account (or one whose sign-in can't be told) refuses headless runs, and a crash is never restarted by itself (its history says why); an account on a Kimi Platform API key may run headless | `kimi_code_adapter::launches_per_mode_and_headless_only_on_an_api_key`; `juggler-app` lib `restart::tests::an_interactive_only_account_is_never_restarted`; `kimi_provider::a_membership_account_that_crashes_is_never_started_again_by_itself` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Sign-in: Kimi Code's own `kimi login` in the account's home; the probe reads which files are in `credentials/` (in `sign_in.rs`), never one's contents | `kimi_code_adapter::{sign_in_runs_kimi_login_in_the_accounts_home, the_sign_in_probe_reads_which_files_are_there_never_what_they_hold}`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Bring-in (ADR-0018): any home in place, unwritten; a copy keeps settings, skills, `mcp.json` and `AGENTS.md` but never `credentials/`, `server.token`, a settings file with a key written in it, or the sessions (their list may name the original's folders) | `kimi_code_adapter::{a_copy_keeps_settings_but_never_a_sign_in_a_key_or_the_sessions, a_home_is_used_where_it_is_unwritten_and_its_setup_read_from_it}`; kit `copies_no_credentials` (a canary in the membership file, an MCP sign-in and `config.toml`); `kimi_provider::the_default_home_is_used_where_it_is_and_never_written` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Rules, skills and MCP join the sync: `AGENTS.md`, `skills/<name>/SKILL.md` and `mcp.json` (`mcpServers`, read as Claude's) listed; a server from another tool added beside the rest and undone only while unchanged; a secret never written | `kimi_inventory` unit tests | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| ADR-0019: Kimi Code is data (spec, terms in AJ-E1518's register, the letter mark); no tool-id literal outside the adapter; it shows in Add account, first run and its page | `kimi_code_adapter::the_spec_describes_kimi_code_as_data`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `python3 scripts/tool_literal_lint.py`; `ui/tests/visual/more-tools-gallery.spec.ts` (run locally, 8 of 8; captures checked by eye, not committed) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| It passes the kit (AJ-E1521) | `adapter_conformance` `kimi_code::*` — 6 of 6 (a tool may now refuse a headless run or a first message, saying so) | 6 pass | 6 pass | PASS | branch head | 2026-10-24 | claude-opus |
| Live: sign-ins separate across two homes, devices per home, a session starts and resumes, the interactive-only guardrail, an API key's headless run | the maintainer's checklist, `docs/internal/MAINTAINER-STEPS.md` step 16 (Kimi Code) | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |

## For the maintainer

- **Req.3's usage source.** The documented `GET /api/v1/oauth/usage` on
  `kimi web`'s local server needs the bearer token in the home's
  `server.token`, a long-lived `kimi web` per account, and is answered
  "live from the account service". Building it changes two Privacy page
  lines (no tokens read; no requests to AI providers). Until a ruling,
  Kimi Code's limits say they aren't read.
- **The device quota per home** is unknown (each home mints its own
  device id); the live check counts it.

## Not claimed (on the live checklist)

- `kimi --version`'s exact output; the membership file's name
  (`kimi-code.json`, from the source's test) and an API key's sign-in file
  (any other file in `credentials/`, inferred); the fields of
  `state.json` and `session_index.jsonl` beyond their documented names.
- A headless prompt starting with "-" after `-p`; `-m` for the model.
- Starting with a first message, or resuming with one: not documented, so
  refused (a switch to Kimi Code isn't offered).
- Whether a copied session list would point at the original (so
  sessions stay behind for now).
- Kimi Code Desktop (track-only, AJ-E1610); the legacy CLI's `kimi
  migrate`.

## Redaction checklist

- [x] No credentials, tokens or keys (every sign-in in the fixtures is a made-up string)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
