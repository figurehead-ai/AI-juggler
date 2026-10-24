# Evidence manifest — AJ-E1604

Task: `docs/tasks/E16-more-tools.md` AJ-E1604 (Grok Build CLI). Status:
**IN-PROGRESS** — Req.1–3 evidenced on fixtures and stub binaries; Req.4's
measured tokens wait for the live check (the fields of `signals.json`
aren't documented); the live run on a spare account is the maintainer's
(AJ-E1612, `docs/internal/MAINTAINER-STEPS.md` step 16). 2026-10-24,
reviewer claude-opus.

## Basis

Research `docs/research/grok.md` (xAI's docs and the Apache-2.0
`xai-org/grok-build` source, crate 1.0.45, its shipped user guide), read
2026-10-21. Nothing was installed or run, and no real `~/.grok` was read.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 the official `grok` found by path and kind: the installer's `~/.grok/bin/grok` first, never a Node script by the same name (the unofficial `grok-cli`); version from `--version`, never claimed verified | `cargo test -p juggler-adapters --test grok_build_adapter` (`the_official_grok_is_found_by_path_and_kind_never_the_unofficial_script`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.1 isolation: `GROK_HOME` per account, pinned after Launch-settings overrides; the default `~/.grok` runs with it unset; `GROK_AUTH_PATH` and an inherited `XAI_API_KEY` removed | `adapter_conformance` (`grok_build::*`); `grok_build_adapter::each_account_has_its_own_home_and_never_runs_the_users_claude_setup`; `grok_provider::grok_build_is_listed_isolated_and_says_its_limits_arent_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.2 the user's Claude and Cursor setups stay out: every launch, the sign-in and the copyable line set `GROK_CLAUDE_{AGENTS,HOOKS,MCPS,RULES,SKILLS}_ENABLED=0` and the `GROK_CURSOR_*` ones, and no override changes them (`HarnessAdapter::launch_fixed_env`; the kit checks it for every tool) | `grok_build_adapter::{each_account_has_its_own_home_and_never_runs_the_users_claude_setup, sign_in_runs_grok_login_in_the_accounts_home}`; kit `pins_env_after_overrides`; `grok_provider` (the shell line) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.3 sessions from `sessions/<encoded folder>/<id>/summary.json` (a shortened folder's `.cwd` file read), a sub-agent's left out; found for a launch; resumed `--resume <id>`; a running `grok` recognised, helpers not; their folders become projects | `grok_sessions` unit tests; `grok_build_adapter::sessions_are_listed_resumed_and_told_from_helpers`; kit `finds_and_resumes_sessions`; `grok_provider::its_sessions_become_projects_and_its_default_home_is_never_written` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.4 usage: the weekly pool is unknown and said so ("Grok Build doesn't report its limits to AI Juggler yet"); no usage is claimed | `grok_provider::grok_build_is_listed_isolated_and_says_its_limits_arent_read`; kit `labels_usage_honestly` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.4 tokens measured from `signals.json`; a 402 read as the pool exhausted | not built: `signals.json`'s fields and the 402's exact output aren't documented, and no field is guessed | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |
| Sign-in: Grok's own `grok login`; the probe reads each `auth.json` entry's `auth_mode` (in `sign_in.rs`), never a key, token or email | `grok_build_adapter::{the_sign_in_probe_reads_modes_never_values, sign_in_runs_grok_login_in_the_accounts_home}`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Bring-in: any home in place, unwritten; a copy keeps settings, rules and sessions, never `auth.json`, `mcp_credentials.json`, the installer's files or a settings file with a key written in it; the unofficial CLI's `~/.grok/user-settings.json` alone isn't a home | `grok_build_adapter::{a_copy_keeps_settings_and_sessions_but_never_a_sign_in_or_a_key, the_scan_finds_homes_but_not_the_unofficial_clis_folder}`; kit `copies_no_credentials` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Rules and MCP join the sync: `[mcp_servers.<name>]` (read as Codex's) and `rules/*.md` listed; a local server added beside the rest and undone; a remote one with headers refused (undocumented) | `grok_inventory` unit tests | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| ADR-0019: Grok Build is data (spec, terms with the X Subscription's shared pool in AJ-E1518's register, the letter mark); it shows in Add account, first run and its page | `grok_build_adapter::the_spec_describes_grok_build_as_data`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `ui/tests/visual/more-tools-gallery.spec.ts` (run locally, 10 of 10; captures checked by eye, not committed) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| It passes the kit (AJ-E1521) | `adapter_conformance` `grok_build::*` — 6 of 6 | 6 pass | 6 pass | PASS | branch head | 2026-10-24 | claude-opus |
| Live: two homes' sign-ins apart, a Claude hook never fires inside Grok, a session starts and resumes, `--model`, a 402 | the maintainer's checklist, step 16 (Grok Build) | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |

## Not claimed (on the live checklist)

- `grok --version`'s exact output; `summary.json`'s field names (title,
  times, the parent); `signals.json`'s; the `.cwd` file's place.
- `--model` for a Launch-settings model; a first message (refused, so
  nothing is switched to Grok); `[compat.codex]` has no variable to turn
  it off (it reads skills and hooks).
- Whether two homes signed in as one identity sign each other out.
- No update check: Grok Build updates itself (`grok update`), and no
  public package list is asked.

## Redaction checklist

- [x] No credentials, tokens or keys (every sign-in in the fixtures is a made-up string)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
