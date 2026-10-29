# Hermes Agent and Muse Code: control surfaces (research 2026-10-27)

All sources accessed 2026-10-27. Labels: VP verified-primary, VS verified-secondary, NE negative-evidence (not found by this search), I inferred, U unknown.
Nothing was signed in, no prompt was sent, no credential, Keychain or process environment was read. The local `muse` 1.4.6 was run with `--help`/`--version` only; Muse's data folders were listed by name only (no file contents). Hermes is not installed here.

Short URLs: `HD/` = https://github.com/NousResearch/hermes-agent/blob/main/website/docs/ · `HS/` = https://github.com/NousResearch/hermes-agent/blob/main/ (source) · `MD/` = https://dev.meta.ai/docs/muse-code/ · `SDK/` = https://github.com/meta-models/muse-code-sdk/blob/HEAD/

## Identification

- **Hermes = Nous Research Hermes Agent**, repo `NousResearch/hermes-agent`, MIT, ~257.5k stars, binary `hermes`; latest **v0.22.5 (published 2026-10-23)**, after v0.22.4 (2026-10-21). VP: GitHub API releases. Hermes IDE (hermes-ide.com) is a separate competitor (unchanged from 2026-10-21).
- **Muse = Meta's Muse Code**, closed-source binary `muse`; SDK repo `meta-models/muse-code-sdk` (MIT). Latest **1.4.13** (npm `@muse-code/sdk` 2026-10-24, crates.io `muse-code-sdk` 1.4.13, Homebrew cask `muse-code` 1.4.13-R6996.1). Installed here: **1.4.6 (R6000.1)** at `~/.local/bin/muse` (a bash launcher plus `muse-bin-1.4.6-R6000.1`). VP.

## Changes since the 2026-10-21 research

Hermes
- Two releases (0.22.4, 0.22.5). **`hermes import-agent` and `hermes mcp serve` left core** and are now the plugins `agent-importers` and `mcp-serve`. VP: HD/user-guide/import-from-other-agents.md, release notes v0.22.4/v0.22.5.
- **Correction:** `import-agent` never reads credential files (`~/.codex/auth.json`, `~/.claude/.credentials.json`). The credential risk is in a different place: **`auth.adopt_external_logins` (default `true`)** lets Hermes borrow and *refresh* the Codex CLI's `auth.json` and Claude Code's credentials (file or Keychain item `Claude Code-credentials`). Since #98334 Hermes also **writes the refreshed tokens back into that Keychain item**. VP: HD/user-guide/security.md#borrowed-cli-logins; HS/agent/anthropic_credentials.py; HS/hermes_cli/auth_codex.py.
- New: **`hermes sessions import --from claude|codex`** reads other tools' transcripts (read-only, honouring `CLAUDE_CONFIG_DIR`/`CODEX_HOME`). VP: HS/hermes_cli/foreign_sessions.py.
- **The multiplexing gateway is now the default.** One host gateway serves every profile under `~/.hermes/profiles/`; a `HERMES_HOME` outside `profiles/` is a "real boundary" with its own gateway. ACP now runs as a *viewer* of the gateway. VP: HD/developer-guide/multiplexing-gateway.md, HD/user-guide/multi-profile-gateways.md, HD/user-guide/features/acp.md.
- MCP: elicitation (form mode), experimental MCP events (webhook mode), progress notifications. VP: HD/user-guide/features/mcp.md, v0.22.3/0.22.5 notes.

