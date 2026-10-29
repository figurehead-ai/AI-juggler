# Cursor, Antigravity and Kiro: control surfaces for AI Juggler

Research date: 2026-10-27. This sweep re-checks the 2026-10-21 research in `docs/research/{cursor,antigravity,kiro}.md`.

## Method and labels

- **(re 10-27)** marks a claim re-read on 2026-10-27.
- An unmarked claim carries over from the 2026-10-21 research and was not re-read.
- Local check on 2026-10-27: none of the six products is installed.
  - No bundle is in /Applications.
  - None of `agent`, `cursor-agent`, `cursor`, `agy`, `antigravity`, `kiro` or `kiro-cli` is on PATH.
  - So `checked_version` is null everywhere, and no Info.plist URL scheme could be read.
- AI Juggler has no adapter for any of the three tools.

| Label | Meaning |
|---|---|
| VP | Vendor docs or binary |
| VS | Secondary source |
| NE | Searched, not found |
| I | Inferred |
| U | Unknown |

## Changed since 2026-10-21

### Cursor

- **Versions:**
  - The desktop cask is now 3.27.22 (was 3.26.13).
  - The CLI cask is now 2026.10.22-27dcf5f (was 2026.10.19).
  - Source: Homebrew API, generated 2026-10-27.
- **CLI v2026.10.22:** adds `agent env deploy --env`. Nested AGENTS.md rules now apply. Subagents fall back to the parent model. VP https://cursor.com/docs/cli/changelog
- **CLI v2026.10.17:** post-tool hooks now fire for subagents. Subagent transcripts are stored under the parent conversation. VP
- **CLI v2026.10.15:** adds `--mcp-tool-timeout`. VP
- **CLI v2026.10.08:** ACP clients get wakeups from watched background shells. VP
- **Desktop changelog, 2026-10-20:** a new Agent SDK. Agents are code projects, get an `mcp-connections/` folder and can be triggered by webhooks. VP https://cursor.com/changelog
- **Newly documented details:**
  - MCP support covers Tools, Prompts, Resources, Roots, Elicitation and Apps.
  - The stop hook's `followup_message` auto-submits a prompt.
  - There are only three deeplink types.
- **Terms:** unchanged. The AUP is still dated 2026-08-11 and the ToS 2026-09-03.
- **Corrections from the Cursor research agent (re 10-27):** these replace the matching claims below.
  - **MCP install deeplink:** one is documented, `cursor://anysphere.cursor-deeplink/mcp/install?name=&config=<base64>`. VP https://cursor.com/docs/mcp/install-links
  - **Cursor extension API:** `vscode.cursor.mcp.registerServer` / `unregisterServer` and `vscode.cursor.plugins.registerPath` / `unregisterPath`. It still has no Agent or Composer API. VP https://cursor.com/docs/extension-api
  - **Plugins:** `.cursor-plugin/plugin.json` can bundle rules, skills, agents, commands, hooks and MCP servers. VP https://cursor.com/docs/plugins
  - **ACP `session/list`:** advertised via `sessionCapabilities.list` since CLI 2026.04.16 (VS: forum user report). A third-party client cannot switch models at runtime, per staff (VS), so pin the model with `--model`.
  - **Remote Control:** "The agent loop moves to the cloud while its tools keep running on your machine." It works in the Agents Window only and needs a plan with Cloud Agents. No protocol is published. VP https://cursor.com/docs/cloud-agent/mobile
  - **MCP in the CLI:** the CLI docs mention tools only, so whether the CLI supports elicitation is unknown.
  - **Desktop elicitation:** reported not to render in the Agents Window on macOS. VS https://forum.cursor.com/t/mcp-elicitation-does-not-render-in-the-cursor-agents-window-on-macos/162243
  - **Tool limit:** a 40-tool cap was reported in 2025 (VS). Whether it still applies is unknown.
  - **Hook payloads:** none of the documented payloads has token or usage fields. The changelog says otherwise, so this is U.
  - **Agent SDK:** published as `@grok/agent-sdk` 0.3.0, GA on 2026-10-23. It has its own `serve` HTTP API, which covers only its own agents. VP https://cursor.com/docs/agent-sdk
  - **Install script:** now handles a `grok-switch.json` record, "to help finish a pending switch to the Grok agent host". This is a churn risk. VP https://cursor.com/install

