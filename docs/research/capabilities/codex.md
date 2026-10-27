# OpenAI Codex: how a third-party Mac app can observe and control it (CLI and desktop)

Research date: 2026-10-27. Labels follow `docs/research/EVIDENCE-STANDARD.md`: VP (verified primary), VS (verified secondary), NE (negative evidence, with a search log), I (inferred), U (unknown). Every source below was accessed on 2026-10-27 unless a line says otherwise.

`D/` stands for `https://learn.chatgpt.com/docs/`. `developers.openai.com/codex/*` now redirects there with a 308. `SRC/` stands for `https://github.com/openai/codex/tree/rust-v0.164.0/`.

**What was run.** Only these:
- `--help` and `--version`;
- `codex app-server generate-json-schema [--experimental]`, with a scratch `CODEX_HOME`;
- `plutil` on the app's Info.plist files;
- `ps -axo command` (command lines only, no environments).

Nothing signed in, sent a prompt, or wrote to `~/.codex`.

**Versions.**
- Codex CLI 0.163.0 is installed. The latest stable is **0.164.0**, released 2026-10-23 (npm `latest`, GitHub `rust-v0.164.0`). Alphas 0.165.0-alpha.1.2 and 0.166.0-alpha.1 came out on 2026-10-27. **VP**
- The ChatGPT desktop app is **26.1021.31345** (build 21828, bundle id `com.openai.codex`). It bundles codex-cli **0.164.0-alpha.11.1**. The newest desktop entry in the docs changelog is 26.1007 (2026-10-08). **VP**
- `@openai/codex-sdk` is 0.164.0 and the Python `openai-codex` is 0.163.0. **VP**

## What this corrects or extends in the existing notes

1. **The vendor's daemon now has official clients** (corrects ADR-0022's "attach to the vendors' own daemons… revisited if a vendor adds an attach interface"). All of these exist in 0.163.0 (`codex --help`, D/developer-commands, VP):
   - `codex app-server proxy --sock PATH`, which proxies stdio to the daemon's control socket;
   - `codex queue --thread <id|name> --message TEXT [--remote ADDR]`;
   - `codex agents` (a command center over the shared daemon);
   - `--remote` on resume, fork, archive, delete and unarchive;
   - `codex app-server daemon start|stop|restart|version`.

   The ownership objection (ADR-0010) still holds. The "no interface exists" premise no longer does.
2. **Stable app-server methods the codec doesn't use yet** (generated 0.163.0 schema; D/app-server; VP):
   - `thread/inject_items` appends model-visible items "without starting a user turn";
   - `thread/goal/set|get|clear` drives the same state as `/goal`;
   - `thread/read`, `thread/turns/list` and `thread/items/list` read a stored thread "without resuming it";
   - `thread/list` takes `sourceKinds`;
   - `thread/loaded/list`, `thread/unsubscribe` (a 30-minute grace period), `turn/diff/updated`, `turn/plan/updated` and `hook/started|completed`.
3. **Experimental methods** (they need `experimentalApi`; VP):
   - `dynamicTools` on `thread/start`, called back through the stable server request `item/tool/call`;
   - `thread/queue/*`;
   - `thread/search`;
   - `remoteControl/*`;
   - `process/spawn`;
   - `mcpServer/event/stream/*`.
4. **`approval_policy = "untrusted"` is now documented as unsupported**: "untrusted is unsupported, and on-failure is deprecated; use on-request … or never" (D/config-file/config-reference, VP). The 2026-10-25 labs used `approvalPolicy: "untrusted"`, and the 0.163.0 `-a` flag lists only `on-request|never`. **I**: AI Juggler should send `on-request` (or a granular policy) before 0.164+ rejects the old value.
5. **`item/tool/requestUserInput` carries `autoResolutionMs`.** "host clients can resolve the prompt automatically after that interval" (D/app-server, VP). Under D-28 as amended, AI Juggler must never act on it by itself.
6. **Hooks gate on trust.** "Non-managed hooks must be reviewed and trusted before they run", with trust recorded against each hook's hash (D/hooks, VP). So a per-launch Codex hook overlay, the analogue of Claude's `--settings` overlay, would be skipped until the user trusts it in `/hooks`. `--dangerously-bypass-hook-trust` exists but is meant for "automation that already vets hook sources" (I: not appropriate for user sessions).
7. **Desktop facts.**
   - The desktop app runs its own app-server on **stdio** pipes, observed with `-c plugins.codex-app-tools@openai-bundled.mcp_servers.codex_app.enabled=true`. Nothing can attach to it. **VP (local)**
   - It shares `~/.codex/config.toml` with the CLI: "The ChatGPT desktop app, Codex CLI, and IDE extension share this configuration" (D/extend/mcp). **VP**

