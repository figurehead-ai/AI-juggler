# Claude: how AI Juggler can observe and control it (CLI and desktop app)

Research date: 2026-10-27. Every source was accessed on 2026-10-27.
Labels follow `docs/research/EVIDENCE-STANDARD.md`:
- **VP**: the vendor's own docs or binary say it.
- **VS**: a reliable second-hand source says it.
- **NE**: this search didn't find it.
- **I**: our own inference.
- **U**: couldn't verify.

**What was run, and what wasn't:**
- Run locally: `claude --version`, `--help`, `mcp --help`, `plugin --help`, `agents --help`. AI Juggler's own repo was read.
- Read from the Claude app's bundle: `Info.plist` only.
- Strings were read from the installed CLI binary, `~/.local/share/claude/versions/2.1.310`. That is the binary itself, not a profile. Where a claim rests on this it is labelled "VP (binary)": the strings are Anthropic's own code but undocumented, so they can change in any release.
- Nothing signed in or sent a prompt. No profile, credential or Keychain item was opened.

**What this extends:** `docs/research/session-runtime.md` (2026-10-25), `claude-code.md`, `switching-capabilities.md` and ADR-0022. Where those already settle something, it isn't repeated here.

---

## Versions

| Item | Value | Source | Label |
|---|---|---|---|
| CLI on this Mac | 2.1.310 | `claude --version` | VP |
| CLI on npm | `latest`/`next` 2.1.311 (2026-10-26); `stable` 2.1.308 | https://registry.npmjs.org/@anthropic-ai/claude-code | VP |
| CHANGELOG head | 2.1.311 | https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md | VP |
| Agent SDK | TS 0.3.311; Python `claude-agent-sdk` 0.2.174 | npm registry; https://pypi.org/pypi/claude-agent-sdk/json | VP |
| Desktop app on this Mac | 2.42304.2 (2026-10-19), bundle id `com.anthropic.claudefordesktop` | `/Applications/Claude.app/Contents/Info.plist` | VP |
| Desktop app, latest | v3.6556.2 (2026-10-22), "Bundled Claude Code version: 2.1.308" | https://claude.com/docs/cowork/changelog.md | VP |
| Code tab's own copy of Claude Code | "the Code tab runs its own copy of Claude Code … updating one doesn't update the other" | https://code.claude.com/docs/en/desktop.md | VP |

---

## Terms, rechecked 2026-10-27

The wording ADR-0022 relies on is unchanged.

**Agent SDK page.** VP, https://code.claude.com/docs/en/agent-sdk/overview.md:
> "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK."

**Legal page, *Authentication and credential use*.** VP, https://code.claude.com/docs/en/legal-and-compliance.md:
> "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens"

The same section also says:
> "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription"

**Other conditions on the same page:**
- "The Claude Code binary must not be modified."
- "Advertised usage limits for Pro and Max plans assume ordinary, individual usage of Claude Code and the Agent SDK."
- "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice."

**How the wording moved.** Session-runtime.md §2.1 has the history from Wayback (VS): strict on 2026-02-18, softened 2026-04, and the "unmodified binary" sentence added 2026-08-16..30.

**What this means for the new findings (I):**
- Hooks, plugins, mods, MCP, channels, monitors, the inbox socket and deep links are all extension points of the unmodified binary, which Anthropic documents for users to install. Using them leaves the binary unmodified.
- What stays grey is what ADR-0022 already says: AI Juggler as a product "offering rate limits" through `-p`.
- Driving the desktop app's chat through Accessibility would be "automated or non-human means" under the Consumer Terms (§3, quoted in claude-code.md §7). Avoid it.

---

## A. Claude Code CLI

### A1. Official SDK, API and protocol

