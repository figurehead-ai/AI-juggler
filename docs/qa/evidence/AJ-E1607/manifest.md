# Evidence manifest — AJ-E1607

Task: `docs/tasks/E16-more-tools.md` AJ-E1607 (Hermes Agent, Nous
Research's `hermes` CLI). Status: **IN-PROGRESS** — the adapter (each
account a `HERMES_HOME` of its own with its own gateway lock folder, the
guard that keeps Hermes away from Codex's and Claude's homes, sessions from
its `state.db`, usage from `hermes usage --json`, a gateway AI Juggler owns
by its process group) is built and evidenced on fixtures made from the MIT
source and a stand-in that behaves as that source says Hermes does. Hermes
is not installed on the build Mac (installing it waits for the
maintainer's yes), so nothing has run it: every live claim is PENDING on
AJ-E1612's lab checklist (`docs/internal/MAINTAINER-STEPS.md` § Hermes
Agent).

## Basis

- Research `docs/research/hermes.md` (2026-10-21), and its §10, the D-106
  source pass of 2026-11-13: `NousResearch/hermes-agent` at
  `422ad1861856dd9b80b947fbad9f39d1580f6b41`, cloned into a scratch folder
  and read as data (never run or installed), every fact cited by file and
  line there.
- The Nous Portal Terms of Service ("Last update: November 10, 2026"), read
  on Nous Research's site on 2026-11-13 (AJ-E1518's register, § Hermes
  Agent).
- The fixture's store is created by Hermes' own `SCHEMA_SQL` (schema 33),
  its tables copied as the source writes them
  (`crates/juggler-adapters/tests/hermes_fixture/mod.rs`).

## Routes found and gaps (D-106)

