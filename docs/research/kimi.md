# Research notes: Moonshot Kimi (Kimi Code CLI, Kimi Code Desktop, membership, Kimi API)

Research date: 2026-10-21

> Compiled 2026-10-21 for AI Juggler's Provider roadmap. Mirrors the section layout of [codex.md](codex.md).
> Method: vendor docs (Kimi Code docs at `www.kimi.com/code/docs/en/…`, and the same docs as Markdown at `moonshotai.github.io/kimi-code/en/<page>.md`), the MIT `MoonshotAI/kimi-code` repository read through the GitHub API, the Kimi Open Platform docs (`platform.moonshot.ai/docs`), the Kimi ToS, and the installer script, read as text and never run.
> **Nothing was installed, signed in to, or run**, so none of this has passed the billing protocol's live-capture step (AJ-E601).
>
> **Accessed:** every source below was read on **2026-10-21**. **Labels** (docs/research/EVIDENCE-STANDARD.md): **[VP]** Verified-primary · **[VS]** Verified-secondary · **[NE-n]** Negative-evidence, with its search log in §11 · **[I]** Inferred · **[U]** unknown.
>
> Short names:
> - `KD/` = `https://moonshotai.github.io/kimi-code/en/` (CLI docs, Markdown).
> - `KC/` = `https://www.kimi.com/code/docs/en/` (product, desktop and third-party-tool docs).
> - `SRC/` = `https://github.com/MoonshotAI/kimi-code/blob/main/`.

## 0. Line-up at a glance

| Surface | What it is | Official? | AI Juggler verdict (see §10) |
|---|---|---|---|
| **Kimi Code CLI** (`kimi`, npm `@moonshot-ai/kimi-code`) | Terminal coding agent, TypeScript/Node, MIT | Official | **Full adapter, M** |
| **Kimi Code Desktop** | GUI client sharing `~/.kimi-code` with the CLI | Official (closed source) | **Track-only, S** |
| Kimi Code for VS Code (`moonshot-ai.kimi-code`) | Extension that drives the CLI | Official | Out of scope (an IDE surface) |
| **Kimi Code membership API key → Claude Code** | Anthropic-compatible endpoint `api.kimi.ai/coding/` | **Officially documented** | **Account type on Claude Code, S–M** |
| **Kimi Code membership API key → Codex** | Responses-API endpoint `api.kimi.ai/coding/v1` | **Officially documented** | **Account type on Codex, S–M** |
| Kimi Open Platform API key (pay-as-you-go) → Claude Code | `api.moonshot.ai/anthropic` | Officially documented | Account type on Claude Code, S (secondary) |
| Legacy **Kimi CLI** (Python, `kimi-cli`, `~/.kimi`) | Archived predecessor | Official, **archived** | **Not feasible.** Detect only, to suggest migration |
| Kimi Work / Kimi chat apps | General assistant apps | Official | Out of scope |

## 1. Distribution & versioning (Kimi Code CLI)

- **[VP] The CLI was rewritten.** The legacy Python CLI (`MoonshotAI/kimi-cli`, 11.4k stars) is archived: "Existing installations are no longer supported and will stop working." Its last release was 1.52.0 on 2026-09-22 — https://github.com/MoonshotAI/kimi-cli (README and releases API).
- **[VP] The successor** is `MoonshotAI/kimi-code`: MIT, TypeScript, created 2026-05-22, ~8k stars. The latest release is **`@moonshot-ai/kimi-code@2.3.0`, published 2026-10-21** — GitHub releases API. The monorepo also holds `apps/vscode`, `apps/vis`, `packages/node-sdk`, `packages/acp-server`, `packages/kap-server` and `packages/oauth` (contents API).
- **[VP] Install:**
  - `curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash` (macOS/Linux), `irm …/install.ps1 | iex` (Windows);
  - or `npm install -g @moonshot-ai/kimi-code` (Node ≥ 22.19.0);
  - the legacy README also lists `brew install kimi-code` — `KD/guides/getting-started.md`, https://github.com/MoonshotAI/kimi-cli.