Muse
- 1.4.12 and 1.4.13 shipped. New in them: **`muse usage`** (1.4.13), **a Rust SDK `muse-code-sdk`** (created on crates.io 2026-10-19) and a Python SDK `muse-code-sdk`, and `muse serve --preset`. The changelog now mentions **"Desktop and other connected clients"** (1.4.13). VP: MD/changelog, crates.io, SDK/rust/clients/sdk-rs/README.md.
- Found in the binary but not in the docs: **`muse serve --listen [unix:<PATH>]`**, a multi-client socket daemon; **`muse session-message send|list`**; `--provider echo` (a deterministic echo provider); `muse export`; `muse trace inspect`; `--no-foreign-personal-context`. VP: local `--help` output, 1.4.6.
- **Correction to "credentials only in Keychain":** a file named `~/.config/muse/auth.json` (317 bytes, mode 600) exists. Its contents were not read. I.
- The session store has been seen: `~/.local/share/muse/sessions/YYYY/MM/DD/<uuid>/session.jsonl`, plus `session-index.db` and `~/Library/Application Support/muse/session-name-authority/`. I, from the names-only listing.

---

## (a) Findings

### Hermes, CLI layer

**1. SDK, API and protocols**
- **Headless:** `hermes chat --oneshot -q … --format stream-json` writes JSONL events (`system`, `text`, `tool_use`, `tool_result`, `result` with tokens, session_id, exit_code). `hermes -z … --usage-file` writes the final text plus a JSON usage report that includes auxiliary calls. `--source tool` hides these runs from the user's session pickers. **Pitfall:** since 0.21, `-q` on a real TTY *seeds an interactive session* instead of exiting, so always pass `--oneshot`. VP: HD/reference/cli-commands.md.
- **Three protocols run the same core** (VP: HD/developer-guide/programmatic-integration.md):
  - **ACP** (`hermes acp`, stdio). Covers session new/load/resume/fork/list/cancel and permission requests. It needs the `acp` extra installed. VP: HD/user-guide/features/acp.md, acp-internals.md.
  - **TUI-gateway JSON-RPC** (stdio or WebSocket). Covers session.create/list/resume/history/status/usage/steer/interrupt, prompt.submit, command.dispatch, streamed events, and server→client `approval`/`clarify` requests. A client must call `client.capabilities {server_requests:true}`, which was a breaking change for WebSocket clients.
  - **OpenAI-compatible API server** (`127.0.0.1:8642/v1`). Supports runs with SSE events, approval, steer and stop. It needs `API_SERVER_KEY`.
- **Gateway:** the profile's gateway is the **only writer of sessions**. `hermes gateway ensure --json` attaches to it or starts it without installing a service. VP: HD/developer-guide/session-storage.md, cli-commands.md. A third-party local client authenticates through the **same-user private control socket**: a `session-ticket` request returns a one-use ticket that expires in 30 s. This is documented only in developer internals. VP for the mechanism (HD/developer-guide/gateway-internals.md); I that it is stable enough for us.
- **SDK:** no client SDK package. The only embed is in-process Python (`run_agent.AIAgent`). VP.
- **Usage:** `hermes usage --json` (stable keys). VP. Codex/Anthropic windows from it are Hermes-reported only (I, per codex.md §9).
- **URL scheme:** none for the CLI. VP (Desktop only, below).
- **Terms:** the code is MIT. Nous Portal ToS (last update 2026-09-28) §5.1 says "Each Client may have only one Account". §10.2(3) bars programs that send automatic requests unless Nous made the program available. §10.1(7) bars circumventing limits. VP: https://portal.nousresearch.com/terms. That supervising Hermes itself is acceptable is I. Rotating Portal accounts would not be acceptable (I).

**2. Plugins, skills, hooks and rules (VP)**
- **Plugins:** Python, `hermes plugins …`, an official catalog, portable packages that can ship MCP servers, and `hermes://plugin/install`. Source: HD/user-guide/features/plugins.md.
- **Skills:** `$HERMES_HOME/skills`. Source: features/skills.md.
- **Hooks:** shell hooks in `config.yaml` take JSON on stdin and return JSON on stdout; exit code 2 blocks, which is Claude-Code compatible. Events include pre/post_tool_call, pre_llm_call (inject context), session start/end, subagent and approval events. Each (event, command) pair gets a first-use consent prompt. Gateway hooks live in `~/.hermes/hooks`. Source: features/hooks.md.
- **Rules:** `.hermes.md` › `AGENTS.override.md` › `AGENTS.md` › `CLAUDE.md` › `.cursorrules`, plus `SOUL.md`. Source: features/context-files.md.