### Antigravity

- **Versions:**
  - The App is at 2.26.0 (2026-10-22), after 2.25.0 (2026-10-20). VP https://antigravity.google/docs/changelog/
  - The IDE cask is 2.6.0 (was 2.5.5). The CLI cask is 1.4.8 (was 1.4.4). VS
- **CLI 1.4.7:** hooks get `lastUserInput`. Headless `-p` no longer runs the statusline or title commands. VP https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md
- **CLI 1.4.4:** MCP `tools/call` now carries `antigravity.google/root_conversation_id`. VP
- **ACP registry:** `antigravity-acp` is now 1.4.3 (was 1.4.1). VS
- **Work plans (relevant to AJ-E1613):** the Additional Terms say they "do not apply to you" for these plans; the administrator's terms govern instead. VP https://antigravity.google/terms/
  - Gemini Enterprise (Google Cloud)
  - Gemini Enterprise for Business
  - Workspace on the Google Cloud Pre-GA terms
  - Gemini Enterprise API key
- **Open issues:** #31, #155, #381 and #1069 are all still open (checked with `gh api` on 2026-10-27).

### Kiro

- **CLI 2.34.0 (Oct 23):** MCP elicitation through a form or link, in V3.
- **CLI 2.33.0:** V3 workspace trust. Hooks are gated on trusting the workspace.
- **V3 default:** V3 becomes the default "with CLI 3.0 at the end of October." VP https://kiro.dev/changelog/cli/
- **Versions:**
  - The CLI cask is 2.34.2 (was 2.33.0).
  - The IDE is at 1.4.26 (Oct 22), and Memory arrived in 1.4.0. VP https://kiro.dev/changelog/ide/
- **ACP docs:** they say "Compatible third-party editors can also connect to Kiro through ACP." The IDE itself uses ACP. VP https://kiro.dev/docs/acp/
- **Terms:** the pricing FAQ's harness clause is unchanged.

## (a) Findings

### Cursor

#### CLI (`agent`, alias `cursor-agent`)

**1. Official control**

- **Headless mode:** `-p --output-format text|json|stream-json`, with `--resume`, `--force`, `--trust` and `--mode`. VP https://cursor.com/docs/cli/headless
  - Stream events: `system/init`, `user`, `assistant`, `tool_call`, `result`.
  - Usage fields appear only in the changelog, not in the output-format reference (NE).
- **ACP (re 10-27):** `agent acp`. VP https://cursor.com/docs/cli/acp
  - Transport is stdio only.
  - Methods: `initialize`, `authenticate` (`cursor_login`), `session/new`, `session/load`, `session/prompt`, `session/cancel`. There is no `session/list` (NE).
  - Server requests and notifications: `session/update` and `session/request_permission` (allow-once, allow-always, reject-once).
  - Blocking extensions: `cursor/ask_question`, `cursor/create_plan`.
  - Notification extensions: `cursor/update_todos`, `cursor/task`, `cursor/generate_image`.
  - Auth: `agent login`, `CURSOR_API_KEY` or `CURSOR_AUTH_TOKEN`.
  - Limits: Team MCP is not supported, and tools block if permission requests go unanswered.
  - The page says ACP "is intended for building custom clients and integrations."
- **SDK (re 10-27):** the local runtime needs an API key or an SDK browser login stored in `~/.cursor/sdk/auth.json`. VP https://cursor.com/docs/sdk/typescript
  - It "does not read credentials from a local Cursor app installation".
  - It lists and resumes only agents the SDK created. Attaching to desktop or CLI agents was not found (NE).
- **Cloud Agents API:** runs in the cloud. VP
- **`agent persist` / `agent attach`:** VP
- **Terms (re 10-27):**
  - The AUP (2026-08-11) bans "automated or non-human means, whether through a bot, script, or otherwise". It has no exception for the API or CLI. It also bans circumventing metering. VP https://cursor.com/acceptable-use-policy
  - The ToS (2026-09-03) has no wrapper clause. It bans scraping and derivative works. VP https://cursor.com/terms-of-service
  - The AUP conflicts with the ACP docs' invitation to build custom clients (I). This needs a ruling.

**2. Plugins and extensions**

