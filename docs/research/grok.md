# Research notes: xAI Grok (Grok Build, Grok Bot, The X Subscription, xAI API)

Research date: 2026-10-21

> Compiled 2026-10-21 for AI Juggler's Provider roadmap. Mirrors the section layout of [codex.md](codex.md).
> Method: vendor docs (docs.x.ai serves every page as Markdown at `<url>.md`), the open-source `xai-org/grok-build` repository read through the GitHub API, the xAI OpenAPI spec, and archived copies of x.ai pages.
> **Nothing was installed, signed in to, or run.** No binary was downloaded. Every claim below is from documents or source code, not from a live run, so none of it has passed the billing protocol's live-capture step (AJ-E601) yet.
>
> **Accessed:** every source below was read on **2026-10-21**. **Labels** (docs/research/EVIDENCE-STANDARD.md): **[VP]** Verified-primary · **[VS]** Verified-secondary · **[NE-n]** Negative-evidence, with its search log in §11 · **[I]** Inferred.
>
> Notes on sources:
> - x.ai returns HTTP 403 to non-browser clients (Cloudflare). The terms, AUP and pricing pages were read from Wayback Machine snapshots of the vendor's own pages. Snapshots: ToS `20261021055255`, AUP `20261019192323`, pricing `20261019192310`. They are labelled [VP] because they are the vendor's text, with the snapshot noted.
> - `SRC/` = `https://github.com/xai-org/grok-build/blob/main/` (repo synced 2026-09-29, `SOURCE_REV` 559751fd…, crate `xai-grok-pager` version **1.0.45**).
> - `UG/` = `SRC/crates/codegen/xai-grok-pager/docs/user-guide/` (the user guide that ships in the repo and is extracted to `~/.grok/docs/user-guide/` on launch).
> - The vendor now brands itself "SpaceXSI (xAI)" in the docs and "SpaceXAI LLC" in the site footer. Grok, Cursor and X are sold together as "The X Subscription".

## 0. Line-up at a glance