| Claim | Source | Label |
|---|---|---|
| The `-p` + `--input-format/--output-format stream-json` control protocol is unchanged from session-runtime.md §3.1. The 2.1.310 help also lists `--permission-prompts host\|none`, `--replay-user-messages`, `--include-hook-events`, `--include-partial-messages`, `--forward-subagent-text`, `--prompt-suggestions` (emits a `prompt_suggestion` per turn), `--json-schema`, `--max-budget-usd`, `--brief` (SendUserMessage tool), `--restricted` and `--safe-mode` | `claude --help` 2.1.310 | VP |
| **PreToolUse `"defer"`**, new to our notes. In `-p` only, a hook can pause at a tool call (typically AskUserQuestion). The result ends with `stop_reason: "tool_deferred"` and a `deferred_tool_use {id,name,input}`. The host then resumes with `claude -p --resume <id>`, and the hook answers `allow` with `updatedInput`. There is "no timeout or retry limit". A host can keep a question open without keeping a process alive | https://code.claude.com/docs/en/hooks.md (Defer a tool call for later) | VP |
| Transcript stability, in the vendor's words: "The entry format is internal to Claude Code and changes between versions, so scripts that parse these files directly can break on any release. To build on session data, use `/export` or the script interfaces" (the latter are `-p` JSON, `-p --resume`, hooks' and the status line's `transcript_path`, and the SDK) | https://code.claude.com/docs/en/sessions.md | VP |
| **`claude agents --json`** is "the supported way to read session state from outside Claude Code". It lists live interactive and background sessions with `cwd, kind, startedAt, id, state (working\|blocked\|done\|failed\|stopped), pid, status (busy\|waiting\|idle), waitingFor ("permission prompt", "input needed", "sandbox request", "worker request", "dialog open"), sessionId, name`, and "does not require a TTY". The files under `~/.claude/jobs/` "are not a stable interface" | https://code.claude.com/docs/en/agent-view.md; `claude agents --help` | VP |
| Whether `claude agents --json` starts the supervisor. The docs say the supervisor starts "the first time you background a session or open agent view" | same | U |
| No programmatic way was found to answer a background session's permission prompt. A peek reply "doesn't answer it"; the user has to attach | agent-view.md (Peek and reply) | NE (agent-view, cli-reference and hooks pages read) |
| `claude --desktop [--continue \| --resume <id>]` (≥2.1.285) opens Desktop on the folder or session and exits. It needs a subscription sign-in. It "doesn't move a session that is open in another terminal or still running in the background" | https://code.claude.com/docs/en/cli-reference.md; desktop.md (Coming from the CLI?) | VP |
| `--remote-control` connects a local session to claude.ai/code and the mobile app. It is outbound HTTPS to Anthropic; nothing is local | https://code.claude.com/docs/en/remote-control.md | VP. That it gives AI Juggler nothing is I |
| `claude mcp serve` exposes Claude Code's tools as an MCP server. The client has to do the confirmation itself ("your own client is responsible for implementing user confirmation") | mcp.md (Use Claude Code as an MCP server) | VP |

### A2. Plugins

