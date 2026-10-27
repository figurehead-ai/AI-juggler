# Kimi Code and Grok Build: control surfaces for AI Juggler

Research date: 2026-10-27. Every source below was read on 2026-10-27 unless marked *(carried, 2026-10-21)*.
Labels: **VP** Verified-primary (vendor docs, vendor source, registry) · **VS** Verified-secondary · **NE** Negative-evidence (search log given) · **I** Inferred · **U** unknown.

Method: vendor docs (Kimi docs as Markdown at `moonshotai.github.io/kimi-code/en/…`, Kimi product docs at `kimi.com/code/docs/en/…`, `docs.x.ai/….md`); shallow clones of `MoonshotAI/kimi-code` (HEAD e2ad1dcf, 2026-10-27) and `xai-org/grok-build` (HEAD 2bdd1d6a, synced 2026-09-29, crate 1.0.45), read as text; npm/PyPI registries; vendor version pointers. Neither `kimi` nor `grok` is installed on this Mac, so I ran no `--version`. Nothing signed in, sent a prompt, or read credentials.

Short names: `KD/` = https://moonshotai.github.io/kimi-code/en/ · `KS/` = https://github.com/MoonshotAI/kimi-code/blob/main/ · `KC/` = https://www.kimi.com/code/docs/en/ · `GD/` = https://docs.x.ai/build/ · `GS/` = https://github.com/xai-org/grok-build/blob/main/crates/codegen/ · `UG/` = `GS/xai-grok-pager/docs/user-guide/`

## What AI Juggler uses today (from the repo)

- **Kimi** (`crates/juggler-adapters/src/kimi_code.rs`, `kimi_sessions.rs`):
  - Its capabilities are `HEADLESS | RESUME | TRANSCRIPT_WATCH`.
  - It launches the TUI in a PTY, resumes with `--session <id>`, signs in with `kimi login`, and isolates accounts with `KIMI_CODE_HOME`.
  - It runs `-p … --output-format stream-json` only for API-key accounts, because membership accounts are `interactive_only`.
  - It reads `session_index.jsonl` and `state.json`. It does not read `wire.jsonl`, so tokens are unknown.
  - It refuses a Primed launch and a resume with a delta message.
  - No agent bridge, no state reports, no usage, and no Desktop (`desktop_app: None`, tracked as AJ-E1610).
- **Grok** (`grok_build.rs`, `grok_sessions.rs`):
  - The same three capabilities.
  - It isolates accounts with `GROK_HOME`, unsets `GROK_AUTH_PATH` and `XAI_API_KEY`, and sets ten `GROK_CLAUDE_*/GROK_CURSOR_*_ENABLED=0` variables.
  - It resumes with `--resume <id>` and runs headless with `-p … --output-format json`.
  - It reads `summary.json` only.
  - It refuses a Primed launch and a resume with a delta message. No bridge, state reports or usage.
- **Shared plumbing:**
  - The agent bridge MCP server (`juggler-agent --mcp`) is wired for Claude Code and Codex only: `agent_bridge()` returns `None` for Kimi and Grok.
  - `Supervisor::write_input` already writes bytes into a PTY session (`crates/juggler-supervisor/src/supervisor.rs`).

### Changed since (2026-10-27, branch `adapters-0.2-fixes`)

- **Grok** takes a first message: a Primed launch is `grok [--model m] -- "<prompt>"` in the project's folder, and a resume with a delta is `grok --resume <id> [--model m] -- "<delta>"`. That lets a switch start Grok with its handover pointer (the switch's generic Primed launch; Grok has no handover module of its own). `read_dirs` takes no flag: Grok asks before it reads outside the session's folder.
  - **VP** (source 1.0.45): the positional prompt (`cli.rs`, its test `positional_prompt_seeds_interactive_session`) and its use on resume: `dispatch_initial_prompt` says "a `--resume`/`-c`/`-w` session started earlier in startup is reused", and `event_loop.rs` sends it after the startup resume. That settles the **I** below.
  - **VP** (source): a bare word that names a subcommand runs it (`subcommand_takes_precedence_over_positional_prompt`), hence the `--`. **I**: that `--` makes the rest a message in Grok's parser (clap's rule, as Codex's `--` was verified on 0.163.0), and that binary 1.0.70 still behaves as the 1.0.45 source. Both are on the AJ-E1612 checklist.
