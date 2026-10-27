# OpenCode: how a third-party Mac app can observe and control it

Research date: 2026-10-27. Labels: **VP** Verified-primary · **VS** Verified-secondary · **NE** Negative-evidence (search logged) · **I** Inferred · **U** Unknown. All sources were accessed 2026-10-27 unless a different date is noted.

How the research was done: OpenCode is **not installed** on this Mac (`opencode`: command not found; no `/Applications/OpenCode.app`; no `~/.local/share/opencode`). Nothing was signed in, no server was started, and no credential was read. Vendor docs were fetched. The v2 OpenAPI spec was downloaded. The source was shallow-cloned into the scratchpad as data (`dev` @ `f993a89`, which is the 2.2.0 build per `opencode.ai/update/api/latest/cli/npm`, and tag `v1.19.4`).

Short URLs: `OD/` = https://opencode.ai/docs/ (v1) · `OD2/` = https://opencode.ai/v2/docs/ · `V1/` = https://github.com/anomalyco/opencode/blob/v1.19.4/packages/ · `V2/` = https://github.com/anomalyco/opencode/blob/f993a89d13c73ff6f6537d6b97e78e138a3ea363/packages/ · `SPEC` = https://opencode.ai/v2/openapi.json

## 0. Versions and corrections to `docs/research/opencode.md` (2026-10-21)

- **VP** The repo is `anomalyco/opencode`, MIT, 222.1k stars, default branch `dev`; `github.com/sst/opencode` returns 301. — GitHub API `repos/anomalyco/opencode`
- **VP** **v1 line:** npm `opencode-ai` latest **1.19.4** (2026-10-22). GitHub release v1.19.4 also ships a v1 Desktop (`opencode-desktop-mac-arm64.dmg`, `latest-mac.yml` version 1.19.4). `@opencode-ai/sdk` and `@opencode-ai/plugin` are 1.19.4. — https://registry.npmjs.org/opencode-ai ; GitHub releases API
- **VP** **v2 line:** npm `@opencode/cli` latest **2.2.0** (published 2026-10-27 08:59Z, from dev sha f993a89). `@opencode/client`, `@opencode/sdk` and `@opencode/plugin` are 2.2.0, MIT. The Homebrew tap `opencode-v2.rb` and the cask `opencode-desktop` still say **2.1.4**. — https://opencode.ai/update/api/latest/cli/npm ; https://registry.npmjs.org/@opencode/cli ; https://formulae.brew.sh/api/cask/opencode-desktop.json
- **VP** **The stable Desktop download is now v2:** `opencode.ai/download/stable/darwin-aarch64-dmg` returns 302 to `/files/bin/2.2.0/opencode-desktop-mac-arm64.dmg`. `opencode.ai/llms.txt` redirects to the v2 docs index. `opencode upgrade --major` "move[s] to OpenCode 2 and remove[s] OpenCode 1". — curl -I; OD/cli/
- **Correction (VP):** the doc's Desktop section (in-process sidecar with a random password) describes **v1 Desktop**. **v2 Desktop connects to the shared per-user background service** that CLI clients also use. — V2/desktop/src/main/service/background-service.ts
- **Correction (VP):** S-1 ("no Go usage API") does not hold for v2. `GET /api/experimental/usage/limits` returns the Console workspace's **Go allowance meters (rolling / weekly / monthly) and budgets**, or null when not signed in. — SPEC
- **VP** v2 no longer reads `CLAUDE.md` fallbacks. MCP config moves to `mcp.servers` (`enabled` becomes `disabled`). Plugins become `plugins` and v1 plugins must be ported. TUI config moves to `~/.config/opencode/cli.json`. `OPENCODE_SERVER_USERNAME` is removed. — OD2/migrate-v1/

## Layer A: OpenCode CLI/TUI

### A1. Official SDK / API / protocol