| Claim | Source | Label |
|---|---|---|
| **Plugin components:** skills, commands, agents, hooks, MCP servers, LSP servers, `bin/` executables, default `settings` (only `agent` and `subagentStatusLine` take effect), themes, output styles, **channels**, **monitors**, workflows and **mods** (`modules` in `hooks/hooks.json`). `--plugin-dir` (folder or .zip) and `--plugin-url` load one for one session. `CLAUDE_CODE_PLUGIN_DIRS` loads plugin dirs "for apps you can't pass a flag to" | https://code.claude.com/docs/en/plugins/components.md; plugins/mods/reference.md; `claude --help` | VP |
| **Monitors:** "a shell command that runs in the background for the whole session. What it prints reaches Claude as notifications". Interactive sessions only, not `-p` | plugins/components.md (Monitors); plugins/manifest-reference.md | VP |
| **Mods**, the biggest new surface. JS/TS handlers inside Claude Code, on by default from 2.1.287 (Desktop's copy from 2.1.286). A mod can: | https://code.claude.com/docs/en/plugins/mods/overview.md, …/mods/api.md, …/mods/events.md, …/mods/reference.md | VP |
| · draw panes and bands | same | VP |
| · hold, approve or deny a tool call (`tool.call`, `tool.check` → `{decision}`) | same | VP |
| · submit a prompt (`$.prompt.submit({text, asUser})`, which "waits until the session is idle and then starts a new turn") | same | VP |
| · abort a turn (`$.turn.abort`) | same | VP |
| · read the transcript (`$.session.messages()`) | same | VP |
| · read usage (`$.session.usage()` → `{context, rateLimits:[{kind, percentUsed, resetsAt}], cost}`) | same | VP |
| · send to other sessions (`$.session.send`) | same | VP |
| · run processes (`$.process.run/spawn`) and fetch over HTTP (`$.http.fetch`) | same | VP |
| **Where mods run:** CLI, the Desktop Code tab (not WSL), VS Code and `-p` | same | VP |
| **Mod limits:** "A mod can restyle much of Claude Code's interface, but not the permission prompt". Its own execution time is 10 s per event, excluding time inside `next` or mods API calls. `$.http.fetch` has a 30 s budget | same | VP |
| **Org controls on mods:** `allowManagedModsOnly` and `disableAllHooks` stop mods. With managed settings or a Team/Enterprise login, the built-in guard `sec-default` stops a mod's `allow` from getting past `ask` rules and PreToolUse hooks | same | VP |
| A mod that only reports state and carries the user's own click doesn't break "the binary must not be modified" | — | I |

### A3. MCP: what AI Juggler's own server can do in a CLI session

| Feature | What exists | Source | Label |
|---|---|---|---|
| Tools, resources, prompts | Resources as `@` mentions; prompts as `/mcp__server__prompt`; `list_changed` honoured (on the v2 runtime over a held stream) | https://code.claude.com/docs/en/mcp.md | VP |
| Roots | Claude Code answers `roots/list` | mcp.md | VP |
| Elicitation | Form and URL modes; declared as `{form:{},url:{}}` to user-configured servers from 2.1.287. Elicitation inside an MCP task is "served only in interactive terminal sessions". The `Elicitation` and `ElicitationResult` hooks can answer one | mcp.md; hooks.md | VP |
| Sampling | Not in the Claude Code docs. claude.com says "Claude doesn't yet support … Sampling" and resource subscriptions | mcp.md; https://claude.com/docs/connectors/building/index.md | NE (Claude Code) / VP (claude.com) |
| Progress and long calls | A progress notification resets the idle timeout (5 min HTTP, 30 min stdio, `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT`) but not the wall clock (per-server `timeout`, `MCP_TOOL_TIMEOUT`, about 28 h default). A call running past 2 min moves to a background task. The MCP tasks extension is supported | mcp.md | VP |
| Output limits | Warning above 10k tokens; 25k-token cap (`MAX_MCP_OUTPUT_TOKENS`); text over 50,000 chars is saved to a file unless the tool sets `_meta["anthropic/maxResultSizeChars"]`; a WebSocket message over 16 MiB is dropped | mcp.md | VP |
| Annotations | `readOnlyHint` (also runs in parallel), `idempotentHint`; `_meta["anthropic/requiresUserInteraction"]: true` forces a prompt on every call, and a hook can't skip it; `_meta["anthropic/alwaysLoad"]` | mcp.md; hooks.md | VP |
| **Push into the session: channels** | A server declares `capabilities.experimental['claude/channel']` and emits `notifications/claude/channel {content, meta}`. The message arrives as `<channel source=…>` in the running session. With `claude/channel/permission`, Claude Code also relays permission prompts (`notifications/claude/channel/permission_request {request_id, tool_name, description, input_preview}`). The server answers `notifications/claude/channel/permission {request_id, behavior}`, and "whichever answer arrives first" wins over the terminal dialog | https://code.claude.com/docs/en/channels.md; https://code.claude.com/docs/en/channels-reference.md | VP |
| Channels: the catches | All of these hold during the research preview: | same; mcp.md (Push messages with channels) | VP |
| · allowlist | Only plugins on Anthropic's allowlist register. A custom channel needs `--dangerously-load-development-channels server:<name>` with an interactive confirmation | same | VP |
| · not headless | The development flag "ignores" `-p` and the SDK | same | VP |
| · not listed | The flags aren't in `--help` | same | VP |
| · sign-in | Needs a claude.ai or Console sign-in; Team and Enterprise orgs must set `channelsEnabled` | same | VP |
| · protocol revision | On the v2 MCP runtime, a server that negotiates revision 2026-07-28 isn't registered as a channel. `MCP_PROTOCOL_NEGOTIATION=legacy` keeps the older one | same | VP |
| Registration | `claude mcp add -s local\|project\|user`; project `.mcp.json` (needs approval); `--mcp-config` + `--strict-mcp-config` per launch; plugin `.mcp.json`; `managed-mcp.json` | `claude mcp add --help`; mcp.md | VP |

### A4. Reading output and inserting input

| Claim | Source | Label |
|---|---|---|
| **Hook events in 2.1.310 (28):** SessionStart, Setup, InstructionsLoaded, UserPromptSubmit, UserPromptExpansion, **MessageDisplay**, PreToolUse, PermissionRequest, PermissionDenied, PostToolUse, PostToolUseFailure, PostToolBatch, Notification, SubagentStart/Stop, TaskCreated/Completed, Stop, StopFailure, TeammateIdle, ConfigChange, CwdChanged, DirectoryAdded, FileChanged, WorktreeCreate/Remove, Pre/PostCompact, Pre/PostModelSwitch, Elicitation, ElicitationResult, SessionEnd | https://code.claude.com/docs/en/hooks.md | VP |
| `MessageDisplay` gets the assistant's text as it streams (whole lines). It's display-only | hooks.md | VP |
| `UserPromptSubmit` also fires for cross-session messages and scheduled tasks | hooks.md | VP |
| Stop gets `last_assistant_message`, `background_tasks` and `session_crons` | hooks.md | VP |
| Notification types include `agent_needs_input`, `elicitation_dialog` and the `quota_auto_resume_*` types | hooks.md | VP |
| Hooks can block or rewrite: PreToolUse (allow/deny/ask/defer, `updatedInput`), PermissionRequest (allow/deny, `updatedPermissions`), UserPromptSubmit/UserPromptExpansion (block), PreModelSwitch (block), Elicitation (answer). Hooks can inject: `additionalContext`, SessionStart `initialUserMessage` (`-p` only), and an `asyncRewake` hook exiting 2, which "wakes Claude immediately even when the session is idle". Outputs are capped at 10,000 chars | hooks.md | VP |
| PermissionRequest decisions with a `--permission-prompt-tool` or SDK host: "whichever decides first applies" | hooks.md | VP |
| Whether a late PermissionRequest decision from a hook closes the TUI dialog. Lab 2 only showed the dialog drawn 53 ms before the hook started | hooks.md; session-runtime.md Lab 2 | U |
| **The cross-session inbox socket** (≥2.1.224). Each session (interactive, and `-p` unless `--bare`) binds a user-only Unix socket. The path is exported to hooks and Bash as `CLAUDE_CODE_MESSAGING_SOCKET` (also shown as `/status` "Peer address"), with a per-session `CLAUDE_CODE_MESSAGING_TOKEN`. "Read this section … when you want a script or hook to post into a session" | https://code.claude.com/docs/en/cross-session-messaging.md; env-vars.md | VP |
| Inbox socket behaviour: | same | VP |
| · when it's read | Between tool calls in a turn, or a new turn when idle | same | VP |
| · what it can't do | It can't approve anything, and slash commands in it don't run | same | VP |
| · inbound rule | `crossSessionInbound accept\|hold\|refuse`. With no rule, a prompting session delivers unless the sender says it is bypassing permissions | same | VP |
| · limits | A connection closes after 30 s without a full line; at most 100 held messages | same | VP |
| The socket's wire format isn't documented beyond the auth line `{"type":"auth","token":…}`. The binary's own debug hint shows the next line as a stream-json user message: `{"type":"user","message":{"role":"user","content":"hello"}}` | binary strings 2.1.310 | VP (binary), undocumented |
| **Status line JSON** (rate_limits for Pro/Max, cost, context, transcript_path). No change from claude-code.md §5 | https://code.claude.com/docs/en/statusline.md | VP |
| **OpenTelemetry:** metrics (session, tokens, cost, active time, …) and events (user_prompt, tool_result, tool_decision, api_request, api_error, permission_mode_changed, hook_execution_*, compaction, …), sent to any OTLP endpoint set by env. Content attributes are off unless `OTEL_LOG_*` turns them on | https://code.claude.com/docs/en/monitoring-usage.md | VP |
| **PTY, deterministic state:** Claude Code reports working, waiting on you and done through **OSC 7501 (Program Status Protocol)** to a terminal that answers the probe `OSC 7501 ; ?` (≥2.1.295). It doesn't report inside tmux, in background sessions, or with `CLAUDE_CODE_DISABLE_TERMINAL_TITLE=1` | https://code.claude.com/docs/en/terminal-config.md (See session status in your terminal) | VP |
| OSC 7501 payload: `state=…:app=claude-code:id=…:kind=…:progress=…:title=<b64>:msg=<b64>`. OSC 21337 (tab status) and OSC 133 (semantic prompt) also exist | binary strings 2.1.310 | VP (binary) |
| The protocol is Mitchell Hashimoto's proposal, implemented in Ghostty and Claude Code | https://zeli.app/story/49984159 | VS |
| PTY input: `Esc` interrupts and keeps the work done so far; Ctrl+C interrupts; messages typed while Claude works are queued; a paste over 800 chars or 3 lines collapses to `[Pasted text #N]`, and Claude is told pasted text may hold instructions the user didn't write. `--ax-screen-reader` gives flat text output | https://code.claude.com/docs/en/interactive-mode.md; terminal-config.md; `claude --help` | VP |
| Telling working, asking and idle apart from the screen alone needs a guess or a model. OSC 7501, hooks and `agents --json` make it exact | — | I |
| **Deep link `claude-cli://open?q=&cwd=&repo=`** opens a new terminal window with the prompt filled in and never sent (5,000 chars; marked "Prompt from an external link"). The handler `~/Applications/Claude Code URL Handler.app` is registered after the first interactive prompt; `disableDeepLinkRegistration` turns that off | https://code.claude.com/docs/en/deep-links.md | VP |

**What AI Juggler uses today** (repo, read-only, VP local):
- PTY terminal sessions with a per-launch `--settings` overlay. It holds hooks and a status line wrapper (`crates/juggler-adapters/src/claude_hooks.rs`).
- The driven stream-json runtime (`claude_stream/`).
- Pid files `sessions/<pid>.json` and the Claude app's `claude-code-sessions/…/local_<id>.json` (`claude_live.rs`).
- Transcript tails (`claude_asks.rs`, `claude_transcript.rs`).
- Transcript copy with `--resume`/fork (`claude_handover.rs`).
- The MCP bridge through `--mcp-config` (`claude_bridge.rs`).
- Its own plugin marketplace with an MCP server and a skill (`plugins/claude-code/`).
- Limits from local files.
- It never types into a terminal: messages to a terminal session are pulled by that session (`features/messages/views.rs`). No OSC 7501, OTel, channels, monitors, mods, inbox socket, `agents --json` or deep links are used.

---

## B. Claude desktop app (Chat, Code tab, Cowork)

### B1. SDK, API, URL scheme and IPC

| Claim | Source | Label |
|---|---|---|
| Registered schemes: `claude` and `msauth.com.anthropic.claudefordesktop` (Microsoft sign-in) | `Info.plist` | VP |
| No `OSAScriptingDefinition` or `.sdef`, so there is no AppleScript dictionary. `NSAppleEventsUsageDescription` is for Claude controlling *other* apps | `Info.plist`; listing of `Resources/` | VP (local) |
| **Documented `claude://` links** (support article updated 2026-06-30): | https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link | VP |
| · `claude://claude.ai/new?q=` | A new chat with the prompt filled in | same | VP |
| · `claude://claude.ai/chat/{conversation-id}` | Opens that chat | same | VP |
| · `claude://claude.ai/project/{id}` | Opens that project | same | VP |
| · `claude://code/new?q=\|prompt=&folder=&file=` | A new Code session. A folder confirmation always appears, and `file` "not currently supported" | same | VP |
| · `claude://cowork/new?q=&folder=&file=` | A new Cowork task. Folders are repeatable and each needs a confirmation | same | VP |
| · rules for all | Values are URL-encoded; `q` is cut at about 14,000 chars; the prompt is filled in and never sent | same | VP |
| SSH form: `claude://code/new?ssh_host=&ssh_port=&ssh_folder=&q=` (≥2.110.0), confirmed by the user | https://code.claude.com/docs/en/desktop.md | VP |
| **Undocumented link to a Code session:** `claude://resume?session=<cli-session-uuid>` (Desktop ≥1.1.9669) and `claude://code/new?folder=<path>`. The CLI builds these for `--desktop`, opens them with `open`, and checks the Desktop version through `defaults read …/Info.plist CFBundleShortVersionString`. `claude://claude.ai/<path>` opens claude.ai paths in Desktop | binary strings 2.1.310 | VP (binary), undocumented |
| What `resume?session=` does for a session that is already a Code-tab session | — | U |
| Desktop's Code tab can't be scripted: "`--print`, `--output-format`: Not available. Desktop is interactive only"; "Scripting and automation … Not available" | desktop.md | VP |
| No documented local socket or IPC from the Desktop app to other apps | desktop.md, the Desktop changelog, the support articles and the cowork/dispatch docs were searched | NE |
| Each Code-tab session runs the embedded Claude Code, so cross-session messaging covers it ("A session in the VS Code extension or the Desktop app can't show the dialog") | cross-session-messaging.md | VP |
| Code-tab sessions therefore have an inbox socket | — | I |
| Desktop's own "Work across sessions" sees only Desktop's own sessions | desktop.md | VP |
| Accessibility (AX) tree of the Electron window: whether it reliably exposes the conversation and composer | — | U |
| Using AX to type into or send from the app is automation of the Services and is off-limits under the Consumer Terms | — | I |
| Chat conversations are on Anthropic's servers. No documented local file or API holds a readable chat; reading the app's cache would be reverse-engineering and is off-limits | — | NE / I |
| Dispatch and Cowork have no third-party API | cc_cowork docs, dispatch guide | NE |

### B2. Extensions, connectors and plugins

| Claim | Source | Label |
|---|---|---|
| **Desktop Extensions `.mcpb`** (formerly `.dxt`, still accepted): a zip of a local stdio MCP server and `manifest.json`, installed by double-click, drag, or Settings > Extensions > Advanced > Install Extension. Node.js ships with Desktop. "the directory no longer accepts MCPB submissions"; local servers go out inside plugins | https://claude.com/docs/connectors/building/mcpb.md; https://support.claude.com/en/articles/10949351 | VP |
| `claude_desktop_config.json` servers load into **Chat and the Code tab**; on a name clash the Code tab uses the Desktop definition | desktop.md (MCP servers from the Claude Desktop chat app) | VP |
| Remote custom connectors go through Anthropic's cloud | claude.com connectors docs | VP |
| So a local AI Juggler server would have to be stdio or `.mcpb` | — | I |
| **Plugin support by surface:** Chat loads skills and commands (as skills) and ignores hooks, agents and local MCP. **Cowork** loads skills, commands, agents, **hooks**, and local MCP when it runs on the computer. **The Code tab = Claude Code** (everything, including `bin/`, LSP, settings) | https://claude.com/docs/plugins/platform-support.md | VP |
| **Mods run in the Code tab** (≥2.1.286), except WSL | plugins/mods/overview.md | VP |
| The Code tab shares `~/.claude/settings.json` hooks, skills, `.mcp.json` and `~/.claude.json` with the CLI. There is no per-launch flag in Desktop | desktop.md (Shared configuration; CLI flag equivalents) | VP |
| So AI Juggler's per-launch `--settings` overlay can't reach the Code tab. Only the user's own settings or an installed plugin can, each a write the user has to approve | — | I |

### B3. MCP inside the desktop app

| Claim | Source | Label |
|---|---|---|
| Chat and Desktop limits: about 150,000 chars per tool result; 240 s per call | https://claude.com/docs/connectors/building/index.md | VP |
| Not supported: sampling, resource subscriptions and "advanced or draft capabilities" | same | VP |
| MCP Apps (`ui://` UI) render in Desktop and the Code tab | claude.com MCP Apps docs; Desktop changelog | VP |
| Whether Chat supports elicitation | claude.com docs and Desktop changelog searched | NE |
| Cowork honours `requiresUserInteraction` (asks on the in-app card) | Desktop changelog v3.6556.1 | VP |
| Channels in the Code tab | — | U (the Desktop changelog mentions `<channel-message>` turns) |

### B4. Reading and inserting in the desktop app

| Claim | Source | Label |
|---|---|---|
| Reading a Code-tab session: its transcript is the embedded CLI's `projects/<cwd>/<id>.jsonl` (internal format), and AI Juggler already reads the app's undocumented `claude-code-sessions/<acct>/<org>/local_<id>.json` (status word, `cliSessionId`) | sessions.md; `crates/juggler-adapters/src/claude_live.rs` | VP / VP (local) |
| A mod or hooks installed by plugin would give the Code tab the same exact state, limits (`$.session.usage()`) and input (`$.prompt.submit`) as the CLI | mods docs | I, built on VP |
| Inserting: URL links only fill the box and the user sends. In the Code tab, mods and the inbox socket can start a turn | above | VP / I |
| Chat: there is no sanctioned way to insert a message beyond a prefilled `claude://claude.ai/new?q=` | — | NE |

---

## C. What AI Juggler could do that it doesn't today, best first

1. **Answer from AI Juggler in terminal sessions, through the vendor's own relay** (permission relay through a channel).
   - **How:** AI Juggler's bridge MCP server declares `claude/channel` + `claude/channel/permission`. It shows Claude Code's own permission prompt in Needs you and returns the user's click as `allow`/`deny`. The TUI dialog stays live, and the first answer wins. This is exactly ADR-0022's "answers are the user's" and closes its accepted gap ("Claude terminal sessions still can't be answered from AI Juggler").
   - **Risk:** research preview. A custom channel needs `--dangerously-load-development-channels server:ai-juggler`, which asks for a confirmation at every launch, and the flag may change. It needs `MCP_PROTOCOL_NEGOTIATION=legacy` for that server, and doesn't work in `-p`.
   - **Terms:** it is a documented user extension point, so terms are fine.
   - **Fix for the friction:** ask Anthropic to put AI Juggler on the channel allowlist (the preview names its allowlist). Until then, lab only.
2. **Exact state from the PTY itself: OSC 7501.**
   - **How:** AI Juggler's embedded terminal answers the `OSC 7501 ; ?` probe and parses `state=…` sequences. That gives working, blocked (with kind) and done for every Claude terminal session it hosts, with no hooks and no file reads.
   - **Risk:** low. It is our own terminal, read-only and documented. The payload format comes from the binary, so pin a parser per version and keep the hook path as a cross-check.
3. **`claude agents --json` as a supported state source,** in place of parsing pid files and `jobs/`. It gives state, status and `waitingFor` for every live session, including terminal ones AI Juggler didn't start.
   - **Risk:** low.
   - **Unknown:** whether it starts the supervisor. Check on a lab profile first (ADR-0022 point 7).
4. **An AI Juggler mod in its existing plugin.** This is the only route that reaches the Desktop Code tab. The mod would:
   - report exact state and `$.session.usage()` limits (both windows, with `resetsAt`) to `juggler-agent` through `$.process.spawn`;
   - deliver pool messages between turns with `$.prompt.submit` (not `asUser`, so Claude sees AI Juggler as the sender);
   - offer Stop through `$.turn.abort`.

   It works in the CLI, the Desktop Code tab and VS Code.
   - **Risk:** mods run with full user permissions and can approve tool calls, so the mod must never answer a permission by itself (D-28, ADR-0022 point 5).
   - **Consent:** installing it into a profile is a settings write that needs the user's consent.
   - **Org settings:** `allowManagedModsOnly` and `sec-default` can block it.
5. **Message delivery for terminal sessions without typing: the inbox socket.**
   - **How:** the SessionStart hook already runs `juggler-agent`, which can report `CLAUDE_CODE_MESSAGING_SOCKET`. AI Juggler then posts the user's approved message to that socket between turns.
   - **Risk:** only the auth line is documented. The message line shape is from the binary's debug hint, so it needs a lab check per version. It is held or refused under the receiver's `crossSessionInbound`. A message can't approve anything, which is good.
6. **Plugin monitors** as a lighter push. A monitor runs `juggler-agent watch <session>` and prints pool messages, which reach Claude as notifications.
   - **Risk:** interactive sessions only. Whether a monitor line wakes an idle session is U; check in the lab.
7. **PreToolUse `defer` for driven sessions.** A driven session can end its process while a question waits: AI Juggler shows AskUserQuestion, then resumes with the answer. This helps the "comes back by itself" rule.
   - **Risk:** low; documented. Still `-p`, with ADR-0022's terms caveats.
8. **Go to it for desktop sessions.**
   - Use the documented `claude://claude.ai/chat/<id>` and `claude://code/new?folder=` links.
   - Use `claude --desktop --resume <id>` to move a terminal session into Desktop.
   - `claude://resume?session=<id>` (undocumented) needs a check before it's used.
   - **Risk:** low. The user always confirms, and links never send.
9. **Per-launch OTel to a local receiver.** For sessions AI Juggler starts, export OTLP to a loopback endpoint AI Juggler owns. That gives exact per-request tokens and cost, tool decisions and API errors, with content logging left off.
   - **Risk:** low.
   - **Cost:** another listener to secure, with the same peer rules as ADR-0020.
10. **A `claude-cli://open` link** for "Start in my own terminal". The user's terminal opens with AI Juggler's prompt filled in, and the user presses Enter.

**Not recommended:**
- Accessibility automation of the Desktop chat or composer (terms: automated access; fragile).
- Reading Desktop's chat caches.
- Remote Control as an API (it goes through Anthropic, with no local API).
- Bundling the Agent SDK (ADR-0022).
- Auto-answering through hooks, mods or channels (D-28).