- **[VP] Install location:** the script installs `${KIMI_INSTALL_DIR:-$HOME/.kimi-code}/bin/kimi` (keeping `kimi.bak`) and edits shell PATH unless `KIMI_NO_MODIFY_PATH` is set. `KIMI_VERSION` pins a version — https://code.kimi.com/kimi-code/install.sh (read as text, L9–37, L566–571).
- **[VP] Detect on a Mac:**
  - binary `kimi`, at `~/.kimi-code/bin/kimi` (script) or the npm global bin;
  - `kimi --version` / `-V`; `kimi doctor` validates `config.toml` / `tui.toml` (exit 0 or 1) — `KD/reference/kimi-command.md`.
  - **[I]** The legacy CLI also used the binary name `kimi`, so the install path plus the `--version` output tell them apart.
- **[VP] Updates:** `kimi upgrade [-y]` (alias `update`); `[upgrade].auto_install` in `tui.toml` (default on); staged-rollout state lives in `updates/` — `KD/reference/kimi-command.md`, `KD/configuration/data-locations.md`.
- **[VP] Regions:** mainland `kimi.com` (auth `auth.kimi.com`, API `api.kimi.com/coding/v1`) and global `kimi.ai` (auth `auth.kimi.ai`, API `api.kimi.ai/coding/v1`) — `SRC/packages/oauth/src/region.ts`, `SRC/packages/oauth/src/managed-usage.ts` L8–9.

## 2. Configuration system

- **[VP] `KIMI_CODE_HOME`:** the data root defaults to `~/.kimi-code/`. "Once set, **all** Kimi Code data lands under the new path: config, sessions, logs, OAuth credentials, Kimi-specific user Skills, global `AGENTS.md`, and more" — `KD/configuration/data-locations.md`. It is also used for the device id (`resolveKimiHome()` → `createKimiDeviceId(homeDir)` writes `<home>/device_id`) — `SRC/apps/kimi-code/src/cli/telemetry.ts` L43–44, `SRC/packages/oauth/src/identity.ts` L41–62.
- **[VP] Shared within a home:** "Multiple `kimi` instances sharing the same `KIMI_CODE_HOME` will share config and credential files" — `KD/configuration/env-vars.md`.
- **[VP] Not moved by `KIMI_CODE_HOME`:** generic `.agents` resources "stay under the real OS home so they can be shared across tools" (`~/.agents/skills/`, `~/.agents/AGENTS.md`, `~/.agents/agents/`) — `KD/configuration/data-locations.md`, `KD/customization/agents.md`.
- **[VP] Layout:**

  ```
  config.toml, tui.toml, AGENTS.md, mcp.json, skills/, plugins/
  session_index.jsonl
  credentials/<name>.json (0600, dir 0700), credentials/mcp/
  sessions/<workDirKey>/<sessionId>/
  locks/sessions/   (cross-process occupancy locks)
  bin/{rg,fd}, logs/kimi-code.log, updates/, user-history/<md5(workDir)>.jsonl
  server/instances/, server.token
  ```

  Credential writes are atomic (tmp → fsync → rename) — `KD/configuration/data-locations.md`, `KD/reference/kimi-command.md`.
- **[VP] Precedence:** `-m` > `KIMI_MODEL_*` env > `config.toml`. Provider keys are **not** read from shell env unless declared via `api_key_env`: "Running `export KIMI_API_KEY=xxx` … does not give any provider its key" — `KD/configuration/env-vars.md`, `KD/configuration/providers.md`.
- **[VP] Provider types:** `kimi`, `anthropic`, `openai`, `openai_responses`, `google-genai`, `vertexai`. `/provider` and `kimi provider` manage them. "Kimi Code OAuth managed accounts logged in via `/login` do not appear in `/provider`" — `KD/configuration/providers.md`.
- **[VP] Project-local files:** `.kimi-code/local.toml` (extra dirs), `.kimi-code/mcp.json`, `.kimi-code/AGENTS.md` or `AGENTS.md`, `.kimi-code/agents/` — `KD/configuration/config-files.md`, `KD/customization/agents.md`.
- **[VP] Endpoint env vars:** `KIMI_CODE_BASE_URL` (default `https://api.kimi.com/coding/v1`), `KIMI_CODE_OAUTH_HOST` / `KIMI_OAUTH_HOST` — `KD/configuration/env-vars.md`.
- **[NE-2]** No reading of `~/.claude` / `CLAUDE.md` was found; cross-tool sharing goes through `~/.agents`.

## 3. Auth & multiple accounts