- **Rules:** `.cursor/rules/*.mdc`, AGENTS.md (nested rules work since 10.22) and CLAUDE.md. VP
- **Hooks (re 10-27):** VP https://cursor.com/docs/hooks
  - `stop` can return `followup_message`, which "will automatically submit it as the next user message". `loop_limit` defaults to 5. `subagentStop` works the same way.
  - `sessionStart` can return `additional_context` and `env`.
  - `preToolUse` can allow or deny, return `updated_input` and add an `agent_message`.
  - `beforeSubmitPrompt` can return `continue`.
  - `postToolUse` can return `additional_context` and `updated_mcp_tool_output`.
  - `beforeShellExecution` and `beforeMCPExecution` can allow, deny or ask.
  - `workspaceOpen` can return `pluginPaths`.
- **Commands:** the deeplink command type is VP. The `.cursor/commands` file path is U.
- **Skills:** `/skills`. VP

**3. MCP (our own server)**

- **Config:** `~/.cursor/mcp.json` or `.cursor/mcp.json`, over stdio, SSE or HTTP, with OAuth. VP
- **Supported (re 10-27):** Tools, Prompts, Resources, Roots, Elicitation and Apps. VP https://cursor.com/docs/mcp
- **Not found (NE):** sampling, notifications, a tool-count limit, a deeplink to install an MCP server.
- **Pushing into a session:** NE.
- **CLI parity:** `agent mcp list|list-tools|login|enable|disable`, `--approve-mcps` and `--mcp-tool-timeout`. VP

**4. Reading output and inserting input**

- **Where chats live:** VS
  - `~/.cursor/chats/<hash>/<uuid>/{store.db, meta.json}`
  - `~/.cursor/projects/*/agent-transcripts/*.jsonl` (subagent transcripts nest under the parent since 10.17, VP)
- **Format stability:** no promise found (NE).
- **PTY wrapping:** possible, but ACP is the deterministic path (I).
- **Inserting input:** ACP `session/prompt`, or `-p --resume`. A stop hook's `followup_message` can chain prompts. VP
- **Off-limits:** the Keychain items, `~/.cursor/auth.json`, decoding the `store.db` protobuf, dashboard endpoints.

#### Desktop (Cursor.app, `com.todesktop.230313mzl4w4u92`)

- **URL scheme (re 10-27):** VP https://cursor.com/docs/reference/deeplinks
  - Three link types: `cursor://anysphere.cursor-deeplink/prompt?text=…`, `…/command?name=&text=` and `…/rule?name=&text=`.
  - "Deeplinks never trigger automatic execution". There is no link to a conversation or folder (NE).
- **VS Code extension reach into the Agent panel:** no documented API (NE).
  - Forum posts show undocumented command ids such as `composer.newAgentChat`, `workbench.action.chat.open` (fills the box but does not submit) and clipboard-paste tricks. VS https://forum.cursor.com/t/is-it-possible-to-submit-chat-programmatically/157654 and https://forum.cursor.com/t/a-command-for-passing-a-prompt-to-the-chat/138049
  - These are unsanctioned and break with versions (I).
- **Hooks and MCP:** shared with the CLI through `~/.cursor`. VP
- **iOS remote control of local agents:** no third-party protocol found (NE).
- **Origin App Marketplace (2026-10-13):** an API for code hosting, not agent control. VP
- **AppleScript or Accessibility:** NE.
- **Off-limits:** `state.vscdb`, which holds the auth JWT. VS

### Antigravity

#### CLI (`agy`)

**1. Official control**

- **Headless mode (re 10-27):** VP https://antigravity.google/docs/cli/headless/
  - Flags: `-p`, `--output-format text|json|stream-json`, `--input-format stream-json` (long-running stdin driving from an app), `-c`, `--conversation`, `--json-schema`.
  - The JSON envelope has `conversation_id`, `status`, `response`, `duration_seconds`, `num_turns` and `usage`.
  - Events: `init`, `step_update`, `result`.
  - Exit codes: 0, 1 and 2.