**v1 (1.19.x)**
- **VP** `opencode serve [--port 4096 --hostname 127.0.0.1 --mdns --cors]`. The OpenAPI 3.1 spec is at `/doc`. Endpoints: sessions (create, list, status, get, delete, update, children, todo, init, **fork**, **abort**, diff, summarize, revert, unrevert, `permissions/:id`); messages (`POST /session/:id/message` waits, `POST /session/:id/prompt_async` returns 204, command, shell); `/event` and `/global/event` SSE (first event `server.connected`); `/tui/append-prompt|submit-prompt|clear-prompt|execute-command|show-toast|open-*`, `/tui/control/next|response`; `PUT /auth/:id`; `/mcp`. — OD/server/
- **VP** The page says "Running `opencode` starts both a TUI and a server". In source, the v1 TUI talks to an **in-process worker** (`http://opencode.internal`) and listens on the network **only** when `--port`, `--hostname` or `--mdns` is passed. — V1/opencode/src/cli/cmd/tui.ts
- **VP** Auth is HTTP Basic, and **only if `OPENCODE_SERVER_PASSWORD` is set** (user `opencode` or `OPENCODE_SERVER_USERNAME`). Without it the localhost server has no auth. — V1/opencode/src/server/auth.ts; OD/server/
- **VP** `opencode attach [url] --dir -s -c --fork -p/--password -u`; `opencode run … --format json --attach <url> --port`. — OD/cli/
- **VP** SDK `@opencode-ai/sdk`: `createOpencode()` / `createOpencodeClient({baseUrl})`. It has session.* (`prompt` with `noReply: true` adds context without a reply; `share` is "discontinued"), `event.subscribe()`, tui.*, `auth.set`. The `/v2` sub-client adds structured output. No stability statement. — OD/sdk/
- **VP** ACP: `opencode acp` (stdio JSON-RPC); tools, slash commands, MCP from config, permissions; `/undo` and `/redo` unsupported. — OD/acp/