- **[VP] Sign-in:** `/login` in the TUI or `kimi login` (RFC 8628 **device-code** flow; exit 0 on success, 1 on cancel/failure). Choose "Kimi Code (OAuth)" or "Kimi Platform API key". `/logout` clears "credentials for the currently selected account" — `KD/guides/getting-started.md`, `KD/reference/kimi-command.md`, `KD/reference/slash-commands.md`.
- **[VP] Token storage:** OAuth credentials at `$KIMI_CODE_HOME/credentials/<name>.json`, plain JSON with mode 0600 — `KD/configuration/data-locations.md`, `SRC/packages/oauth/src/storage.ts` L5, L95–107. **[VS-source]** The managed file is `credentials/kimi-code.json` (`SRC/packages/oauth/test/storage.test.ts` L65). A Keychain is not used [I, from the storage code]. The managed provider id is `managed:kimi-code` (`KD/reference/server-api.md`).
- **[VP] Single-writer token refresh:** an OAuth token transaction "Serializes OAuth token grants for one credential identity in this process" so "a late SDK callback cannot overwrite or delete a newer durable winner" — `SRC/packages/oauth/src/oauth-token-transaction.ts`. Whether refresh tokens rotate (single-use) is not documented [U].
- **[VP] Devices count against quota.**
  - "All logged-in devices and API Keys share the same quota; devices inactive for over 30 days are automatically unbound" — `KC/kimi-code/membership.html`.
  - The server API mentions "the plan's device quota" — `KD/reference/server-api.md`.
  - **[I]** Each separate `KIMI_CODE_HOME` mints its own `device_id`, so every isolated home is probably a separate "device" for the same account. A per-account home (one account per home) is fine; several homes on one account may hit a device cap (size unknown [U]).