- **Kimi** reads `agents/<agent>/wire.jsonl` (`kimi_sessions/wire.rs`), incrementally and read-only, protocol 1.5 only:
  - **Open sessions** come from Kimi Code's own session locks, `$KIMI_CODE_HOME/locks/sessions/<id>.lock` = `{version, token, pid, hostname, startedAt, clientIdentity, clientDisplayName}`, claimed while a CLI, Desktop or web client holds the session and removed when it lets go. **VP** — `KS/packages/agent-core-v2/src/persistence/backends/node-fs/sessionLockService.ts`.
  - **State** from the main agent's log: `turn.prompt` open → working, an `interaction.request` without its `interaction.resolved` (matched by `id`) → needs you, else idle. **VP** for the records and their fields (`wire-manifest.d.ts`); **I** that a turn's end settles what it asked, and that a turn or request older than the lock's `startedAt` belonged to an earlier process.
  - **Tokens** from every agent's `usage.record` (`{agentId, model, usage: TokenUsage, usageScope}`; `TokenUsage` = `inputOther, output, inputCacheRead, inputCacheCreation`; one record per model call, summed per model by Kimi itself). **VP** — `KS/packages/agent-core-v2/src/agent/usage/usageOps.ts`, `src/session/usage/usageAgentModel.ts`, `src/human/llm/usage.ts`. Each record's `time` is `Date.now()` ms (**VP**, `wire/wireService.ts`). Subagents keep their own logs under `agents/<agentId>/` (**VP**, the path check in `wireService.ts`).
  - A log whose first line isn't `{"type":"metadata","protocol_version":"1.5",…}` gives no state and no usage (D-06). Usage events are keyed by time, model and counts, so a fork's copied records count once (**I** that a fork copies them unchanged).
  - Still on the live checklist (AJ-E1612): a real 2.3.x log, lock and fork; whether a subagent's approval shows in the main agent's log.

## Corrections to docs/research/kimi.md and grok.md

1. **Kimi CLI latest is 2.3.3** (2026-10-26), not 2.3.0. Kimi Code Desktop is at 1.1.4 (2026-10-26), not 1.1.1. **VP**
   - https://registry.npmjs.org/@moonshot-ai/kimi-code
   - `KC/kimi-code-desktop/changelog.html`
2. **The Grok Build binary is at 1.0.70 stable / 1.0.71 alpha.** The public repo is still the 2026-09-29 sync at 1.0.45, so source-derived claims are about 25 patch versions old. **VP**
   - https://x.ai/cli/stable, https://x.ai/cli/alpha
   - GitHub `pushed_at` 2026-09-29
3. **Kimi Desktop has a URL scheme**, `kimi-code://open?root=<path>&new=1`, which is what `kimi app` sends. This narrows NE-1. **VP**
   - `KS/apps/kimi-code/src/cli/sub/app.ts`
4. **Kimi `packages/node-sdk` is private and unpublished.** It is `@moonshot-ai/kimi-code-sdk` 0.21.1, `private: true`, and npm returns 404. The published `@moonshot-ai/kimi-agent-sdk` (0.1.8, 2026-04-13) targets the archived Python Kimi CLI. **VP**
5. **Kimi has more hook events than listed in kimi.md.** The full list adds `UserPromptQueued`, `TurnStarted`, `PermissionRequest`, `PermissionResult`, `SessionEnd`, `TaskStarted`, `StopFailure` and `Interrupt`. **VP**
   - `KD/customization/hooks.md`
6. **`wire.jsonl` has a generated, versioned schema** (protocol 1.5, 67 record types), so kimi.md §4's "[U] wire.jsonl schema" is now answered from source. **VP**
   - `KS/packages/agent-core-v2/docs/wire-manifest.d.ts`
