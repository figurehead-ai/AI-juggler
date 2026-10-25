# Evidence manifest — AJ-E1704

Task: `docs/tasks/E17-agent-pools.md` AJ-E1704, the agent bridge, read-only (P0).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR), with the spike's three live points **UNPROVEN** until the maintainer's run on spare accounts (AJ-E1701 Req.2, `docs/internal/MAINTAINER-STEPS.md` §17). 2026-10-25, claude-opus.

Design: `docs/adr/0020-agent-bridge.md` (with its STRIDE table). Test output: [bridge-tests.txt](docs/qa/evidence/AJ-E1704/bridge-tests.txt), [conformance.txt](docs/qa/evidence/AJ-E1704/conformance.txt).

## What exists now

1. **`juggler-agent`** (new crate `crates/juggler-agentbridge`, `unsafe_code = "forbid"`, depends on nothing of ours; serde and serde_json only, already the workspace's: no new dependency).
   - `--mcp`: an MCP server on stdio, JSON-RPC by hand (`mcp.rs`): `initialize` (2024-11-05 to 2025-11-25, echoing a known version), `server/discover` (2026-07-28), `ping`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `resources/templates/list`, `prompts/list`; JSON-RPC's own errors for anything else, never a crash.
   - Otherwise a command line for tools without MCP: `whoami`, `roster`, `cards`, `status <text>` (exit 0 done, 2 refused, 1 not reached, 64 not a command). No option takes the key.
   - No state: each call is one line to the app over its socket and one line back (`client.rs`); the key is sent only to a socket in a folder no one else can write, owned by the same user.
2. **The socket** (`features/agent_bridge/server.rs`): `<data folder>/agent/bridge.sock`, per edition; folder `0700`, socket `0600`; peer uid checked; one request per connection, ≤ 8 KiB, 2 s to send, 16 at once; a stale socket removed only when it is ours. Served lazily.
3. **Identity** (`features/agent_bridge/mod.rs`): 32 random bytes per launch, in the launch's environment (`JUGGLER_AGENT_TOKEN`, with `JUGGLER_AGENT_SOCKET` and `JUGGLER_AGENT`), never argv or a file. Every launch path mints a new one (new session, resume, switch, switch back, undo, continue a conversation, a paired Mac's hand-over); the session's older keys stop, and so does an ended session's. Only the SHA-256 is kept (memory, and a `0600` file in the `0700` folder so sessions outliving a restart keep their tools). The app never reads another process's environment (D-37).
4. **Injection** (D-30), through the adapters (`HarnessAdapter::agent_bridge`, `Capabilities::AGENT_BRIDGE`; `claude_bridge.rs`, `codex_bridge.rs`; the fake too):
   - Claude Code: `--mcp-config <overlays>/agent/<session>/juggler-mcp.json` (its `env` is `${JUGGLER_AGENT_TOKEN}`/`${JUGGLER_AGENT_SOCKET}`), never `--strict-mcp-config`; `--allowedTools mcp__juggler__whoami,mcp__juggler__pool_roster,mcp__juggler__set_status`; both after the launch's own arguments.
   - Codex: `-c mcp_servers.juggler.{command,args,env_vars,enabled_tools}`, `default_tools_approval_mode="prompt"` and `tools.<tool>.approval_mode="approve"` for the three tools only, before `--`.
   - Off until the one-time question is answered yes; off per project and per profile; a launch that can't get the bridge starts without it.