- **Quota:** `agy -p /usage --output-format json` reads it without spending any. VP
- **Statusline JSON:** gives quota, `plan_tier`, `email` and `agent_state`. Since 1.4.7 it runs in interactive mode only. VP
- **Status signals:** OSC 7501 and the title hook. VP
- **ACP:** not native (#31 is open). A proprietary `agy_acp_server.par` 1.4.3 is listed in the registry. VS https://github.com/agentclientprotocol/registry/blob/main/antigravity-acp/agent.json
- **SDK:** works with an API key or ADC only. VP
- **Remote Control:** no third-party protocol found (NE).

**2. Hooks (re 10-27)** VP https://antigravity.google/docs/hooks/

- Events: SessionStart, PreToolUse, PostToolUse, PreInvocation, PostInvocation and Stop. They apply to the App, the IDE and the CLI.
- SessionStart, PreInvocation and PostInvocation can return `injectSteps` (`systemMessage`, `userMessage`, `ephemeralMessage` or `toolCall`).
- Stop can return `decision:"continue"`.
- PreToolUse can deny, ask or allow, and can `overwrite` the arguments.
- Payloads include `conversationId`, `transcriptPath` and `lastUserInput`.

Rules, skills, plugins and sidecars are as recorded on 2026-10-21. Workflows ended on 2026-10-19.

**3. MCP (re 10-27)** VP https://antigravity.google/docs/mcp/

- Config: `~/.gemini/config/mcp_config.json` or `.agents/mcp_config.json`. The App manages servers in its Settings UI.
- Only tools and resources are confirmed. Prompts, elicitation, sampling and roots were not found (NE).
- No tool limit is stated.
- No way to push into a session (NE). The server does receive `root_conversation_id`.
- Off-limits: OAuth tokens kept in plain text in `mcp_oauth_tokens.json`.

**4. Reading output and inserting input**

- **Transcripts:** `brain/<id>/.system_generated/logs/transcript.jsonl`. The path is documented (VP); the schema is not (NE).
- **Conversations:** stored in SQLite.
- **Inserting input:** `stream-json` on stdin, or `--conversation`. VP
- **Isolation:** there is no home env var (#155). The HOME workaround is VS. A shared Keychain item is VS/I.

**5. Terms (re 10-27)**

- **FAQ:** third-party software accessing Antigravity violates the ToS (no plan qualifier). Wrapping `agy` or automating account rotation is "not supported under consumer plans". VP https://antigravity.google/docs/faq/
- **Additional Terms §6:** treats such use as a breach (the OpenClaw example) and excludes work plans. VP https://antigravity.google/terms/
- **Enterprise docs:** enterprise use "operates under your existing Google Cloud Terms of Service", and the docs say nothing on wrappers. The standalone IDE is not supported for enterprise. VP https://antigravity.google/docs/enterprise/
- **Not yet read:** the Google Cloud ToS and the Service Specific Terms. U (next step for AJ-E1613).

#### Desktop (App 2.x `com.google.antigravity`; IDE `com.google.antigravity-ide`)

- **Surfaces:** no headless mode, no launcher, one unified sign-in.
- **Hooks, rules and MCP:** apply to the App and the IDE. VP
- **URL scheme:** `antigravity://` was not found (NE). The Info.plist is unreadable here (U).
- **IDE extension reach into the agent panel:** NE. The Agent Manager is slated for removal from the IDE. VP
- **Editor extensions:** exist for VS Code, Visual Studio, JetBrains, Zed and Xcode, but their transport is undocumented. NE https://antigravity.google/docs/ide/extensions
  - The registry's ACP server links to that page, so the extensions probably use ACP (I).

### Kiro

#### CLI (`kiro-cli`)

**1. Official control**

- **ACP (re 10-27):** `kiro-cli acp --agent-engine=v3`. VP https://kiro.dev/docs/cli/v3/acp-migration/
  - Methods: `initialize`, `authenticate`, `session/new`, `list`, `load`, `fork`, `prompt`, `cancel`, `set_mode` and `set_config_option`.
  - Server requests and notifications: `session/update` and `session/request_permission`.
  - `_kiro/*` extensions:

    | Area | Methods |
    |---|---|
    | Session | `session/context`, `session/compact`, `session/rename` |
    | Account and auth | `account/getUsage` (shape U), `auth/getAccessToken` (needs `--auth-method=client`) |
    | MCP | `mcp/status`, `mcp/resetServer` |
    | Agent features | `knowledge`, `codeIntelligence`, `hooks/list`, `workflow/*`, `tools/didChange` |
    | Other | `openExternalUrl`, `governance/state`, `error/rate_limit` |

  - `_session/steer` adds input during a turn.
  - Storage is "server-owned".
- **Third-party editors:** may connect. VP https://kiro.dev/docs/acp/
- **Engine:** V3 becomes the default with CLI 3.0. VP
- **Headless mode:** `chat --no-interactive --output-format stream-json`, exit codes 0, 1, 3 and 4. VP
- **Profiles:** `KIRO_HOME` (VP), with bugs #10417, #11348 and #11960 (VS).
- **Status signals:** OSC 7501 and the terminal title. VP

**2. Hooks and configuration (re 10-27)** VP https://kiro.dev/docs/hooks/

- Hooks work in the IDE and the CLI.
- Files: `.kiro/hooks/*.json`, plus `~/.kiro/hooks` in V3.
- Actions: a `command` gets JSON on stdin. An `agent` action "Injects a prompt into the current conversation."
- Triggers include PostFileSave, PreToolUse, PostToolUse and Stop (`confirmCommand`).
- Workspace trust gates hooks.
- Also available: steering, AGENTS.md, custom agents, Powers, specs and skills.

**3. MCP (re 10-27)** VP https://kiro.dev/docs/mcp/

- **Supported:**
  - Tools over stdio or HTTP/SSE.
  - Prompts and resources (via `#`).
  - Elicitation, in the CLI from V3 2.34.0.
- **Parity:** the IDE, CLI and Web match.
- **Limits:**
  - A tool name must be 64 characters or fewer and match `^[a-zA-Z][a-zA-Z0-9_]*$`.
  - A description over 10k characters triggers a warning.
  - No tool-count cap is stated.
- **Install links:** `kiro://` links open a confirmation dialog.
- **Not found (NE):** sampling, roots, Apps, push into a session.

**4. Reading output and inserting input**

- **Reading:** ACP `session/list` and `session/load`.
- **Inserting:** ACP `session/prompt` and `_session/steer`.
- **PTY wrapping:** the state signals are deterministic, but the text is not (I).
- **Off-limits:** `data.sqlite3` `auth_kv`, `_ get-kas-token`, private usage APIs.
- **Cut-off:** old builds stop connecting on 2026-11-09.

**5. Terms (re 10-27)** VP https://kiro.dev/pricing/

- The FAQ allows "ACP compatible IDEs" and CI/CD use.
- It does not permit third-party harnesses that "route requests outside of Kiro's native interfaces".
- A supervisor that spawns native `kiro-cli acp` stays inside the native interfaces (I). A written ruling is still needed.

#### Desktop (Kiro IDE, `dev.kiro.desktop` per I)

- **ACP:** the IDE uses ACP (VP). Whether an external client can attach is U.
- **Extension API:** none documented (NE). The only source is a third-party analysis of `kiro.kiroagent` (VS https://github.com/ghuntley/amazon-kiro.kiro-agent-source-code-analysis), which is off-limits under the AWS IP License. The docs describe chat as UI only. VP https://kiro.dev/docs/ide/chat/
- **URL scheme:** `kiro://` MCP install links (VP). Nothing else found (NE).
- **Shared with the CLI:** hooks, steering and MCP. VP
- **Chat export:** to a zip. VP
- **On-disk stores:** undocumented. VS

## (c) What to build first (ranked, I)

1. **Kiro CLI over ACP V3, with a per-account absolute `KIRO_HOME` (AJ-E1606).**
   - Why: the fullest sanctioned surface. It has list, load, fork, prompt, steer, permissions and usage, and the vendor invites ACP clients. Support V3 only.
   - Risks:
     - The FAQ's harness clause needs a written AWS ruling.
     - Sign-in separation on macOS needs a live test (#11960).
     - Suspension reports after account switching.
2. **Desktop track-only for all four apps (AJ-E1610).**
   - What: detect by bundle id, focus windows, and read transcripts read-only only where the path is documented.
   - Optional: "Open in Cursor" through the prompt deeplink, with the user confirming.
   - Optional: a Cursor companion extension that only registers AI Juggler's MCP server or plugin path, through the documented API. It would read nothing and drive nothing.
   - Not: VS Code extension control. None of the vendors documents an agent-panel API.
   - Never touch `state.vscdb`, the Keychain or token files.
3. **Cursor CLI over `agent acp` (AJ-E1609).**
   - Condition: a maintainer ruling on the AUP's "automated or non-human means" clause (no carve-out).
   - Isolation needs a live test, through `CURSOR_API_KEY` or a per-account HOME.
   - Quota stays unknown.
   - Sanctioned hooks for later: `followup_message` and `additional_context`.
4. **Antigravity CLI (AJ-E1613).**
   - Consumer plans are track-only: the FAQ says wrapping is unsupported, and there is a suspension precedent.
   - Work plans are excluded from the Additional Terms. Read the Google Cloud ToS and Service Specific Terms before building.
   - If cleared, drive `agy -p --input-format stream-json --output-format stream-json` and read quota with `-p /usage`.
   - There is no per-account home yet (#155, #381).
5. **An AI Juggler MCP server in all three tools (later, cross-tool).**
   - Tools work in all three. Elicitation is documented in Cursor and Kiro.
   - No tool lets an MCP server push into a session, so MCP is not a control plane.
   - The sanctioned injection points are hooks: Cursor's `followup_message`, Antigravity's `injectSteps` and Kiro's agent action. Each needs a consented config write, or an AI Juggler-owned home.

## Still open

- The Google Cloud ToS and Service Specific Terms on wrapping.
- The `antigravity://` scheme, and every bundle's `CFBundleURLTypes`. This needs the apps installed.
- The response shapes of Kiro's `getUsage` and Cursor's stream-json usage. These need live captures (AJ-E1612).
- Whether the Kiro IDE harness accepts an external ACP client.


## Kiro re-verification (2026-10-27, applied over the entries above)


- CLI latest 2.34.2 (vendor manifest https://desktop-release.q.us-east-1.amazonaws.com/latest/manifest.json, VP); changelog newest 2.34.0 (2026-10-23). IDE latest 1.4.26 (2026-10-22, VP https://kiro.dev/changelog/ide/).
- V3 is NOT the default yet: still "early release" alongside 2.x; opt-in startup prompt in a gradual rollout since 2.30.0 (VP https://kiro.dev/docs/cli/v3/, changelog 2.30). 2.34.0's default ACP engine is v2 (VS GH#12118). Changelog says V3 default "with CLI 3.0 at the end of October".
- ACP launch: `kiro-cli acp --agent-engine=v3 --auth-method=cli|client` (VP). V3 refuses --agent/--model/--effort/--trust-*; use session/set_config_option (model, effortLevel); no session/set_model in V3. v2 has session/set_model but no session/list (GH#7098 open).
- Register session/update before create/load: live updates not buffered (VP). Permission options must be echoed exactly; never synthesize (VP).
- MCP: tools, resources(+templates), prompts, elicitation (form/URL; CLI local V3 from 2.34; IDE), tools/list_changed (CLI 2.14+). Not documented: sampling, roots, MCP Apps; progress not rendered (VS GH#10134). Names ≤64 incl. server prefix, ^[a-zA-Z][a-zA-Z0-9_]*$. Untrusted workspace: every MCP call asks, even autoApprove. No MCP push-into-session (NE).
- Hooks: v1 JSON .kiro/hooks/*.json; global ~/.kiro/hooks V3 only. Triggers SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostFile{Create,Save,Delete}, Pre/PostTaskExec, Stop (+SessionEnd V3). Exit 0 stdout → agent context; Stop {"decision":"block"} continues. Exit-code docs conflict (exit-codes page: only 2 blocks; hooks/actions: any non-zero). Workspace Trust (2.33.0) gates hooks/agents/MCP/steering in interactive V3.
- Output/input: KIRO_ACP_RECORD_PATH records TUI ACP traffic to JSONL (VP; may carry token traffic — keep private). OSC 7501 + title states (VP; pending ask reported as error, VS GH#11987). /chat save-via-script sends session JSON to a script (VP). Session live in one process only ("Session is active in another process") — can't attach to a user's running TUI (VP). Shell integration exec's kiro-cli-term; guard PROCESS_LAUNCHED_BY_KIRO=1 (VS GH#11170).
- Fresh-install `kiro-cli acp` blocks session/new ~66 s downloading an 83 MB embedding model (VS GH#12118).
- Desktop (IDE): agent is built-in extension kiro.kiro-agent using ACP internally (VS, maint); no documented way for external ACP agent or extension API to drive the agent (NE). URL scheme: kiro://kiro.mcp/add?name=&config= and kiro://kiro.powers/add?name= (VP, from kiro.dev launch-page JS), confirmation dialog; no prompt/open link (NE). "Open with Kiro CLI" hand-off (VP). Status dots on tabs. KIRO_HOME in IDE: NE.
- Terms: pricing/FAQ unchanged — "ACP compatible IDEs" allowed; "third-party automation harnesses (such as OpenClaw)" routing outside native interfaces not permitted. ACP docs invite "custom integration" (VP). AWS Service Terms updated 2026-10-22, nothing on harnesses. Customer Agreement §6.4 bans reverse engineering. Reading: spawning native kiro-cli acp is the defensible path; ask AWS.

## Antigravity re-verification (2026-10-27, applied over the entries above)

- Latest: CLI 1.4.8 (changelog commit 2026-10-27, VP); App 2.26.0 (2026-10-22, VP); IDE 2.6.0 (Homebrew cask, 2026-10-27, VS; vendor pages still show 2.5.5); antigravity-acp 1.4.3 (ACP registry, 2026-10-23, VS); SDK 0.1.22 (2026-10-13).
- Headless multi-turn (VP, /docs/cli/headless/): `--input-format stream-json` with `--output-format stream-json`; input lines `{"event":"user","message":{"content":"…"}}`; one `result` per turn; `control_request`/`control_response` and CLI slash commands give ERROR, exit 2 — no permission handshake or interrupt over the stream. Approval-needing tools are soft-denied in headless. Exit code 3 + `AGY_ERROR:` on stderr for model/API failures. `--print-timeout` default differs between pages (5m vs 0s).
- Remote Control: `agy remote-control start|status|stop|serve` (LaunchAgent); protocol reachable only through Google's web UI (NE). `agy --hub` (background server started by Google's VS Code extension): protocol NE, off-limits.
- ACP: no native ACP in `agy` (issue #31 open). `antigravity-acp` 1.4.3 is a separate proprietary binary with its own home and credentials (`~/.gemini/antigravity-acp/`), no usage or quota (#1045), and non-Gemini models only for allowlisted client names (#1318) (VS). Google's docs sanction it for Zed, JetBrains and Xcode (VP).
- SDK `google-antigravity` spawns its own bundled `localharness`; API key, Vertex/ADC or local models only; no consumer OAuth (VP/NE). It does not drive the App or CLI.
- App: sidecars with `agentapi new-conversation` / `send-message` are the only documented way to put input into the App (VP), but a repo collaborator calls it "internal implementation details, not meant to be called directly" (VS, #519). `antigravity://` scheme: NE. AppleScript/AX: NE. IDE agent panel: no documented API (NE).
- Hooks (VP, App/CLI/IDE): `.agents/hooks.json`, `~/.gemini/config/hooks.json`, plugins — hooks in CLI `settings.json` are not loaded (correction). `injectSteps` (system/user/ephemeral message, toolCall) on SessionStart/Pre-/PostInvocation; Stop `decision:"continue"`; PreToolUse allow/auto_approve/deny/ask/force_ask/deny_unless_prior_grant with argument `overwrite`. 1.4.7 adds `lastUserInput`.
- MCP (VP): stdio or Streamable HTTP (`serverUrl`, and `url` also works); tools, resources, progress, lazy tool loading; `_meta["antigravity.google/root_conversation_id"]` on `tools/call`. Prompts, elicitation, sampling, roots, MCP Apps: NE. No push into a session (NE).
- Reading (VP): status line JSON (quota with reset, plan_tier, cost, `agent_state` idle|working|tool_use|reviewing|authenticating|initializing|error, pending_input_count, tool_confirmation_pending) — not under `-p` since 1.4.7; OSC 7501 working/waiting/done/failed; `transcript.jsonl` path in hook payloads (schema NE); `cache/last_conversations.json`. Live TUI input: no supported route (#1303).
- Terms: consumer plans — FAQ wording ("Wrapping agy inside third-party coding agents or applications … not supported under consumer plans") appeared between 2026-10-10 and 10-13 (Wayback); the Additional Terms count third-party access as a breach; forum guidance split (a staff moderator said a local unmodified `agy` subprocess "is permitted" on 10-07; later non-staff replies say the FAQ supersedes it). Work plans (Gemini Enterprise, Enterprise for Business, Workspace on Cloud Pre-GA, Enterprise API key) are outside the Additional Terms; Google Cloud Service Specific Terms (2026-10-12) have no wrapper or orchestrator clause (NE); the Pre-GA terms were not read (U).