---

## (A) Codex CLI

### A1. Official SDK, API and protocol

**App-server** (D/app-server; `codex app-server --help`; generated schema).
- **Transport and handshake.** JSON-RPC 2.0 without the `jsonrpc` field. `--listen stdio://` (the default) speaks JSONL. `unix://[PATH]` and `ws://IP:PORT` exist (WebSocket is "experimental and unsupported"), and `off` turns the local transport off. The handshake is `initialize {clientInfo, capabilities{experimentalApi, optOutNotificationMethods}}`, then `initialized`. **VP**
- **Size.** 105 stable client requests, 10 server requests and 85 notifications (172 and 11 with `--experimental`). **VP**
- **Approvals as server requests.** `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/permissions/requestApproval` (scope `turn` or `session`), `item/tool/requestUserInput`, `mcpServer/elicitation/request` and `item/tool/call` (dynamic tools). **VP**
- **Limits and usage.** `account/rateLimits/read`, `account/rateLimits/updated`, `account/usage/read` and `thread/tokenUsage/updated`. **VP**
- **Resume and fork.** `thread/resume` and `thread/fork`. Full-history hydration is deprecated in favour of `excludeTurns` plus `thread/turns/list` (lab of 2026-10-25, VP).
- **Daemon mode.** Since 0.157 the TUI starts or attaches to a daemon per `CODEX_HOME`. `--no-daemon` runs the server in-process and "Can't be combined with --remote or codex agents". **VP**
- **Remote TUI.** `codex --remote ws://…|wss://…|unix://[PATH]`, plus `--remote-auth-token-env`. **VP**
- **Stability.** "The app-server command and WebSocket transport are experimental and aren't supported for production workloads." **VP**
- **Schemas.** `generate-json-schema` and `generate-ts` produce output that is "specific to the Codex version you ran". **VP**
- **Bundled-build flags.** The desktop-bundled 0.164-alpha adds `--startup-client-info`, `--print-startup-config` and `--startup-sign-out` (`--help` diff, VP).

**Other surfaces.**
- **`codex exec --json`** emits JSONL events and cannot answer approvals (D/non-interactive-mode). **VP**
- **`@openai/codex-sdk`** spawns `codex exec --experimental-json` (`SRC/sdk/typescript/src/exec.ts:95`), so it has no approval callback. **VP**
- **Python `openai-codex`** is an app-server client whose default handler returns `accept` for command and file approvals (`SRC/sdk/python/src/openai_codex/client.py:833-838`). **VP**
- **`codex mcp-server` was removed on 2026-09-05.** `codex mcp-server` now falls through to the main help. **VP**
- **Deep links:** see the desktop section. `codex app [PATH]` opens the desktop app. **VP**
- **AppleScript and Accessibility:** not applicable to a terminal program. **I**

**Terms** (D/app-server, *Auth endpoints*, verbatim; the paragraph first appeared between 2026-09-27 and 2026-09-29 according to the Wayback record in `session-runtime.md` §2.2, and was re-read 2026-10-27). **VP**

> "If you've built a local or open-source application using Codex app-server authentication, you can continue using it, though we recommend migrating to Sign in with ChatGPT so users have greater control over and visibility into their usage. … App-server authentication has never been permitted for commercial or hosted services."

Also on that page: "Use clientInfo.name to identify your client for the OpenAI Compliance Logs Platform." **VP**

The OpenAI Terms of Use (effective 2026-01-01; read 2026-10-25 through Wayback, and 403 on 2026-10-27, so they were not re-read today) bar sharing credentials, circumventing rate limits, and "Automatically or programmatically extract data or Output". **VP (2026-10-25)**

### A2. Plugins and extensions

- **Plugins.**
  - **Layout:** a portable `plugin.json` (Agent Plugins schema) or `.codex-plugin/plugin.json`, with `skills/`, `.mcp.json`, `.app.json`, hooks and assets.
  - **Marketplaces:** `~/.agents/plugins/marketplace.json`, `$REPO/.agents/plugins/marketplace.json`, or `codex plugin marketplace add owner/repo|git|path`.
  - **Enabling:** per project with `[plugins."name@marketplace"] enabled`.
  - **App-server:** `plugin/*` and `marketplace/*` methods.

  Sources: https://developers.openai.com/plugins/build/plugins and `codex plugin --help`. **VP**