- Isolation: `HERMES_HOME` (whole home), `HERMES_SHARED_AUTH_DIR` (the
  Nous sign-in shared across a root's profiles), `HERMES_GATEWAY_LOCK_DIR`
  (one gateway per lock folder multiplexes every home that dials it — not
  under the home), `HERMES_KANBAN_HOME`, `HERMES_DATA_DIR_SUFFIX`.
- The guard's route: Hermes' own administrator layer, `HERMES_MANAGED_DIR`
  (launch-only; its `config.yaml` and `.env` win over the user's), setting
  `auth.adopt_external_logins: false` (the one switch that stops Hermes
  borrowing and refreshing `~/.codex/auth.json`, Claude Code's
  `.credentials.json` and its Keychain item) and pointing `CODEX_HOME` and
  `CLAUDE_CONFIG_DIR` at empty folders.
- Gateway: `gateway.pid` (JSON record), `--external-supervisor` (owned
  foreground run), `hermes gateway stop`, `gateway ensure|attach --json`; a
  client-started gateway is a launchd job on macOS.
- Found, not built: ACP over stdio (`hermes acp`) and the TUI gateway's
  JSON-RPC (`hermes-gateway-v1`) for a driven runtime.
- Gaps: Nous Portal credits have no `hermes usage` reader (only a session's
  `/usage`); first-time `hermes setup`'s "Coming from another agent?" import
  finds `~/.claude` and `~/.codex` by `HOME`, not by the guarded variables
  (AI Juggler never runs `setup` or `import-agent`; the lab checks whether a
  first run reaches it); the app doesn't yet start or stop an owned gateway
  on a session's lifecycle (the adapter's `start_gateway`/`stop_gateway`
  exist; wiring them waits for the driven runtime).

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 Nous Research's Hermes Agent is the one meant (D-74): found as the installer's `~/.local/bin/hermes`, then on the PATH and Homebrew's; versioned from `hermes --version`'s banner; never claimed verified; a running Hermes is a session only in its chat (no command or `chat`), `--resume <id>` naming it; its gateway (`hermes gateway run`) is the home's session server | `cargo test -p juggler-adapters --test hermes_adapter` (`hermes_is_found_by_its_wrapper_and_its_banner`, `a_running_hermes_says_whether_it_is_a_session_or_its_gateway`); `--lib hermes::tests` (3) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R1 registered as data (ADR-0019): the registry (`state.rs`), Add account and first run from `hermes_spec()`; the preview catalog's entry; the letter mark (`hermes`, Nous Research, not drawn); the Nous Portal terms line (§5.1, §5.2, §10.1(7)) with its URL and date, checked on the live page 2026-11-13 | `cargo test -p juggler-app --test trust` (`every_tool_names_its_terms_and_the_preview_matches`), `--test tools_catalog`, `--test no_bare_tools`, `--test tool_updates`; `cd ui && pnpm exec vitest run` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 each account a home of its own: `HERMES_HOME=<account>`, `HERMES_GATEWAY_LOCK_DIR=<account>/.ai-juggler/gateway-locks`, `HERMES_SHARED_AUTH_DIR=<account>/shared`, all after Launch-settings overrides; `HERMES_KANBAN_HOME` and `HERMES_DATA_DIR_SUFFIX` removed; the user's own `~/.hermes` runs with its home, lock and shared variables unset | `hermes_adapter::each_account_is_a_hermes_home_of_its_own_and_the_users_own_runs_as_hermes_does`; kit `hermes_agent::{isolates_each_home, pins_env_after_overrides}` (15 of 15) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 two accounts' Hermes sign-ins and gateways kept apart live | live, two lab accounts: AJ-E1612 § Hermes Agent ("Two separate homes", "Its gateway is its own") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| R2 "profiles": the user's own `~/.hermes` and each `~/.hermes/profiles/<name>` found; used where it is, or copied without `auth.json`, `.env`, `shared/` and `state.db` | `hermes_adapter::the_users_homes_and_profiles_are_found_and_a_copy_leaves_sign_ins_and_sessions`; kit `hermes_agent::copies_no_credentials` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 usage from `hermes usage --json` (its documented, stable document): run under the account's isolation and guard, bounded (20 s, its group ended), only while signed in; `Session`/`Current session` → 5-hour, `Weekly`/`Current week` → week, as of `fetched_at`, labelled Hermes' report (`LIMIT_WINDOWS_REPORTED`); a model's own week, a null percent or an unknown label left out; `unavailable_reason` and exit 1 are errors (a Nous Portal account says Hermes has no usage for it) | `--lib hermes_usage::tests` (3); `hermes_adapter::usage_is_hermes_own_report_only_while_signed_in` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 real `hermes usage --json` output and numbers | live, lab account B: AJ-E1612 § Hermes Agent ("Usage") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| R2 its gateway managed: one AI Juggler starts runs `hermes gateway run --quiet --external-supervisor` with the account's environment in a process group of its own, is the process its `gateway.pid` names, is refused a second time while one serves the home, and ends with everything it forked; one Hermes started for a client is ended by Hermes' own `hermes gateway stop`; a record is read only as `{"kind":"hermes-gateway"}` or a bare pid, never pid 0 or 1 | `hermes_adapter::{the_gateway_ai_juggler_starts_is_its_own_and_ends_with_everything_it_forked, a_gateway_record_is_read_only_in_the_shapes_hermes_writes}`; `--lib common::child::tests` (3, a worker that ignores SIGTERM ended with its group); `juggler-platform --lib process::tests` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 the app starting and stopping an owned gateway with a session's life | not wired: the adapter's `start_gateway`/`stop_gateway` exist; a terminal session's client starts Hermes' own idle-exit gateway in the account's home and lock folder | — | — | PENDING (follow-up with the driven runtime) | branch head | 2026-11-13 | claude-opus |
| R2 a live gateway's life (launchd job or `gateway run`, idle exit, `gateway stop`) | live: AJ-E1612 § Hermes Agent ("Its gateway is its own", "Nothing left behind") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| R2 sessions from Hermes' own `state.db`, read-only and WAL-aware (`open_tool_store`), its format checked (schema ≥ 33 and every column read, else refused whole); root sessions not hidden, newest first; title Hermes' own, else its first prompt; token totals; found for a launch, linked `<store>#<id>`; resume `--resume <id>` (never an option); a first message `chat --query=<text>`; headless refused | `hermes_adapter::{sessions_come_from_hermes_own_store_read_only, a_store_in_another_format_is_refused_whole, the_terminal_launch_resumes_seeds_and_never_runs_headless}`; kit `hermes_agent::{finds_and_resumes_sessions, refuses_option_like_resume_ids}` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 a real session started, listed and resumed; the live schema version | live: AJ-E1612 § Hermes Agent ("A session starts and resumes") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| R3 the guardrail: Hermes never imports or edits a managed `~/.codex` or Claude home. Every run (session, resume, first message, sign-in, usage read, owned gateway; the user's own home used in place too) carries `HERMES_MANAGED_DIR` = the account's policy folder (`auth.adopt_external_logins: false`; `CODEX_HOME`/`CLAUDE_CONFIG_DIR` at empty folders), rewritten if changed; Launch settings can't set them; with an account `.env` pointing at a managed Codex account and `~/.claude`, the stand-in (which borrows, refreshes and writes `config.toml` as Hermes' source does) changes not a byte of `~/.codex`, `~/.claude` or a managed Codex account, and writes no `config.toml` there; the same stand-in without the guard does edit them; a Mac with `/etc/hermes` is refused, never overridden | `hermes_adapter::{hermes_never_reads_or_edits_a_codex_or_claude_home, the_stand_in_does_edit_them_without_the_guard, an_administrators_hermes_policy_is_never_replaced}`; `--lib hermes_guard::tests` (3) | pass | pass | PASS (on the stand-in) | branch head | 2026-11-13 | claude-opus |
| R3 the guard live: Hermes honours the policy folder (`hermes config show` lists the setting as managed; `~/.codex/auth.json`, `~/.claude/.credentials.json` and the `Claude Code-credentials` item's dates unchanged after a session) | live: AJ-E1612 § Hermes Agent ("The guard holds") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| R3 first-time setup's "Coming from another agent?" import (reads `~/.claude`/`~/.codex` by `HOME`) never reached | AI Juggler runs neither `hermes setup` nor `hermes import-agent`; whether a first run reaches it: AJ-E1612 § Hermes Agent ("Does first run offer an import?") | — | — | PENDING | branch head | 2026-11-13 | claude-opus |
| Handover note: the Nous Portal allows one account per person (§5.1) | the terms line in Add account and the register; no flow offers a second Nous account for capacity | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |

## Not built

- A driven runtime over ACP or the TUI gateway (routes recorded in research
  §10.6).
- Subagents (delegated tasks are sessions with a parent, left out of the
  list).
- Hermes Desktop (`com.nousresearch.hermes`): AJ-E1610's.
- Inventory (skills, MCP servers): not read yet.
