# Research notes: OpenCode (Anomaly; formerly sst/opencode)

Research date: 2026-10-21

> Input to the multi-Provider evaluation. Mirrors the structure of [codex.md](codex.md). Nothing was installed, no account was signed in, no binary was downloaded, and no local OpenCode, `~/.local/share`, `~/.claude` or credential directory was read. Source files were read as text from GitHub (raw / API). Every claim was accessed on 2026-10-21.
>
> **Legend (per [EVIDENCE-STANDARD.md](EVIDENCE-STANDARD.md)):**
> **[Verified-primary]** the vendor's own docs, source or published package metadata say it ·
> **[Verified-secondary]** a reliable second-hand source (Homebrew API, a vendor-repo issue by a user, press) says it ·
> **[Negative-evidence]** not found by this search; the search is logged under Search log (S-n) ·
> **[Inferred]** our own reasoning; never a fact.
>
> Surfaces: **CLI** = the `opencode` TUI/CLI (and its headless server). **Desktop** = OpenCode Desktop (`OpenCode.app`). **Subscriptions** = OpenCode Zen (pay-as-you-go), OpenCode Go / Go Plus (monthly), and the historical "Black" tiers.
> Short URLs: `OD/` = `https://opencode.ai/docs/`. `OD2/` = `https://opencode.ai/v2/docs/`. `SRC/` = `https://github.com/anomalyco/opencode/blob/v1.19.3/packages/` (tag `v1.19.3`, released 2026-10-16). `DEV/` = the same tree on branch `dev` (the v2 line).

## 1. Distribution & versioning

- **[Verified-primary]** The repo is now `anomalyco/opencode`. `github.com/sst/opencode` returns HTTP 301 to it. The repo is MIT-licensed, about 219.5k stars, described as "The open source coding agent", default branch `dev`. — Source: https://github.com/anomalyco/opencode and the GitHub API `repos/anomalyco/opencode`
- **[Verified-primary]** The company is Anomaly Innovations, Inc. (support address `help@anoma.ly`). — Source: https://opencode.ai/legal/terms-of-service
- **[Verified-primary]** There are **two release lines right now**:
  - **v1:** npm `opencode-ai` `latest` = **1.19.3** (published 2026-10-16). GitHub release `v1.19.3` ships `opencode-darwin-arm64.zip`, `opencode-darwin-x64(-baseline).zip`, Linux tarballs, and the desktop app (`opencode-desktop-mac-arm64.dmg`, `.zip`, `latest-mac.yml`). — Source: https://registry.npmjs.org/opencode-ai; GitHub API `repos/anomalyco/opencode/releases`
  - **v2:** the Homebrew tap formula `anomalyco/tap/opencode-v2` is version **2.1.2**, downloads from `https://opencode.ai/files/bin/2.1.2/opencode-darwin-arm64.zip`, and `conflicts_with "opencode", because: "both install an opencode binary"`. The v2 docs install from `https://opencode.ai/v2/install` and say older mise versions "install OpenCode 1". — Source: `https://github.com/anomalyco/homebrew-tap/blob/main/opencode-v2.rb`; OD2/