7. **Grok Build takes a positional initial prompt**, e.g. `grok "fix the bug"`. AI Juggler's "can't be started with a first message yet" is wrong for Grok. **VP**
   - `GS/xai-grok-pager/src/app/cli.rs`
8. **Grok has more surfaces than grok.md lists.** **VP**
   - `grok agent serve` (WebSocket), relay `headless` and `leader` modes.
   - Per-session `_meta.pluginDirs` and per-process `--plugin-dir`.
   - A `GROK_CONFIG` overlay that is allowlisted and cannot add MCP servers or hooks.
   - Sources: `UG/15-agent-mode.md`, `UG/09-plugins.md`, `UG/05-configuration.md`.

## (a) Findings

### Kimi Code — CLI layer (Kimi Code CLI `kimi`, MoonshotAI/kimi-code, MIT, latest 2.3.3)

**1. SDK / API / protocol**

- **Headless print mode** **VP**
  - `kimi -p "<prompt>" [--output-format text|stream-json]`. stream-json prints assistant messages (with `tool_calls`), tool messages, and `meta` lines (`system.version`, `session.resume_hint`, `turn.step.retrying`).
  - **No usage object.** The source confirms NE-4.
  - Print mode uses the "auto" permission policy and can't be combined with `--yolo`, `--auto` or `--plan`.
  - Project MCP servers stay off unless `KIMI_CODE_TRUST_WORKSPACE=1`.
  - Sources: `KD/reference/kimi-command.md`, `KS/apps/kimi-code/src/cli/prompt-render.ts`.
- **ACP: `kimi acp`, JSON-RPC on stdio.** **VP**
  - It implements all 3 core and all 11 session methods: new, load, resume, list, fork, close, delete, prompt, cancel, set_mode, set_config_option, plus the `session/set_model` extension.
  - Reverse calls to the client: `session/update`, `request_permission`, `fs/*`, `terminal/*`, and `elicitation/create` (form).
  - `session/new` accepts `mcpServers` per session.
  - Not implemented: `providers/*`, `nes/*`, `document/*`, `elicitation/complete`.
  - Source: `KD/reference/kimi-acp.md`.
- **Local server: `kimi web`.** **VP**
  - REST `/api/v1` plus WebSocket `/api/v1/ws`, listening on 127.0.0.1:58627 and up. The bearer token is in `server.token`. Live specs are at `/openapi.json` and `/asyncapi.json`.
  - Covers sessions, prompts (submit, steer, abort), approvals and questions, tasks, MCP, plugins, workspace trust, fs, and OAuth usage.
  - The docs mark it experimental: "interface stability is not guaranteed".
  - **It drives only sessions loaded in that server process.** A session a TUI holds returns `40931` (occupied). `:takeover` is only for a holder that is gone.
  - Source: `KD/reference/server-api.md`.
- **SDK: none usable.** The in-repo SDK is private, and the published one is for the legacy CLI (correction 4). **VP**
- **Terms for third-party apps on the subscription** *(re-read 2026-10-27)* **VP**
  - "Kimi Code subscriptions are for interactive use only."
  - "Don't use Kimi Code for non-interactive automation."
  - "Don't spoof or alter client identity information."
  - Third-party tools (Claude Code, OpenCode, …) are explicitly welcomed.
  - Source: `KC/kimi-code/community-guidelines.html`.
  - **I:** An app where a person drives sessions (over the TUI, or over ACP with a person approving) fits the terms. Unattended `-p` runs, auto-resume and scheduled prompts on a membership account do not.

**2. Plugins and extensions** **VP**