- **[VP] Multiple devices and tools are allowed:** "Switching between devices … or different coding tools (Kimi CLI, VS Code, Claude Code) is a completely normal usage pattern" — `KC/kimi-code/community-guidelines.html`.
- **[NE-3]** No documented way to hold two Kimi OAuth accounts at once inside one home. **[I]** One `KIMI_CODE_HOME` per account is the isolation pattern. The docs themselves suggest `KIMI_CODE_HOME` "to isolate configs for different projects with independent environments" (`KD/configuration/data-locations.md`).
- **[VP] Multiple accounts and the ToS:** "If there is evidence … that you have improperly registered or used multiple accounts, we may take measures such as freezing, deleting … or refusing to provide Services" — Kimi Terms of Service (https://www.kimi.ai/user/agreement/modelUse, "Effective Date: January 5, 2024", §Account). **[I]** The clause targets *improper* multi-accounting (same pattern as ZCode's agreement), not owning a work and a personal account.

## 4. Sessions

- **[VP] Location:** `$KIMI_CODE_HOME/sessions/<workDirKey>/<sessionId>/`, with `workDirKey = wd_<slug>_<first-12-of-sha256(path)>`. There is a top-level `session_index.jsonl` (`sessionId`, `sessionDir`, `workDir` per line) — `KD/configuration/data-locations.md`.
- **[VP] Per-session files:**
  - `state.json` — title, `lastPrompt`, timestamps, `forkedFrom`;
  - `agents/main/wire.jsonl` — "complete communication record, used for session resumption and replay";
  - `agents/<sub>/wire.jsonl`, `agents/main/plans/`, `tasks/`, `cron/`, `logs/`.
  
  "Do not manually edit files inside the `sessions/` directory" — `KD/guides/sessions.md`.
- **[VP] Token usage is reconstructable from the session.** The VS Code replay adapter derives `token_usage { input_other, output, input_cache_read, … }` and `context_usage` from agent usage state — `SRC/apps/vscode/src/runtime/replay-adapter.ts` L471–486. The exact `wire.jsonl` line schema is not documented [U].
- **[VP] Resume and headless:**
  - `kimi --continue/-c`, `kimi --session/-S [id]` (hidden alias `-r/--resume`), `/sessions`, `/fork`, `/title`;
  - `kimi -p "…" [--output-format text|stream-json]` (stream-json = one JSON message per line; thinking not written; no human approval in `-p`; no usage object documented [NE-4]);
  - `kimi export <id> -o x.zip`, `kimi vis` — `KD/reference/kimi-command.md`, `KD/guides/sessions.md`.
- **[VP] Goal mode in `-p`** exits 0 on completion, 3 when blocked, 6 when paused — `KD/reference/slash-commands.md`.
- **[VP] Occupancy:** `locks/sessions/` gives cross-process locks. Desktop and CLI "read the same session data"; opening a session that is in use shows an occupancy prompt. The server API returns `40931` "session occupied by another client" — `KD/configuration/data-locations.md`, `KC/kimi-code-desktop/project-and-session-management.html`, `KD/reference/server-api.md`.
- **[VP] Control plane:** `kimi web` runs a local REST + WebSocket server (default `127.0.0.1:58627`, bearer token in `~/.kimi-code/server.token`, `/openapi.json` and `/asyncapi.json`). "The REST and WebSocket APIs … are experimental: interface stability is not guaranteed" — `KD/reference/server-api.md`. ACP: `kimi acp` supports `session/new`, `session/load` and `session/resume` (`KD/reference/kimi-acp.md`).

## 5. Usage & quota

- **[VP] Plans (new, since Sept–Oct 2026):** Kimi Code is part of Kimi membership; "Kimi Code is not sold separately" per press.
  - **China (RMB), from the vendor help page:**

    | Plan | Monthly | Annual | Kimi Code |
    |---|---|---|---|
    | Go | ¥49 | ¥468/yr | **No coding quota** |
    | Plus | ¥99 | ¥948/yr | Yes |
    | Pro | ¥199 | ¥1,908/yr | Yes |
    | Max | ¥699 | ¥6,708/yr | Yes |
    | Ultra | ¥1,399 | ¥13,428/yr | Yes |

    https://www.kimi.com/en/help/membership/membership-pricing.
  - **[VS]** Global USD prices: Plus $19, Pro $39, Max $99, Ultra $199/mo (≈20% off annually) — https://codingplan.org/en/plans/kimi, https://ki-ai.chat/pricing/membership/ [NE-5 for a vendor page].
  - **[VP]** Legacy tiers continue: Andante, Moderato, Allegretto, Allegro — `KC/kimi-code/membership.html`, the help page above.
- **[VP] Entitlements by model** — `KC/kimi-code/models.html`, `KC/third-party-tools/claude-code.html`:
  - Model IDs: `k3` (2.8T, up to 1M context), `k3-256k`, `kimi-for-coding` (K2.8 Preview, 1M), `kimi-for-coding-highspeed` (K2.7 Code HighSpeed, "3× quota usage").
  - K3 needs Plus/Moderato or higher; 1M context and HighSpeed need Pro/Allegretto or higher.
  - Above-tier requests return **401**.
- **[VP] Limit model** — `KC/kimi-code/membership.html`:
  - New members: **rolling 5-hour window** and a **monthly total** quota; the weekly limit was removed.
  - Legacy members: 5-hour, **weekly (7 days from subscription date)** and monthly.
  - The quota is shared by CLI, VS Code, Desktop and third-party tools.
  - Extra Usage wallet: RMB, close to Open Platform prices, optional monthly cap.
  - Visible in the Kimi Code Console, Kimi web "My Quota", and CLI `/usage`.
- **[VP] Usage sources, best first:**
  1. **`GET /api/v1/oauth/usage`** on the local `kimi web` server is documented. It returns `{ usages: { limit5h, limit7d, monthTotal, monthCode: { usedRatio 0–1, resetAt RFC3339 } }, extraUsage: { balanceCents, totalCents, monthlyChargeLimitEnabled, monthlyChargeLimitCents, monthlyUsedCents, currency } }`, "fetched live from the account service" — `KD/reference/server-api.md` §Login and usage. The vendor documents it, so under the billing protocol it is a **documented-sanctioned candidate**. Caveats: it is marked experimental, it is local, it covers the OAuth managed account only, and it needs a live capture.
  2. That endpoint proxies **`GET {KIMI_CODE_BASE_URL}/usages`** (fields `limit_5h`, `limit_7d`, `limit_month_total`, `limit_month_code` with `used_ratio` / `reset_time`) — `SRC/packages/oauth/src/managed-usage.ts` L21–135. The remote endpoint itself is undocumented, so calling it directly is **prohibited** [I, per billing protocol]. Whether it accepts a Console API key is unknown [NE-6].
  3. CLI `/usage` ("token usage, context consumption, and quota information") and the status line (`[status_line].command` gets a JSON snapshot: model, cwd, git, mode, context usage, session id, version — no quota listed) — `KD/reference/slash-commands.md`, `KD/configuration/config-files.md`.
  4. Local per-session tokens from the session data (§4) are **local-observable** candidates.
- **[VP] Exhaustion signals** (HTTP 403 with fixed text) — `KC/kimi-code/error-reference.html`:
  - "You've reached your 5-hour usage limit";
  - "…weekly (7-day) usage limit";
  - "…monthly usage limit for this billing cycle";
  - "You've reached your concurrent request limit" (risk control, "unrelated to your remaining quota").
  
  HTTP 429 means overload or too many requests.
- **[VP] Open Platform (separate system):** pay-as-you-go per token, "no subscription plan" — https://platform.moonshot.ai/docs/pricing/chat. Tiered RPM/TPM/TPD by cumulative recharge — https://platform.moonshot.ai/docs/pricing/limits. **Documented balance API** `GET https://api.moonshot.ai/v1/users/me/balance` — https://platform.moonshot.ai/docs/api/balance. Membership keys and Open Platform keys are "not interchangeable" — `KC/kimi-code/error-reference.html`.

## 6. Extensibility surface

- **[VP] MCP:** `mcp.json` (`mcpServers` format) at `$KIMI_CODE_HOME/mcp.json` and project `.kimi-code/mcp.json`. Supports stdio, HTTP and SSE; `/mcp-config` skill; `/mcp` status — `KD/customization/mcp.md`. The legacy CLI had `kimi mcp add|list|remove|auth`.
- **[VP] Rules:** `$KIMI_CODE_HOME/AGENTS.md` (global, Kimi-specific), `~/.agents/AGENTS.md` (cross-tool), project `AGENTS.md` / `.kimi-code/AGENTS.md`. Agent files can override the system prompt (`override: true`) — `KD/customization/agents.md`.
- **[VP] Hooks:** `[[hooks]]` in `config.toml` (`event`, `matcher`, `command`); fail-open; exit `2` = block.
  - Events: `PreToolUse`, `Stop`, `UserPromptSubmit` (blocking), `Notification`, `SessionStart` (`startup|resume`), **`SessionHeartbeat`** (every 60 s), `SubagentStop`, and more.
  - Desktop honours the same hooks — `KD/customization/hooks.md`, `KC/kimi-code-desktop/settings-and-extensions.html`.
- **[VP] Embedding:**
  - `kimi acp` (Zed, JetBrains; `session/load`, `session/resume`, `logout`);
  - `kimi web` server API (sessions, prompts, approvals, login, usage);
  - `packages/node-sdk` — `KD/reference/kimi-acp.md`, `KD/reference/server-api.md`, repo contents.
- **[VP] Other:** plugins and marketplace, skills, subagents, Tower multi-agent (experimental), Remote Control, `kimi vis` — `KD` llms.txt index.

## 7. Terms of service

Sources:
- **Kimi Code Community Guidelines** — `KC/kimi-code/community-guidelines.html` (no date shown);
- **Kimi Terms of Service**, effective 2024-01-05 — https://www.kimi.ai/user/agreement/modelUse;
- Extra Usage Agreement (linked, not read).

- **(a) Multiple accounts.**
  - **[VP]** The ToS lets them freeze or delete for "improperly registered or used multiple accounts" (§Account).
  - **[VP]** "Account Transfer … may not be transferred."
  - **[VP]** Guidelines: "Don't resell your account or API access"; sharing "through a reverse proxy is a violation".
- **(b) Automation — the main risk.**
  - **[VP]** "Kimi Code subscriptions are for **interactive use only**" and "**Don't use Kimi Code for non-interactive automation** … such as scripted batch execution or data annotation pipelines."
  - **[VP]** Violations lead to review, then limits "such as limiting concurrent access" (the 403 concurrent-limit error); purchases through unauthorized channels may be suspended (Guidelines).
  - **[I]** AI Juggler launching interactive TUI sessions for a human fits. Unattended `kimi -p` loops, auto-resume and scheduled automations on a subscription account are *outside* the guideline. They should be gated or allowed only on Open Platform API-key accounts.
- **(c) Third-party wrappers.**
  - **[VP]** Third-party tools are explicitly welcomed ("compatible with … Kimi CLI, VS Code, Claude Code, OpenCode, OpenClaw").
  - **[VP]** But: "**Don't spoof or alter client identity information**" (Guidelines).
  - **[VP]** ToS: no commercial use of generated content without consent; no reverse engineering.
  - **[I]** A supervisor that launches the official `kimi` or `claude` binaries unmodified, without touching `User-Agent` / identity env (`KIMI_CODE_IDENTITY_SLUG` changes the User-Agent product token — `KD/configuration/env-vars.md`), stays compliant. AI Juggler must **never set `KIMI_CODE_IDENTITY_*`**.
- **[NE-7]** No explicit statement on third-party *supervisor or launcher* apps.

## 8. Pain points and context

- **[VP]** Churn: the CLI was rewritten (Python to Node) and the legacy one "will stop working". Plans were renamed and the weekly limit removed for new members only, so legacy and new rules coexist — `KC/kimi-code/membership.html`.
- **[VP]** The vendor's Claude Code and Codex guides write the key and endpoint into the **global** `~/.claude/settings.json` / `~/.claude.json` / `~/.codex/config.toml`. They ship a cleanup script because leftover `env` entries "override environment variables exported in the terminal" — `KC/third-party-tools/claude-code.html`, `KC/third-party-tools/codex.html`. That global mutation is exactly what per-account isolation avoids.
- **[VP]** Switching models or efforts invalidates the context cache and raises quota use — `KC/kimi-code/models.html`.
- **[VS]** Press notes that plan names and entitlements shifted several times in 2026 — https://www.usagepricing.com/blueprint/activity/moonshot-ai-2026-09-28-kimi-plus-pro-max-ultra-plans.

## 9. Special case — the Kimi subscription powers Claude Code **and** Codex

This is the most important finding: both of AI Juggler's existing tools have **vendor-documented** Kimi setups that use the *membership* quota.

- **[VP] Claude Code**:
  - Set `ANTHROPIC_BASE_URL=https://api.kimi.ai/coding/` (China: `https://api.kimi.com/coding/`) and `ANTHROPIC_API_KEY=<Kimi Code Console key>`.
  - Set `ANTHROPIC_MODEL` and **every** tier variable (`ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU,FABLE}_MODEL`, `CLAUDE_CODE_SUBAGENT_MODEL`) to `k3-256k` / `k3[1m]`; also `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` and `CLAUDE_CODE_EFFORT_LEVEL`. "Configuring only some of these variables will silently break the corresponding scenarios."
  - Pre-seed `~/.claude.json` with `hasCompletedOnboarding: true` (and `penguinModeOrgEnabled`) to skip Anthropic login.
  - Verify with `/status` showing the Kimi base URL.
  - Source: `KC/third-party-tools/claude-code.html`.
- **[VP] Codex:**
  - `[model_providers.kimi]` with `base_url = "https://api.kimi.ai/coding/v1"`, `env_key = "KIMI_API_KEY"`, `wire_api = "responses"`.
  - `model = "k3-256k"`, `model_provider = "kimi"`, `model_catalog_json = "~/.codex/models.json"` (a catalog listing `k3` and `k3-256k` with context windows and efforts).
  - The Desktop app needs `launchctl setenv KIMI_API_KEY`; `experimental_bearer_token` is the fallback.
  - Source: `KC/third-party-tools/codex.html`.
- **[VP] Open Platform alternative (pay-as-you-go):** Claude Code with `ANTHROPIC_BASE_URL=https://api.moonshot.ai/anthropic` and `ANTHROPIC_AUTH_TOKEN=<platform key>`, models `kimi-k3[1m]` / `kimi-k2.7-code` — https://platform.moonshot.ai/docs/guide/claude-code-kimi.
- **[I] How it maps onto AI Juggler:**
  - **Claude Code account type "Kimi Code (membership)":**
    - Use an isolated `CLAUDE_CONFIG_DIR`. Its `settings.json` `env` (or the launch env) carries the base URL, the model variables and the key. Its own `.claude.json` (inside the config dir) carries `hasCompletedOnboarding`.
    - AI Juggler already strips inherited `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` / `CLAUDE_CODE_OAUTH_TOKEN` and keeps keys that live in the account's own settings — `crates/juggler-adapters/src/claude_code.rs` (`INHERITED_CREDENTIAL_KEYS`, `unset_inherited_credentials`).
    - **Usage:** Claude Code's own rate-limit or statusline quota fields would describe nothing real. Kimi quota for a *key* account has no sanctioned feed (the Console is web-only; the `kimi web` usage endpoint covers OAuth accounts) → Estimate / unknown [NE-6]. Per-turn tokens from Claude transcripts still apply.
  - **Codex account type "Kimi Code (membership)":** a per-account `CODEX_HOME` holding `config.toml` + `models.json`, with `KIMI_API_KEY` injected at launch (not in a shell rc). Same usage caveat.
  - **Key storage:** the vendor's recipes put the key in plaintext files. AI Juggler should keep it in the macOS Keychain and inject it as env at launch [I].

## 10. Verdicts for AI Juggler

### Kimi Code CLI — **Full adapter, size M**

1. **What and detect:** `kimi` (`~/.kimi-code/bin/kimi` or npm global), `kimi --version`, `kimi doctor` [VP].
2. **Isolation:** `KIMI_CODE_HOME` per account moves *all* data including OAuth credentials [VP]. `~/.agents/*` stays shared (cross-tool by design) [VP].
3. **Auth:** device-code OAuth (`kimi login`, exit codes) or a Platform API key; credentials in `credentials/*.json` [VP]. The adapter only checks that the file exists and never reads it [I].
4. **Sessions:** documented layout plus `session_index.jsonl` (cheap indexing); `--session <id>` / `--continue`; `-p --output-format stream-json`; ACP; `kimi web` server API; session locks [VP].
5. **Usage:**
   - 5h / weekly / monthly `usedRatio` + `resetAt` via the **documented** local `GET /api/v1/oauth/usage` [VP]. This is the best quota surface found across Codex, Grok and Kimi, but it is experimental and needs `kimi web` running per home [I].
   - 403 messages name the exhausted window [VP].
   - Per-session tokens are local-observable [VP-source].
6. **Extensibility:** MCP (`mcp.json`), AGENTS.md, hooks (incl. `SessionHeartbeat`, `Stop`), ACP, server API, Node SDK [VP].
7. **Terms:** "interactive use only / no non-interactive automation" [VP]; no client-identity spoofing [VP]; "improper" multi-accounts can be frozen [VP].

**Blockers:**
- **The automation guideline.** Hand-over, auto-resume and headless features must be off (or warned about) for subscription accounts.
- **The device quota per home** [I / U].
- **Experimental server API** stability [VP].
- **Region** (kimi.com vs kimi.ai) affects endpoints and sign-in [VP].
- **Desktop shares `~/.kimi-code`** with the default home, so the default account and Desktop are one identity [VP].

**Open questions:**
- (a) Device cap size, and whether isolated homes count as devices [U].
- (b) Does `/usages` accept a Console API key? [NE-6]
- (c) Refresh-token rotation semantics [U].
- (d) The stable schema of `wire.jsonl` [U].
- (e) Is a long-lived `kimi web` per account acceptable, or should usage be read only on demand? [I]

### Kimi Code Desktop — **Track-only, size S**

- **[VP]** macOS (Apple Silicon and Intel), Windows and Linux. Latest 1.1.1 (2026-10-21) — `KC/kimi-code-desktop/changelog.html`.
- **[VP]** Config, credentials, sessions and logs live in `~/.kimi-code/` (shared with the CLI); UI state in `~/Library/Application Support/kimi-code-app/`; updater cache in `~/Library/Caches/kimi-code-app-updater/` — `KC/kimi-code-desktop/settings-and-extensions.html`.
- **[VP]** `kimi app [path]` opens it — `KD/reference/kimi-command.md`.
- **[VP]** Sessions are visible to both Desktop and CLI.
- Not found: bundle id, app path, and whether Desktop honours `KIMI_CODE_HOME` [NE-1]. **[I]** So it cannot be isolated per account. AI Juggler can detect it (`~/Library/Application Support/kimi-code-app/`) and show its sessions under the default-home account.

### Account type on Claude Code — "Kimi Code membership" — **S–M**

- **[VP]** Documented by Moonshot (§9). It reuses the Claude Code adapter's isolation, sessions and transcripts.
- **Blockers:**
  - plaintext-key handling (move it to the Keychain + launch env) [I];
  - all eight model variables must be set, or features break silently [VP];
  - the quota has no sanctioned feed for key accounts (Estimate) [NE-6];
  - Claude-side usage UI must be suppressed or relabelled [I];
  - the "interactive use only" guideline [VP].
- **Open questions:** do Claude Code updates keep honouring `penguinModeOrgEnabled` / onboarding skips? [U] Does the Kimi endpoint support every Claude Code feature (e.g. `/effort` mapping is documented [VP]; tool search and others are not) [U]?

### Account type on Codex — "Kimi Code membership" — **S–M**

- **[VP]** Documented by Moonshot (§9). It reuses the Codex adapter (`CODEX_HOME` per account).
- **Blockers:** the `models.json` catalog must track Kimi's model list (it changes) [VP / I]; key injection [I]; quota is Estimate [NE-6]; Codex `token_usage_record` lines remain valid local tokens [I].

### Account type on Claude Code — Kimi Open Platform key — **S (lower priority)**

- **[VP]** Documented (`api.moonshot.ai/anthropic`, `ANTHROPIC_AUTH_TOKEN`).
- **[VP]** Pay-as-you-go balance is readable through the **documented** `GET /v1/users/me/balance`. That makes it a documented-sanctioned candidate for a "balance" field (live capture needed).
- No interactive-only restriction was found for Platform keys [I].

### Legacy Kimi CLI (Python) — **Not feasible**
Archived; "will stop working" [VP]. Detect `~/.kimi/` only, to suggest `kimi migrate` (OAuth isn't migrated) [VP, `KD/guides/migration.md`].

## 11. Search logs for Negative-evidence claims (all 2026-10-21)

- **NE-1 — Kimi Code Desktop bundle id, app path, `KIMI_CODE_HOME` support.**
  - Sources checked: `KC/kimi-code-desktop/{getting-started, help, settings-and-extensions, project-and-session-management, interface-and-sessions, changelog}.html` (data locations given, no bundle id, no env var).
  - The Desktop source is not in `MoonshotAI/kimi-code`: `apps/` = kimi-code, kimi-inspect, vis, vscode.
  - Code searches in that repo: `"kimi-code-app"` → 0 hits, `CFBundleIdentifier` → 0 hits.
- **NE-2 — reading `~/.claude` / `CLAUDE.md`.**
  - `grep -E '\.claude|CLAUDE\.md'` over every downloaded `KD/` page (22 pages, including config-files, agents, plugins, mcp, hooks) → no hits outside third-party-tool guides.
- **NE-3 — several OAuth accounts at once in one home.**
  - Sources checked: `KD/reference/slash-commands.md` (`/login` "Select an account or platform", `/logout` "currently selected account"), `KD/configuration/providers.md`, `KD/reference/server-api.md` (a single `managed:kimi-code` provider).
  - GitHub `search/issues repo:MoonshotAI/kimi-code "multiple accounts"` → 1 unrelated PR (#2795).
- **NE-4 — usage fields in `-p --output-format stream-json`.**
  - Source checked: `KD/reference/kimi-command.md` §Non-Interactive Execution (describes assistant and tool messages only, no usage object).
- **NE-5 — vendor page with USD prices.**
  - https://www.kimi.com/en/help/membership/membership-pricing lists RMB only.
  - https://www.kimi.ai/code and https://www.kimi.ai/membership/pricing render plans client-side (curl'd HTML has no prices).
  - WebSearch "Kimi membership new plans Go Plus Pro price per month Kimi Code 2026" → USD figures only from third-party pages.
- **NE-6 — sanctioned quota feed for API-key (Console) accounts.**
  - Sources checked: `KC/kimi-code/membership.html` (Console and `/usage` only), `KD/reference/server-api.md` (usage endpoint is OAuth-managed-provider scoped), `SRC/packages/oauth/src/managed-usage.ts` (undocumented `/usages`; key acceptance not stated).
- **NE-7 — statement on third-party supervisor/launcher apps.**
  - Sources checked: `KC/kimi-code/community-guidelines.html`, Kimi ToS (modelUse, EN), `KC/kimi-code/faq.html`. Only tool-compatibility, resale and identity-spoofing clauses.
