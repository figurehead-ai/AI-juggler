# ZCode and Gemini CLI: control surfaces (research 2026-10-27)

All sources accessed 2026-10-27. Labels: VP verified-primary, VS verified-secondary, NE negative-evidence, I inferred, U unknown. Nothing signed in, no prompt sent, no tool database or credential opened. ZCode `--help`/`--version` ran with `HOME=<scratchpad>` so the real `~/.zcode` was not touched.

## (a) Findings

### What changed since the earlier research (zcode.md 2026-10-12/15, gemini.md 2026-10-16, antigravity.md / E16 2026-10-21)

**ZCode**
1. **ZCode is now open source.** [VP] `github.com/zai-org/ZCode` is Apache-2.0, created 2026-09-20, last synced 2026-10-20 (root 3.16.2, `apps/zcode-cli` 0.17.0). It ships Chinese CLI usage docs (`apps/zcode-cli/docs/usage/*`, checked against source on 2026-10-14) and design docs. This retires zcode.md "Key unknown 1" (no public CLI docs). The CLI surface no longer has to come from binary strings.
2. **Versions.** [VP] The installed app is 3.15.3 (build 3.15.3.8237) with engine 0.16.9. The latest desktop is 3.16.3 (2026-10-23). No separately published CLI binary was found. The source can build a `zcode` distribution (`install.sh` into `~/.zcode/runtime`, which adds `--web`), but its download URL is a placeholder.
3. **Headless output.** [VP] `--output-format text|json|stream-json` (NDJSON ending in a `type:"result"` line) is documented. The installed 0.16.9 build contains the string, but its help lists only `--json`, so it is unverified there. Other new or now-documented flags: `--mode guarded`, `--workflow-mode`, `--target-replace`, `--memory-bench`, `zcode hooks trust ...`.
4. **The database is not an API.** [VP] Vendor wording: the database structure "is an implementation detail". Use read-only connections and the SQLite backup API. The 3.16.3 changelog fixes "data corruption from multiple sessions writing at once". SPEC §4.3 relies on SQLite tables, so that reliance is now a stated vendor risk. The DB path can be moved with `storage.sessionDbPath` or `ZCODE_SESSION_DB_PATH`.
5. **Isolation recipe is now vendor-documented.** [VP] Set `HOME`, `USERPROFILE` and `ZCODE_DATA_BASE_DIR` per run, with a whitelisted env. "Only changing sessionId or ZCODE_STORAGE_DIR is not enough." This matches the earlier B4 experiment.
6. **app-server documented.** [VP] Its methods are documented (see A2 below). The vendor calls it the desktop's private agent server.
7. **MCP features.** [VP] MCP elicitation (form) is supported. MCP Apps (UI resources) arrived in 3.15.1. Workspace MCP servers now auto-connect with no trust prompt, a security note. Plugins also read `.claude-plugin/` and `.codex-plugin/`.
8. **Terms updated 2026-06-15.** [VP] §6.2: non-commercial, personal study/research use unless you have a paid service or another agreement. §3.2, §3.3 and §4.3 are unchanged in substance.
9. **Desktop deep link.** [VP] `zcode://workspace/open?path=` exists in the installed build, and a Finder service is installed.
10. **Community account switchers.** [VS] Tools such as `pjpv/zcode-switch` (464 stars) swap ZCode logins. This shows demand. Each one is a credential-handling and terms risk we will not copy.

**Gemini CLI**
1. **Versions.** [VP] Latest stable is 0.65.0 (2026-10-20; earlier research had 0.64.0). Preview is 0.66.0-preview.0 and nightly 0.67.0-nightly.20261027. The repo is active and not archived.
2. **Sunset re-confirmed, banner text changed.** [VP] The geminicli.com banner now reads "Unpaid tier and Google One users: Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." and adds "Gemini CLI has not been optimized for Gemini 4 Argon". The blog (2026-05-19) says Code Assist Standard/Enterprise is unchanged and paid Gemini / Gemini Enterprise Agent Platform API keys stay. The repo docs still list consumer tiers, which is stale (contradiction recorded). [U] The free API-key tier after the sunset is unclear.
3. **ACP flag.** [VP] `--acp` is now the flag; `--experimental-acp` is a deprecated alias in `config.ts`. cli-reference.md lags.
4. **Transcripts are documented.** [VP] They are append-only JSONL `session-<ts>-<id>.jsonl` under `~/.gemini/tmp/<project_hash>/chats/`, with token fields. Earlier research called the format "community-documented". New flags: `--list-sessions` and `--delete-session`. Default retention is 30 days.
5. **No MCP elicitation.** [VP] PR #28089 closed unmerged and issue #28074 closed not_planned.
6. **SDK and A2A.** [VP] `@google/gemini-cli-sdk` is in the repo but not on npm. `@google/gemini-cli-a2a-server` 0.65.0 is on npm and marked experimental.
7. **Gemini app for Mac.** [VP] It exists (2026-04-15) but has no coding or developer integration.