5. **The question and the switch** (UI): "Let agents in your projects see each other and work together?" on a pool's page (members tab) and in Settings › Agent access until answered (never a pop-up); then Settings › Agent access › Agent tools in sessions: one switch, and a fold with a switch per project and per profile, read only when it opens. Commands `agent_bridge_get`, `agent_bridge_answer`, `agent_bridge_set`.
6. **Tools**: `whoami`, `pool_roster`, `set_status`, and the cards as the resource `juggler://pool/cards`. Every call: one request shape (`deny_unknown_fields`) → the key → 30 calls / 10 s → the switches → the session's pool and role → **`allowed(pool, action)`** (audited) → the answer. `whoami`/`pool_roster` are `read_roster`, the cards `read_cards`, `set_status` `set_status`. An ask is a refusal that says only the user can allow it (no pending-ask store yet: **noted for E18**).
7. **D-33**: agents see names, places, tools, states, roles and profile labels; never an email, a plan, an account id or a folder. What an agent says (`set_status`, one line, ≤ 140 characters) takes the card's "says" line, marked as its own words, for the user and the pool.
8. **Ledger**: `guess_role` knows `juggler-agent` (by its executable, or its `--mcp` command line) as `McpServer`: owned by its session, never auto-cleaned (ADR-0010's tiers), flagged in the kill sheet as an MCP server.
9. **Signing**: `.github/workflows/build-macos.yml` builds `juggler-agent` for both chips, `lipo`s it to `binaries/juggler-agent-universal-apple-darwin`, sets Tauri's `externalBin`, and checks it is in `Contents/MacOS`; `scripts/sign_macos.sh` names it as the one helper, signs it on its own (hardened runtime, no entitlements) before the app, verifies it, and refuses a bundle without it. `.gitignore` keeps `binaries/` out. `just dev`/`dev-bridge` build it beside the dev executable.
10. **Privacy page**: two lines in "What it changes" (`agent-tools`, `agent-tools-sees`), each naming the tests below; `docs/guide/trust.md` says them word for word.
11. **Docs**: ADR-0020; SECURITY-GUIDELINES' threat model row (TB6); MAINTAINER-STEPS §17 (the spike); E17's statuses.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a `juggler-agent` program, MCP with `--mcp`, a command line otherwise | `mcp::tests` (4), `cli::tests` (2); `bridge.rs::the_command_line_reads_the_key_from_its_environment_only` | PASS |
| Req.1 in the signed bundle | `macos_bundle.rs::the_agent_bridge_ships_signed_inside_the_bundle`; `bash -n scripts/sign_macos.sh` | PASS (the rehearsal itself not run, as asked) |
| Req.2 a per-edition Unix socket, `0600`, no state | `bridge.rs::the_socket_is_0600_in_a_0700_folder_and_answers_only_current_keys`; `client::tests::the_key_goes_only_to_a_socket_in_a_closed_folder` | PASS |
| Req.3 a random key per epoch, in the environment, never argv; rotated on switch and resume | `bridge.rs::the_key_is_in_the_environment_only_and_nothing_lands_in_the_account_folder`, `a_new_epoch_gets_a_new_key_and_the_old_one_stops`, `the_command_line_reads_the_key_from_its_environment_only` | PASS |
| Req.4 injection through the adapters; Claude `--mcp-config`, never strict; Codex `-c mcp_servers.juggler.*`; own tools pre-allowed | `claude_bridge::tests`, `codex_bridge::tests`; conformance kit check 7 for all 7 subjects ([conformance.txt](docs/qa/evidence/AJ-E1704/conformance.txt), 49 passed) | PASS |
| Req.4 the one question, then on, off per project and profile | `bridge.rs::nothing_is_added_before_the_question_is_answered`, `each_level_…` (switched off: refused at once); `settings::tests`; vitest `agent-tools.test.tsx` (5) | PASS |
| Req.4 per-launch files in AI Juggler's folder, nothing in the account's | `bridge.rs::the_key_is_in_the_environment_only_and_nothing_lands_in_the_account_folder`, `a_fake_tool_speaks_mcp_to_the_real_binary_through_the_app_launch` | PASS |
| Req.5 `whoami`, `pool_roster`, `set_status`, the cards resource | `bridge.rs::a_fake_tool_speaks_mcp_to_the_real_binary_through_the_app_launch` (the fake tool launched by `launch_profile`, the real binary, the real core; the card then says the agent's line) | PASS |
| Every call through `allowed()`, audited; refusals per level | `bridge.rs::each_level_refuses_what_it_doesnt_allow_and_every_call_is_audited` (Work together, Read, Custom, observer, switched off; 10 audit rows) | PASS |
| D-33 labels only | `bridge.rs::agents_see_account_labels_never_emails_or_plans` | PASS |
| Hard lines (no path to a session, a setting or a secret) | `bridge.rs::what_agents_reach_has_no_path_to_a_session_a_setting_or_a_secret`; `wire::tests::a_request_round_trips_and_refuses_strangers` | PASS |
| Req.6 the ledger shows the helper as an MCP server, excluded from cleanup | `bridge.rs::the_ledger_knows_the_bridge_as_an_mcp_server_and_never_cleans_it_up` | PASS |
| Performance: a call answers in < 50 ms | `bridge.rs::calls_answer_in_under_50_ms` (slowest of 5 over the socket: 2.7 ms, 8 sessions); the fake tool's `tools/call` and `resources/read` each < 50 ms through the binary | PASS |
| Privacy page: the bridge's lines, with tests | `trust.rs::every_trust_claim_names_tests_that_exist`; vitest `privacy-page.test.tsx` "says what the agent tools expose, per pool level…" | PASS |
| Visual: the opt-in question; the Settings switch | `ui/tests/visual/agent-tools-gallery.spec.ts` (both themes, axe 0 critical/serious, keyboard) | UNPROVEN (written; the Docker gate wasn't run, as asked: baselines below) |
| Spike: `--mcp-config` loads in Claude Code without an approval prompt | MAINTAINER-STEPS §17 | UNPROVEN |
| Spike: Codex's `env_vars` passes the key; the per-tool approval pre-allows | MAINTAINER-STEPS §17 | UNPROVEN |
| Spike: which MCP revision each tool sends | MAINTAINER-STEPS §17 (both lines answered meanwhile) | UNPROVEN |

## Baselines to make (Docker visual gate, not run here)

- `ui/tests/visual/__baselines__/linux/gallery/92-agent-tools/agent-tools-question-1280x800-dark.png` (the family's gate state, new).
- Gallery captures to `docs/qa/evidence/_gallery/92-agent-tools/`: `agent-tools-question-1280x800-{dark,light}.png`, `settings-agent-tools-1280x800-{dark,light}.png`.
- Possibly changed: `gallery/84-privacy/tests-open-dark.png` and `terms-dark.png` (two more lines above them on the Privacy page); `page-dark.png` if the "What it changes" group reaches the first screen. The pools gallery seeds the question as answered, so its captures don't change.

## Not covered here

- Pending asks agents can wait on (Needs you): E18. Until then an ask is a refusal in words.
- A hook notice at turn boundaries (research §2.3): not in the read-only bridge.
- A release build made with `just app` has no `juggler-agent` beside it (only the release workflow bundles it): Settings says "This build of AI Juggler doesn't include them."

Last updated: 2026-10-25.
