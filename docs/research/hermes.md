# Research notes: Hermes (Nous Research Hermes Agent, and other "Hermes" candidates)

Research date: 2026-10-21; source pass 2026-11-13 (§10)

> Input to the multi-Provider evaluation. Mirrors the structure of [codex.md](codex.md). Nothing was installed, no account was signed in, no binary was downloaded, and no local `~/.hermes`, `~/.claude`, `~/.codex` or credential directory was read. Docs were read as Markdown sources from the vendor repo via raw.githubusercontent.com (the same files that build hermes-agent.nousresearch.com/docs). Every claim was accessed on 2026-10-21.
>
> **Legend (per [EVIDENCE-STANDARD.md](EVIDENCE-STANDARD.md)):**
> **[Verified-primary]** the vendor's own docs, source or terms say it ·
> **[Verified-secondary]** a reliable second-hand source (press, comparison site, another vendor's docs) says it ·
> **[Negative-evidence]** not found by this search; the search is logged under Search log (S-n) ·
> **[Inferred]** our own reasoning; never a fact.
>
> Short URL: `HD/` = `https://github.com/NousResearch/hermes-agent/blob/main/website/docs/`.

## 0. Identification: which "Hermes"?

| # | Candidate | Evidence | Likelihood |
|---|---|---|---|
| **1** | **Hermes Agent** by **Nous Research**: an open-source (MIT) agent harness with a CLI/TUI (`hermes`), Hermes Desktop (Electron), a gateway, messaging bots and cron, used for coding among other things | **[Verified-primary]** Repo `NousResearch/hermes-agent`, MIT, about 255.8k stars, "The agent that grows with you", release **v0.22.3 on 2026-10-21** — GitHub API. **[Verified-primary]** OpenCode Go lists "Hermes" (≥ v0.21.6) among the clients validated for its coding plan, next to Claude Code and Codex — https://opencode.ai/v2/docs/console/go/. **[Verified-secondary]** Comparisons with Claude Code, Codex and OpenCode (composio.dev, haimaker.ai, standardcompute.com). | **Most likely.** It is the one "Hermes" that is an AI coding agent with subscriptions (Nous Portal), and it sits in the same ecosystem lists as AI Juggler's tools. |
| 2 | **Hermes IDE** (`hermes-ide.com`, GitHub org `hermes-hq`): a desktop console that runs Claude Code, Codex, Antigravity CLI, OpenCode, Copilot CLI, goose, Aider, Kiro CLI and Hermes Agent side by side, one branch per task, with an inbox for agents waiting on you. BSL 1.1, free | **[Verified-primary]** Its own page: "a desktop console for coding agents from any vendor"; "not affiliated with Nous Research or its Hermes Agent" — https://hermes-ide.com/ | **Possible, but as a competitor, not a tool to supervise.** It overlaps heavily with AI Juggler's own pitch. Worth flagging to the maintainer. |
| 3 | Nous Research's **Hermes 4** models (Hermes-4-70B / 405B) | **[Verified-primary]** Available via Nous Portal; "not recommended for use inside Hermes Agent" (tuned for chat, not tool loops) — HD/integrations/nous-portal.md | A model family, not a tool. |
| 4 | hermesagents.net | **[Verified-secondary]** A fan site that says it is not connected to Nous Research (search snippet of https://hermesagents.net/terms) | Not a product. |
| 5 | Hermes, Meta's JavaScript engine for React Native | **[Inferred]** Well known, but not an AI coding agent | Ruled out. |

**Pick: candidate 1, Nous Research Hermes Agent.** The rest of this document is about it. Hermes IDE is noted as a competitor (see the open questions).

## 1. Distribution & versioning

- **[Verified-primary]** Install: Hermes Desktop (`Hermes.app` from a DMG; **Apple Silicon only** — "macOS on x86 (Intel) processors is not a supported platform"), or CLI-only `curl -fsSL https://hermes-agent.nousresearch.com/install.sh | bash`, Windows `install.ps1`/MSIX, Docker, Nix, or Termux. `hermes desktop` builds and launches Desktop from a CLI install. — Source: HD/getting-started/installation.md, HD/getting-started/platform-support.md
- **[Verified-primary]** Install layout (POSIX script): code in `~/.hermes/hermes-agent/`, entry point a `~/.local/bin/hermes` wrapper, user data in `~/.hermes/`. Desktop bundles carry their own runtime and use the "Platform default Hermes data directory". — Source: HD/getting-started/installation.md ("Install layout")
- **[Verified-primary]** `hermes --version` / `-V` prints the version; `hermes update [--check] [--backup]` updates. — Source: HD/reference/cli-commands.md
- **[Verified-primary]** Desktop is **Electron** with a React chat surface; it "can install the Hermes Agent runtime into `HERMES_HOME` … the same layout a CLI install uses, which is why the two are interchangeable." The macOS bundle id is **`com.nousresearch.hermes`** (used in the docs' `tccutil reset … com.nousresearch.hermes`). — Source: HD/user-guide/desktop.md
- **[Verified-primary]** Release cadence: v0.22.1 (2026-10-14), v0.22.2 (2026-10-15), v0.22.3 (2026-10-21). — Source: GitHub API `repos/NousResearch/hermes-agent/releases`
- **[Inferred]** Mac detection: `hermes` on `PATH` (often `~/.local/bin/hermes`), `hermes --version`; `/Applications/Hermes.app` with `CFBundleIdentifier` `com.nousresearch.hermes`; data at `$HERMES_HOME` or `~/.hermes`.

## 2. Configuration & local state

- **[Verified-primary]** **`HERMES_HOME`** "Select[s] the configuration and user-data home", defaulting to `~/.hermes` (POSIX) or `%LOCALAPPDATA%\hermes`. `get_hermes_home()` resolves a context override, then `HERMES_HOME`, then the platform default. "Changing `HOME` is not a safe way to select a named profile"; "subprocess spawners should pass" `HERMES_HOME`. — Source: HD/reference/environment-variables.md; HD/developer-guide/session-storage.md
- **[Verified-primary]** **Profiles are first-class isolated homes:** "A profile is a separate Hermes home directory", stored at `~/.hermes/profiles/<name>/` with its own `config.yaml`, `.env`, `SOUL.md`, memories, sessions, skills, cron jobs and `state.db`. They are selected with `hermes -p <name> …` or `hermes profile use`, or with generated shell aliases. Commands: `hermes profile list|use|create|delete|show|rename|export|import|…`. "Profiles use the `HERMES_HOME` environment variable." — Source: HD/user-guide/profiles.md; HD/reference/profile-commands.md
- **[Verified-primary]** Home contents: `config.yaml`, `.env` (API keys, bot tokens), `auth.json` (provider logins), `SOUL.md`, `memories/`, `skills/`, `state.db`, `logs/` (including `desktop.log`), `hooks/`, `desktop-plugins/`, `npmrc`. — Source: HD/user-guide/profiles.md, HD/user-guide/desktop.md, HD/user-guide/features/subscription-proxy.md
- **[Verified-primary]** Profile cloning warns that "Anthropic (Claude Pro/Max), OpenAI Codex, and xAI OAuth logins use **single-use refresh tokens** — a copy of one is not a second credential … the first profile to refresh it revokes it for every other copy." — Source: HD/user-guide/profiles.md
- **[Inferred]** Isolation is **better than Codex's**: one `HERMES_HOME` per account (or one Hermes profile per account) is the vendor-documented pattern.

## 3. Auth, providers & multiple accounts

- **[Verified-primary]** Hermes is **multi-provider** (Nous Portal, OpenRouter, Anthropic, OpenAI Codex, xAI, Gemini, Bedrock, Azure, local, and any OpenAI-compatible endpoint). Credentials are managed with `hermes auth` (`add|list|remove|priority|reset|status|logout`); `hermes login`/`logout` are deprecated. — Source: HD/integrations/providers.md; HD/reference/cli-commands.md
- **[Verified-primary]** Provider logins are stored in **`$HERMES_HOME/auth.json`** ("the same place all Hermes provider logins live"). — Source: HD/user-guide/features/subscription-proxy.md; HD/integrations/providers.md
- **[Verified-primary]** **Credential pools = multiple accounts per provider**, with rotation and cooldown on rate or plan limits (`fill_first`, priorities). "Only **different** OpenAI accounts rotate independently: two logins of the same account share one token family upstream, so OpenAI revokes the older one." — Source: HD/user-guide/features/credential-pools.md
- **[Verified-primary]** **Consumer-subscription sign-ins:**
  - ChatGPT/Codex: device-code OAuth (or `--browser` PKCE on `localhost:1455`). Hermes "can import existing Codex CLI credentials from `~/.codex/auth.json`".
  - **Anthropic OAuth works only on Claude Max with purchased extra-usage credits**: "the base Max allowance is never consumed by Hermes". Claude Pro cannot use the OAuth path.
  - SuperGrok / X Premium+ is also listed.
  - Source: HD/integrations/providers.md
- **[Verified-primary]** **Nous Portal** is Nous Research's own subscription: one OAuth login (`hermes portal` = `hermes auth add nous --type oauth`) covering "300+ frontier models" and a Tool Gateway (web search, image generation, TTS, browser, …). `hermes portal info|status|tools|open`. — Source: HD/integrations/nous-portal.md
- **[Verified-primary]** The optional **Codex App-Server Runtime** hands `openai*` turns to the Codex CLI app-server, which reads `~/.codex/auth.json`, registers an MCP callback in `~/.codex/config.toml`, and migrates MCP servers and plugins into it. — Source: HD/user-guide/features/codex-app-server-runtime.md
- **[Inferred]** **Conflict risk with AI Juggler's Codex/Claude accounts:** importing `~/.codex/auth.json` duplicates a single-use refresh token, and the Codex runtime **writes to `~/.codex/config.toml`**. An adapter must give Hermes its own `HERMES_HOME`, never point the Codex runtime at a Codex home that AI Juggler manages, and warn before a user adds the same ChatGPT or Claude account to both.

## 4. Sessions

- **[Verified-primary]** Sessions are in SQLite `$HERMES_HOME/state.db` (session metadata, full messages, model config), which replaced per-session JSONL. "The profile's gateway is the **only writer of canonical sessions**"; the CLI, TUI, Desktop, API server, ACP and cron are clients over `/api/ws`. The `sessions` table includes `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_write_tokens`, `reasoning_tokens`, `billing_provider`, `billing_mode`, `estimated_cost_usd`, `actual_cost_usd`, `cost_status` and `cost_source`. — Source: HD/developer-guide/session-storage.md
- **[Verified-primary]** Resume: `--resume/-r <id|title|latest>`, `--continue/-c [name]`, `--in <dir>`. Browse with `hermes sessions` (browse/export/prune/rename/delete); `hermes insights` shows token, cost and activity analytics. — Source: HD/reference/cli-commands.md
- **[Verified-primary]** Headless: `hermes chat --oneshot -q "…"`, `-Q/--quiet`, **`--format stream-json`** (JSONL), `hermes -z "…"` (final answer only) with **`--usage-file <path>`** (a JSON report: tokens and `estimated_cost_usd`), and documented exit codes. — Source: HD/reference/cli-commands.md
- **[Verified-primary]** **Three programmatic protocols** drive the same core: **ACP** (`hermes acp`, JSON-RPC over stdio), the **TUI gateway** (JSON-RPC over stdio or WebSocket, with `session.create`, `prompt.submit`, streamed events and approvals), and an **OpenAI-compatible API server** (`http://127.0.0.1:8642/v1`, `API_SERVER_KEY`; `/v1/chat/completions`, `/v1/responses`, `/v1/runs`). `hermes serve` is the headless backend that powers Desktop. The gateway "keeps running after you quit" Desktop. — Source: HD/developer-guide/programmatic-integration.md; HD/user-guide/features/api-server.md; HD/user-guide/desktop.md

## 5. Usage, quota & subscriptions

- **[Verified-primary]** **`hermes usage [--provider openai-codex|anthropic|openrouter] [--json]`** shows "the configured account's rate-limit windows": Codex 5-hour/weekly windows with plan and banked resets, Anthropic OAuth windows, and OpenRouter credits. It runs without starting a session; the `--json` keys are "stable; new keys may be added". It exits 1 when no credential is configured or the provider has no usage endpoint. — Source: HD/reference/cli-commands.md (`hermes usage`)
- **[Inferred]** Hermes reads Codex/Anthropic windows from somewhere that codex.md §9 judged unsanctioned for AI Juggler. **AI Juggler should not adopt Hermes' numbers as Measured for Codex/Claude accounts.** For a Hermes account, it may show them labelled as Hermes-reported.
- **[Verified-secondary]** Nous Portal tiers: **Plus $20/mo ($22 credit), Super $100/mo ($110 credit), Ultra $200/mo ($220 credit)**, plus a 10% discount on token-billed providers (comparison pages verified 2026-07-17). — Source: https://standardcompute.com/alternatives/nous-portal, https://standardcompute.com/flat-rate-ai-coding-plans/devpass-vs-nous-portal. **[Negative-evidence]** The official pricing page returned HTTP 429 twice (S-2).
- **[Verified-primary]** In-terminal billing exists (`/subscription`, `/topup`, tier changes with prorated charges; a "Daily plan-change limit … 5 per org"). — Source: HD/developer-guide/billing-lifecycle.md
- **[Verified-primary]** Nous may "set and change limits on use … and may throttle, suspend or restrict access" (Portal ToS §8.5). — Source: https://portal.nousresearch.com/terms

## 6. Extensibility

- **[Verified-primary]** Context files, first match wins: `.hermes.md`/`HERMES.md` → `AGENTS.override.md` → `AGENTS.md` → `CLAUDE.md` → `.cursorrules`; plus the global `$HERMES_HOME/SOUL.md`. — Source: HD/user-guide/features/context-files.md
- **[Verified-primary]** MCP (`hermes mcp`, and Hermes can run as an MCP server), plugins and a plugin catalog, skills, gateway hooks (`~/.hermes/hooks/<name>/HOOK.yaml` + `handler.py`), outbound webhooks (`hooks.outbound`), Desktop plugins, `hermes import-agent` (imports `~/.claude` / `~/.codex` setups), worktrees (`hermes -w`), cron and kanban. — Source: HD/reference/cli-commands.md; HD/user-guide/features/hooks.md; HD/user-guide/profiles.md

## 7. Terms of service

Sources: **Nous Portal Terms of Service, last updated 2026-09-28** — https://portal.nousresearch.com/terms. The Hermes Agent code is MIT (repo LICENSE).

- **[Verified-primary] (a) Multiple accounts:** §5.1 "Each Client may have only one Account." §10.1(7) bars using multiple accounts "to obtain additional Free Trials, credits or capacity". **Multiple Nous Portal accounts per person are not allowed.**
- **[Verified-primary] (b) Sharing:** §5.2 "Login Credentials must not be used by multiple persons."
- **[Verified-primary] (c) Automation:** §10.2(3) bars access "by means of programs that send them automatic enquiries or requests"; §10.1(7) bars circumventing "any usage limit, rate limit or access control". **[Inferred]** Running the agent is the product's purpose, but an orchestrator that rotates accounts to stretch capacity would breach §10.1(7).
- **[Verified-primary] (d) Reselling:** §10.2(1) bars making the services "available in whole or in part to any third persons".
- **[Inferred]** Third-party providers' terms apply to their own subscriptions. Hermes' own docs restrict Anthropic OAuth to Max plans with extra-usage credits.

## Verdict for AI Juggler [Inferred]

| Surface | Verdict | Size | Why |
|---|---|---|---|
| **CLI / TUI (`hermes`)** | **Full adapter** | **M** | Vendor-documented isolation (`HERMES_HOME` / profiles), SQLite sessions with cost columns, stable `--format stream-json`, `hermes usage --json`, ACP and TUI-gateway JSON-RPC. The work is the gateway model: a long-lived per-profile daemon is the sole session writer, so the adapter must manage or attach to it. |
| **Desktop (`Hermes.app`)** | **Track-only** | **S** | Shares `HERMES_HOME` with the CLI; detect `com.nousresearch.hermes`; sessions are visible through `state.db` / the gateway. Per-account Desktop instances are not documented (S-3). |
| **Nous Portal subscription** | **Account type on the Hermes adapter** (one per person, ToS §5.1) | **S** | One OAuth login. Its subscription proxy can also serve other OpenAI-compatible clients, but that is out of scope. |

**Blockers**

1. The gateway daemon outlives the app and serves many profiles. Lifecycle ownership (start, stop, which `HERMES_HOME`) must be designed, similar to Codex's app-server daemon (codex.md §10).
2. Credential cross-talk with AI Juggler's Codex and Claude accounts (imported `auth.json`, the Codex runtime editing `~/.codex/config.toml`, single-use refresh tokens).
3. Nous Portal allows one account per person. An "add a second Nous account" flow must not be offered as a way to get more capacity.
4. Apple Silicon only on macOS.

**Open questions for the maintainer**

- **Which Hermes did you mean?** Nous Research **Hermes Agent** (assumed here), or **Hermes IDE** (hermes-ide.com, a multi-agent console that competes with AI Juggler)?
- Should a Hermes account be allowed to use a ChatGPT or Claude subscription that AI Juggler already manages as a Codex or Claude Code account, or must those stay disjoint?
- Is "Hermes-reported" quota (from `hermes usage --json`) acceptable to show, given codex.md §9's ruling on unsanctioned Codex limit feeds?

## 10. Source pass (D-106, 2026-11-13, AJ-E1607)

The MIT source was read, never run or installed: `NousResearch/hermes-agent`
at **`422ad1861856dd9b80b947fbad9f39d1580f6b41`** (main, 2026-11-13; the
newest tag was `v2026.9.24`, and `hermes --version` prints
`Hermes Agent v<version> (<release date>)`, `hermes_cli/_startup_fast.py:195`,
followed by a passive update check, `:236`). Every line below is
**[Verified-primary]** from that commit unless marked; paths are in the
repo. Hermes isn't installed on the build Mac (installing it needs the
maintainer's yes), so nothing here ran live: AJ-E1612's lab checks it.

### 10.1 What moves its state

| Variable | What it moves | Source |
|---|---|---|
| `HERMES_HOME` | The whole home: `config.yaml`, `.env`, `auth.json`, `SOUL.md`, memories, skills, `state.db`, logs, `gateway.pid`. Context override, then the variable, then `~/.hermes` | `hermes_constants.py:206` (`get_hermes_home`) |
| (the root) | A home outside `~/.hermes` is its own root; `<root>/profiles/<name>` belongs to `<root>` | `hermes_constants.py:111` (`_hermes_root_of`), `:314` |
| `HERMES_SHARED_AUTH_DIR` | The Nous Portal sign-in shared by every profile of a root, default `<root>/shared/nous_auth.json` (written on login and every refresh) | `hermes_cli/auth_nous.py:350` |
| `HERMES_GATEWAY_LOCK_DIR` | The gateway's host lock and rendezvous record, default `$XDG_STATE_HOME/hermes/gateway-locks` (else `~/.local/state/…`): **not under the home**, one per OS user | `gateway/status.py:321` (`_get_lock_dir`); `gateway/host_rendezvous.py:1-31, 217` |
| `HERMES_MANAGED_DIR` | An administrator's policy folder (`config.yaml` and `.env` whose values win over the user's), default `/etc/hermes`; launch-only, so no `.env` can set it | `hermes_cli/managed_scope.py:31, 45`; `hermes_constants.py:874`; `hermes_cli/config.py:2063`; `hermes_cli/env_loader.py:445` |
| `HERMES_KANBAN_HOME` | One kanban board across homes | `hermes_cli/kanban_db.py:405` |
| `HERMES_DATA_DIR_SUFFIX` | Moves the default home to `~/.hermes<suffix>` | `hermes_constants.py:69-76` |
| `CODEX_HOME`, `CLAUDE_CONFIG_DIR` | Where Hermes looks for the Codex CLI's and Claude Code's homes (see 10.2) | `hermes_cli/auth_codex.py:590`; `agent/anthropic_credentials.py:345` |

The process ledger (`spawn-ledger.json`) is at the root
(`hermes_cli/process_identity.py:38, 261`).

### 10.2 Sign-ins, and the Codex and Claude homes (the guardrail, Req.3)

- Provider sign-ins and credential pools: `$HERMES_HOME/auth.json`
  (`hermes_cli/auth.py:481`); keys and bot tokens in `$HERMES_HOME/.env`;
  the Nous Portal sign-in also in `<root>/shared/nous_auth.json` (10.1). A
  file, not the Keychain, for Hermes' own sign-ins.
- **Hermes borrows and refreshes other tools' sign-ins by default.**
  `auth.adopt_external_logins: True` (`hermes_cli/config_defaults.py:1738`):
  with no login of its own, it reads `$CODEX_HOME/auth.json` (else
  `~/.codex/auth.json`, `hermes_cli/auth_codex.py:587`) and Claude Code's
  `$CLAUDE_CONFIG_DIR/.credentials.json` **and the login Keychain item
  `Claude Code-credentials`** (`agent/anthropic_credentials.py:48, 359`),
  and writes a refreshed Claude token back (`:543`). Both refresh tokens
  are single-use, so one program's refresh signs the other out. With the
  setting false, none of them is read (`agent/credential_sources.py:32`,
  gating `auth_codex.py:252`, `anthropic_credentials.py:367`,
  `credential_pool.py:2558`).
- Its Codex app-server runtime writes MCP servers into
  `$CODEX_HOME/config.toml` (`hermes_cli/codex_runtime_plugin_migration.py:424`);
  its session import reads `$CODEX_HOME/sessions` and
  `$CLAUDE_CONFIG_DIR/projects` (`hermes_cli/foreign_sessions.py:176`).
- **Route used (AJ-E1607):** every run AI Juggler starts sets
  `HERMES_MANAGED_DIR` to a policy folder of the account's own
  (`config.yaml`: `auth.adopt_external_logins: false`; `.env`: `CODEX_HOME`
  and `CLAUDE_CONFIG_DIR` at empty folders), so neither the Keychain item nor
  a Codex or Claude home is read, whatever the user's own config or `.env`
  says. A client-started gateway inherits it (only supervisor markers are
  stripped, `hermes_cli/gateway_runtime_start.py:20-27`). A Mac with
  `/etc/hermes` is refused rather than overridden.
- **Gap that remains:** first-time `hermes setup` offers "Coming from another
  agent?" — an import of `~/.claude` or `~/.codex` found by `Path.home()`,
  not by those variables, run only on the user's yes through the
  `agent-importers` plugin (`hermes_cli/setup_agent_import.py`). AI Juggler
  never runs `hermes setup` or `hermes import-agent` (its sign-in is
  `hermes model`); whether a first `hermes` run reaches that step is a lab
  check.

### 10.3 Sessions

`$HERMES_HOME/state.db` (`hermes_state.py:182`), SQLite in WAL, schema
version 33 (`hermes_state_common.py:296`): `sessions` (`:468`: `id`,
`source`, `title`, `cwd`, `started_at`/`last_activity_at`/`ended_at` as REAL
epoch seconds, `parent_session_id`, `hidden`, `archived`, the token and cost
columns, `profile_name`) and `messages` (`:538`: `role`, `content`,
`timestamp`, …). Written only by the home's gateway. Resume:
`hermes --resume/-r <id|title|latest>`, `--continue/-c [name]`, `--in <dir>`
(`hermes_cli/_parser.py:181-195`; `hermes chat` takes `--resume` too, `:280`).

### 10.4 Usage

`hermes usage [--provider P] [--json]` (`hermes_cli/subcommands/usage.py:15,
38`): one JSON document — `provider`, `source`, `title`, `plan`,
`fetched_at`, `windows[]` (`label`, `used_percent`, `resets_at`, `detail`),
`details`, `unavailable_reason`; exit 1 with nothing on stdout when nothing
could be fetched. Readers exist for `openai-codex` (`Session`/`Weekly`),
`anthropic` (`Current session`, `Current week`, per-model weeks) and
`openrouter` (credits), plus a provider plugin's hook
(`agent/account_usage.py:911, 944`). **Nous Portal has none:** its credits
show only in a session's `/usage` (`agent/account_usage.py:263, 498`), so a
Nous Portal account gets no windows from `hermes usage`. Its usage cache is
in memory only (`agent/account_usage_cache.py`), no local file.

### 10.5 The gateway

- One per home records itself in `$HERMES_HOME/gateway.pid`
  (`{"pid","kind":"hermes-gateway","argv","start_time"}`, an older one a bare
  pid), `gateway.lock`, `gateway_state.json` (`gateway/status.py:40, 309,
  782`).
- One host owner per OS user per lock folder multiplexes every profile:
  `host-gateway.json`, `.lock`, a 0600 `.token`; a second start attaches
  instead of binding a second port; the owner answers `/api/host/identity`
  (`gateway/host_rendezvous.py:60, 217-227`). So an account needs a lock
  folder of its own, or its client attaches to the user's own gateway.
- A client with no gateway starts one: on macOS a one-off **launchd job**
  `com.nousresearch.hermes.runtime.<id>`, with `--idle-exit`
  (`hermes_cli/gateway_runtime_start.py:71`;
  `hermes_cli/gateway_launchd_runtime.py:26, 100`); elsewhere a detached
  (`setsid`) child.
- `hermes gateway run --external-supervisor` runs one in the foreground owned
  by its starter: restarts and updates exit back to it
  (`hermes_cli/subcommands/gateway.py:110`). `hermes gateway stop` stops the
  home's (`:159`), proving the process from its record first.
  `hermes gateway ensure|attach --json` start or find one and print its
  WebSocket URL with a single-use ticket (`:122, 133`).
- Ports: the API server `127.0.0.1:8642` (`gateway/platforms/api_server.py:155`);
  the gateway's own port is published in the host record.

### 10.6 Protocols for driving sessions (found, not built yet)

ACP over stdio (`hermes acp`, `hermes_cli/subcommands/acp.py:10`;
`acp_adapter/`), the TUI gateway's JSON-RPC (`hermes-gateway-v1` over
`/api/ws` with a ticket from `hermes gateway attach --json`, or stdio,
`tui_gateway/entry.py`; methods `session.create`, `prompt.submit`,
`session.resume`, approvals as server→client requests,
`website/docs/developer-guide/programmatic-integration.md`), and the
OpenAI-compatible API server. Headless `hermes -z` skips approvals
(`hermes_cli/_parser.py:150`), so AI Juggler doesn't offer it.

### 10.7 Terms, checked again

The Nous Portal Terms of Service were read again on 2026-11-13 ("Last update:
November 10, 2026"): §5.1, §5.2, §8.5 and §10.1(7) say what §7 above records
(§10.1(7) now reads "to circumvent any Plan, usage or rate limit or any
suspension"). Recorded in AJ-E1518's register.

## Search log (Negative-evidence)

- **S-1 Other "Hermes" coding agents** (2026-10-21). WebSearch "Hermes AI coding agent CLI" (extended); WebSearch `"Hermes" coding agent -Nous IDE OR CLI developer tool 2026` (extended). Results named only Nous Hermes Agent, Hermes IDE and fan sites. No other AI coding agent named Hermes was found by this search.
- **S-2 Official Nous Portal pricing** (2026-10-21). WebFetch https://portal.nousresearch.com/manage-subscription → HTTP 429; WebFetch https://portal.nousresearch.com/pricing → HTTP 429; WebSearch "Nous Portal subscription price tiers Hermes Agent per month" (third-party results only). HD/integrations/nous-portal.md and HD/developer-guide/billing-lifecycle.md have no prices. The official tier prices were not read first-hand in this search.
- **S-3 Desktop per-account instances** (2026-10-21). Read HD/user-guide/desktop.md (93 KB; grep for `HERMES_HOME`, profile, Application Support, bundle) and HD/guides/desktop-native-signin.md. Desktop follows `HERMES_HOME` and has per-gateway sign-in, but no documented way to run two Desktop instances bound to different homes was found by this search.