**3. What our own MCP server can do (VP: HD/user-guide/features/mcp.md, HD/reference/mcp-config-reference.md)**
- **Config:** `mcp_servers` in `$HERMES_HOME/config.yaml` (stdio or HTTP, OAuth 2.1), or per ACP session through `session/new` `mcpServers`.
- **Supported:** tools; resource and prompt wrappers when the server offers them; `tools/list_changed` triggers a live refresh; progress notifications are shown as tool progress; per-server `trust`, `tool_costs`, `spend_limit` and include/exclude lists.
- **Elicitation:** form mode works, routed through Hermes' approval surface (default on, 300 s timeout). URL mode is declined.
- **Sampling:** on by default, with rate, timeout and token caps.
- **Push:** only the experimental **MCP events** feature. Each event *starts a new run* through the gateway's webhook listener, which needs a public HTTPS URL. There is no injection into an existing live session.
- **Limits:** `prompts/list_changed` and `resources/list_changed` are ignored. The MCP env is stripped to safe vars plus the configured `env`. Hermes can also *be* an MCP server through the `mcp-serve` plugin, which is messaging-oriented.

**4. Reading output and inserting input**
- **Deterministic reads:**
  - `$HERMES_HOME/state.db`: SQLite in WAL mode, schema v31. The sessions table carries tokens, cost, cwd, git_branch and title; the messages table carries content, tool_calls and message_uid. Open it read-only (`mode=ro`). VP: session-storage.md.
  - `hermes sessions export` (JSONL). VP.
  - stream-json for runs we start. VP.
  - Gateway events for attached viewers. VP.
- **Insert:**
  - Into a gateway-owned live session, *including one the TUI or Desktop is showing*: `session.resume` (adds a subscriber), then `prompt.submit` or `session.steer`. Needs a control-socket ticket. VP mechanism (HD/developer-guide/gateway-local-sessions.md).
  - Through the API server: `/v1/runs/{id}/steer`. VP.
- **PTY:** works with the classic CLI or `--tui`. No echo or fake provider was found (NE). Deterministic fixtures would need a local OpenAI-compatible stub through `--provider custom --base-url` (I).
- **Codex import, and keeping Hermes away from managed Codex and Claude homes.** All VP:
  - (i) `auth.adopt_external_logins: false` in the Hermes home's `config.yaml` stops both borrowing and the Keychain write-back. This is the only thing that stops the Keychain read, because that read uses a fixed service name. Source: security.md; HS/agent/anthropic_credentials.py.
  - (ii) Set `CODEX_HOME` and `CLAUDE_CONFIG_DIR` in Hermes' env to a Hermes-owned empty folder. Token import (HS/hermes_cli/auth_codex.py `_import_codex_cli_tokens`), the Codex app-server runtime (it writes a managed block into `<CODEX_HOME>/config.toml`, migrates MCP servers and plugins there, and registers Hermes as an MCP server; HD/user-guide/features/codex-app-server-runtime.md) and `sessions import` (foreign_sessions.py) all honour these variables.
  - (iii) Don't run `hermes setup`'s import offer, `import-agent` or `import-agent --sync` against managed homes. Audit `$HERMES_HOME/import-sync.json`.
  - (iv) `hermes auth add openai-codex` still asks before it imports a Codex login. Answer no, or avoid the flow.
  - Whether `import-agent` honours `CODEX_HOME` is not documented: it auto-detects `~/.codex` (U).

### Hermes, Desktop layer (Hermes.app, `com.nousresearch.hermes`, Apple Silicon only)