### ZCode — A: CLI

**1. SDK / API / protocol**
- [VP] **Headless.** `zcode -p "<task>" --cwd <abs> --mode <m> --json` or `--output-format json|stream-json`. The JSON result is `{sessionId, traceId, response, eventCount, projection, usage}`. stdout carries business output and stderr carries diagnostics; errors exit with 1. The default mode for `-p` is **yolo**, so always pass `--mode`. (https://github.com/zai-org/ZCode/blob/main/apps/zcode-cli/docs/usage/02-cli-reference.md)
- [VP] **app-server (ZCode Protocol).** Start it with `zcode app-server --stdio` (alias `agent-server`). It speaks strict NDJSON. Methods:
  - `session/{create,resume,list,read,messages,events,subscribe,send,stop,setModel,setThoughtLevel,setMode,close}`
  - `workspace/readState`
  - `process/childProcesses`, which returns live MCP child pids (useful for our cleanup ledger)
  - server-to-client `interaction/requestPermission` and `interaction/requestUserInput`

  The vendor describes it as the private desktop integration; v4 schemas live in `packages/shared`. (https://github.com/zai-org/ZCode/blob/main/apps/zcode-cli/docs/design/v2/zcode-protocol-agent-server.md)
- [NE] **No ACP, no SDK.** Searched the repo tree for acp/agent-client, `--help`, the changelog and the web. `@zcode/cli` is `private:true`.
- [VP] **Terms.** The GLM Coding Plan is "only within officially supported tools", with no account sharing (https://docs.z.ai/devpack/usage-policy). ZCode itself is a supported tool. The ZCode agreement bans "automated means" (scraping-oriented wording), reverse engineering, improper multi-accounts, and non-personal/commercial use without a paid plan (https://zcode.z.ai/cn/terms, updated 2026-06-15).
  - [I] Launching the official binary with an isolated HOME stays inside the vendor's own documented "external task launcher" pattern (usage/13-automation.md). A maintainer ruling is still advisable.

**2. Plugins and extensions** [VP] (vendor-shipped `zcode-guide-plugin` skills inside the app bundle)
- Plugin manifest: `.zcode-plugin/plugin.json`. Components are commands, skills, hooks, mcpServers and agents. `channels` is recorded but not executed.
- Marketplaces come from GitHub, git, a local dir or a file.
- Skills and commands live under `~/.zcode|~/.agents|<repo>/.zcode|<repo>/.agents`.
- Instructions: `AGENTS.md`.
- Hooks have 7 events: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Stop.
  - Config-file hooks need `hooks.enabled:true`; plugin hooks auto-enable the runner.
  - A hook can return `additionalContext` and allow/ask/deny decisions.

**3. MCP — what our server can do** [VP]
- **Config:** `mcp.servers` in `<home>/.zcode/cli/config.json` or the workspace `.zcode/config.json` (fallback `.agents/mcp.json`). Transports are stdio, http and sse, with OAuth for http/sse.
  - The schema is strict: an unknown key drops the server silently.
  - `${}` templates expand only for plugin servers.
  - Default timeout is 30 s.
  - `protocolVersion` accepts auto, legacy or 2026-07-28.
- **Tools:** named `mcp__<server>__<tool>`; `tools/list_changed` is honoured.
- **Resources:** list/read/subscribe, used mainly for MCP Apps UI. Logging notifications are kept.
- **Elicitation:** form mode, mapped to the user-question UI.
- **Prompts:** not consumed (no listPrompts in the client).
- **Push into session:** none. The substitute is pull through a hook's additionalContext [I].

**4. Output and input**
- [VP] **Store.** SQLite (`session`/`message`/`part` + `model_usage`). Logs are daily JSONL. `rollout/model-io-*.jsonl` is bounded (64 MiB, last 3 sessions) and sensitive.
- [VP] **Resume.** `--resume sess_…` or `-c` (never use `-c` with concurrent sessions). TUI `/fork` and `/rewind` work on workspace checkpoints.
- [VP] **Input.** app-server `session/send`, or a headless `-p --resume`.
- [I] **PTY.** Writing to an owned PTY works, but it is unsupported.
- **What is deterministic vs. needs a model.**
  - Deterministic: process, exit code, DB rows, hooks, protocol events.
  - Needs heuristics: TUI screen state.

### ZCode — B: Desktop (ZCode.app)
- [VP] **Deep links.** `zcode://workspace/open?path=<enc abs dir>`, plus oauth/payment callbacks and `share/import?code=`. There is no session or prompt route. (https://github.com/zai-org/ZCode/blob/main/docs/desktop/finder-open-folder-deep-link.md; installed app.asar contains `OpenWorkspacePath`.)
- [VP] **No external agent channel.** The desktop spawns its own private stdio agent server.
- [I] **Desktop sessions** land in the same `~/.zcode/cli/db/db.sqlite`, so they can be observed read-only.
- [NE] **No supported input path.** Remote Control (phone) and Bot Channel (WeChat/Feishu/Telegram) are vendor cloud relays.
- [VP] **MCP, plugins and hooks** behave as in the CLI, managed in Settings. Desktop-managed MCP lists can override the file.
- **Terms:** the same as the CLI. UI automation would fall under "automated means" risk.

### Gemini CLI — A: CLI
- [VP] **Eligibility first.** Consumer Google login (free Code Assist, AI Pro, AI Ultra) has not been served since 2026-06-18; that path is Antigravity CLI, covered by antigravity.md. Remaining paths: Code Assist Standard/Enterprise, paid API key, and Vertex / Agent Platform keys.
- **1. Protocols.**
  - [VP] Headless `-p` with `--output-format json|stream-json`. Exit codes: 0, 1, 42, 53.
  - [VP] **ACP** via `gemini --acp`, JSON-RPC over stdio:
    - Methods: initialize (the client can register its MCP server), authenticate, newSession, loadSession, prompt, cancel, setSessionMode, unstable_setSessionModel.
    - Permission requests go to the client; a file-system proxy runs through the client.
    - Gemini CLI is listed in the ACP registry.
  - [VP] **SDK:** none published.
  - [VP] **A2A server:** published on npm, experimental.
  - [VP] **Terms.** tos-privacy.md: "Directly accessing the services powering Gemini CLI … using third-party software … is a violation." We may only launch the official binary and must never reuse its OAuth.
- **2. Extensions** [VP]
  - `gemini-extension.json` (mcpServers, contextFileName, excludeTools, settings) plus `commands/*.toml`, hooks and skills.
  - Custom TOML commands.
  - Hierarchical `GEMINI.md`.
  - Hooks events: BeforeTool, AfterTool, BeforeAgent, AfterAgent, BeforeModel, BeforeToolSelection, AfterModel, SessionStart, SessionEnd, Notification, PreCompress.
  - Hook input includes `session_id` and `transcript_path`.
- **3. MCP**
  - [VP] **Config:** settings.json `mcpServers`, with trust, include/exclude tools, timeout (600 s default) and OAuth.
  - [VP] **Prompts** become slash commands.
  - [VP] **Resources:** `@server://uri` plus list/read tools.
  - [VP] **List changes:** tools, resources and prompts `list_changed` are all handled.
  - [VP] **Progress** notifications are handled; the client offers roots.
  - [VP] **Elicitation:** unavailable.
  - **Push into a session:**
    - [VP] There is no prompt push. The IDE-companion `ide/contextUpdate` sends context only (open files and selection) through a discovery file in `os.tmpdir()/gemini/ide/`, `GEMINI_CLI_IDE_PID` or `GEMINI_CLI_IDE_SERVER_PORT`.
    - [I] AI Juggler could act as such an "IDE". This is untested.
- **4. Output and input**
  - [VP] **Transcripts:** append-only JSONL with `$set`, `$patch` and `$rewindTo` records and per-message token fields.
  - [VP] **Session commands:** `--resume latest|N|uuid`, `--list-sessions`, `--delete-session`.
  - [VP] **Checkpointing** (off by default) uses a shadow git in `~/.gemini/history/<hash>`.
  - [VP] **Isolation:** `GEMINI_CLI_HOME`.
  - **Input:**
    - [VP] ACP `prompt`, or a headless `-r <id> -p`.
    - [I] PTY writes for sessions AI Juggler owns.

### Gemini CLI — B: Desktop
- [NE] **Gemini app for Mac** (2026-04-15; v1.72.2.419 reported 2026-06-12): no coding agent, CLI link, MCP, URL scheme or API was found.
- Gemini CLI has no desktop app. It reaches desktops only as an IDE terminal companion (VS Code family including Antigravity, JetBrains, Zed, Xcode, Sublime) or as an ACP agent in Zed/JetBrains.
- The consumer desktop coding product is the Antigravity App, which is out of scope here (antigravity.md).

## (c) What each adapter should use first (ranked)

### ZCode
**CLI (AJ-E1605)**
1. **Launch:** the bundled engine `node …/glm/zcode.cjs` with a per-account `HOME`, `USERPROFILE` and `ZCODE_DATA_BASE_DIR`, and a whitelisted env.
   - Interactive sessions run the TUI in a PTY.
   - Detect the engine by path plus `--version`, and pin to the app version.
2. **Observe and control sessions AI Juggler hosts:** `app-server --stdio`, using `session/list|read|messages|events|subscribe|send|stop`, `interaction/*` and `process/childProcesses`.
   - Risk: the protocol is private and unversioned. Schema-validate every frame and gate on the engine version.
   - Fallback: `-p --output-format stream-json --resume`.
3. **Observe any session:** read-only SQLite (`mode=ro`, WAL-aware, gated on `schema_migration`).
   - Treat it as best-effort because the vendor calls it an implementation detail.
   - Never write; the 3.16.3 corruption fix shows concurrent writers are a real failure.
4. **Status events:** an opt-in AI Juggler plugin from a local marketplace.
   - Hooks: SessionStart, UserPromptSubmit, PermissionRequest, Stop.
   - Each hook calls `juggler-agent` and installs only in the managed home, after user approval.
5. **Our MCP server** in the managed home's `mcp.servers`, for coordination tools. Elicitation is available for questions to the user. There is no push.
6. **Usage:** `model_usage` rows plus the published credit formula (unchanged). The per-run JSON `usage` field is a cross-check.
- **Terms and risk:**
  - ZCode is an official Coding Plan tool, so the plan stays inside it, and the vendor documents external launchers.
  - Open terms issues: the §4.3 "automated means" clause, the §6.2 non-commercial clause, and the "improper multi-account" grey zone.
  - Record a maintainer ruling. No credential reads, no account rotation.

**Desktop**
1. Observe only: process detection (`dev.zcode.app` with its `zcode.cjs app-server` child), read-only DB and logs.
2. "Go to it": activate the app, or `zcode://workspace/open?path=` for a project. No session-level deep link exists.
3. No input insertion. Offer "continue in CLI" (`--resume`) only after the desktop session is idle, since there is a concurrent-writer risk.
4. MCP and plugin install into the real `~/.zcode` only with explicit approval, and only while the desktop is closed or followed by a restart.

### Gemini CLI
**CLI**
0. **Gate on auth type.** Full adapter only for Code Assist Standard/Enterprise, paid API key and Vertex/Agent Platform. Consumer Google accounts are track-only and point to Antigravity, whose FAQ bans wrapping and rotation on consumer plans.
1. **Launch** the official `gemini` binary with `GEMINI_CLI_HOME` per account. The key or Vertex env goes only to that child (Keychain-held, D-34). Interactive sessions run in a PTY.
2. **Observe:** tail the JSONL transcript in `<home>/.gemini/tmp/<hash>/chats/`, with tolerant parsing. In the managed home, set `general.sessionRetention` so history is not deleted after 30 days.
3. **Status:** an AI Juggler extension in the managed home with hooks — Notification/ToolPermission means waiting for approval, AfterAgent means turn done, plus SessionStart and SessionEnd.
4. **Managed or background sessions:** `gemini --acp` (newSession, loadSession, prompt, cancel, setSessionMode), passing our MCP server in initialize. The fallback is `-p --output-format stream-json -r <id>`.
5. **Our MCP server:** tools, prompts as slash commands, and resources. There is no elicitation, so questions to the user go through our own UI. The IDE-companion context channel is optional and experimental.
- **Terms and risk:** never reuse Gemini OAuth outside the binary. Cloud and API terms apply to the remaining tiers. The doc and website disagree on tiers, so re-verify before UI copy.

**Desktop:** unavailable. No adapter. Track the Gemini Mac app as "not a coding tool". Desktop coding belongs to an Antigravity adapter.
