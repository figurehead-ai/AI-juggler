# Evidence manifest — AJ-E1605

Task: `docs/tasks/E16-more-tools.md` AJ-E1605 (ZCode). Status:
**IN-PROGRESS** — round 1 (R1, R2, R3, R7) and round 2 (R4, R5, R6's
ZCode-to-pack half) evidenced on fixtures and stand-in programs; round 3
(2026-11-13) moved ZCode to a driven runtime over its engine's own
app-server, on the app's own runtime, evidenced on fixtures, a stand-in
app-server and one live run of ZCode's own import (no sign-in, no prompt).
Sending a turn is UNPROVEN until AJ-E1612's lab profile (D-107).

**Correction, 2026-11-13 (claude-opus, checked live in a throwaway home, no
sign-in, no prompt):** the interactive launch rounds 1 and 2 built (`node
<engine>` with no command, Interactive and Resume) **cannot run with the real
engine**. First it fails "cannot locate the built-in provider config" (it
needs `ZCODE_BUILTIN_PROVIDER_CONFIG_FILE`); with that set, under `node` and
under ZCode's own Electron (`ELECTRON_RUN_AS_NODE=1`), it fails "Cannot find
package '@zcode/tui'": ZCode.app doesn't ship the terminal UI (only the
standalone build has it, and that isn't distributed). ZCode's desktop runs the
engine only as `app-server --stdio`. So every row below that rests on a
terminal launch (R2's launch and launch check, R3's resume, the stand-in
launches) proves the adapter's logic on fixtures, not a working ZCode
session. Round 3 moves ZCode to a driven runtime over its own app-server
(research § E), and the rows below are rewritten for it. The live run of a
turn is AJ-E1612's, in a separate lab profile (D-107).

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