**v2 (2.2.0)**
- **VP** **One shared background server per user** "owns sessions, configuration, integrations, permissions, and tool execution". Clients discover it or start it. `--standalone` runs a private server; `--server <url>` targets one; `opencode service start|stop|restart|status|get|set|unset`. — OD2/cli/
- **VP** Discovery contract: `$XDG_STATE_HOME/opencode/service.json` (mode 0600) holds `url, pid, version, password`. "Reading it is all a client needs". The service's own config (port, password, hostname, cors, env, remote, disabled) is at `$XDG_CONFIG_HOME/opencode/service.json`. Default port **49374** (0xC0DE) for stable, beta and next. — V2/client/src/effect/service.ts; V2/cli/src/services/service-config.ts; OD2/troubleshooting/
- **VP** HttpApi: 156 operations under `/api/*`, described as "**Experimental** HttpApi surface" (info.version 0.0.1). The relevant ones:
  - `session.create/list/get/update/remove/fork/compact/command/shell/switchModel/move`.
  - `POST /api/session/:id/prompt {text, files, agents, skills, delivery: "steer"|"queue", resume}` ("Durably admit one session input").
  - `POST /api/session/:id/synthetic`, `POST /api/session/:id/interrupt?resume=`, `GET /api/session/active`, `GET /api/session/:id/inbox`.
  - Permission list and reply; forms list and reply.
  - `PUT /api/session/:id/environment` (the env for that session's shell commands).
  - `GET /api/event` SSE: "Volatile by contract … events during disconnection are missed".
  - `GET /api/experimental/session/:id/log?after=&follow=` (durable event log with live follow).
  - `/api/pty` and persistent-PTY WebSockets.
  - `/api/credential` (with `/activate`), `/api/experimental/usage/limits`, `/api/rpc/{rpcID}/{method}` (plugin RPC), `POST /api/pair` (pairing code).
  - — SPEC; OD2/api/
- **VP** `opencode api <operationId | METHOD path> [-d body] [--param]` calls the API from the CLI with auth handled. — V2/cli/src/commands/commands.ts
- **VP** Clients: `@opencode/client` (`OpenCode.make({baseUrl, headers})`, `Service.discover()` without spawning, `Service.ensure()`, events live-only with no replay) and `@opencode/sdk` (embedded host, no HTTP listener). — OD2/build/client/, OD2/build/sdk/
- **VP** v2 TUI flags: `[directory] -c -s/--session --prompt` (no `--model`), plus global `--standalone`, `--server`, `--auto`, `--dangerously-skip-permissions`. `run` adds `-c -s --fork -m --agent --format json -f --title --thinking`. There is no `attach` subcommand. — V2/cli/src/commands/commands.ts
- **VP** v2 ACP: ACP v1. Sessions new, load (replay), resume, fork (no replay), list, close, delete; cancel; `set_config_option` for model, effort and mode; form elicitation; client-passed stdio/HTTP MCP servers; `usage_update`; `--login`. Limits: it runs a **private** server ("no connection to the shared background service") and allows one prompt at a time. — OD2/cli/acp/
- **I** **Port collision.** AI Juggler pins `XDG_STATE_HOME` and `XDG_CONFIG_HOME` per account. Each managed v2 account therefore has its own registration and no service config, and its TUI spawns `serve --service` on the **same default port 49374**. While the user's default service or Desktop runs, the managed launch fails ("Managed service port 49374 … already in use … `opencode service set port`"). The reverse also happens: a managed service started first blocks the user's own CLI and Desktop. No credential leak follows, because the incumbent check needs the account's own registration file. — V2/cli/src/server-process.ts (the EADDRINUSE path)
- **VP** **`--standalone` gives a launch its own server, which ends with it.** A client given `--standalone`, or whose `$XDG_CONFIG_HOME/opencode/service.json` says `disabled: true`, never discovers or starts the shared service. It spawns `serve --stdio --port 0` (a port the system picks) with a random 32-byte password passed only in that child's environment, and the child "treats EOF on this pipe as the end of its ownership lease", which the OS closes "even when the TUI is killed". `--standalone` is a root option and a global option of `run`, `auth` and the other server-backed commands. — V2/cli/src/services/standalone.ts, server-connection.ts (`resolve`), commands/commands.ts; the service config's fields (`disabled, port, password, hostname, cors, env, remote`) in V2/cli/src/services/service-config.ts
- **AI Juggler since 2026-10-27 (VP, from its code):** every launch of a **managed** account on a 2.x binary (the TUI, `run`, `auth login`) passes `--standalone`; the line is told from the binary's `--version`, cached by path, size and time, never from its name. 1.x launches, a version that can't be read, and the default folder used in place (which keeps the user's shared service, as their own `opencode` does) are unchanged. Nothing is written to the account and no `service.json` is read, so there is no port to pick, no password on disk and no daemon to stop. Chosen over a per-account `service.json` port and password because that service outlives the TUI (AI Juggler would have to own and reap it) and needs files written into every existing account. **I**, on the AJ-E1612 checklist: that `run --standalone` and `auth login --standalone` parse in that position, that 2.x before 2.2.0 had the option, and that a hand-run copyable line (which has no flag) is the remaining way to start a per-account service. — crates/juggler-adapters/src/opencode/service.rs

**Terms and auth**
- **VP** The client is MIT. OpenCode's ToS (hosted Zen/Go/Console) bars "multiple accounts to circumvent usage limits" and automated extraction of data or Output (read 2026-10-21, still fresh). — https://opencode.ai/legal/terms-of-service
- **VP** Anthropic: "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, or Max plan credentials". OpenCode's docs relay this ("Anthropic explicitly prohibits this"; the bundled plugins were removed in 1.3.0). Community Pro/Max plugins still exist (**VS**). AI Juggler must never offer them. — https://code.claude.com/docs/en/legal-and-compliance ; OD/providers/
- **VS / U** One secondary article says OpenAI partnered with OpenCode so that ChatGPT/Codex subscriptions can be used. No first-party OpenAI statement was found, so the status stays U. The doc lists a "ChatGPT Plus/Pro" option. The earlier warning about refresh-token collisions with Codex homes still stands (**I**). — https://www.sovereignmagazine.com/article/openai-codex-plugins-ai-coding-tools ; OD/providers/
- **VP** v2 multi-account: several saved credentials per provider, one active (`opencode auth switch <provider> <label|id>`; `/connect` can activate, rename and delete), stored in `opencode.db`. "Don't edit the database directly." — OD2/cli/providers/

### A2. Plugins / extensions
- **VP** v1 plugins live in `.opencode/plugins/`, `~/.config/opencode/plugins/` or npm `plugin: []`. Input: `{client (SDK bound to the instance), project, directory, worktree, serverUrl, $}`. Hooks:
  - `event` (all bus events: session.idle/status/error/updated, message.part.updated, permission.asked/replied, todo.updated, tui.*);
  - `config`, `auth`, `chat.message`, `chat.params`, `chat.headers`;
  - `permission.ask` (it can set allow or deny);
  - `command.execute.before`, `tool.execute.before/after` (change args or block by throwing), `shell.env`, `tool.definition`;
  - `experimental.chat.system.transform`, `experimental.chat.messages.transform`, `experimental.session.compacting`, `experimental.text.complete`;
  - custom tools via `tool()`; a plugin tool overrides a built-in of the same name.
  - — OD/plugins/; V1/plugin/src/index.ts
- **VP** v2 server plugins: `Plugin.define({id, setup(ctx)})`, where ctx is a full server client. Hooks:
  - `ctx.session.hook("prompt"|"context"|"compaction"|"generate"|"title"|"model.request"|"http.request"|"http.response"|"retry"|experimental.ws.*)`;
  - `ctx.permission.hook("evaluate")`, `ctx.shell.hook("create.before")`, `ctx.tool.hook("execute.before"|"execute.after")`;
  - registry transforms (tools, agents, providers, MCP…);
  - `ctx.session.synthetic/prompt/interrupt/wait`, `ctx.event.subscribe`;
  - **RPC**: `Rpc.define` + `ctx.rpc.register`, callable by external clients (`client.rpc(Def)` at `/api/rpc/{rpcID}/{method}`), with events `rpc.<id>.<name>` on the event stream.
  - — OD2/build/plugins/server/, OD2/build/plugins/rpc/
- **VP** v2 TUI plugins (`./tui` export or `cli.json`) get commands and keymaps, UI slots, toasts, dialogs, routes, `context.client`, and `context.data` with listeners. — OD2/build/plugins/cli/
- **VP** Custom agents, commands, skills and themes are files or config; `opencode agent create`; `opencode plugin <module>`; `--pure` / `OPENCODE_PURE` runs without external plugins. — OD/cli/, OD2/migrate-v1/

### A3. MCP: what our own MCP server can do
- **VP** v1 config is `mcp.<name>`: `{type:"local", command[], environment, cwd, enabled, timeout}` or `{type:"remote", url, headers, oauth{clientId, clientSecret, scope}|false}`. A 401 auto-starts OAuth with DCR; tokens go to `mcp-auth.json`; `opencode mcp add|list|auth|logout|debug`. Per-agent enabling uses tool globs `<server>_*`. — OD/mcp-servers/
- **VP** The v1 client declares only `roots`. Sampling (#11948), elicitation (#23066) and tasks (#28567) are commented out. It catalogs tools, prompts and resources and handles `tools/list_changed` and logging notifications. — V1/opencode/src/mcp/index.ts
- **VP** v2 config is `mcp.servers.<name>`: `disabled`, `codemode`, `protocol` (`legacy` default, up to 2025-11-25, `auto`, or `2026-07-28`), timeouts (`startup` 30 s, `catalog` 30 s, `execution` 12 h). Tools are named `<server>_<tool>` and grouped under Code Mode by default. Prompts become `/<server>:<prompt>` commands. Resources are read through built-in tools. **Elicitation** (form and URL) is supported. Server instructions are added to the system prompt. OAuth uses PKCE with client_id, CIMD or DCR. — OD2/mcp-servers/; V2/core/src/mcp/client.ts
- **VP** v2 experimental **client-hosted MCP**: a client opens a WebSocket at `GET /api/experimental/client-mcp?name=&ticket=` and hosts an MCP server whose tools exist while the socket is open. — SPEC
- **NE (S-5)** No path was found for an MCP server to **push a message into a running session**: no sampling, and elicitation is "Location-scoped, not Session-scoped". The sanctioned ways in are the API (`/synthetic`, `/prompt` with `delivery`) or a plugin (`ctx.session.synthetic`).
- **VP, about AI Juggler** `opencode_inventory.rs` reads and writes only v1-shaped `mcp.<name>`. A v2-authored `{"mcp":{"servers":{…}}}` produces **no items**, because `mcp_item("servers", …)` finds no `type` and returns None. Whether v2 accepts a mixed v1 and v2 `mcp` object is **U**. The migrate page says only that v1 fields are normalized in memory. — crates/juggler-adapters/src/opencode_inventory.rs:48-70, 251+; OD2/migrate-v1/
- **AI Juggler since 2026-10-27 (VP, from its code):** the inventory reads both forms, file by file: `mcp.<name>` entries with a `type` (off when `enabled: false`) and `mcp.servers.<name>` (off when `disabled: true`; a 1.x server that happens to be named `servers` has a `type` and stays one). A set-up writes the form the settings file already uses, else the installed binary's (1.x when it can't be told), so it never mixes forms in a file; a 2.x-form file with OpenCode 1 installed takes nothing. The entry's fields (`type`, `command`, `environment`, `url`, `headers`) are the same in both forms (VP, OD2/mcp-servers/). **I**, on the live checklist: that 2.x reads the written entry, and how it treats a file a user wrote in both forms (both are listed). — crates/juggler-adapters/src/opencode_inventory.rs

### A4. Reading output and inserting input
- **VP** Storage: SQLite `$XDG_DATA_HOME/opencode/opencode.db` (WAL). v1 has the `session`/`message`/`part` tables. v2 has `session_v2`, `session_message`, `session_inbox`, a durable `event` table, `credential`, `account` and others. The `storage/` JSON layout of old docs is legacy, migrated. The v2 schema churns: 54 migrations, the newest 2026-10-25 (`drop_session_allowance`, `drop_session_pending`). `opencode db [query] --format json`, `session list --format json` and `export` are the vendor's own readers. — V2/core/schema.json, V2/core/src/database/migration/; OD/cli/
- **VP** Live output:
  - v1 has `/event` SSE, but only on a server that AI Juggler started (`serve`, or TUI `--port`).
  - v2 has `/api/event` (event types include session.text.delta, session.tool.called/success/failed, session.status, session.idle, session.usage.updated, session.inbox.*, permissions) and the durable, replayable `/api/experimental/session/:id/log`.
  - — SPEC; V2/schema/src
- **VP** Input:
  - v1: `POST /session/:id/prompt_async`, or `/tui/append-prompt` + `/tui/submit-prompt` to drive the visible TUI.
  - v2: `POST /api/session/:id/prompt` with `delivery: steer|queue` (steering works mid-run), and `/interrupt`. Every client of the shared service, TUI or Desktop, renders it live.
  - — OD/server/; SPEC
- **VP/I** PTY:
  - The v1 home route auto-submits `--prompt` once the model is ready (V1/tui/src/routes/home.tsx).
  - v2 passes `--prompt` as the session's startup prompt (V2/tui/src/app.tsx); whether it submits is **U**.
  - The launch argv and env are deterministic, and typing into the composer needs no model. Reading state from the OpenTUI full-screen render is heuristic (**I**), so prefer the DB or API.
- **AI Juggler today (VP, from its code):**
  - `opencode.rs`: launches the TUI in a PTY (`--session`, `--prompt=`, model via `OPENCODE_CONFIG_CONTENT`) and headless `run --format json -- <prompt>`; sign-in through `auth login` in a PTY; XDG isolation per account, cache shared.
  - `opencode_store.rs`: reads `opencode.db` read-only/immutable (sessions, cost, tokens, the DbWatcher, links `<db>#<id>`).
  - `opencode_environment.rs`: imports a data folder by `VACUUM INTO`, with the private tables emptied.
  - `opencode_inventory.rs`: MCP and AGENTS.md set-up.
  - It never starts `serve`, and uses no SDK, events, plugins, ACP or Desktop. Every version is still "unverified" (`VERIFIED_AGAINST` is empty).

## Layer B: OpenCode Desktop (`OpenCode.app`, Electron, `ai.opencode.desktop`)

### B1. SDK / API / protocol / IPC
- **VP** Two Desktops exist. The **v2 Desktop (2.2.0)** is the stable download. The v1 Desktop (1.19.4) remains on GitHub releases with electron-updater.
- **VP** v2 Desktop starts or joins the **shared v2 background service** through its bundled CLI, so its sessions, DB and events are the same as the CLI's. Everything in A1 to A4 for v2 applies to Desktop sessions. Desktop restarts the service when the CLI version differs. — V2/desktop/src/main/service/background-service.ts; OD2/troubleshooting/
- **VP** v1 Desktop runs a private sidecar server with a random Basic password, kept in memory, and shares `XDG_DATA_HOME` with the CLI. Its "default server URL" can point it at another server. — V1.19.3 desktop/src/main/sidecar.ts and server.ts (read 2026-10-21)
- **VP** URL scheme `opencode://` (`setAsDefaultProtocolClient`):
  - v1 routes: `opencode://open-project?directory=<abs>` and `opencode://new-session?directory=<abs>&prompt=<text>`, local server only; whether the prompt is submitted is **U**. — V1/app/src/pages/layout/deep-links.ts, layout.tsx
  - v2 main parses only `opencode://console/authorized?window=` (the Console sign-in return). Other links are forwarded to the renderer as an event, and **no v2 consumer was found (NE, S-6)**. — V2/desktop/src/main/lifecycle/
- **NE (S-6)** No documented external IPC was found. Electron IPC is internal only.

### B2. Extensions
- **VP** GUI plugins use the `./gui-experimental` export. The window side adds panels, commands, settings, slots, server sources, link handlers, and sessions and composer APIs. The main side gets Ipc contracts, menus, WebContentsView embeds, `ctx.serverEndpoints` and full Node/electron. They are labelled experimental ("may change between releases") and load from the global `opencode.json`. — OD2/build/plugins/gui/

### B3. MCP
- **I** Desktop uses the service's MCP config, so A3 applies. No Desktop-specific MCP surface was found.

### B4. Output and input
- **VP/I** Read: v2 through the service API and events, or through `opencode.db`; v1 through `opencode.db`. Insert: v2 through `POST /api/session/:id/prompt` (Desktop renders it live); v1 only through the new-session deep link. Per-account Desktop is still not possible (single instance; S-4).
- **VS** Detection: `/Applications/OpenCode.app`, ids `ai.opencode.desktop` / `.beta` / `.dev`, macOS 13 or later. — Homebrew cask

## Search log (Negative-evidence)
- **S-5 MCP push into session** (2026-10-27): OD/mcp-servers/, OD2/mcp-servers/, SPEC (all `mcp` and experimental operations), V1/opencode/src/mcp/index.ts, V2/core/src/mcp/{client,index}.ts. Grep for sampling, elicitation, notifications and synthetic. No server-to-session push was found by this search.
- **S-6 Desktop routes and IPC** (2026-10-27): grepped V2 and V1 `packages/desktop/src`, `packages/app/src`, `ui` and `gui-plugins` for `opencode://`, `deep-link`, `open-url`, `consumeInitialDeepLinks`. In v2 only the `console/authorized` route was found. No external IPC was found by this search.

## (c) Highest-value additions for AI Juggler, ranked

1. **Fix the v2 service-port collision before any live v2 test.** Risk: high if left (managed accounts fail while the user's OpenCode or Desktop runs, or block them). Two fixes:
   - Write `<account>/config/opencode/service.json` with a per-account free port and an AI-Juggler-generated password (equivalent to `opencode service set port …`).
   - Or launch managed TUIs with `--standalone`.

   This also gives AI Juggler the password without reading anything. The service is a daemon that outlives the TUI, so AI Juggler must own and reap it (`opencode service stop`). It belongs on the AJ-E1612 live checklist.

   **Done 2026-10-27 with `--standalone`** (see A1): its server ends with the TUI, so nothing is left to reap. Item 2 can later run its own per-account service when it needs a server other clients join.
2. **Live control plane for managed sessions (v2 `/api/*`, v1 `--port` + own password).** Covers prompt with `steer`/`queue`, interrupt, fork, permission reply, `/api/session/active`, and `/api/event` + `/session/:id/log` instead of DB polling. This gives exact running/idle/needs-permission status, cross-session messages for coordination, and "send to session" without a PTY. Risks: the API is labelled Experimental (pin per version, keep the DB path as fallback), and a v1 server without a password is unauthenticated (always set one).
3. **Sanctioned Go/Console quota for v2:** `/api/experimental/usage/limits` returns rolling, weekly and monthly meters. Honest usage moves from Estimate to Measured for OpenCode Go accounts. This revises S-1. Risk: experimental, and it needs the Console sign-in in that account's data dir. Label it "vendor-reported".
4. **Fix the v2 MCP inventory:** read and write `mcp.servers` (and `disabled`) when the file is v2-shaped, and pick the shape by line. Today a v2 config shows no servers, and set-up writes a v1-style sibling key. Risk: low; test mixed shapes live. **Done 2026-10-27** (see A3); the live test of mixed files remains.
5. **Per-session environment (v2 `PUT /api/session/:id/environment`, or service `env`):** keep the XDG isolation variables on the OpenCode process while agent-spawned tools see the user's normal env. This addresses the known XDG-leak trade-off. Risk: medium, because the call replaces the whole env; verify in the lab.
6. **Track Desktop through the shared service (v2):** list and monitor Desktop sessions, and "Go to it" via `open -b ai.opencode.desktop`. For the user's default service this means reading `~/.local/state/opencode/service.json`, which holds a password. Under real-accounts-read-only that is a **maintainer ruling**; until then use only the read-only `opencode.db` (already supported for the default folder). The v1 `opencode://new-session?directory=&prompt=` link is a cheap "open in Desktop" for v1 only.
7. **ACP as an alternative headless driver:** sessions list, load, resume and fork, cancel, permissions, `usage_update`, client-passed MCP servers. It is a clean, typed stdio protocol shared with other tools. Risks: private server, one prompt at a time, MCP OAuth not supported.
8. **Optional AI Juggler plugin (v2 RPC, or a v1 `event` hook):** a bridge that streams events and accepts injected prompts even in sessions AI Juggler didn't launch. Risk: it runs code in the user's agent and changes the user's config, so it needs explicit opt-in, and v1 and v2 need separate builds. Below 2 because the API covers managed sessions.
9. **Terms guardrails in the UI:** never offer Claude Pro/Max through OpenCode (Anthropic VP). Warn about ChatGPT token collisions with Codex homes. OpenAI's sanction is U.