1. **The agent is reachable.** Desktop is a client of the same home gateway (`gateway ensure`, `/api/ws`), and its sessions land in the same `state.db`. The gateway outlives the app. VP: HD/user-guide/desktop.md. **Settings → Gateways** can register several backends (Local, remote `hermes serve`, SSH, Cloud). VP. Per-account Desktop instances are still not documented (NE).
   - **URL scheme:** `hermes://skill/install`, `hermes://mcp/install` and `hermes://plugin/install`, each with a confirmation step. VP. No link to open a session was found (NE).
   - **Version:** the site's DMG shows a build id only. The release notes say the .dmg "updates with the next bundled release". VP. The current Desktop version is U.
2. **Plugins:** the Desktop Plugin SDK. A `plugin.js` in `$HERMES_HOME/desktop-plugins/<id>/` gets `host.state`, `host.request` (gateway RPC), `host.onEvent` and `host.composer.insertText/submit(sessionId, text)`. VP: HD/developer-guide/desktop-plugin-sdk.md. Using it means writing code into the user's home, which needs explicit consent (I).
3. **MCP:** the same `config.yaml` as the CLI. Desktop adds an MCP setup UI and an OAuth relay. VP.
4. **Read and insert:** the same as the CLI. Read through `state.db` or gateway events. Insert through the gateway or a Desktop plugin. Windows are tracked by bundle id. VP. Note: two doc pages disagree on whether Desktop launches its own `hermes serve` (cli-commands) or only the gateway (desktop.md), so treat it as U.

### Muse Code, CLI layer