- **Plugins** use a `kimi.plugin.json` manifest: skills, agents, `sessionStart.skill`, `systemPrompt(Path)`, `mcpServers`, `hooks`, `commands`. Plugin skills are invoked as `<pluginID>:<skill>`.
- **Skills** live in `$KIMI_CODE_HOME/skills`, `~/.agents/skills` and the project. `--skills-dir` replaces them for one launch; `--agent` and `--agent-file` pick the agent for one launch.
- **Rules** are AGENTS.md files.
- **Hooks** are `[[hooks]]` entries (event, matcher, command, timeout), fail-open. Blocking events are `UserPromptSubmit`, whose stdout is appended to the context, `PreToolUse` and `Stop`. `SessionHeartbeat` fires every 60 s.
- **There is no per-launch config, hook or plugin flag.** "The CLI currently reads a single user-level config file."
- Sources: `KD/customization/plugins.md`, `hooks.md`, `skills.md`, `KD/configuration/overrides.md`.

**3. What our own MCP server can do** **VP**

- **Config:** `$KIMI_CODE_HOME/mcp.json` or project `.kimi-code/mcp.json` (`mcpServers` with stdio, http or sse entries; `enabledTools`, `deferred`, `bearerTokenEnvVar`). It can also come from a plugin, or from ACP `session/new` `mcpServers`.
- **Tools only.** The `MCPClient` interface is `listTools`, `callTool` and `ping`. The SDK client is built without capabilities, so there are no resources, prompts, elicitation, sampling, roots, or `list_changed` handling.
- **Naming and approval:** tools are named `mcp__<server>__<tool>`. Each call asks for approval unless config.toml has a `[[permission.rules]]` allow entry.
- **Timing:** a server added mid-session joins only new sessions.
- **Identity:** `clientInfo.name` is `kimi-code`.
- **Push into a session:** none through MCP. The documented paths are the `UserPromptSubmit` hook's stdout, and the server's `prompts`/`:steer` for sessions the server owns.
- Sources: `KD/customization/mcp.md`, `KS/packages/agent-core-v2/src/mcpCore/types.ts`, `client-stdio.ts`.

**4. Reading output and inserting input**

- **Files** **VP**
  - `session_index.jsonl`, plus `state.json` and `agents/main/wire.jsonl` in each session folder.
  - `wire.jsonl` opens with a `{type:"metadata", protocol_version:"1.5"}` line. Each later line is `{type, …payload, time}`.
  - Useful record types:
    - `turn.prompt` (input, origin)
    - `turn.ended` (`completed|cancelled|failed|blocked`, error)
    - `context.append_message`
    - `usage.record` (`model`, `TokenUsage`, `usageScope`)
    - `interaction.request` (`approval|question|user_tool`) and `interaction.resolved`
  - Sources: `KD/configuration/data-locations.md`, `KS/packages/agent-core-v2/docs/wire-manifest.d.ts`.
  - **I:** This gives a deterministic working / needs-you / idle state and exact per-turn tokens without a model. It needs a live capture to confirm (AJ-E1612).
- **PTY** **VP**
  - The TUI (pi-tui) supports bracketed paste and has a paste-burst detector. A prompt sent mid-turn is queued (the `UserPromptQueued` event).
  - **There is no positional prompt**: extra arguments error out (`KS/apps/kimi-code/src/cli/commands.ts`).
  - **I:** Injection means pasting the text and then sending CR. Send it only when no `interaction.request` is open, or the keystrokes land in the dialog.
- **ACP** `session/prompt` works for sessions AI Juggler itself drives. **VP**

### Kimi Code — Desktop layer (Kimi Code Desktop 1.1.4, closed source)

- **Shares the CLI's data.** The same `~/.kimi-code` holds config, credentials, sessions and logs. UI state is in `~/Library/Application Support/kimi-code-app/`. **VP**
  - `KC/kimi-code-desktop/settings-and-extensions.html`
- **Shares the CLI's extensions.** The same `mcp.json`, `[[hooks]]` and plugins apply to Desktop. **VP**
- **Shares sessions, with occupancy.** Sessions appear in both apps, with an occupancy prompt and a takeover option. **VP**
  - `KC/kimi-code-desktop/project-and-session-management.html`
- **URL scheme:** `kimi-code://open?root=&new=1` opens a new draft in a workspace. No prompt parameter was found. **VP** (source)
- **Not found by this search** **NE**
  - Any documented API, IPC or prompt injection into Desktop.
  - Whether Desktop honours `KIMI_CODE_HOME`.
  - Its bundle id.
  - Search log: the KC desktop pages (getting-started, help, settings, project/session, interface, changelog), and `grep kimi-code://` over the CLI source.