Round 2 (the D-106 undocumented-route pass first, research § D): the
engine reads its user settings, rules and skills from the home folder for
every App (`os.homedir()`/`HOME` in its own code), so servers, skills and
rules are one shared set, listed and set up in `~/.zcode` and said to be for
every App; it has no option or variable for a first message or a model,
and keeps no record of the plan's pools (the billing routes are only in its
endpoint table; nothing calls them). Each finding is sourced in § D.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 discovery: `ZCode.app` in `/Applications` or `~/Applications`, taken as ZCode's when it holds ZCode's engine where ZCode keeps it (the bundle id isn't read); the engine `Contents/Resources/glm/zcode.cjs` runs on the app's own runtime `Contents/MacOS/ZCode` with `ELECTRON_RUN_AS_NODE=1` (no user Node.js looked for; round 3); the version from `version` under that runtime; a missing runtime or provider settings reported; never claimed verified; `AI_JUGGLER_ZCODE_ENGINE` overrides the engine (the runtime and settings then of the bundle it is in) | `cargo test -p juggler-adapters --test zcode_adapter` (`zcode_is_found_by_its_app_bundle_and_versioned_by_its_engine`, macOS only); `juggler-platform` `apps` (`executable_of`, `bundle_of`) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 isolation without moving `HOME`: each App's engine runs with `ZCODE_STORAGE_DIR=<account>/.zcode`, `ZCODE_DATA_BASE_DIR=<account>`, `ZCODE_SESSION_DB_PATH=<account>/.zcode/cli/db/db.sqlite` and `ZCODE_BUILTIN_PROVIDER_CONFIG_FILE=<app>/Contents/Resources/config/provider/zcode-builtin.json` (round 3), plus `ELECTRON_RUN_AS_NODE=1`, all after Launch-settings overrides; `ZCODE_SESSION_DB` and an inherited `ZCODE_PERSONAL_PROVIDER_CONFIG_FILE` removed; `ZCODE_APP_VERSION` not set (it only labels telemetry); the default `~/.zcode` runs with the tree variables unset; no copyable terminal line (no terminal UI) | `zcode_adapter::{each_app_has_its_own_tree_and_zcodes_own_runs_with_the_variables_unset, sessions_run_driven_on_the_apps_own_runtime_and_no_terminal_is_offered}`; kit `zcode::{isolates_each_home, pins_env_after_overrides}` (on the driven launch: the kit's `launch` helper); `zcode_provider::zcode_is_listed_isolated_and_offers_no_terminal_line` (the isolation checklist checks the driven launch) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 sign-in: ZCode's own `login` in the App's tree, on the app's runtime with the provider settings; the probe sees whether `<tree>/v2/credentials.json` is there and not empty, never its contents | `zcode_adapter::the_sign_in_probe_sees_the_file_never_its_contents_and_login_runs_in_the_tree`; `no_bare_tools::no_sign_in_file_is_read` | pass | pass | PASS (the login itself not run: AJ-E1612) | branch head | 2026-11-13 | claude-opus |
| R2 launch check: after a run the engine's store is in the App's tree, else the App is refused with a reason (`zcode::isolation_took`; a function, not yet called by the app) | `zcode_adapter::the_launch_check_refuses_an_app_whose_tree_the_engine_did_not_write`; live: the import run wrote its store in the App's tree and nothing under the real `~/.zcode` (below) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 the variables honoured by a real engine run (sessions, sign-in and usage land in the App's tree; the shared user config's effect) | live, on a spare account: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R3 sessions from `<tree>/cli/db/db.sqlite`, read-only (a closed WAL store opened `immutable`, left byte for byte): interactive, not archived, not a sub-agent's; title, folder, times; found for a launch; a running engine recognised by its command line, helpers (`app-server`, `doctor`, `version`, `login`, …) not; their folders become projects. Resumed driven (`session/resume <sess_…>`, the id never an option); "Continue in AI Juggler" on a ZCode conversation starts it driven | `zcode_adapter::{sessions_come_from_the_store_read_only_and_byte_for_byte_unchanged, sessions_run_driven_on_the_apps_own_runtime_and_no_terminal_is_offered, a_running_engine_is_a_session_and_its_helpers_are_not}`; kit `zcode::{finds_and_resumes_sessions, refuses_option_like_resume_ids}`; `zcode_provider::its_sessions_become_projects_and_the_store_is_left_unchanged` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R3 against a store a real engine wrote: the live import's session read back by the adapter's own reader, its two turns exact | `AI_JUGGLER_ZCODE_LIVE_HOME=<empty folder> cargo test -p juggler-adapters --test zcode_app_server live_import -- --ignored --nocapture` (run 2026-11-13, see "Live import run") | 2 turns | 2 turns | PASS | branch head | 2026-11-13 | claude-opus |
| R2 driven runtime (round 3): one app-server per session over stdio, started as ZCode's desktop starts it (`<app runtime> <engine> app-server --stdio`, `ELECTRON_RUN_AS_NODE=1`, piped, never a PTY); `session/create` or `session/resume`, `session/subscribe` (`desktop-continuous`), `session/send`, `session/stop`; events (turn started/completed/failed, text and output deltas, tool items) into the feed, a sub-agent's left out; the default runtime, Stable; every terminal launch refused with the reason, no terminal view; the app starts a tool without a terminal UI driven or refuses in words (never a terminal that can't run; a first message refused rather than lost) | `cargo test -p juggler-adapters --test zcode_app_server` (3: a stand-in app-server speaking the engine's schemas); `zcode_adapter::sessions_run_driven_on_the_apps_own_runtime_and_no_terminal_is_offered`; kit `zcode::runs_both_runtimes_honestly`; `juggler-app` builds (`planned_terminal`, `start_driven_only`) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 asks are the user's: `interaction/requestPermission` → an approval with the engine's own options (its effect read from each option's own decision), answered only with that option's own decision; `interaction/requestUserInput` → a question, answered `{action: accept, content: {answers}}`; a re-announced ask asked once; anything else the engine asks refused | `zcode_app_server::a_session_opens_subscribes_and_runs_a_turn_with_the_users_answers` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 the provider-headers request answered `{headersApplied: true, requestAuth: {}}`, never a credential (ADR-0003); a sign-in failure shown as "ZCode couldn't use this App's sign-in" | `zcode_app_server::{a_session_opens_subscribes_and_runs_a_turn_with_the_users_answers, a_failed_turn_says_a_limit_or_a_sign_in_plainly}` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R2 a turn sent live: the engine uses the App's own sign-in after `requestAuth: {}` (a completed turn and a `model_usage` row in the App's tree), or fails plainly | live, lab profile: AJ-E1612 (D-107) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R4 usage Computed per session and per App: each completed `model_usage` row × the published multipliers (GLM-5.3 6.9/1.7/24, GLM-5.3-Flash 2.3/0.56/8), half off-peak; each event names its session (`sess_…`); the conversations list carries each session's own tokens (none when it has no row, never zero); an unknown model carries no credits and is Estimated | `zcode_adapter::usage_is_read_from_the_apps_own_store`; `quota_ingest::zcode_events_compute_credits_with_peak_and_offpeak`; kit `zcode::labels_usage_honestly` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R4 the 5-hour and weekly pools are the plan's caps the user enters, never claimed read: ZCode declares `LIMITS_FROM_PLAN_CAPS`, so the provider list shows its limit rows and the Usage screen its two windows (not "doesn't report its limits", the round-1 regression), never a Measured badge; no local source of the pools exists (step 0, research § D) | `zcode_provider::its_limits_are_the_plans_caps_and_never_claimed_read`; kit `zcode::reads_limit_hits_honestly` (no limit hits claimed) | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R4 limit hits (keyless): a session whose last model call Z.ai refused (`status = 'error'`, `error_type = 'rate_limited'`, message exactly `[1308][Usage limit reached for 5 hour. …reset at YYYY-MM-DD HH:MM:SS][<hex>]` → 5-hour, `[1310][Weekly/Monthly Limit Exhausted. …][<hex>]` → week) holds a hit with its reset; a later call clears it; anything else refused; `LIMIT_HITS` declared, read per session (`session_limit_hit`); a driven turn failing so is a rate-limit error. The message's shape measured on a real store by counts only (236 of 236 rows) | `zcode_limits` unit tests (2); kit `zcode::reads_limit_hits_honestly` (upgraded: `limit_records`); `zcode_app_server::a_failed_turn_says_a_limit_or_a_sign_in_plainly` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R4 the reset's time zone (read as UTC+8: every measured 5-hour reset 0–4.5 h after its hit) and 1310's window (read as the week; Z.ai's words say "Weekly/Monthly") | live: AJ-E1612 sees a hit | — | — | UNCONFIRMED | branch head | 2026-11-13 | claude-opus |
| R4 credits against a store a real engine wrote; the multipliers still the published ones | live: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R5 rules, skills and MCP join the sync: `~/.zcode/cli/config.json` `mcp.servers` (the engine's own spellings read as it reads them; disabled ones left out), `~/.zcode/skills/<name>/SKILL.md`, `~/.zcode/AGENTS.md`; one set for every App (the engine reads them from the home folder), and every verdict says so; a server from another tool added beside the rest and undone only while unchanged; a secret never written (left out of `env`, the user told to provide it; a header's: not set up) | `zcode_inventory` unit tests (3); `zcode_adapter::every_app_shares_the_rules_skills_and_servers_in_the_home_folder` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R5 bring-in (ADR-0018): ZCode's own `~/.zcode` used where it is, unwritten (any other tree refused in place, with why); a copy into the App's tree takes its settings and plugins, never `v2/credentials.json`, the database (`cli/db/`, any `*.sqlite`/`*.db`), logs, model-call records (`cli/rollout/`, `cli/debug/`) or each session's files (`cli/exec/`, …), nor a settings file with a key in it; what every App reads from the home folder isn't copied; the original byte for byte unchanged; the scan finds `~/.zcode`, `$ZCODE_STORAGE_DIR` and `~/.zcode-*` | `zcode_adapter::a_tree_is_used_in_place_or_copied_without_its_sign_in_store_or_logs`; kit `zcode::copies_no_credentials` (now with a seeded tree and a canary sign-in); `sign_in::tests::zcodes_sign_in_path_is_the_probed_file`; `no_bare_tools` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R5 a server set up in `~/.zcode` reaching a managed App's engine; a copied App signing in on its own | live: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R6 what a ZCode session gives a pack: its user and assistant text from the store, in order, read-only (tool calls, reasoning and synthetic text left out; a record in another shape refused whole), through a new adapter seam (`ToolTranscripts::conversation_turns`) that the app's pack staging asks before reading a transcript file | `zcode_adapter::a_sessions_turns_come_from_the_store_for_a_handover`; `juggler-app` lib `features::switch::back::tests::a_packs_turns_come_from_the_tools_own_store_or_its_transcript_file` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R6 a pack into ZCode: started driven with the handover as its first message through ZCode's own import (`session/create` with `importedHistory`, `persistence: immediate`, `titleGenerationEnabled: false`, id `sess_<uuid>`; no model call), from the app's pack start for any tool whose driven runtime `imports_history` | `zcode_app_server::a_pack_comes_in_by_zcodes_own_import`; `zcode_adapter::sessions_run_driven_on_the_apps_own_runtime_and_no_terminal_is_offered`; live: `zcode_app_server::live_import` (below) | import written, no model call | session + 2 messages + 2 text parts, 0 `model_usage` rows | PASS | branch head | 2026-11-13 | claude-opus |
| R6 switching from ZCode (a replay into Claude Code or Codex) and every pair unvalidated until the live check | live: AJ-E1612 | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |
| R7 ADR-0019: ZCode is data (its spec; its terms in AJ-E1518's register, read 2026-11-13; its letter mark), registered with no new tool-id literal outside its adapter (the usage pipeline's own ZCode branch removed); shown in Add account, first run and its page from its spec alone | `zcode_adapter::the_spec_describes_zcode_as_data`; `trust::every_tool_names_its_terms_and_the_preview_matches`; `just tool-literals`; `zcode_provider::zcode_is_listed_isolated_and_its_shell_line_pins_its_tree` | pass | pass | PASS | branch head | 2026-11-13 | claude-opus |
| R7 it passes the kit (AJ-E1521), now with driven-only tools (a refused terminal launch checked on the driven launch, no terminal view) and store-kept limit hits (`limit_records`) | `cargo test -p juggler-adapters --test adapter_conformance` — 120 of 120 (ZCode 15 of 15) | 120 pass | 120 pass | PASS | branch head | 2026-11-13 | claude-opus |
| R7 the screens with ZCode in them, looked at | not captured this round (the preview catalog carries ZCode; no visual baseline was regenerated) | — | — | UNPROVEN | branch head | 2026-11-13 | claude-opus |