**1. SDK, API and protocols**
- **Headless:** `muse exec --json` writes JSONL. It supports `--session-id`, `--prompt-file`, `--max-model-steps`, `--output-schema`, `--user-input-auto-resolve` and `--api-key-stdin`. Exit codes are 0/1/2/130/143. A `run.usage` line has carried tokens and allowance since 1.4.11. VP: MD/extending, MD/changelog, local `exec --help`. The docs don't list the event types (NE).
- **`muse serve` (MSP):** JSON-RPC 2.0, schemaVersion 1, a fingerprint on every connection. Methods include session/start/resume/fork/leave/list/read/sideChat, turn/start/steer/interrupt/cancel/unqueue, approval/*, userInput/answer, view/subscribe, usage/read with usage/changed, plugin/*, skill/*, hook/list, task/*, goal/*, workflow/* and subagent/*. VP: SDK/schema/msp/msp.d.ts.
- **SDKs:** TypeScript `@muse-code/sdk`, Python `muse-code-sdk`, and a **Rust crate `muse-code-sdk` that is pinned with `=X.Y.Z` to the muse version** (Tokio; it can spawn a host or attach to an existing transport). All are a "Developer Preview" before 1.0 with no removal window. The Python SDK refuses a fingerprint mismatch; TypeScript and Rust only warn. VP: SDK/README.md, SDK/rust/clients/sdk-rs/README.md, https://meta-models.github.io/muse-code-sdk/1.4/compatibility/.
- **Socket daemon:** `serve --listen [unix:<PATH>]` runs a multi-client daemon on "the well-known per-user-machine socket". VP in the binary only; not in the docs.
- **ACP:** the changelog (1.4.0) mentions a "Zed (ACP) adapter", but there is no `acp` command in 1.4.6, and a community `muse-code-acp` is listed. An official entry point was not found (NE).
- **URL scheme:** none found (NE).
- **Terms:** the SDK exists to "programmatically control Muse Code". VP: SDK repo description. The subscription credential "is for use with Muse Code only" and "works only through the Muse Code CLI". It is governed by the Meta Subscription ToS, the Meta Model API ToS and the AUP. VP: MD/subscriptions. That driving `muse serve`/`muse exec` stays within those terms is I. No multi-account clause was found (NE). A tracker lists a clause against sharing API keys (VS: conductatlas).

**2. Plugins, skills, hooks and rules (VP)**
- **Plugins:** `muse plugins …` covers marketplaces, capability approve/reject and hook tests, all with `--json`. Plugins can supply MCP servers (1.4.11). Source: local help, changelog.
- **Skills:** `$XDG_CONFIG_HOME/muse/skills`, `~/.agents/skills` and repo `.agents/skills`. **Muse also discovers `~/.claude/skills` and `$CODEX_HOME/skills` (falling back to `~/.codex/skills`).** Source: MD/extending. The guards are `--no-foreign-personal-context` (exec help), a user preference, or pointing `CODEX_HOME` at a sentinel folder.
- **Hooks:** 17 events in `.muse/hooks.json`, user settings or a managed path. They run outside the sandbox with a cleared env. The wire format is undocumented (NE).
- **Rules:** AGENTS.md and CLAUDE.md variants. Source: MD/configuration, as read on 2026-10-21 and not re-read today.

**3. What our own MCP server can do**
- **Config:** `mcp_servers` in `settings.json` (stdio or streamable_http; `mode` required or optional; `${VAR}`; OAuth through `muse mcp login`). `MUSE_SESSION_ID` is passed to stdio servers. VP: MD/extending.
- **Per-session config:** MSP `session/start` takes `config.mcpServers` (capability `sessionMcp`), so we can add our server to the sessions we host without editing the user's settings. VP: msp.d.ts.
- **Supported:** only tools are documented. Resources, prompts, list_changed and progress are not documented (NE).
- **Elicitation:** not found (NE). Muse's own question tool works over MSP `userInput/answer`.
- **Push into a session:** not through MCP. **Session messaging** can: `muse session-message send --target <uuid|name> < body` steers the active turn, queues the message, or delivers it as notify-only, and it can wake an idle session. Limits: 8,192 bytes of plain text; the content is "unverified agent-provided data" that may trigger an approval prompt and can never approve tools; macOS and Linux only; interactive sessions only; rollout-gated; the capability is experimental. VP: MD/session-messaging, local help. Whether a sender that is not a Muse session is accepted is U.

**4. Reading output and inserting input**
- **Deterministic:**
  - MSP `session/list` (it includes sessions an open TUI is using) and `session/read` (reads without attaching).
  - `view/subscribe` (a live read-only view with no lease) and the item/turn notifications.
  - `usage/read`.
  - Sources for all of these: VP (msp.d.ts, changelog).
- **Offline transcript tools:** `muse export --session <uuid|path> --out f [--redacted]` writes one JSON document (export_schema_version 1, network-free). `muse trace inspect --session-log <session.jsonl> --format json`. VP: local help.
- **Raw files:** the on-disk paths come from the names-only listing (I), and the raw format is undocumented (NE), so prefer MSP or export.
- **Insert:** `turn/start` and `turn/steer` on hosts we own. A session held by another host is refused (`sessionInUse`), and `session/leave` hands it over. VP. For a TUI-owned session, use session messaging or PTY input.
- **PTY:** `--provider echo --echo-delay-ms` gives deterministic replies with no model or account, which is ideal for fixtures. VP: local help.
- **Isolation:** still unresolved. `XDG_CONFIG_HOME` is honoured (VP). Data lives in `~/.local/share/muse` plus a macOS `Application Support` session-name authority, and session names are unique across all the user's sessions. There is an `auth.json` file plus Keychain use. `META_API_KEY` takes precedence (VP). Whether browser sign-ins stay separate per home is U, pending a live probe.

### Muse Code, Desktop layer
- **Unavailable.** No Meta desktop app was found (NE). Searches: "Meta Muse Code desktop app macOS"; "Meta Muse app Mac coding … desktop terminal handoff October 2026"; MD index; changelog. The changelog's "desktop app" (1.4.10) and "Desktop and other connected clients" (1.4.13) mentions are unexplained. Third-party wrappers (muse-desktop, AFK Pilot, Paseo) are not from Meta (VS).

---

## (c) What each adapter should use first (ranked)

### Hermes (AJ-E1607)
1. **A per-account `HERMES_HOME` outside `~/.hermes/profiles/`, plus `hermes gateway ensure --json` and lifecycle ownership.** Each home gets its own gateway, which stays out of the user's multiplexer. **Before first launch**, seed `config.yaml` with `auth.adopt_external_logins: false` and set `CODEX_HOME` and `CLAUDE_CONFIG_DIR` to Hermes-owned empty folders. Add guardrail tests for the Keychain item, `~/.codex/config.toml` and `import-sync.json`. Risk: high if skipped, because Hermes otherwise rotates single-use refresh tokens and writes to Claude Code's Keychain item.
2. **Read `state.db` read-only** for sessions, tokens and cost. Deterministic, VP. Risk: the schema moves (v31), so read columns by name.
3. **`hermes chat --oneshot --format stream-json --source tool`** for runs we start, and `hermes usage --json` for Hermes-reported windows. Low risk.
4. **ACP (`hermes acp`)** for interactive hosting with approvals, resume and fork, and for our MCP server per session. Requires the `acp` extra. Medium risk, but it is a public, editor-grade protocol.
5. **TUI-gateway JSON-RPC with a control-socket ticket**, only to attach to and insert into live TUI or Desktop sessions. Risk: it is an internal protocol with recent breaking changes, so gate it behind a version check.
6. **Desktop:** track-only. Use the bundle id and window focus; sessions come from the gateway or `state.db`. Do not install Desktop plugins without explicit consent.
- **Terms:** one Nous Portal account per person. Never offer account rotation to stretch capacity. Codex and Claude subscriptions inside Hermes stay disjoint from AI Juggler's managed accounts.

### Muse Code (AJ-E1608)
1. **The Rust `muse-code-sdk` over `muse serve`** (pinned to the installed `muse --version`; handle `fingerprint_warning`) for sessions, turns, approvals, `view/subscribe`, `usage/read` and per-session `mcpServers`. This is the official route, and Meta explicitly invites it. Risk: Developer Preview and pre-1.0, and the self-updating binary drifts away from the pin, so AJ-E1611's update check must flag a pin mismatch.
2. **`muse exec --json --no-foreign-personal-context`** for one-shot runs, and `muse usage` (1.4.13) or `run.usage` for allowance. Low risk.
3. **`--provider echo` fixtures** for the conformance kit: deterministic, with no account.
4. **`muse export` / `muse trace inspect --format json`** for offline transcripts. Avoid parsing raw `session.jsonl`.
5. **Session messaging (`muse session-message send`)** to push into a TUI-owned session. Experimental and rollout-gated; the content counts as unverified, so it suits status and handoff notes only.
6. **The `serve --listen` socket daemon** only after it is documented. Today it appears in the binary help only.
- **Isolation and terms:** API-key accounts (`META_API_KEY`) first. Subscription accounts wait for a live probe of the XDG homes, `auth.json` and the Keychain. Never read or relocate `~/.config/muse/auth.json`. The subscription works only through the Muse Code CLI, so never reuse its credential elsewhere. Point `CODEX_HOME` at a sentinel folder or use the foreign-context switch, so that Muse doesn't load the skills from managed `~/.codex` and `~/.claude` homes.
- **Desktop:** none to support. If Meta ships one, it would likely be an MSP client (I).

## Search log (NE)
- **Muse desktop:** WebSearch "Meta Muse Code desktop app macOS" (extended); "Meta Muse app Mac coding "Muse Code" conversations desktop terminal handoff October 2026" (extended); WebFetch MD index and the full changelog (both halves).
- **Muse ACP:** WebSearch `"Muse Code" Zed ACP agent adapter`; changelog; local `muse --help` 1.4.6.
- **Muse MCP elicitation:** MD/extending, the changelog, and a grep of SDK/schema/msp/msp.d.ts and SDK/CHANGELOG.md for elicit/sampling.
- **Hermes session URL scheme:** grep for `hermes://` across 31 downloaded docs pages (routes found: skill/plugin/mcp install, plugin-agent, plugin-desktop, index-network).
- **Hermes echo provider:** cli-commands.md provider list.