- **I:** So the Desktop agent is reachable only indirectly: files in the default home, hooks and MCP set in `~/.kimi-code` (the user's own setup, so this needs consent), and the URL scheme to open a folder.

### Grok Build — CLI layer (Grok Build `grok`, xai-org/grok-build, Apache-2.0; binary 1.0.70, source 1.0.45)

**1. SDK / API / protocol**

- **Headless mode** **VP**
  - `grok -p|--single`, `--prompt-json` or `--prompt-file` for the prompt.
  - `--output-format plain|json|streaming-json|streaming-messages-json`. The `json` object carries `sessionId`, `stopReason`, `usage`, `modelUsage` and `total_cost_usd`.
  - Sources: `GD/cli/headless-scripting.md`, `UG/14-headless-mode.md`.
- **ACP** **VP**
  - Four transports: `grok agent stdio`, `grok agent serve --bind … --secret …` (WebSocket; state survives reconnects), relay `headless`, and `leader`.
  - Standard methods: session/new, load and prompt; `session/update` (message and thought chunks, tool calls, plan); permission requests; `configOptions` (model, reasoning_effort).
  - `_meta` options: `rules`, `systemPromptOverride`, `agentProfile`, `yoloMode`, `autoMode`, `pluginDirs`.
  - `x.ai/*` extensions cover fs, git, worktree, terminal, fork, rewind, compact and auth.
  - Source: `UG/15-agent-mode.md`.
  - **VS:** Vercel `@ai-sdk/harness-grok-build` 1.0.103 and TanStack `@tanstack/ai-grok-build` 1.0.0 both drive Grok Build over ACP (their npm dependencies on harness-acp and ai-acp).
- **SDK** **NE**
  - The docs point to generic ACP SDKs.
  - The source names an official `grok-agent-sdk`, but it is not on npm, and the PyPI name is a third-party reservation (0.0.0). Treat it as unreleased.
- **Terms** *(carried, 2026-10-21)* **VP**
  - The Build docs call headless "ideal for scripts, automations, or integration into other apps".
  - The AUP prohibits "unauthorized automated or non-human means".
  - Grok Build reads multiple accounts as allowed (only credential sharing is banned).
  - **I:** Driving the official binary is sanctioned use. Never reuse `auth.json`, and never scrape usage.

**2. Plugins and extensions** **VP**

- **Plugin layout:** `skills/`, `commands/`, `agents/`, `hooks/hooks.json`, `.mcp.json`, `.lsp.json`, optional `plugin.json`.
- **Plugin scopes:**
  - `_meta.pluginDirs` (one ACP session, trusted automatically)
  - `grok agent --plugin-dir` (one agent process; "hooks and MCP servers activate without a prompt"; ignored in leader mode)
  - `.grok/plugins` (needs folder trust)
  - `$GROK_HOME/plugins`
  - `[plugins].paths`
- **Hooks**
  - Types: `command` or `http` (POSTs the event; `${VAR}` expands in the url).
  - Events: SessionStart/End, UserPromptSubmit, PreToolUse, PostToolUse(/Failure), PermissionDenied, Stop, StopFailure, StopCancelled, Notification (`idle_prompt`, `permission_prompt`, `task_complete`), Subagent*, Pre/PostCompact.
  - An allowing UserPromptSubmit hook's stdout is discarded.
- **Config overlay:** `GROK_CONFIG` / `GROK_CONFIG_PATH` is allowlisted (models, features, toolset) and "cannot spawn commands … or add a discovery source".
- Sources: `UG/09-plugins.md`, `UG/10-hooks.md`, `UG/05-configuration.md`, `GS/xai-grok-pager/src/app/cli.rs`.

**3. What our own MCP server can do** **VP**

- **Config:** `[mcp_servers.<name>]` in `$GROK_HOME/config.toml` or project `.grok/config.toml`, `grok mcp add`, a plugin's `.mcp.json`, or ACP `mcpServers`.
- **How the model reaches it:**
  - Tools through `search_tool` / `use_tool` (deferred loading).
  - Resources through `ListMcpResources` / `FetchMcpResource`.
  - `tools/list_changed` and `resources/list_changed` are handled.
- **Elicitation:** form (with schema validation) and url modes, advertised only when an interactive client is attached and not over the ACP transport. Also advertises MCP Apps (`io.modelcontextprotocol/ui`).
- **Not supported:** sampling and roots are never advertised. No prompts support was found.
- **Identity:** `clientInfo.name` is `grok-shell-<server>`.
- **Push into a session:** no server-initiated turn. Live list changes and elicitation are all an MCP server gets. Hooks: `PostToolUse` can add `additionalContext`.
- Sources: `GS/xai-grok-mcp/src/servers.rs`, `elicitation.rs`, `GS/xai-grok-shell/src/extensions/mcp.rs`, `GD/features/mcp-servers.md`.

**4. Reading output and inserting input**

- **Session files** in `sessions/<cwd>/<id>/` **VP**
  - `summary.json`
  - `updates.jsonl`, the ACP update stream, "authoritative" for /resume
  - `signals.json`: turn and tool counts, `context_tokens_used`, `context_window_tokens`
  - `grok usage <id>` gives token and cost totals
  - Sources: `UG/17-sessions.md`, `GS/xai-grok-shell/src/session/signals.rs`.
- **Live roster:** `$GROK_HOME/active_sessions.json` holds the open TUI sessions `{session_id, pid, cwd, opened_at}`, pruned by PID. It is internal, but deterministic. **VP**
  - `GS/xai-grok-active-sessions/src/lib.rs`
- **Input**
  - **Positional initial prompt.** `grok "…"` conflicts only with the `-p` family. **VP**
  - **Resume with a prompt.** `--resume <id> "…"` should parse. **I**, from the clap conflicts in `cli.rs`. Since confirmed from the source (**VP**, 1.0.45): `dispatch_initial_prompt` reuses the resumed session; see "Changed since" above.
  - **PTY paste.** The TUI handles crossterm `Event::Paste`. **VP**
  - **ACP prompts.** `session/prompt` works for sessions AI Juggler drives. **VP**
  - **Dashboard replies.** The TUI dashboard replies to its own live sessions in-process. **VP**, `GD/features/dashboard.md`
- **Leader mode (internal).** `--leader` shares one backend between clients over `~/.grok/leader.sock`, using a length-prefixed internal protocol. It is off by default. **VP** that it exists, **I** that it should not be used.

### Grok Build — Desktop layer

- **No first-party desktop app reaches the Grok Build agent.** **NE**
  - Search log: docs.x.ai `llms.txt` (the Build pages cover CLI and ACP only) and the grok-bot/* docs.
  - WebSearch "Grok Desktop xAI official app download Mac 2026" and "Grok desktop app macOS Grok Build coding agent desktop October 2026" returned only community apps (liaan/grok-desktop, rimusz/grok-build-desktop, Rushour0, Jane-o-O-o-O, productcompass) and a MacDailyNews piece (2026-08-29) saying there is no native Grok chat app.
- **"Grok Desktop" appears only in the source.** It is a terminal brand Grok Build detects (`UG/21-terminal-support.md`, `UG/05-configuration.md`), and "grok-desktop" is named as an ACP client in the shell CHANGELOG. **I:** an unreleased or internal client.
- **Grok Bot is a different product.** The desktop app (0.77.0, from Cursor's update API) is official, but it runs cloud agents "on a persistent cloud computer". **VP**
  - Bots touch the local machine only under a local-computer policy: "Ask every time" by default.
  - Sources: `docs.x.ai/grok-bot/overview.md`, `computer-and-apps.md`, `deployment.md`.
  - It is not Grok Build, so it doesn't count as this layer.

## (c) Most valuable additions, ranked

1. **Grok: support Primed launch and resume-with-message using the positional prompt.** (S, low risk)
   - Build `grok "<prime>"` and `grok --resume <id> "<delta>"`. This unblocks the A→B→Grok switch handover (a 0.1 core feature) without PTY tricks.
   - Risk: resume plus prompt is inferred, so verify live. Pass the prompt as one argv element, never through a shell.
2. **Kimi: read `wire.jsonl` for live state and tokens.** (M, low risk)
   - Get working, needs-you and idle from `turn.prompt`/`turn.ended`/`interaction.*`, and exact tokens from `usage.record`. That replaces "tokens unknown".
   - Risk: the schema is versioned in source, not user docs. Gate on `protocol_version` and fall back to unknown. Size-cap reads.
3. **Grok: read `signals.json` / `updates.jsonl` and `active_sessions.json`.** (M, low risk)
   - Gives context use, turn counts and live "open in a TUI (pid)". `grok usage <id>` is the documented cross-check for tokens.
   - Risk: the source lags the binary by 25 patch versions. Live-capture against 1.0.70 first.
4. **Agent bridge (MCP) for Kimi and Grok in AI Juggler-owned homes.** (M, medium risk)
   - Write a managed `[mcp_servers.juggler]` (Grok) or `mcp.json` entry (Kimi) into the per-account home AI Juggler created. Never into the default `~/.kimi-code` or `~/.grok` without the user's consent.
   - Kimi sees tools only: no resource for the pool cards, so expose cards as a tool. It also needs a `[[permission.rules]]` allow for `mcp__juggler__*`.
   - Grok defers MCP tools behind `search_tool`, so tool descriptions must be findable.
   - Risk: this edits config files. Keep a managed block and remove it on account removal.
5. **State reports through hooks.** (M, medium risk)
   - Kimi: `TurnStarted`, `Stop`, `PermissionRequest`, `Interrupt`, `SessionHeartbeat`.
   - Grok: `Notification` (`permission_prompt`, `idle_prompt`), `Stop`, `StopCancelled`, `StopFailure`.
   - Both call `juggler-agent hook <event>` from the managed home, mirroring D-40. Never return a decision.
   - Risk: Kimi hooks fail open and only live in config.toml. Grok's http hook type would need a localhost endpoint plus a token, so prefer `command`.
6. **ACP control plane as an optional "in-app session" mode.** (L, medium risk)
   - `kimi acp` and `grok agent stdio` give streamed output, in-app approvals, per-session `mcpServers` (and Grok's `_meta.pluginDirs`) with no file edits, and resumable sessions in the same store.
   - Risk: it duplicates the terminal UI. Kimi membership terms need a person in the loop, so no auto-approve on membership accounts. A session held by a TUI can't be shared (Kimi lock; Grok unknown).
7. **Kimi Primed or delta input through PTY paste.** (S–M, medium risk)
   - Bracketed paste, then CR, only when `wire.jsonl` shows idle and no open `interaction.request`.
   - Risk: keystrokes can land in a dialog. Never do it unattended on a membership account ("interactive use only").
8. **Kimi Desktop: detect, list, and open.** (S, low risk)
   - Detect `~/Library/Application Support/kimi-code-app/`, list Desktop sessions under the default-home account, and add "Open in Kimi Code Desktop" via `kimi-code://open?root=…&new=1`.
   - Risk: an undocumented scheme, which only opens a draft. No per-account isolation.
9. **Switching into Kimi: offer `kimi import-sessions`.** (S, low risk)
   - Kimi's own Claude Code/Codex importer honours `CLAUDE_CONFIG_DIR` and `CODEX_HOME`, so AI Juggler could launch it in the target account's home as a switch helper.
   - Risk: it is interactive, and it gives one summary message rather than a faithful copy.

**Not recommended:**
- Kimi `kimi web` for usage or prompts. It is experimental, reads a token file, and can't reach TUI-held sessions. That matches the existing maintainer ruling.
- Grok leader mode (internal protocol).
- Grok Bot (a cloud product).
- Any GUI scripting of either desktop app.