- **[Verified-primary]** v1 install methods: `curl -fsSL https://opencode.ai/install | bash`; `npm|bun|pnpm i -g opencode-ai`; `brew install anomalyco/tap/opencode`; AUR; choco/scoop; Docker `ghcr.io/anomalyco/opencode`; GitHub Releases. Windows is best via WSL. — Source: OD/
- **[Verified-primary]** The binary is `opencode`. `opencode --version` (`-v`) prints the version. `opencode upgrade [target]` updates (`--method curl|npm|pnpm|bun|brew`). `OPENCODE_DISABLE_AUTOUPDATE` turns off update checks. — Source: OD/cli/
- **[Verified-primary]** **Desktop** is an **Electron** app (`electron-builder.config.ts`, `latest-mac.yml`/`.blockmap` artifacts). App ids by channel: `ai.opencode.desktop` (prod, product name "OpenCode"), `ai.opencode.desktop.beta` ("OpenCode Beta"), `ai.opencode.desktop.dev` ("OpenCode Dev"). — Source: SRC/desktop/electron-builder.config.ts
- **[Verified-secondary]** Homebrew cask `opencode-desktop` 2.1.2 installs `/Applications/OpenCode.app`. Its zap list names `~/Library/Application Support/ai.opencode.desktop`, `~/Library/Caches/ai.opencode.desktop`, `~/Library/Logs/ai.opencode.desktop`, `~/Library/Preferences/ai.opencode.desktop.plist`, `~/Library/WebKit/ai.opencode.desktop`, and related files. — Source: https://formulae.brew.sh/api/cask/opencode-desktop.json
- **[Verified-primary]** Desktop downloads: macOS arm64/x64 `.dmg`, Windows x64/ARM64, Linux `.deb`/`.rpm`; `brew install --cask opencode-desktop`. IDE extensions exist for VS Code, Cursor, Zed, Windsurf and VSCodium. — Source: https://opencode.ai/download
- **[Inferred]** Mac detection: the CLI is `opencode` on `PATH` (npm global bin, `$(brew --prefix)/bin/opencode`, or the curl installer's `~/.opencode/bin` [unverified path]); run `opencode --version` to tell v1 (1.x) from v2 (2.x). Desktop: `/Applications/OpenCode.app` with `CFBundleIdentifier` `ai.opencode.desktop` (or the `.beta`/`.dev` ids).
- **[Verified-primary]** Release cadence is days: v1.18.32 (2026-09-21) through v1.19.3 (2026-10-16) is eight releases in 25 days. — Source: GitHub API releases

## 2. Configuration & local state

- **[Verified-primary]** Config precedence, lowest to highest: remote `.well-known/opencode` → global `~/.config/opencode/opencode.json` → `OPENCODE_CONFIG` (a file) → project `opencode.json` → `.opencode/` directories → `OPENCODE_CONFIG_CONTENT` (inline JSON) → managed files (`/Library/Application Support/opencode/` on macOS) → macOS managed preferences (`ai.opencode.managed`). `OPENCODE_CONFIG_DIR` loads a custom config directory after the global one. — Source: OD/config/
- **[Verified-primary]** **All state paths come from `xdg-basedir`:** `data = $XDG_DATA_HOME/opencode` (default `~/.local/share/opencode`), `cache = $XDG_CACHE_HOME/opencode`, `config = $XDG_CONFIG_HOME/opencode`, `state = $XDG_STATE_HOME/opencode`. `log/` and `repos/` sit under data; `bin/` under cache. `OPENCODE_CONFIG_DIR` overrides only the config root. `OPENCODE_TEST_HOME` overrides only the home used for `~` lookups. — Source: SRC/core/src/global.ts
- **[Verified-primary]** v2 keeps the XDG roots ("XDG on runtimes with a home directory"). — Source: DEV `packages/util/src/global.ts`
- **[Verified-primary]** The data dir holds `auth.json` (API keys and OAuth tokens), `log/`, and project/session data. `~/.cache/opencode` holds provider packages. — Source: OD/troubleshooting/
- **[Verified-primary]** **Sessions are in SQLite:** `$XDG_DATA_HOME/opencode/opencode.db` for the `latest`/`beta`/`prod` channels. Other channels use `opencode-<channel>.db` unless `OPENCODE_DISABLE_CHANNEL_DB=1`. `OPENCODE_DB` overrides the path (absolute, `:memory:`, or relative to data). — Source: SRC/core/src/database/database.ts
- **[Verified-primary]** The docs' troubleshooting page still describes the older JSON layout (`project/<slug>/storage/`), and a migration `20260601010001_normalize_storage_paths` exists. **Treat the docs page as stale; the source is authoritative.** — Source: OD/troubleshooting/; SRC/core/src/database/migration/
- **[Verified-primary]** MCP OAuth tokens: `~/.local/share/opencode/mcp-auth.json`. — Source: OD/mcp-servers/
- **[Verified-primary]** OpenCode reads Claude Code's files by default: project `CLAUDE.md` when there is no `AGENTS.md`, `~/.claude/CLAUDE.md` when there is no global OpenCode file, and `.claude/skills`. `OPENCODE_DISABLE_CLAUDE_CODE=1` (plus `_PROMPT` / `_SKILLS` variants) turns this off. — Source: OD/rules/, OD/cli/
- **[Inferred]** **Isolation per account is possible.** Setting `XDG_DATA_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` (plus `XDG_CONFIG_HOME` or `OPENCODE_CONFIG_DIR`) per child process gives each one its own `auth.json`, DB, logs and caches. This is the equivalent of `CODEX_HOME`, split across four variables. Caveat: the XDG variables are inherited by every tool the agent shells out to. An adapter should set them only on the `opencode` process if possible, or accept that tools spawned by the agent see the moved roots. Unproven until a live run with a throwaway home.

### Desktop

- **[Verified-primary]** Desktop runs the OpenCode server **in-process as an Electron utility process** (the "sidecar"). It sets `OPENCODE_SERVER_USERNAME=opencode` and a random `OPENCODE_SERVER_PASSWORD`, sets `OPENCODE_CLIENT=desktop`, and sets `XDG_STATE_HOME` to the Electron userData folder **only if it is unset**. It leaves `XDG_DATA_HOME` alone, so **Desktop and CLI share `~/.local/share/opencode` (auth.json + opencode.db)**. — Source: SRC/desktop/src/main/sidecar.ts (`prepareSidecarEnv`), SRC/desktop/src/main/server.ts (`preferAppEnv`)
- **[Verified-primary]** Desktop can point at another server ("default server URL", stored in `opencode.settings.dat` under `~/Library/Application Support`). — Source: SRC/desktop/src/main/server.ts (`getDefaultServerUrl`); OD/troubleshooting/
- **[Inferred]** Desktop probably honours `XDG_DATA_HOME` if launched with it (for example `open -n --env XDG_DATA_HOME=… /Applications/OpenCode.app`), because the sidecar inherits the environment. This is unverified. Electron's single-instance lock and the shared userData folder would still make two Desktop instances awkward.

## 3. Auth, providers & multiple accounts

- **[Verified-primary]** OpenCode is **multi-provider**: it uses the AI SDK and Models.dev to support "75+ LLM providers" plus local models. Sign-in is `/connect` in the TUI, or `opencode auth login [--provider p] [--method m]`, `opencode auth list`, `opencode auth logout`. — Source: OD/providers/, OD/cli/
- **[Verified-primary]** Subscription sign-ins offered in `/connect`: **ChatGPT Plus/Pro** (browser OAuth), **GitHub Copilot** (device code), and **Claude Pro/Max**. For Claude Pro/Max the docs say Anthropic "explicitly prohibits" using it through third-party plugins, and the bundled plugins were removed in 1.3.0. Also OpenCode Zen / Go (Console API key). — Source: OD/providers/
- **[Verified-primary]** **v1 auth store:** `$XDG_DATA_HOME/opencode/auth.json`, written mode `0600`. It is a map keyed by provider id, holding one entry per provider: `{type:"oauth", refresh, access, expires, accountId?, enterpriseUrl?}`, `{type:"api", key, metadata?}`, or `{type:"wellknown", key, token}`. **`OPENCODE_AUTH_CONTENT`** (inline JSON) replaces the file contents when set. — Source: SRC/opencode/src/auth/index.ts
- **[Verified-primary]** **v2 auth store moves into SQLite:** a `credential` table (`id, connector_id/integration_id, method_id, label, value, active`) with a unique index allowing **one active credential per connector**. A migration imports the legacy `auth.json`. The credential API has `create`, `activate`, `update` and labels, so **several accounts per provider with one active** is a v2 design. — Source: SRC/core/src/database/migration/20260611035744_credential.ts; DEV `packages/core/src/credential.ts`; DEV `packages/core/src/database/migration/20260805200742_import_legacy_credentials.ts`
- **[Verified-secondary]** A user issue says "the TUI (`/connect`) and CLI (`opencode auth switch`) support adding, activating, renaming, and deleting accounts", but Desktop only has an add flow. — Source: https://github.com/anomalyco/opencode/issues/50556 (open, 2026-09-22). **[Negative-evidence]** `auth switch` is not in the v1.19.3 `providers`/`auth` command (subcommands are `list`, `login`, `logout`), so it is v2-only (S-3).
- **[Verified-primary]** OpenCode Console account login (`opencode console login|logout|switch|orgs|open`, hidden commands) is stored in DB tables `account` (`email, url, access_token, refresh_token, token_expiry`) and `account_state` (`active_account_id, active_org_id`). — Source: SRC/opencode/src/cli/cmd/account.ts; SRC/core/src/account/sql.ts
- **[Verified-secondary]** Multi-account demand: #18391 "multiple ChatGPT accounts" was closed not-planned (2026-03-20). #36778 (load-balancing across accounts) was closed as a duplicate of #8145/#5391/#23620/#29085. — Source: GitHub issues
- **[Inferred]** **Token collision risk with AI Juggler's Codex accounts:** if a user connects OpenCode with "ChatGPT Plus/Pro" for the same ChatGPT account that a Codex home already holds, and OpenAI refresh tokens are single-use (codex.md §10), the two will sign each other out. The adapter must warn, not share.

## 4. Sessions

- **[Verified-primary]** Session schema (SQLite): `session(id, project_id, workspace_id, parent_id, slug, directory, path, title, version, share_url, summary_*, metadata, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write, revert, permission, agent, model, time_*)`, plus `message`, `part`, `todo`, `session_message`, `session_input` and `session_context_epoch`. **Per-session cost and tokens are columns.** — Source: SRC/core/src/session/sql.ts
- **[Verified-primary]** CLI: `opencode` (TUI) and `opencode run "<prompt>"` (non-interactive), with `--continue/-c`, `--session/-s <id>`, `--fork`, `--model/-m provider/model`, `--agent`, `--file`, `--attach <url>`, `--dir`, and **`--format json`** for raw JSON events. `opencode session list [--format json]`, `opencode session delete <id>`, `opencode export [id] [--sanitize]` (JSON), `opencode import <file>`. — Source: OD/cli/
- **[Verified-primary]** **Server mode:** `opencode serve` gives a headless HTTP server on `127.0.0.1:4096`, with HTTP basic auth via `OPENCODE_SERVER_PASSWORD` (user `opencode`, or `OPENCODE_SERVER_USERNAME`). The OpenAPI 3.1 spec is at `/doc`. Endpoints include `POST /session`, `GET /session/:id`, `POST /session/:id/message`, `POST /session/:id/prompt_async`, `POST /session/:id/abort`, `GET /event` (SSE; first event `server.connected`), `PUT /auth/:id`, `GET|PATCH /config`, `GET /provider`, and `/tui/*` to drive a TUI. "Running `opencode` starts both a TUI and a server"; `opencode attach [url]` connects a TUI to a running server; `opencode web` adds a web UI. — Source: OD/server/, OD/cli/
- **[Verified-primary]** **SDK:** `@opencode-ai/sdk`. `createOpencode()` starts a server (default 127.0.0.1:4096) and returns a client. `createOpencodeClient({baseUrl})` attaches to a running server. API groups: session (create/list/prompt/abort/share…), `event.subscribe()` (SSE), `auth.set()`, config, project, files, tui. No stability statement on the page. — Source: OD/sdk/
- **[Verified-primary]** ACP is supported (`opencode acp`). — Source: OD/acp/, SRC/opencode/src/cli/cmd/acp.ts
- **[Inferred]** A read-only indexer can read `opencode.db` (SQLite, WAL likely) for session titles, cwd (`directory`), model and cost without a network call. Reading while OpenCode writes needs read-only, WAL-safe access. The schema changes often (40+ migrations since 2026-01), so pin to a schema check.

## 5. Usage, quota & subscriptions

- **[Verified-primary]** `opencode stats [--days N] [--tools] [--models] [--project]` shows token usage and cost, computed from the session table. — Source: OD/cli/, SRC/opencode/src/cli/cmd/stats.ts
- **[Verified-primary]** **OpenCode Zen** is pay-as-you-go: "Add $20 Pay as you go balance" plus a $1.23 card fee, "pay per request with zero markups", auto top-up of $20 when the balance falls to $5, monthly spend limits, and a refund within 14 days if credits are unused. Models are hosted in the US/EU with zero-retention providers (exceptions: OpenAI/Anthropic retain 30 days). — Source: https://opencode.ai/zen; OD/zen/ (now rendered as "OpenCode Console")
- **[Verified-primary]** **OpenCode Go** costs **$10/month**; **Go Plus** costs **$40/month** ("Higher usage limits"). Go is a curated set of mostly open models (GLM-5.3, Kimi K3, DeepSeek V4, Qwen3.8, MiniMax M3, Grok 4.7, GPT 6 Luna, Claude Haiku 5.5, **Muse Spark 1.2/1.3 Contributor**, Gemini 4 Argon Contributor, and others). Limits are **dollar-denominated per model**: a 5-hour window = 20% of the monthly allowance, weekly = 50%, monthly = 100%. Example: a "$15" model allows ≈ $3 / 5 h and $7.50 / week; Go Plus allowances are $60–$240 per model. At the limit you can use free models (200 requests/day) or turn on "Extra usage" from the Zen balance. "Go and Go Plus are for single-member workspaces." "You can track your current usage in the console." — Source: OD2/console/go/; https://opencode.ai/go
- **[Verified-primary]** Go requires clients to send a stable session id (`x-opencode-session`); "Traffic is monitored for abuse". Validated clients: **Claude Code, Codex, Hermes (≥ v0.21.6), ZCode, Pi, jcode, Kilo Code CLI**. — Source: OD2/console/go/
- **[Verified-primary]** **Go works from Claude Code and Codex directly:**
  - Claude Code: `ANTHROPIC_BASE_URL=https://opencode.ai/inference/go/anthropic`, `ANTHROPIC_AUTH_TOKEN=oc_sk_…`, and model ids such as `minimax-m3` / `claude-haiku-5-5`; "If you're signed in to a Claude account, run `/logout` first."
  - Codex: a `[model_providers.opencode-go]` block with `base_url = "https://opencode.ai/inference/go/openai/v1"`, `env_key = "OPENCODE_API_KEY"`, `wire_api = "responses"`; "Codex only reads model providers from `~/.codex/config.toml`".
  - Source: OD2/console/harnesses/claude-code/, OD2/console/harnesses/codex/
- **[Verified-secondary]** "OpenCode Black" was a set of higher tiers ($20 / $100 / $200 reported), with sign-ups reported paused. `opencode.ai/black` now shows the Go/Go Plus page. — Source: https://horadecodar.com.br/?p=40107 (secondary); https://opencode.ai/black (shows Go)
- **[Negative-evidence]** No public API or CLI command for Go's 5h/weekly/monthly usage was found. #16017 "Add Go plan usage/balance API endpoint" (138 reactions) is closed, but users were still asking in Aug–Sep 2026 and pointing to third-party plugins that read the console session. Go docs point only to the console (S-1).
- **[Inferred]** The best sanctioned Go signal is local: the session `cost` columns (priced from Models.dev) summed over rolling 5h/7d/30d windows per model, compared with the published per-model dollar allowances. That is an **Estimate**, never Measured. Server-side pooling bugs (#52962, #49014) show the server's accounting can differ from the docs.

## 6. Extensibility

- **[Verified-primary]** MCP: an `mcp` key in `opencode.json`, `type: local` (command) or `remote` (url, headers, OAuth with dynamic client registration). Commands: `opencode mcp auth|list|logout|debug`. — Source: OD/mcp-servers/
- **[Verified-primary]** Rules: project `AGENTS.md` (searched upward), global `~/.config/opencode/AGENTS.md`, `CLAUDE.md` fallbacks, and an `instructions` array (files, globs, URLs). — Source: OD/rules/
- **[Verified-primary]** Plugins: JS/TS in `.opencode/plugins/` or `~/.config/opencode/plugins/`, or npm packages in `plugin: []` (installed with Bun into `~/.cache/opencode/node_modules/`). Events include `session.idle`, `session.status`, `session.error`, `permission.asked`, `tool.execute.before/after`, `message.updated`, `server.connected`. — Source: OD/plugins/
- **[Verified-primary]** Also: custom agents, commands, skills, custom tools, LSP, formatters, permissions (`OPENCODE_PERMISSION` inline), GitHub/GitLab integrations, and an IDE extension built on the server. — Source: OD/ sidebar

## 7. Terms of service

Source: **OpenCode Terms of Service, effective 2026-08-15** — https://opencode.ai/legal/terms-of-service (Anomaly Innovations, Inc.).

- **[Verified-primary] (a) Multiple accounts:** prohibited when used to dodge limits: users may not create, maintain or use "multiple accounts to circumvent usage limits, access restrictions, billing obligations". No flat one-account-per-person rule was found (S-2).
- **[Verified-primary] (b) Automation:** the restrictions include "automatically or programmatically extracts data or Output", and processes that run "while you are not logged into the Services", plus "placing an unreasonable load". Driving the MIT-licensed CLI locally is ordinary use **[Inferred]**. Scraping the console for Go usage is closer to the line **[Inferred]**.
- **[Verified-primary] (c) Wrappers / third parties:** "You will only use the Services for your own internal use, and not on behalf of or for the benefit of any third party". The client itself is MIT, so wrapping the CLI is fine **[Inferred]**. Go explicitly expects third-party harnesses ("You can use it with OpenCode or any agent").
- **[Verified-primary]** Provider terms still apply: the docs relay Anthropic's prohibition on Claude Pro/Max through third-party tools. — Source: OD/providers/

## 8. Pain points (community)

- **Go usage opacity:** #16017 (138 reactions), #31084, #40584 (`/usage` for Go in TUI), #43983, #49693 (report plan usage to ACP clients, open). Third-party "opencode-go-usage" plugins fill the gap.
- **Go limit accounting:** #52962 "single account-wide usage pool contradicts documented per-model limits"; #49014 "5-hour usage limit blocks all models after one model hits its own limit".
- **Multi-account:** #18391 (closed, not planned), #36778, #21061 (multiple Copilot accounts), #50556 (Desktop lacks the account switcher).
- Source: GitHub search API `repo:anomalyco/opencode … in:title` (S-1, S-3).

## Verdict for AI Juggler [Inferred]

| Surface | Verdict | Size | Why |
|---|---|---|---|
| **CLI (v1 and v2)** | **Full adapter** | **M** | Per-account isolation via XDG roots (+ `OPENCODE_CONFIG_DIR`); sessions in one SQLite DB with cost/token columns; `run --format json`, `serve` (OpenAPI + SSE) and an SDK give a real control plane; resume by session id. |
| **Desktop** | **Track-only** | **S** | Same data dir as the CLI (index sessions from `opencode.db`, detect `ai.opencode.desktop`), but no documented way to start it per account; single-instance Electron. |
| **OpenCode Go / Go Plus** | **Account type on Claude Code and Codex** (and an account type on the OpenCode adapter) | **S** | Vendor-documented base URL + `oc_sk_` key for both tools. Quota = Estimate only. |
| **OpenCode Zen** | **Account type** (pay-as-you-go balance) | **S** | Same key/endpoint pattern; balance not readable via a sanctioned API (S-1). |

**Blockers**

1. Two live lines (v1 1.19.3 with `auth.json`; v2 2.1.x with the `credential` table). The adapter must detect the version and handle both stores, or support only one.
2. The XDG variables leak into child tools. This needs a live test that agent-spawned shells behave.
3. No Go usage API. The quota is always shown as an estimate.
4. Do not offer "Claude Pro/Max" sign-in through OpenCode (Anthropic prohibition relayed by the vendor). Warn about ChatGPT token collisions with Codex homes.

**Open questions for the maintainer**

- Support v1, v2 or both? (`brew` and `/download` already push v2.)
- Is "go subscriptions" a request to add **OpenCode Go as an account type on the existing Claude Code / Codex accounts** (cheap, vendor-documented), as an OpenCode adapter, or both?
- Is an estimated Go quota (from local cost columns) acceptable, given there is no sanctioned meter?

## Search log (Negative-evidence)

- **S-1 Go usage API** (2026-10-21). Queries: WebSearch "OpenCode Go usage API endpoint remaining limit programmatic"; GitHub search `repo:anomalyco/opencode go usage limit in:title` (32 hits); read #16017, #31084 (comments + timeline). Sources: OD2/console/go/ ("track your current usage in the console"), https://opencode.ai/go, OD/cli/ (no usage command besides `stats`). Result: no documented endpoint or CLI command for Go plan usage was found by this search.
- **S-2 One-account rule** (2026-10-21). WebFetch of https://opencode.ai/legal/terms-of-service asking for accounts-per-person, sharing, automation and reselling clauses. Only the circumvention clause and the internal-use clause were found. No "one account per person" clause was found by this search.
- **S-3 `opencode auth switch` in v1** (2026-10-21). Read SRC/opencode/src/cli/cmd/providers.ts (commands `providers` alias `auth`: `list`, `login`, `logout`). OD/cli/ lists only login/list/logout. It was not found in v1.19.3. Issue #50556 attributes it to the current (v2) CLI.
- **S-4 Desktop per-account launch** (2026-10-21). Read OD/troubleshooting/, https://opencode.ai/download, SRC/desktop/src/main/{server,sidecar}.ts, and the Homebrew cask. No documented profile, data-dir flag or multi-instance option for Desktop was found by this search.