| Surface | What it is | Official? | AI Juggler verdict (see §10) |
|---|---|---|---|
| **Grok Build** CLI/TUI (`grok`) | Terminal coding agent, Rust, Apache-2.0 | Official | **Full adapter, M** |
| Grok Build over ACP (`grok agent stdio`) | Editor and orchestrator protocol for the same binary | Official | Part of the Grok Build adapter |
| **Grok Bot** desktop app | Cloud agents ("Bots") on a cloud computer run by Cursor | Official | **Not feasible** (cloud, not local sessions) |
| Grok in **Cursor** | Cursor IDE and agent, same subscription pool | Official (same company) | Out of scope (AI Juggler doesn't support Cursor) |
| **xAI API** (`api.x.ai`) | Pay-as-you-go API keys | Official | Account type *inside* the Grok Build adapter (`XAI_API_KEY`). Not a Claude Code account type (Anthropic compat deprecated). Codex use is undocumented (§9) |
| `superagent-ai/grok-cli` (`grok`, npm `grok-dev`) | Community terminal agent on the xAI API | **Unofficial** | Not feasible. Detect only to avoid a collision |
| "Grok Build for VS Code (Community)" and others | Community VS Code UIs over the official CLI via ACP | **Unofficial** | Not feasible (covered by the CLI adapter) |
| grok.com web, iOS and Android apps | Chat | Official | Out of scope |

## 1. Distribution & versioning (Grok Build)

- **[VP] What it is:** "a powerful and extensible coding agent", usable as an interactive TUI, headless in scripts, or through the Agent Client Protocol (ACP) — https://docs.x.ai/build/overview.
- **[VP] Open source:** `xai-org/grok-build`, Apache-2.0, Rust, created 2026-07-14, ~27.5k stars. External contributions are not accepted, issues are disabled (`has_issues=false`), and the repo is "synced periodically from a monorepo" — https://github.com/xai-org/grok-build (GitHub API).
- **[VP] Timeline:** the docs release notes record "Grok Build is now available in beta" and a coding model `grok-build-0.1` "in early access" — https://docs.x.ai/developers/release-notes. **[VS]** Third-party write-ups put v1.0 / GA on 2026-08-07 (https://blog.buildfastwithai.com/grok-build-xai-cli-ai-agents-2026). The source crate is at **1.0.45** (`SRC/crates/codegen/xai-grok-pager/Cargo.toml`).
- **[VP] Platforms:** macOS, Linux and Windows (PowerShell, Git Bash, MSYS2; WSL uses the Linux binary) — `UG/01-getting-started.md`.
- **[VP] Install:** `curl -fsSL https://x.ai/cli/install.sh | bash` (macOS/Linux); `irm https://x.ai/cli/install.ps1 | iex` (Windows); WinGet `xAI.GrokBuild`. A version can be pinned (`| bash -s 0.1.42`, `GROK_VERSION` on PowerShell) — `UG/01-getting-started.md`, https://docs.x.ai/build/overview.
- **[VP] Install location (macOS):** the bash installer downloads to `~/.grok/downloads/grok-<platform>` and installs `~/.grok/bin/grok`, overridable with `GROK_BIN_DIR`. It also writes completions to `~/.grok/completions/` and may write `~/.grok/managed_config.toml` / `requirements.toml` — `SRC/crates/codegen/xai-grok-pager/scripts/install.sh` (lines ~202–399). The installer always uses `$HOME/.grok` and ignores `GROK_HOME`.
- **[VP] Detect on a Mac:** binary `grok` (built artefact `xai-grok-pager`); `grok --version` or `grok version`; `grok inspect [--json]` reports the config it discovered — `UG/01-getting-started.md`, https://docs.x.ai/build/cli/reference. There is no `.app` bundle: Grok Build is CLI-only [NE-3].
- **[I] Detection pitfall:** the unofficial `superagent-ai/grok-cli` also installs a binary named `grok` and uses `~/.grok/` (`user-settings.json`) — https://github.com/superagent-ai/grok-cli. Detection must tell them apart by path (`~/.grok/bin/grok`) and by `grok --version` output, not by name.
- **[VP] Updates:** `grok update` (`--check`, `--version <V>`, `--alpha`, `--stable`); `cli.auto_update` / `GROK_DISABLE_AUTOUPDATER`; `--no-auto-update` per run; channels `stable|alpha` — https://docs.x.ai/build/cli/reference, `UG/26-config-reference.md`, https://docs.x.ai/build/cli/headless-scripting.

## 2. Configuration system

- **[VP] `GROK_HOME`:** "Home directory: `~/.grok`. Set `GROK_HOME` to override it." Config is `$GROK_HOME/config.toml` (TOML); TUI appearance is in `pager.toml` — `UG/05-configuration.md`, https://docs.x.ai/build/settings. Source: `GROK_HOME` is used "verbatim when non-empty, else `<home>/.grok`" — `SRC/crates/codegen/xai-dirs/src/lib.rs`.
- **[VP] Sessions follow `GROK_HOME`:** "Base directory: `~/.grok`. Set `GROK_HOME` to change it" — `UG/17-sessions.md`.
- **[VP] `GROK_AUTH_PATH`** overrides the `auth.json` location; otherwise it is `$GROK_HOME/auth.json` — `SRC/crates/codegen/xai-grok-login/src/storage.rs` L47–56.
- **[VP] Layers (highest first):** CLI flags → env vars → `requirements.toml`/MDM → `GROK_CONFIG` (inline JSON) / `GROK_CONFIG_PATH` overlay → `~/.grok/config.toml` → `managed_config.toml` → defaults. A project `.grok/config.toml` may set only `[mcp_servers]`, `[plugins]` and `[permission]` — `UG/05-configuration.md`. `/etc/grok/requirements.toml` and `managed_config.toml` are system-wide — https://docs.x.ai/build/enterprise.
- **[VP] Reads other tools' config from the real HOME.** `[compat.claude]` / `[compat.cursor]` booleans (`agents`, `hooks`, `mcps`, `rules`, `skills`) and `[compat.codex]` (`hooks`, `skills`), with env overrides `GROK_CLAUDE_<X>_ENABLED` / `GROK_CURSOR_<X>_ENABLED` — `UG/26-config-reference.md`. In practice Grok loads:
  - rules from `~/.claude/`, `~/.cursor/`, `.claude/rules/`, `.cursor/rules/` and `CLAUDE.md` — https://docs.x.ai/build/features/project-rules;
  - MCP servers from `~/.claude.json`, `.cursor/mcp.json` and project `.mcp.json` — https://docs.x.ai/build/features/mcp-servers;
  - hooks from `.claude/settings.json` and `.cursor/hooks.json` — https://docs.x.ai/build/features/hooks.
  
  `home_dir()` (HOME, not GROK_HOME) is used "for sibling dot dirs (`~/.claude`, `~/.agents`, ...)" — `SRC/crates/codegen/xai-dirs/src/lib.rs` L11. The defaults were not stated in the docs [NE-8].
- **[VP] Importers:** `grok import [targets...]` imports Claude Code sessions — https://docs.x.ai/build/cli/reference. The source scanner honours `CLAUDE_CONFIG_DIR` and `CODEX_HOME` — `SRC/crates/codegen/xai-grok-foreign-sessions/src/claude.rs` L31–47, `…/codex/mod.rs` L19–39. `/import-claude` imports settings from `~/.claude` — `UG/04-slash-commands.md`.
- **[VP] Claude-compat flags:** `--allowedTools`, `--disallowedTools`, `--append-system-prompt`, `--system-prompt` and `--dangerously-skip-permissions` are accepted as aliases — https://docs.x.ai/build/cli/reference. The Claude Code `managed-settings.json` is partly honoured — https://docs.x.ai/build/enterprise.
- **[VP] Session cleanup:** `storage.cleanup_ttl_days` deletes idle session folders (unset/0 = off) — `UG/26-config-reference.md`.

## 3. Auth & multiple accounts

- **[VP] Sign-in methods** — `UG/02-authentication.md`:
  - browser OAuth on first `grok` / `grok login` ("SpaceXAI OAuth at `auth.x.ai`");
  - `grok login --device-auth` / `--device-code`;
  - `XAI_API_KEY` from console.x.ai (used only when no session token is active);
  - customer OIDC (`[grok_com_config.oidc]` or `GROK_OIDC_ISSUER` / `GROK_OIDC_CLIENT_ID`);
  - external token command (`GROK_AUTH_PROVIDER_COMMAND`).
  
  Precedence: per-model key → session token in `auth.json` → `XAI_API_KEY`.
- **[VP] Token storage:** `~/.grok/auth.json` (0600), MCP OAuth in `~/.grok/mcp_credentials.json` (0600) — `UG/02-authentication.md`. No Keychain use was found [NE-6].
  - `auth.json` is a map keyed by scope (e.g. legacy `https://accounts.x.ai/sign-in`, `xai::api_key`).
  - Each `GrokAuth` entry holds `key`, `auth_mode` (`web_login|oidc|external|api_key`), `user_id`, `email`, `team_id`/`team_name`, `refresh_token`, `expires_at`, and more. `UserInfo` can carry a live `subscription_tier` — `SRC/crates/codegen/xai-grok-login/src/model.rs` L1–107, L295–306.
  - Writes are atomic and guarded by a file lock with a heartbeat (`AuthFileLock`) — `storage.rs` L11–40, L191–194.
  - The adapter must never read or copy `auth.json` contents.
- **[VP] Switching:** "`grok login` … replaces your cached session"; `grok logout` clears it — `UG/02-authentication.md`. Running several accounts at once is not described [NE-1].
- **[I] Parallel accounts:** one `GROK_HOME` per account (or `GROK_AUTH_PATH` per account) is the natural pattern, mirroring `CODEX_HOME`.
- **[VS] A competitor already ships this.** AgentsRoom runs "one directory per Grok account, pointed at by GROK_HOME", watches for `auth.json`, and pins accounts per project, alongside Claude, Codex and Cursor accounts — https://agentsroom.dev/features/grok-multi-account.
- **[U] Refresh-token rotation:** whether two homes holding the same identity sign each other out (as Codex's single-use refresh tokens do) was not found in docs or the files read.
- **[VP] Enterprise pinning:** `force_login_team_uuid` rejects personal logins (maps to Claude's `forceLoginOrgUUID`) — https://docs.x.ai/build/enterprise.

## 4. Sessions

- **[VP] Location:** `~/.grok/sessions/<url-encoded-cwd>/<session-id>/` (long names → slug + hash + `.cwd` file). IDs are UUIDv7 — `UG/17-sessions.md`.
- **[VP] Files per session** — `UG/17-sessions.md`:
  - `updates.jsonl` — the ACP update stream, "authoritative log used for `/resume`";
  - `chat_history.jsonl`, `rewind_points.jsonl`, `feedback.jsonl`;
  - **`summary.json`** — title, timestamps, model ID, message counts, parent session;
  - **`signals.json`** — "token usage and tool/turn counters";
  - `plan.json`, `system_prompt.txt`, `prompt_context.json`, `tool_definitions.json`, `compaction_checkpoints/`, `subagents/`.
- **[VP] Resume:**
  - `grok --resume [<id-or-title>]`, `grok -c/--continue`;
  - headless `grok -p "…" -r <id>` / `-c`;
  - `-s/--session-id <UUID>` creates a session with a chosen ID;
  - `--fork-session`, `/fork`, `-w` worktrees;
  - ACP `session/load` — `UG/17-sessions.md`, https://docs.x.ai/build/features/sessions.
- **[VP] Listing and export:** `grok sessions list|search|delete` (search uses a local SQLite FTS5 index "and also returns remote results"); `grok export <id> [file]` (Markdown); `grok usage <session-id> [turn]` gives token and cost totals — `UG/17-sessions.md`, https://docs.x.ai/build/cli/reference.
- **[VP] Headless output:** `--output-format plain|json|streaming-json|streaming-messages-json` — `UG/14-headless-mode.md`.
  - `json` carries `sessionId`, `stopReason`, `usage`, `modelUsage`, `total_cost_usd`, and `total_cost_usd_ticks` (1 USD = 10^10).
  - `streaming-messages-json` mirrors Claude Code's stream-json (`system/init` with `apiKeySource` `user|oauth`, `assistant`, `user`, `result` with `total_cost_usd`, `usage`).
  - "Missing cost means unreported, not free."
  - Exit codes: 0 ok, 1 auth/network/runtime, 130 SIGINT, 143 SIGTERM.
- **[VP] Live roster:** `/dashboard` / `grok dashboard` lists live top-level sessions; `cli.session_registry` controls "the cross-process session registry" — https://docs.x.ai/build/features/dashboard, `UG/26-config-reference.md`.

## 5. Usage & quota

- **[VP] Plans (The X Subscription, new customers):**

  | Plan | Price | Usage |
  |---|---|---|
  | Free | $0 | "Limited" |
  | Plus | $8/mo ($80/yr) | Base |
  | Premium | $30/mo ($300/yr) | 4x |
  | Super | $100/mo ($1000/yr) | 15x |
  | Ultra | $200/mo ($2000/yr) | 50x |

  Teams cost $30/$100/$200 per seat (Plus isn't available for teams). Grok Bot is included from Premium up — https://docs.x.ai/x-subscription/pricing. The pricing page lists Grok Build as "Limited" on Free — x.ai/pricing (snapshot 20261019192310).
- **[VP] Legacy plans continue:** "Existing Cursor, SuperGrok, and X Premium plans continue unless you change or cancel them" — https://docs.x.ai/x-subscription/pricing. Legacy tier names seen in vendor docs: SuperGrok, SuperGrok Plus, SuperGrok Heavy (https://docs.x.ai/grok-bot/overview). X Premium / Premium+ are billed by X, not xAI (https://docs.x.ai/grok/faq). **[VS]** Legacy prices: SuperGrok Lite $10, SuperGrok $30, SuperGrok Plus $100, SuperGrok Heavy $300/mo ($3,000/yr) — https://krater.ai/blog/grok-pricing, https://justinmckelvey.com/blog/supergrok-pricing.
- **[VP] Unification is one-way:** switching to The X Subscription combines Cursor, Grok and X accounts into one workspace and bill, and "The switch is permanent" — https://docs.x.ai/x-subscription/overview.
- **[VP] Limit model:** since June 2026 there is "one shared weekly usage pool" across Chat, Imagine, Voice and Build (and Grok Bot, and Grok models in Cursor). It is shown "as a percentage used", broken down by product (API, Build, Chat, Imagine, Voice), with a weekly reset date in Settings → Usage. Extra Usage Credits, Auto Top-Up and spending caps exist — https://docs.x.ai/grok/faq, https://docs.x.ai/x-subscription/usage-limits. No 5-hour window is documented for the subscription.
- **[VP] Where usage is visible:** grok.com Settings → Usage, grok.com/dashboard, and the Build `/usage` (alias `/cost`): "View credit usage and the account allowance, plus session context and token totals" — `UG/04-slash-commands.md`, https://docs.x.ai/x-subscription/manage.
- **[VP] The status line carries no quota.** The status-line JSON has cost, tokens and context fields, but "The payload contains no rate-limit data, quota, plan, or account information" — `UG/25-status-line.md`.
- **[VP] OpenTelemetry carries no tier:** `grok_code.token.usage` and `grok_code.api_request` metrics; "subscription tier is never exported" — `UG/24-monitoring-usage.md`.
- **[NE-2] No documented API for the subscription pool.** Build fetches billing internally (`Effect::FetchBilling`, `SRC/crates/codegen/xai-grok-pager/src/app/dispatch/status.rs`) from a non-public backend. Under the billing protocol that source is **prohibited**.
- **[VP] Exhaustion signal:** "402 Payment Required always means a credit or spend block here (Build pool and IC spend blocks)"; a 403 counts only when the body contains "run out of credits" — `SRC/crates/codegen/xai-grok-pager/src/app/dispatch/billing.rs` L60–66. Headless exits 1 with an `error` object; quota errors are not documented in the headless guide (`UG/14-headless-mode.md`).
- **[VP] Local token data:** per-session `signals.json`, `grok usage <id>`, and headless `usage` / `modelUsage` / cost fields (§4). Under the billing protocol these are **local-observable** candidates for per-turn tokens. They need a live capture before product use. The pool percentage stays **unknown**/Estimate.
- **[VP] API is billed separately:** "Paid plans include Grok API access. API usage is billed separately at standard model rates and doesn't draw from your plan's usage pool" — x.ai/pricing FAQ (snapshot).
  - API rates: grok-4.7 $2/$0.50/$6 per 1M; `grok-build-0.1` $1/$0.20/$2 — https://docs.x.ai/developers/pricing.
  - Tiered RPS/TPM by cumulative spend — https://docs.x.ai/developers/rate-limits.
  - `GET /v1/me` "Works with both API keys and OAuth tokens" — https://docs.x.ai/openapi.json.
  - "Grok 4.7 Fast … available only in Cursor and Grok Build … billed through your plan" — https://docs.x.ai/developers/release-notes.

## 6. Extensibility surface

- **[VP] MCP:** `grok mcp add|list|remove|doctor` (`--json`); `[mcp_servers.<name>]` in `config.toml` (stdio or `url`, `${VAR}` expansion); project scope `.grok/config.toml`; OAuth tokens in `~/.grok/mcp_credentials.json`; also loads `~/.claude.json`, `.cursor/mcp.json` and `.mcp.json` — https://docs.x.ai/build/features/mcp-servers.
- **[VP] Rules:** `AGENTS.md`, `Agents.md`, `AGENT.md`, `CLAUDE.md`, `Claude.md` and `CLAUDE.local.md` in each directory from repo root to cwd, plus `.grok/rules/*.md`. User-level rules are `$GROK_HOME/rules/`. `grok inspect` lists what loaded — `UG/12-project-rules.md`, https://docs.x.ai/build/features/project-rules.
- **[VP] Hooks:** JSON files in `~/.grok/hooks/*.json` and `<project>/.grok/hooks/*.json`, `type` `command|http`.
  - Events: `SessionStart/End`, `UserPromptSubmit`, `PreToolUse` (the only blocking event), `PostToolUse(/Failure)`, `PermissionDenied`, `Stop`, `StopFailure` ("ends with an API error"), `Notification`, `SubagentStart/Stop`, `Pre/PostCompact`.
  - Project hooks need trust (`~/.grok/trusted_folders.toml`) — https://docs.x.ai/build/features/hooks.
- **[VP] Embedding:** ACP via `grok agent stdio` (JSON-RPC on stdio; `session/prompt`, `session/update`, `session/load`). It needs approvals through ACP; `streaming-messages-json` is read-only — https://docs.x.ai/build/cli/headless-scripting, `UG/14-headless-mode.md`. The changelog names VS Code and "grok-desktop" as persistent ACP clients (`SRC/crates/codegen/xai-grok-shell/CHANGELOG.md` ~L2003).
- **[VP] Custom models:** `[model.<id>]` with `base_url`, `env_key`, `api_backend` `chat_completions|responses|messages` (Anthropic Messages supported) — `UG/11-custom-models.md`. So Grok Build can itself run Claude via an Anthropic key.
- **[VP] Other:** plugins and marketplaces, skills, subagents, `/loop`, `/goal`, workflows, sandbox profiles, `grok wrap`, `grok clone` (Grove) — https://docs.x.ai/build/cli/reference, `UG/27-grok-clone.md`.
- **[VP] SDKs (API, not the agent):** `xai_sdk` (Python), `@ai-sdk/xai`, OpenAI SDK against `https://api.x.ai/v1` — https://docs.x.ai/build/overview.

## 7. Terms of service

Sources:
- **Terms of Service – Consumer**, "Last Updated: October 12, 2026" (x.ai/legal/terms-of-service, snapshot 20261021055255);
- **Acceptable Use Policy** (x.ai/legal/acceptable-use-policy, snapshot 20261019192323);
- docs.x.ai X Subscription pages.

- **(a) Multiple accounts.**
  - **[VP]** The ToS prohibits sharing, not plural ownership: "You may not share your account credentials or make your account available to anyone else…" (ToS §Accounts).
  - **[NE-7]** No "one account per person" clause was found in the consumer ToS.
  - **[VP]** The X Subscription assumes people hold several accounts: "If you have more than one account, make sure you choose the account you intend to use" (https://docs.x.ai/x-subscription/sign-in); "people can belong to more than one workspace" (https://docs.x.ai/x-subscription/overview).
  - **[VP]** Grok Bot has a first-party "Switch account / Add account" menu (https://docs.x.ai/grok-bot/faq).
- **(b) Automation.**
  - **[VP]** The AUP prohibits "Accessing the Services through unauthorized automated or non-human means, whether through a bot, script, or otherwise", "using bots to access", and "circumventing any rate limits or restrictions" (AUP).
  - **[VP]** Grok Build's own docs present headless mode as "ideal for scripts, automations, or integration into other apps" (https://docs.x.ai/build/overview).
  - **[I]** Launching the official binary (TUI, `-p`, ACP) is authorised use. Scraping grok.com usage pages, or calling the internal billing endpoint, is on the prohibited side.
- **(c) Third-party wrappers.**
  - **[VP]** The AUP bans "Modifying, copying … reselling … reverse engineer" the Service.
  - **[VP]** The pricing FAQ: "Subscriptions are only sold directly … We do not authorize any resellers."
  - **[VP]** The FAQ warns about a third-party app reusing Grok credentials: "revoke and change those credentials for safety" (https://docs.x.ai/grok/faq, Grok Studio entry).
  - **[I]** A supervisor that never touches `auth.json` and launches the Apache-2.0 binary stays clear of these clauses.
- **[VP] Termination is discretionary:** suspension "at our sole discretion" for risk or breach; "inactive for more than 120 days" (ToS §9).

## 8. Pain points and context (community)

- **[VS]** Usage caps are opaque. Third-party guides note xAI "still prints no cap for any" tier (https://justinmckelvey.com/blog/grok-usage-limits); the docs only give the relative multipliers (4x / 15x / 50x Plus).
- **[VS]** Third-party sources disagree on Build's launch date and default model (https://innfactory.ai/en/ai-harness/grok-cli/, https://mer.vin/2026/05/grok-build-cli-xai-terminal-coding-agent-with-plan-mode-subagents-and-headless-ci/).
- **[VS]** No first-party VS Code extension. The most-used one is community-built over ACP (`PawelHuryn.grok-vscode-phuryn`) — https://www.productcompass.pm/p/grok-build-vscode [NE-4].
- **[VP]** The account model is in flux: X Subscription unification merges Cursor and Grok accounts permanently (§5).

## 9. Special case — can a Grok subscription power Claude Code or Codex?

- **[VP] Claude Code: no.**
  - The xAI API still lists `POST /v1/messages` ("compatible with the Anthropic API") in the OpenAPI spec, but the docs say "**The Anthropic SDK compatibility is fully deprecated**. Please migrate to the Responses API" — https://docs.x.ai/developers/rest-api-reference/inference/legacy, https://docs.x.ai/openapi.json.
  - It is API-key billing, not the subscription pool (§5 FAQ).
  - No xAI guide for Claude Code was found [NE-5].
- **[I] Codex: technically plausible, undocumented, API-billed only.** `api.x.ai/v1/responses` exists (OpenAPI). A Codex `[model_providers.xai]` with `wire_api = "responses"` and `env_key = "XAI_API_KEY"` would likely work, but xAI publishes no Codex guide, and it would spend API credits, not the X Subscription pool.
- **Conclusion [I]:** the subscription is reachable only through first-party surfaces (Grok Build, Cursor, Grok Bot, grok.com). For AI Juggler, Grok is **a new adapter, not an account type** on Claude Code or Codex. An API key is an account type *within* the Grok Build adapter (`XAI_API_KEY`).

## 10. Verdicts for AI Juggler

### Grok Build CLI / TUI — **Full adapter, size M**

1. **What and detect:** `grok` at `~/.grok/bin/grok` (or `$GROK_BIN_DIR`); `grok --version`; `grok inspect --json` [VP]. Rule out the unofficial `grok` by path and version string [I].
2. **Isolation:** `GROK_HOME` per account [VP]; optional `GROK_AUTH_PATH` [VP]. Set `GROK_DISABLE_AUTOUPDATER` / `--no-auto-update` for pinning [VP].
3. **Auth:** browser OAuth or device code into `$GROK_HOME/auth.json`; `XAI_API_KEY` for API accounts [VP]. Sign-in = run `grok login` with the account's `GROK_HOME` and watch for the file [I]. Parallel accounts are not documented [NE-1].
4. **Sessions:** documented directory layout, `summary.json` index, `--resume <id>`, headless `json` / `streaming-messages-json`, ACP `session/load` [VP]. Indexing needs no replay (`summary.json`) [I].
5. **Usage:** per-session tokens and cost are local-observable (`signals.json`, `grok usage`, headless `usage` / `total_cost_usd`) [VP]. The weekly pool percentage has no sanctioned source → **Estimate / unknown** [NE-2]. HTTP 402 marks exhaustion [VP-source].
6. **Extensibility:** MCP, AGENTS.md, hooks (a `Stop` / `StopFailure` hook makes a turn-finished signal), ACP [VP].
7. **Terms:** multiple personal accounts are not forbidden [VP / NE-7]. Automation through the official binary is sanctioned [VP]. Never touch credentials or scrape usage [I].

**Blockers:**
- **Cross-tool bleed.** By default Grok reads the user's real `~/.claude` rules, hooks, skills and MCP servers (`~/.claude.json`), plus `~/.cursor`, regardless of `GROK_HOME` [VP]. AI Juggler must decide whether to set `GROK_CLAUDE_*_ENABLED=0` / `GROK_CURSOR_*_ENABLED=0` per launch. A Claude `PreToolUse` hook would otherwise fire inside Grok sessions.
- **The installer writes `$HOME/.grok` regardless of `GROK_HOME`** [VP]. The default home also holds the binary and downloads, so "the default account" and the install share one folder [I].
- **Name and path collision** with the community `grok-cli` (`~/.grok/user-settings.json`) [VP].
- No quota feed (Estimate only) [NE-2].

**Open questions:**
- (a) Does one identity signed in under two homes invalidate the other (refresh rotation)? [U]
- (b) Defaults of the `compat.*` flags [NE-8].
- (c) Does the Free tier's "Limited" Build need special handling?
- (d) What does a quota-exhausted headless run emit (exact `error` text)? Needs a live capture with a sacrificial account (maintainer action).
- (e) Does `cli.session_registry` coordinate across different `GROK_HOME`s? [U]

### Grok Build via ACP (`grok agent stdio`) — part of the same adapter, size S increment
The control-plane option, analogous to Codex app-server: approvals, session load, streamed updates [VP]. Use it only if AI Juggler wants in-app approvals; otherwise launch the TUI.

### Grok Bot desktop app — **Not feasible**
Bots run "on a persistent cloud computer"; "The computers Bots work on run in Cursor's cloud" [VP, https://docs.x.ai/grok-bot/overview]. There are no local coding sessions to supervise, and multi-account is first-party [VP]. At most it could be listed as a usage consumer of the same pool (it is: "Bot usage draws from that seat's shared weekly usage pool" [VP, pricing]).

### xAI API key — **Account type inside the Grok Build adapter, size S**
`XAI_API_KEY` into Grok Build [VP]. Not usable for Claude Code (compat deprecated) [VP]. Codex is undocumented [I]. Usage: API billing is org-scoped (Management API / console), separate from the subscription [VP].

### Community CLIs / extensions — **Not feasible (unofficial)**
- `superagent-ai/grok-cli`: MIT; "not affiliated with, endorsed by, or sponsored by xAI"; `GROK_API_KEY`; `~/.grok/user-settings.json`; `--session`; `-p --format json` [VP, its repo]. API-key only.
- Community VS Code UIs wrap the official CLI via ACP [VS].

Support neither. Detect only to avoid misidentifying `grok`.

## 11. Search logs for Negative-evidence claims (all 2026-10-21)

- **NE-1 — no documented way to run several Grok accounts at once.**
  - Sources checked: `UG/02-authentication.md` (only "replaces your cached session"), `UG/05`, `UG/26`, docs.x.ai/build/* (all 24 Build pages in https://docs.x.ai/llms.txt), docs.x.ai/x-subscription/sign-in.
  - Queries: WebSearch "Grok Build multiple accounts GROK_HOME switch account github issue" (only third-party agentsroom.dev); GitHub `search/issues repo:xai-org/grok-build account` → 0 results (issues disabled).
- **NE-2 — no documented API or local file for the subscription pool percentage.**
  - Sources checked: docs.x.ai/x-subscription/{usage-limits, manage, extra-usage, pricing}, docs.x.ai/grok/faq (Usage & Limits), `UG/24-monitoring-usage.md`, `UG/25-status-line.md` (explicit "no rate-limit data, quota, plan"), `UG/14-headless-mode.md`, https://docs.x.ai/openapi.json (40 paths; none for usage pools), docs.x.ai/developers/{cost-tracking, management-api-guide}.
  - Source search: `search/code "usage_limit" / "UsageInfoContext" repo:xai-org/grok-build` → internal `FetchBilling` only.
- **NE-3 — no Grok Build desktop app.**
  - Sources checked: docs.x.ai/llms.txt and sitemap.xml (Build has CLI/ACP pages only), x.ai/pricing snapshot.
  - Query: WebSearch "Grok macOS desktop app download xAI 2026" → reports of no native Grok chat app; only the Grok Bot desktop app (cloud agents).
- **NE-4 — no first-party VS Code / JetBrains extension.**
  - Query: WebSearch "\"Grok Build\" VS Code extension marketplace xAI official" → community extensions only.
  - Sources checked: docs.x.ai sitemap (no IDE page), Build CLI reference (ACP only).
- **NE-5 — no xAI guide for Claude Code via `ANTHROPIC_BASE_URL`.**
  - Query: WebSearch "Claude Code ANTHROPIC_BASE_URL api.x.ai grok messages endpoint" → nothing xAI-specific.
  - Sources checked: `grep -i anthropic|claude` over the downloaded docs.x.ai pages (only Build compat notes and the deprecation notice); sitemap.xml.
- **NE-6 — no Keychain storage of Grok credentials.**
  - Source search: `search/code keyring repo:xai-org/grok-build` → 0 hits.
  - Docs checked: `UG/02-authentication.md` (names only `auth.json` / `mcp_credentials.json`).
- **NE-7 — no "one account per person" clause.**
  - `grep -i "multiple|account"` over the ToS snapshot 20261021055255 and AUP snapshot 20261019192323. Only the credential-sharing and fake-accounts ("creating fake accounts" to mislead) clauses.
- **NE-8 — defaults of `compat.claude.*` / `compat.cursor.*` not stated.**
  - Sources checked: `UG/26-config-reference.md`, https://docs.x.ai/build/settings/reference.md. The documented behaviour (rules and MCPs "are read") implies on-by-default [I], but no default is printed.