- **Skills.** `.agents/skills` from the cwd up to the repo root, `$HOME/.agents/skills`, `/etc/codex/skills`, and bundled system skills (D/build-skills). **VP** Custom prompts were "deprecated" on 2026-01-22 (D/changelog). **VP**
- **AGENTS.md.** `$CODEX_HOME/AGENTS.override.md` or `AGENTS.md`, then the project's files (codex.md §10). **VP**
- **Hooks.**
  - **Where:** `~/.codex/hooks.json`, `[hooks]` in config, `<repo>/.codex/hooks.json`, and plugin hooks.
  - **Events:** SessionStart, SessionEnd, PreToolUse, PermissionRequest, PostToolUse, Pre/PostCompact, UserPromptSubmit, SubagentStart/Stop, Stop and Interrupt. Handlers are `command` or `mcp_tool`.
  - **What a hook can do:**
    - PermissionRequest can allow or deny;
    - Stop with `decision:"block"` "automatically creates a new continuation prompt … using your reason as that prompt text";
    - async hooks deliver `additionalContext` "at the next safe point";
    - trust is per hash.

  Source: D/hooks. **VP**
- **Notify.** `notify = [...]` runs on `agent-turn-complete` with a JSON payload. It is user-level only (D/config-file/config-reference; codex.md §5). **VP**
- **ChatGPT apps and connectors.**
  - Plugins can include Apps SDK "apps".
  - MCP Apps UI extensions include "Extended message sending: Send messages to the current conversation, or a new one on desktop and Work on the web" (https://developers.openai.com/plugins/build/extensions). **VP**
  - Whether these reach CLI threads: **I**, no, because the CLI renders no MCP Apps UI.

### A3. What our own MCP server can do

- **Tools and policy** (D/extend/mcp, VP).
  - **Transport:** stdio or streamable HTTP; `codex mcp add NAME -- CMD | --url`; per launch with `-c mcp_servers.<n>.*`.
  - **Approvals:** `default_tools_approval_mode auto|prompt|writes|approve` and per-tool `approval_mode`.
  - **Tool lists and output:** `enabled_tools` and `disabled_tools`; per-tool `output_token_limit`.
  - **Defaults:** `startup_timeout_sec` 10 and `tool_timeout_sec` 60.
  - **Server guidance:** the server's `instructions` field is read, and its first 512 characters matter most.
- **Resources.** Codex gives the model `list_mcp_resources`, `list_mcp_resource_templates` and `read_mcp_resource` (`SRC/codex-rs/core/src/tools/handlers/mcp_resource/`), and app-server has `mcpServer/resource/read`. **VP**
- **Prompts.** Not found by this search. `on_prompt_list_changed` is only logged. **NE**
- **Elicitation.** It becomes `mcpServer/elicitation/request`. It is auto-declined when `approval_policy = never` (`SRC/codex-rs/codex-mcp/src/elicitation.rs` `elicitation_is_rejected_by_policy`), or when `granular.mcp_elicitations=false`. **VP**
- **Approvals for our tools.** They appear as normal approvals. PermissionRequest hooks see `mcp__server__tool`. **VP**
- **Pushing into a running thread: none.**
  - Progress, logging, resource-updated and list-changed notifications are logged only (`SRC/codex-rs/rmcp-client/src/logging_client_handler.rs`). **NE**
  - MCP Events (webhooks) are "available in Work chats on ChatGPT web, in Work chats in the desktop app with Cloud selected, and with dots" (https://developers.openai.com/plugins/build/mcp-events). That is not local Codex threads. **VP**
  - The supported ways to push are client-side: `turn/steer`, `thread/inject_items` and `thread/queue/add`.

### A4. Reading output and inserting input

- **Rollouts.**
  - **Layout:** `sessions/YYYY/MM/DD/rollout-*.jsonl` (and `.zst`), `session_index.jsonl` and `state_5.sqlite` (codex.md §10). **VP**
  - **Stability:** "the transcript format isn't a stable interface for hooks and may change over time" (D/hooks). **VP**
  - **Migration:** a `codex migrate-rollouts` command exists. **VP**
- **Streaming.** Driven sessions get `item/*`, deltas and `turn/*`. Any stored thread can be read with `thread/read`, `thread/turns/list` and `thread/items/list` without resuming it. **VP**
- **PTY.**
  - **Deterministic:** process liveness, writer locks, rollout `task_started` and `task_complete`, and typing a prompt or Ctrl-C into the PTY.
  - **Not deterministic:** whether an approval is showing (screen heuristics), and the meaning of a free-form question (that needs a model or the protocol). **I**
- **Inserting input.**
  - **Between turns:** `turn/start`.
  - **During a turn:** `turn/steer {expectedTurnId}`.
  - **Context without a turn:** `thread/inject_items`.
  - **Into a daemon-hosted session AI Juggler doesn't own:** `codex queue`. **VP**

---

## (B) Desktop: Codex inside ChatGPT.app

### B1. SDK, API and protocol

- **What the bundle contains** (Info.plist and the bundle, VP local).
  - It is an Electron/Chromium app: `ElectronAsarIntegrity`, Chromium 156.0.8078.48, and helpers `Codex (Renderer|GPU|Service|Alerts).app` plus "Aperitif" variants.
  - `Codex Framework.framework` is the renamed Electron framework.
  - The bundled CLI is `Contents/Resources/codex-cli/CodexCLI.app` (`com.openai.codex.cli`, 0.164.0-alpha.11.1).
  - Computer use runs as `~/.codex/computer-use/Codex Computer Use.app` (`SkyComputerUseService`).
- **Attaching to its app-server: not possible.** It runs as a stdio child (`ps`, VP local). No document describes an external attach (NE, searched D/app-server, D/remote-connections, D/developer-commands and D/app). The app uses `~/.codex` (shared config, VP).
- **Deep links** (D/reference/commands, VP). The scheme is `codex` (`CFBundleURLSchemes ["codex","http","https"]`, VP local).
  - `codex://threads/<thread-id>` opens "A local chat".
  - `codex://threads/new` and `codex://new?prompt=&path=&originUrl=`, where `prompt` only "Sets the initial composer text".
  - Settings, skills, automations, plugin install and detail pages, and pets.
- **AppleScript.** `NSAppleScriptEnabled = false`. The Apple Events usage string is for the app controlling *other* apps. **VP local**
- **Accessibility.** An Electron AX tree exists, but it is undocumented and unsupported. **I**
- **Terms.** No desktop-specific third-party statement was found (**NE**). The app-server terms in A1 apply to app-servers AI Juggler starts, not to the desktop app's process.

### B2. Plugins and extensions

- **Bundled plugins.** The marketplace `openai-bundled` ships these (VP local):
  - `codex-app-tools`: "Exposes Codex desktop app tools through one local MCP server". Its prompt-approval tools are `create_thread`, `send_message_to_thread`, `fork_thread`, `handoff_thread`, `move_thread_to_project` and `automation_update`. It reaches the app over `CODEX_APP_TOOLS_PIPE_PATH`. It is proprietary and off by default; the app turns it on for its own server.
  - `browser`, `chrome` (a Chrome extension host whose hooks call `node_repl.turn_ended` on Stop, SubagentStop and Interrupt), `computer-use`, `unified-computer-use`, `computer-history`, `record-and-replay`, `deep-research`, `latex`, `visualize`, `messages` and `image-guided-assets`.
- **Personal marketplace.** `~/.agents/plugins/marketplace.json`, then restart the app. Hooks are "manually installed Codex desktop plugins only" (plugins/build/plugins). **VP**
- **Skills and connectors.** Skills use the same paths. Apps and connectors come with plugins, and MCP Apps UI extensions are available. Whether they render in Codex chats (as opposed to Work chats) is **U**.

### B3. Our MCP server

- **Adding it.** Through `~/.codex/config.toml` or Settings > MCP servers. It is shared with the CLI, and the app needs a restart (D/extend/mcp, VP).
- **No per-launch injection.** AI Juggler doesn't launch the app's server, so a persistent user config edit is the only route. **I**
- **Capabilities.** The same as the CLI: tools, resources, elicitation in the app's UI (I), and no push into a thread (**NE**).

### B4. Reading output and inserting input

- **Reading.**
  - Rollouts in `~/.codex/sessions`: originator "Codex Desktop", observed earlier (codex.md §4). **VP**
  - Alternatively, `thread/read` and `thread/turns/list` from an app-server AI Juggler owns on the same home, without resuming. **VP**, documented; not lab-tested against desktop threads.
- **Inserting.**
  - **Supported:** a deep-link prefill that the user sends themselves, and OpenAI's own Remote Control.
  - **Off-limits:**
    - calling the private `codex_app` pipe;
    - `thread/resume` on a thread the app has loaded (two writers);
    - UI scripting;
    - editing rollouts or SQLite.
  - **`codex queue` aimed at a desktop-loaded thread:** **U**. The desktop server isn't the daemon, so the queued item lands in the state DB and may never be drained.

---

## (C) The most valuable things AI Juggler doesn't do yet, ranked

1. **Read any Codex thread through the protocol** (`thread/list` with `sourceKinds`, `thread/read`, `thread/turns/list`, `itemsView`) from one read-only app-server per profile, instead of parsing rollouts for history.
   - **Value:** desktop and terminal history uses the vendor's own model, with paging that suits "Instant and paged" and survives rollout format changes.
   - **Risk:** the server starts Codex (cache writes, a `config.toml` mtime churn, already seen). It never resumes. Its terms are the local/open-source permission. **Low.**
2. **Move off `approvalPolicy: "untrusted"`** to `on-request` or a granular policy (with `mcp_elicitations=true`) before a Codex release rejects it.
   - **Value:** avoids a hard break.
   - **Risk:** none. It is a correctness fix.
3. **Resume with `excludeTurns` and page the history** (already flagged by the lab), and show `turn/diff/updated` and `turn/plan/updated` in the session page. **Low risk**, all stable methods.
4. **Dynamic tools** (`dynamicTools` plus `item/tool/call`) for driven sessions: the bridge's `whoami`, `pool_roster` and `set_status` with no MCP subprocess, token or socket.
   - **Risk:** experimental (it needs `experimentalApi`, which ADR-0022 avoids). Keep MCP as the default until it is stable.
5. **`thread/inject_items` for E18 handover context**, giving the model context without a fake user turn.
   - **Risk:** raw Responses API item shapes. Whether other clients show the injection is U. Use it for AI Juggler's own handover notes only, never for unreviewed text.
6. **Thread goals** (`thread/goal/*`), so a switch can carry `/goal` state A→Codex→A and the session page can show the goal's progress. **Low risk, stable.**
7. **Daemon-hosted terminal sessions the user started** (`codex` without `--no-daemon`).
   - **What it gives:** observe them through `codex app-server proxy` or `--remote unix://` (`thread/loaded/list`, live status, approvals), and deliver user-typed messages with `codex queue`.
   - **Risk:** AI Juggler doesn't own the daemon (ADR-0010). A second subscriber also receives every approval. Read-only plus an explicit Answer is the safe shape. Needs a lab on spare accounts.
8. **A Codex Stop or async hook** as an exact turn-boundary signal and a queued-message delivery for Codex terminal sessions.
   - **Risk:** hook trust (a per-hash review in `/hooks`). It must never be bypassed with `--dangerously-bypass-hook-trust` on user sessions. It is only worth it if the user trusts an AI Juggler hook once.
9. **An opt-in "AI Juggler" Codex plugin** in a personal marketplace that bundles the bridge MCP server and skills for the desktop app.
   - **Value:** brings the bridge to desktop threads.
   - **Risk:** it writes the user's `~/.agents` and `~/.codex`, needs an app restart, and plugin hooks need trust. It is a switch-preflight-style approval item.

**Not worth doing, or off-limits:**
- any attach to the desktop app's process;
- `codex_app` tools;
- UI scripting of ChatGPT.app;
- relying on `autoResolutionMs`;
- Python SDK defaults;
- bundling either SDK.

## Search log (negative evidence)

- **Attaching to the desktop app-server, or a desktop message-insert API.**
  - Read: D/app, D/remote-connections, D/developer-commands, D/app-server, D/reference/commands, D/notifications and D/integrated-terminal.
  - Looked at: the ChatGPT.app Info.plist and the bundled plugin manifests.
  - Found: only deep-link prefill, Remote Control and the app's private `codex_app` MCP.
- **An MCP server pushing into a local thread.** Read D/extend/mcp, https://developers.openai.com/plugins/build/mcp-events, D/app-server, and `SRC/codex-rs/rmcp-client/src/{logging_client_handler,event_notification_transport,tool_list_changes}.rs`. Found none.
- **MCP prompts support.** Read the same sources. No prompt surface found.
- **A desktop-specific third-party terms statement.** Read D/app-server, D/sign-in-with-chatgpt and D/open-source. None found.
- **Re-reading the Terms of Use on 2026-10-27.** openai.com returned 403, so the 2026-10-25 reading stands.