## Live import run (2026-11-13, claude-opus)

`AI_JUGGLER_ZCODE_LIVE_HOME=<scratch>/zc-r3 cargo test -p juggler-adapters
--test zcode_app_server live_import -- --ignored --nocapture`, in an empty
throwaway folder used as `HOME` and the App's folder (`<scratch>/zc-r3/acc`),
no sign-in, no prompt (no `session/send`), the installed engine 0.16.9 on
ZCode 3.15.3's own runtime, started by the adapter's own `driven_spec` and
spoken to by its own `ZCodeSession`. Output:

```
server ended by itself: true
session: title "Live import check", directory is the work folder: true
turns read back by the adapter: 2
user turns marked as ZCode's import: 1; assistant turns at no cost: 1; text parts: 2
model calls recorded for it: 0
test live_import ... ok
```

The engine wrote its store and its provider-settings copy
(`v2/runtime/provider/darwin-aarch64/0.0.0-dev/…`) inside the App's tree;
nothing under the real `~/.zcode` changed (`find ~/.zcode -newer <scratch>`
empty); no engine process was left (`pgrep -fl zcode.cjs` empty); the
throwaway folder was deleted afterwards.

## Not offered yet (each says so in words)

- A terminal: ZCode.app ships no terminal UI (round 3, checked live), so
  every terminal launch, the copyable line and the terminal view refuse
  with that reason; sessions run driven.
- Headless runs (`-p`, which defaults to `--mode yolo`).
- A first message typed at the start (a driven session starts empty; a
  pack comes in by ZCode's own import instead).
- A model from Launch settings: no option or variable (step 0); the model
  is each App's own choice, kept in its tree's `v2/provider_config.json`.
- Per-App MCP servers, hooks, rules or skills: the engine reads them from
  the home folder (step 0); a per-App plugin would need the shared
  `~/.zcode/cli/config.json` written for its enablement and an install
  step the inventory seam lacks (research § E), so it waits; per-session
  `mcpServers` has no per-App set to carry.
- The plan's pools read from anywhere: the engine never asks for them.
- Its sub-agents: "not supported" with the reason; the agent tools; the
  desktop app per App.
