# Evidence manifest — AJ-E1608

Task: `docs/tasks/E16-more-tools.md` AJ-E1608 (Muse Code, Meta's `muse`
CLI). Status: **IN-PROGRESS** — the adapter, its driven runtime over
`muse serve`, sessions from Muse's own index, the plan's windows and its
inventory are built and evidenced on fixtures, the MIT SDK's own golden
transcripts, and one live run of `muse serve --provider echo` (no account,
no network, no prompt to Meta). What needs a signed-in Muse (a real turn,
real usage, two subscription accounts kept apart) is UNPROVEN until
AJ-E1612's lab profile (D-107).

## Basis

- Research `docs/research/muse-code.md` (2026-10-21, corrected 2026-11-13
  below), `docs/research/capabilities/hermes-muse.md` and the 2026-11-13
  research pass (the coordinating session's).
- The installed Muse Code 1.4.13 (`1.4.13-R6996.1`): `--help` of `muse`,
  `serve`, `resume`, `login`, `auth set` and `usage`, and the binary's own
  strings (`strings -a`), read for its variables and its credential
  backends (the D-106 undocumented-route pass, below). Every run had
  `MUSE_NO_AUTO_UPDATE=1` and a throwaway `HOME`, `XDG_CONFIG_HOME` and
  `XDG_DATA_HOME`, under `sandbox-exec` with outbound network, the
  Keychain's services and writes under the real home denied. No sign-in,
  no prompt to Meta, no `auth.json` or Keychain item read; the real
  `~/.config/muse`, `~/.local/share/muse` and `~/Library/Application
  Support/muse` untouched (their times checked after).
- Meta's MIT SDK `meta-models/muse-code-sdk` at `07e1548c` (2026-11-10):
  `schema/msp/msp.d.ts`, `schema/msp/experimental/msp.schema.json`, and
  seven golden transcripts vendored with the license note
  (`crates/juggler-adapters/tests/muse_fixture/transcripts/`).
- The Meta Model API Terms of Service (updated 2026-10-09), read 2026-11-13
  (AJ-E1518's register, § Muse Code).

## The credential backend (D-106 pass)

`TBH_CREDENTIAL_BACKEND` is read by 1.4.13 (it sits in the binary's list of
environment variables beside `XDG_CONFIG_HOME`, `XDG_DATA_HOME`,
`CODEX_HOME` and `CLAUDE_CONFIG_DIR`). Its values aren't printed anywhere,
but the session record the binary writes names the backend in effect as
`file`, `keychain` or `keychain_fallback_file` ("valid pairings: file/none,
keychain/none, keychain_fallback_file/any non-none"; "off-macOS stamps
`file`"), and its auth file reader knows `auth.json` as `empty`,
`malformed`, `ready` or `partial`, with fields for tokens, an API key and
the account's name. So `TBH_CREDENTIAL_BACKEND=file` is the likely way to
keep a sign-in in the folder's own `auth.json`. It is wired
(`MuseAdapter::with_separate_sign_ins`, `muse::SEPARATE_SIGN_INS`) and
**off**: whether Muse honours it, and with that value, is the lab's to show
(D-107). The Keychain item is a generic password, service
`ai.meta.dev.credentials`, account `meta`, both fixed in the binary.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 Meta's `muse` is the one meant (D-74): found as the installer's launcher `~/.local/bin/muse`, then on the PATH and Homebrew's; versioned from the launcher's own `.muse-version` (`1.4.13-R6996.1` → `1.4.13`), else `muse --version` with `MUSE_NO_AUTO_UPDATE=1`; never claimed verified; a running Muse told by `muse-bin-<version>`, its helpers (`serve`, `login`, `usage`, …) not, `resume <id>` naming its conversation | `cargo nextest run -p juggler-adapters --test muse_adapter` (`muse_is_found_by_its_launcher_and_its_version_record`); `--lib muse::tests` (3) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R1 registered as data (ADR-0019): the registry, Add account and first run from `muse_spec()`; the preview catalog's entry; the letter mark (`muse`, Meta, not drawn); Meta Model API ToS §3.1, §3.2, §10.1(xii) quoted with its URL and date; the runtime's terms line on the Privacy page | `cargo nextest run -p juggler-app --test trust` (`every_tool_names_its_terms_and_the_preview_matches`), `--test no_bare_tools`, `--test tool_updates`; `cd ui && pnpm vitest run` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R1 every probe and launch with `MUSE_NO_AUTO_UPDATE=1` (fixed, never overridable) | kit `muse_code::pins_env_after_overrides`; `muse_adapter::each_account_is_a_home_of_its_own_and_the_users_own_runs_as_muse_does` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 each account a home of its own: `HOME=<account>`, `XDG_CONFIG_HOME=<account>/.config`, `XDG_DATA_HOME=<account>/.local/share`, `CODEX_HOME` and `CLAUDE_CONFIG_DIR` at empty folders inside it (none of the user's Claude or Codex skills load), all after Launch-settings overrides; the launcher's `MUSE_AUTH_PATH`, `MUSE_PROJECTS_HOME` and `TBH_CREDENTIAL_BACKEND` removed; the user's own `~/.config/muse` runs with the XDG variables unset and `HOME`, `CODEX_HOME`, `CLAUDE_CONFIG_DIR` as they are | `muse_adapter::each_account_is_a_home_of_its_own_and_the_users_own_runs_as_muse_does`; kit `muse_code::{isolates_each_home, pins_env_after_overrides, runs_both_runtimes_honestly}` (15 of 15) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 Muse keeps its sessions where the account says: `initialize`'s `museHome` is `<XDG_DATA_HOME>/muse` (checked live in a throwaway home), and a driven session whose `museHome` isn't its account's data folder is refused | live `muse serve --provider echo` (below); `muse_serve::session::connect` (the same check, `live_echo_session`) | match | match | PASS | branch head | 2026-11-13 | claude-opus |
| R2 API-key accounts first: an account other than the user's own runs on `META_API_KEY` from its own Launch settings, set for its launches only; an inherited `META_API_KEY` is removed from every launch | `muse_adapter::{each_account_is_a_home_of_its_own_and_the_users_own_runs_as_muse_does, the_terminal_takes_a_first_message_a_model_and_a_resume}`; kit (its `launch_env`) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 the subscription sign-in is one per Mac (the Keychain item, fixed name): only the user's own Muse holds one; any other account without a key is refused at launch and at sign-in, saying why (`muse::ONE_SIGN_IN`), and reports its sign-in as not knowable from its folder; the probe reads presence only (`auth.json`'s, then the Keychain item's by service and account through `juggler_platform::keychain::item_exists`, never `-w`/`-g`; only for the real user's own folder) | `muse_adapter::a_second_subscription_account_is_refused_saying_why`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 separate subscription sign-ins (`TBH_CREDENTIAL_BACKEND=file` per folder, the user's own on the Keychain) wired behind a flag that stays off | `muse_adapter::with_separate_sign_ins_on_each_folder_keeps_its_own_in_a_file` | pass | pass | PASS (the wiring); the flag stays off | branch head | 2026-11-13 | claude-opus |
| R2 two subscription accounts kept apart on one Mac (whether the file backend separates them) | live, two lab profiles signed in: AJ-E1612 (D-107) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R2 terminal: a first message (`-- <message>`, never read as an option), the Launch settings' model (`--model`), resume `muse resume <id>` (the id never an option); a headless run refused with the reason; the user's own setup used where it is, no copy offered | `muse_adapter::{the_terminal_takes_a_first_message_a_model_and_a_resume, only_the_users_own_setup_is_used_where_it_is_and_none_is_copied}`; kit `muse_code::{refuses_option_like_resume_ids, copies_no_credentials}` | pass | pass | PASS (`--` before the message: Muse's own parser, UNPROVEN until AJ-E1612) | branch head | 2026-11-13 | claude-opus |
| R2 reasoning effort from Launch settings | AI Juggler has no Launch-settings field for an effort, for any tool; each account keeps Muse's own `reasoning_effort` setting | — | — | UNPROVEN (not built: no field to read) | branch head | 2026-11-13 | claude-opus |
| R3 driven over `muse serve` (MSP v1, stdio NDJSON): `initialize` (stable API), `session/start` (with the model and per-session `mcpServers`) or `session/resume`, which subscribe the connection; `turn/start`, `turn/steer`, `turn/interrupt`, `session/setModel`, `model/list`; events (turn started and completed, text and output deltas, tool items, a whole agent message) into the feed, another session's left out; a failed turn says why (a sign-in failure, a quota stop) | `cargo nextest run -p juggler-adapters --test muse_serve`: the SDK's transcripts `session-start`, `text-run-single-turn`, `cancel-mid-turn`, `provider-failure-turn` replayed, every client frame made by the codec's own call and matched (method, session, `requirementId`, choice, answers; a UUIDv7 `commandId`) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 approvals and questions are the user's asks: `approval/request` acknowledged at once and shown once with Muse's own choices (allow once, a rule with its preview, deny, deny and stop), decided by `approval/decide` with the choice and requirement Muse named; `userInput/request` answered with an offered label or the user's own words; settled ones closed | `muse_serve::{an_approval_is_the_users_ask_with_muses_own_choices, a_denied_approval_ends_the_turn_as_muse_says, a_question_on_resume_is_answered_with_an_offered_label}` (transcripts `approval-round-trip`, `approval-deny-round-trip`, `userinput-answer-round-trip`); kit `muse_code::runs_both_runtimes_honestly` (no bypass flag) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 a real `muse serve` driven end to end: `muse serve --provider echo` 1.4.13 in a throwaway home (sandboxed: no network, no Keychain), through the app's own `connect`: `museHome` checked, a session started, a turn sent and its text streamed, `usage/read` answered with no windows (no account), the session then listed from Muse's own index by the adapter's reader; nothing left running | `AJ_MUSE_LIVE_BIN=~/.local/bin/muse AJ_MUSE_LIVE_HOME=<throwaway> AJ_MUSE_LIVE_SANDBOX=<profile> cargo nextest run -p juggler-adapters --test muse_serve --run-ignored only` (`live_echo_session`); `pgrep -fl muse-bin` empty after | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 a real turn against Meta, an approval from a real tool call | live, lab profile: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R3 sessions from Muse's own index `$XDG_DATA_HOME/muse/session-index.db`, read-only and WAL-aware (`open_tool_store`), its format checked first (schema version 1 and every column read; another version refused whole); root sessions only; found for a launch, linked `<index>#<id>`; their folders become projects. The fixture is the `sessions` table 1.4.13 itself created | `muse_adapter::sessions_come_from_muses_index_and_an_unknown_format_is_refused`; kit `muse_code::finds_and_resumes_sessions`; `live_echo_session` (a real index read back) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 `session/list` and `session/read` over a running host | the codec names both methods; the index is read for every account instead (a host is started only for a driven session, and the app's index refresh doesn't start one) | — | — | UNPROVEN (not wired: the index is the source) | branch head | 2026-11-13 | claude-opus |
| R3 usage: the plan's 5-hour and weekly windows from Muse's own report — `muse serve` started briefly, `initialize` with the experimental API, `usage/refresh` at most once a minute per account (else `usage/read`, no network), stopped before it returns; `muse usage --json` as the fallback; each window Measured as of `observedAtMs`, over 100 kept, a window of another length not guessed, nothing seen is no reading, no subscription an error; the plan's printed `tierName` kept as the account's plan when it names none; only for the user's own signed-in Muse; on the usage cadence or "Measure now" only | `--lib muse_usage::tests` (3); `muse_serve::the_plans_windows_come_from_usage_read_and_its_changes`; `muse_adapter::a_second_subscription_account_is_refused_saying_why` (offered only for the user's own) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 real usage numbers, `usage/refresh`'s answer and `muse usage --json`'s shape | live, lab profile: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| Inventory: MCP servers from the account's `settings.json` `mcp_servers` (Muse's `transport` read as the shared reader's type; no secret kept) and skills from its `skills/`; a skill set up by copying it in, never over one there; a server or rules file is the user's to add | `--lib muse_inventory::tests` (2); `muse_adapter::its_inventory_is_the_accounts_own_config` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |

## Live `muse serve --provider echo` (2026-11-13)

Run twice in throwaway homes in a scratch folder, under `sandbox-exec`
(outbound network, the Keychain's services and writes under the real home
denied), `MUSE_NO_AUTO_UPDATE=1`, `TBH_DISABLE_TELEMETRY=1`: once by hand
(`initialize` with the experimental API: `museHome`
`<home>/.local/share/muse`, `sandboxAvailability` `unavailable`;
`account/read` `{"state":"loggedOut","credentialRequired":true}`;
`usage/read` `{}`; `session/start` refused a v4 `commandId` — "expected
UUIDv7" — and took a v7 one; a turn "hello echo" answered "echo: hello
echo"; `session/list` and `session/read` listed it), and once by the test
`live_echo_session`. The index's schema was read from the throwaway index
(`schema_meta.schema_version` `1`). `pgrep -fl muse-bin` was empty after
each run.

## Research corrections

Recorded in `docs/research/muse-code.md` § Corrections (2026-11-13): the
version (1.4.13, not 1.4.6 or 1.4.11), the Keychain item's fixed service and
account and what it means for several accounts, `museHome` following
`XDG_DATA_HOME`, the index's schema, `session/start` subscribing the
connection by itself, and UUIDv7 command ids.
