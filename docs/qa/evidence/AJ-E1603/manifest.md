# Evidence manifest — AJ-E1603

Task: `docs/tasks/E16-more-tools.md` AJ-E1603 (Kimi Code CLI). Status:
**IN-PROGRESS** — R1, R2 and R4 evidenced on fixtures and stub binaries
(2026-10-24); R3 (usage, D-106) built and evidenced on fixtures and a
stand-in `kimi web` (2026-11-13); every live part waits for AJ-E1612 (a
lab profile on the maintainer's account, D-107). Reviewer claude-opus.

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
| R1 discovery of `kimi` by path and version: the installer's `~/.kimi-code/bin/kimi` first, then PATH, Homebrew, npm; 2.x is Kimi Code, never claimed verified; the archived Python Kimi CLI (0.x, 1.x, same command name) is told apart and not offered | `cargo test -p juggler-adapters --test kimi_code_adapter` (`versions_parse_and_the_old_python_cli_is_told_apart`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| R1 isolation: `KIMI_CODE_HOME` per account, pinned after Launch-settings overrides; the default `~/.kimi-code` runs with it unset; `KIMI_CODE_IDENTITY_*` never set | `adapter_conformance` (`kimi_code::*`); `kimi_code_adapter::each_account_has_its_own_home_and_the_default_runs_as_a_plain_kimi`; `kimi_provider::a_kimi_account_has_its_own_home_and_its_limits_wait_for_kimi_web` (the isolation checklist and the shell line) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| R1 existing homes found: `~/.kimi-code`, `$KIMI_CODE_HOME`, `~/.kimi-code*`, never the old CLI's `~/.kimi` | `kimi_code_adapter::the_scan_finds_the_default_an_override_and_other_homes_but_not_the_old_cli` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| R2 sessions from the documented layout and `session_index.jsonl`: listed (title, else the last prompt; folder; times, else the file's), found for a launch and resumed `--session <id>`; a running `kimi` recognised, helpers (`web`, `login`, `acp`…) not; tokens unknown (never zero) | `kimi_sessions` unit tests; `kimi_code_adapter::sessions_are_listed_resumed_and_told_from_helpers`; kit `finds_and_resumes_sessions`; `kimi_provider::its_sessions_are_listed_and_their_folders_become_projects` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| R3 limit hits, keyless: a `turn.ended` `reason: "failed"` with Kimi's quota error (`APIProviderQuotaExhaustedError`, or a 429 `provider.api_error`/`provider.rate_limit` with a limit message) is a hit at the record's time; its window (5-hour, weekly, monthly) from the vendor's fixed message openings, matched as parsed and dropped; any other message no window (never guessed), a busy 429 or a localized plain 429 no hit; a later completed turn clears it; protocol 1.5 only, a tail without its version line says nothing; a subagent's log counts for its session; `LIMIT_HITS` declared | `cargo test -p juggler-adapters --lib kimi_limits` (4: `the_quota_error_names_its_window_by_the_vendors_words`, `a_plain_429_counts_only_with_a_limit_message`, `the_quota_error_with_other_words_is_a_hit_on_no_named_window`, `a_line_of_another_shape_is_no_hit`); `kimi_sessions::wire::tests::{a_turn_failed_at_the_limit_names_its_window_until_a_turn_completes, other_failures_versions_and_cut_lines_are_no_hit, a_sessions_hit_is_read_from_every_agents_log_and_listed_with_its_file}`; kit check 12 `adapter_conformance` `kimi_code::reads_limit_hits_honestly` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 attach: a live `<home>/server/instances/*.json` (pid alive, heartbeat within 60 s, host loopback or every interface, 2.x or no version), the oldest first; a dead, stale, LAN, `::1`, 3.x, unbound or malformed record skipped; healthz first with no credential, then `GET /api/v1/oauth/usage` with `Bearer <trimmed server.token>`, `Host: 127.0.0.1:<port>`, no Origin or cookie; an empty or missing token is unknown | `kimi_usage::tests::{the_live_instance_is_the_oldest_live_reachable_kimi_code_2, the_token_is_trimmed_and_an_empty_or_missing_one_is_none, attaching_sends_the_homes_token_only_to_its_server_and_keeps_it_nowhere, an_error_while_attached_never_carries_the_token}`; `local_http::tests` (2) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 token canary: the home's `server.token` reaches only the instance's port on 127.0.0.1; never in a result, an error, a log or a copy (a copy already leaves it); read in one function, held as a zeroed-on-drop value; the adapters' one socket is the loopback client | `kimi_usage::tests::attaching_sends_the_homes_token_only_to_its_server_and_keeps_it_nowhere`, `an_error_while_attached_never_carries_the_token`; `trust::{the_kimi_server_token_is_read_in_one_place_for_one_request, the_loopback_client_reaches_only_127_0_0_1, only_the_listed_crates_can_reach_the_network}`; `quota_ingest::no_http_client_anywhere_in_the_adapters_crate`; kit `copies_no_credentials` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 format check: `code` 0, `data.kind` ok or error, `usages` keys only the four (a renamed one, e.g. `limit_5h`, makes the reading unknown), `usedRatio` a finite number in 0–1.05 (shown at most 100%), `resetAt` RFC 3339, `extraUsage` absent, null or its six typed fields; other keys tolerated; `limit5h`/`limit7d`/`monthTotal` → 5-hour/weekly/monthly Measured readings, `monthCode` a part of the month, a window left out absent; 401/403/other status and Kimi's own error (with or without its status) say so without their text | `kimi_usage::tests::{every_window_maps_with_its_ratio_and_reset_and_the_code_share_is_a_part, a_window_left_out_isnt_there, an_answer_that_fails_the_format_check_is_unknown_never_zero, refusals_and_kimis_own_errors_say_so_without_their_text}` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 self-started: with no live instance, `kimi web --no-open --port 0` through the helper seam (a cleared environment: the allowlist, `PATH`, `KIMI_CODE_HOME`, `KIMI_CODE_PASSWORD` only), output discarded unread (the stand-in's banner token is never used), its port from the record with its pid, healthz then usage with `Bearer <per-run secret>`; stopped at once (SIGTERM, then KILL past the grace), reaped, the ledger back to 0; one that never registers reaped at the deadline; one that crashes started once, never again; at most one across every account; a bad answer still stops it; the secret never on AI Juggler's own process | `cargo test -p juggler-adapters --test kimi_usage` (6: `a_started_server_answers_with_the_per_run_secret_and_is_stopped_at_once`, `a_server_that_never_registers_is_reaped_at_the_deadline`, `one_that_ignores_sigterm_is_killed_and_one_that_crashes_is_never_restarted`, `at_most_one_runs_across_every_account`, `an_answer_that_fails_the_format_check_is_unknown_and_the_server_still_stops`, `only_a_membership_account_is_asked_through_the_adapter`) against `fake-kimi`; `pgrep -fl kimi` empty after | 6 pass, no process left | 6 pass, none left | PASS | branch head | 2026-11-13 | claude-opus |
| R3 only membership accounts: an account on a Kimi Platform API key, or with no sign-in, gets no usage call; not an agent run (no session, no model turn), so R4's interactive-only rule doesn't apply to it | `kimi_usage::only_a_membership_account_is_asked_through_the_adapter`; `kimi_provider::an_api_key_account_is_never_asked` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 in the app, labelled experimental: on the usage cadence (D-39's ask: every 15 min while in use) and "Measure now", the windows named are kept as Measured readings from the tool's local server; the 5-hour window shows with "Kimi Code's own local server (experimental)"; a week left out reads "reports no weekly limit"; the month kept as its own window; before any reading the limits say they show after use (no longer "doesn't report") | `kimi_provider::{measure_now_reads_the_windows_from_kimi_web_labelled_experimental, a_kimi_account_has_its_own_home_and_its_limits_wait_for_kimi_web}` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 Privacy page: the exception by name in README § privacy, `docs/guide/trust.md`, SPEC §8 and the in-app page (`trust-claims.json`: `kimi-usage`, and the `no-credentials` and `no-ai-provider` lines naming it) | `trust::every_trust_claim_names_tests_that_exist`; `ui/src/screens/privacy-page.test.tsx` | pass | pass (Rust); UI test not run here | PASS | branch head | 2026-11-13 | claude-opus |
| R3 shown on screen: the monthly window and the month's code share; an open Kimi session marked at its limit in the app (the adapter reads the hit, `ToolQuota::session_limit_hit`; the app's marking runs only for a supervised session) | — (no UI for a monthly window yet; no app-level test drives a supervised Kimi session) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R3 live: a real `kimi web` (attach and self-started, `--port 0`, the instance record's shape and the pid it records), a real quota message in `wire.jsonl`, Moonshot's real answer | AJ-E1612 (a lab profile on the maintainer's account, D-107) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R4 interactive use only on a subscription: a membership account (or one whose sign-in can't be told) refuses headless runs, and a crash is never restarted by itself (its history says why); an account on a Kimi Platform API key may run headless | `kimi_code_adapter::launches_per_mode_and_headless_only_on_an_api_key`; `juggler-app` lib `restart::tests::an_interactive_only_account_is_never_restarted`; `kimi_provider::a_membership_account_that_crashes_is_never_started_again_by_itself` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Sign-in: Kimi Code's own `kimi login` in the account's home; the probe reads which files are in `credentials/` (in `sign_in.rs`), never one's contents | `kimi_code_adapter::{sign_in_runs_kimi_login_in_the_accounts_home, the_sign_in_probe_reads_which_files_are_there_never_what_they_hold}`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Bring-in (ADR-0018): any home in place, unwritten; a copy keeps settings, skills, `mcp.json` and `AGENTS.md` but never `credentials/`, `server.token`, a settings file with a key written in it, or the sessions (their list may name the original's folders) | `kimi_code_adapter::{a_copy_keeps_settings_but_never_a_sign_in_a_key_or_the_sessions, a_home_is_used_where_it_is_unwritten_and_its_setup_read_from_it}`; kit `copies_no_credentials` (a canary in the membership file, an MCP sign-in and `config.toml`); `kimi_provider::the_default_home_is_used_where_it_is_and_never_written` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Rules, skills and MCP join the sync: `AGENTS.md`, `skills/<name>/SKILL.md` and `mcp.json` (`mcpServers`, read as Claude's) listed; a server from another tool added beside the rest and undone only while unchanged; a secret never written | `kimi_inventory` unit tests | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| ADR-0019: Kimi Code is data (spec, terms in AJ-E1518's register, the letter mark); no tool-id literal outside the adapter; it shows in Add account, first run and its page | `kimi_code_adapter::the_spec_describes_kimi_code_as_data`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `python3 scripts/tool_literal_lint.py`; `ui/tests/visual/more-tools-gallery.spec.ts` (run locally, 8 of 8; captures checked by eye, not committed) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| It passes the kit (AJ-E1521) | `adapter_conformance` `kimi_code::*` — 6 of 6 (a tool may now refuse a headless run or a first message, saying so) | 6 pass | 6 pass | PASS | branch head | 2026-10-24 | claude-opus |
| Live: sign-ins separate across two homes, devices per home, a session starts and resumes, the interactive-only guardrail, an API key's headless run | the maintainer's checklist, `docs/internal/MAINTAINER-STEPS.md` step 16 (Kimi Code) | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |

## For the maintainer

- **R3 (D-106, built 2026-11-13).** No keyless source of the windows
  exists (research `kimi.md` §12); the limit hits are keyless. The windows
  come from `kimi web`: the user's own (its `server.token`, one request),
  else one started briefly with a per-run secret. The Privacy page names
  the exception; the website's privacy page (another repository) still
  needs the same line.
- **The device quota per home** is unknown (each home mints its own
  device id); the live check counts it.

## Not claimed (on the live checklist)

- R3: whether the installed 2.x records the same instance fields and pid
  for `--port 0` (read in the 2.6.1 source; 2.5.3 is installed here); the
  exact English of the quota messages as the server sends them; whether
  a membership account's `kimi web` needs anything beyond its sign-in to
  answer.

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
