# Evidence manifest — AJ-E1605

Task: `docs/tasks/E16-more-tools.md` AJ-E1605 (ZCode). Status:
**IN-PROGRESS** — round 1 (R1, R2, R3, R7) evidenced on fixtures and stand-in
programs; R4's labels, R5 (rules, skills, MCP and bring-in) and R6 (packs)
are not built yet; the live run on a spare account is the maintainer's
(AJ-E1612). 2026-11-13, reviewer claude-opus.

## Basis

Research `docs/research/zcode.md` § C (the installed app re-checked on
2026-11-13: desktop 3.15.3, engine 0.16.9) and the engine's own code
(`Contents/Resources/glm/zcode.cjs`), read for the variables it honours.
`node <engine> version`, `--help` and `doctor` were run in a throwaway
`HOME`; no sign-in, no prompt, and no real `~/.zcode` was read or opened.

Found while building, against the research's two variables: the session
store (`storage.sessionDbPath`) follows the home folder, not
`ZCODE_STORAGE_DIR`, so every launch pins `ZCODE_SESSION_DB_PATH` too
(appended to the research, § C); and the user config
`~/.zcode/cli/config.json` stays shared by every App (no variable moves it;
on the live checklist).

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 discovery: `ZCode.app` in `/Applications` or `~/Applications`, taken as ZCode's when it holds ZCode's engine where ZCode keeps it (an app by that name without it is not; the bundle id `dev.zcode.app` isn't read: the platform lint keeps bundle names and ids behind the OS seam); its engine `Contents/Resources/glm/zcode.cjs` run on a `node` from PATH or Homebrew; the version from `node <engine> version`; never claimed verified; `AI_JUGGLER_ZCODE_ENGINE` overrides the engine | `cargo test -p juggler-adapters --test zcode_adapter` (`zcode_is_found_by_its_app_bundle_and_versioned_by_its_engine`, macOS only: the install folders are a macOS seam, `juggler_platform::apps`); `juggler-platform` `apps::tests::an_app_is_found_by_its_name_in_the_install_folders` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 isolation without moving `HOME`: each App runs with `ZCODE_STORAGE_DIR=<account>/.zcode`, `ZCODE_DATA_BASE_DIR=<account>` and `ZCODE_SESSION_DB_PATH=<account>/.zcode/cli/db/db.sqlite`, pinned after Launch-settings overrides; `ZCODE_SESSION_DB` and an inherited `ZCODE_PERSONAL_PROVIDER_CONFIG_FILE` removed; the default `~/.zcode` runs with all three unset; the copyable line keeps the same rules | `zcode_adapter::each_app_has_its_own_tree_and_zcodes_own_runs_with_the_variables_unset`; `adapter_conformance` `zcode::{isolates_each_home, pins_env_after_overrides}`; `zcode_provider::zcode_is_listed_isolated_and_its_shell_line_pins_its_tree` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 sign-in: ZCode's own `login` in the App's tree; the probe sees whether `<tree>/v2/credentials.json` is there and not empty, never its contents (in `sign_in.rs`) | `zcode_adapter::the_sign_in_probe_sees_the_file_never_its_contents_and_login_runs_in_the_tree`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 launch check: after a launch the engine's store is in the App's tree, else the App is refused with a reason (`zcode::isolation_took`; a function this round, not yet called by the app) | `zcode_adapter::the_launch_check_refuses_an_app_whose_tree_the_engine_did_not_write` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 the variables honoured by a real engine run (sessions, sign-in and usage land in the App's tree; the shared user config's effect) | live, on a spare account: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R3 sessions from `<tree>/cli/db/db.sqlite`, read-only (a closed WAL store opened `immutable`, left byte for byte, nothing written beside it): interactive, not archived, not a sub-agent's; title, folder, times; found for a launch (never a guess between two); resumed `--resume <sess_…>`; a running engine recognised by its command line, helpers (`app-server`, `doctor`, `version`, `login`, …) not; their folders become projects | `zcode_adapter::{sessions_come_from_the_store_read_only_and_byte_for_byte_unchanged, launches_run_the_engine_on_node_and_resume_by_session_id, a_running_engine_is_a_session_and_its_helpers_are_not}`; kit `zcode::finds_and_resumes_sessions`; `zcode_provider::its_sessions_become_projects_and_the_store_is_left_unchanged` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 against a store a real engine wrote | live: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R4 usage Computed per session and account, its labels, the plan's caps set by the user | NOT YET: this round only moved the existing `model_usage` read behind the adapter (`usage_since`, the App's own store; `zcode_adapter::usage_is_read_from_the_apps_own_store`, kit `zcode::labels_usage_honestly`) | — | — | NOT YET | branch head | 2026-11-13 | claude-opus |
| R5 rules, skills and MCP in the sync; bring-in in place or as a copy without the sign-in, store, logs or rollouts | NOT YET (the kit's `zcode::copies_no_credentials` holds that every bring-in refuses for now) | — | — | NOT YET | branch head | 2026-11-13 | claude-opus |
| R6 packs both ways with Claude Code and Codex | NOT YET | — | — | NOT YET | branch head | 2026-11-13 | claude-opus |
| R7 ADR-0019: ZCode is data (its spec; its terms in AJ-E1518's register, read 2026-11-13; its letter mark), registered with no new tool-id literal outside its adapter (the usage pipeline's own ZCode branch removed); shown in Add account, first run and its page from its spec alone | `zcode_adapter::the_spec_describes_zcode_as_data`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `just tool-literals`; `zcode_provider::zcode_is_listed_isolated_and_its_shell_line_pins_its_tree` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R7 it passes the kit (AJ-E1521) | `cargo test -p juggler-adapters --test adapter_conformance zcode` — 15 of 15 | 15 pass | 15 pass | PASS | branch head | 2026-11-13 | claude-opus |
| R7 the screens with ZCode in them, looked at | not captured this round (the preview catalog carries ZCode; no visual baseline was regenerated) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |

## Not offered yet (each says so in words)

- Runs without a terminal: ZCode's `-p` defaults to `--mode yolo` (skips its
  permission prompts), so a headless launch is refused until checked live.
- A first message, or a message on resume: its terminal takes none.
- A model from Launch settings: its terminal has no model option; ZCode's
  own choice stands.
- Its sub-agents: "not supported" with the reason; the agent tools; the
  desktop app per App.
