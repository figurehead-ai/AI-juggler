# Research notes: OpenAI Codex

> Deep dive compiled 2026-10-12 as input to the [proposal](../PROPOSAL.md) and [spec](../SPEC.md).
> Legend: **[V]** = verified fact with source · **[I]** = inference · **[U]** = unknown / could not verify.
> `developers.openai.com/codex/*` now redirects to `learn.chatgpt.com/docs/*` (checked 2026-10-12) — the latter URLs are canonical.

## 1. Distribution & versioning

- **[V] Install methods:** `npm install -g @openai/codex`; `brew install --cask codex`; standalone `curl -fsSL https://chatgpt.com/codex/install.sh | sh` (macOS/Linux) and a PowerShell `install.ps1`; prebuilt tarballs from GitHub Releases. https://github.com/openai/codex, https://learn.chatgpt.com/codex/cli
- **[V] Rust:** implementation lives in the `codex-rs` Rust workspace; release tags are `rust-vX`. https://github.com/openai/codex
- **[V] Current version:** `@openai/codex` **0.162.1** on npm (2026-10-12, https://registry.npmjs.org/@openai/codex/latest), matching GitHub release `rust-v0.162.1` published 2026-10-09. Release cadence is days, not weeks. License Apache-2.0, ~129k stars.
- **[V] Desktop app:** Codex is a surface of the **ChatGPT desktop app** (macOS dmg primary; Windows/Linux pages exist), launched via `codex app` or the ChatGPT app. https://learn.chatgpt.com/codex/app
- **[V] IDE extension:** VS Code extension `openai.chatgpt` plus Cursor/Windsurf; Xcode and JetBrains integrations. https://learn.chatgpt.com/codex/ide

## 2. Configuration system

- **[V] `CODEX_HOME`:** "Codex stores its local state under CODEX_HOME (defaults to ~/.codex)" — holds `config.toml`, `auth.json` (when file-based), `history.jsonl`, logs, sessions, caches. https://learn.chatgpt.com/docs/config-file/config-advanced
- **[V] `config.toml` key families** (https://learn.chatgpt.com/docs/config-file/config-reference): `model`, `model_provider`, `model_reasoning_effort`, `model_verbosity`, `model_context_window`; `model_providers.<id>` (`base_url`, `env_key`, `auth.command`, `wire_api`); `mcp_servers.<id>` (stdio/HTTP, oauth, tool filters, timeouts, `required`); `projects.<path>.trust_level`; `approval_policy`; `sandbox` (`read-only | workspace-write | danger-full-access`); `permissions`; `notify`; `shell_environment_policy`; `tui` (`status_line`, `notifications`, `resume_cwd`); `features`; `agents` (incl. `max_concurrent_threads_per_session`); `plugins`/`marketplaces`; `otel`; `cli_auth_credentials_store`.
- **[V] Profiles — breaking change:** "Since Codex 0.134.0, the legacy `[profiles.profile-name]` table and the `profile = "profile-name"` selector no longer work." Profiles are now **separate files** `$CODEX_HOME/<name>.config.toml` containing top-level keys, selected with `--profile <name>`; names are letters/digits/hyphens/underscores (https://learn.chatgpt.com/docs/config-file/config-advanced). Example: `~/.codex/deep-review.config.toml` run as `codex --profile deep-review`.
- **[V/I] What a profile can override:** docs show model/reasoning/approval keys; profile files "can override this per profile" generally. Profiles are config layers, **not** auth identities — auth lives in `auth.json` per `CODEX_HOME`.
- **[V] Project-local config cannot override auth-ish keys:** `openai_base_url`, `chatgpt_base_url`, `model_provider`, `model_providers`, `notify`, `profile(s)`, `otel` are ignored in `.codex/config.toml` (config-advanced).
- **[V] One-off overrides:** `codex --model X`; `codex --config model='"X"'` (values parsed as TOML; dot-paths set nested keys).
- **[V] Per-project config:** every `.codex/config.toml` from project root to cwd is loaded, "the closest file to your working directory wins," only for **trusted** projects. Root detection walks to `.git`; customizable via `project_root_markers`.
- **[V] AGENTS.md:** read into the session's first turn; sized by `project_doc_max_bytes`, alternate names via `project_doc_fallback_filenames`; `/init` scaffolds it.
- **[V] Managed/enterprise layer** `requirements.toml` exists (https://github.com/openai/codex/blob/main/docs/config.md).

## 3. Auth & multiple accounts

- **[V] Sign-in:** `codex login` (ChatGPT OAuth browser flow; default), `printenv OPENAI_API_KEY | codex login --with-api-key`, `codex login --with-access-token`, `codex login --device-auth`, `codex login status`, `codex logout`. https://learn.chatgpt.com/docs/auth
- **[V] `auth.json`** (inspected locally, 2026-10-12): `~/.codex/auth.json`, shape `{auth_mode, OPENAI_API_KEY, tokens: {id_token, access_token, refresh_token, account_id}, last_refresh}`. Docs: credentials cached "in a plaintext file at ~/.codex/auth.json or in your OS-specific credential store"; tokens auto-refresh; "treat auth.json like a password." Storage mode: `cli_auth_credentials_store = file | keyring | auto | ephemeral` **[V]**.
- **[V] Shared login across surfaces:** "The CLI and IDE extension share the cached login; logging out of either requires re-login." The desktop app also writes into `~/.codex` (session files with `originator: "Codex Desktop"` observed locally).
- **[V] Multiple ChatGPT accounts in parallel — no first-class support.** Docs never mention concurrent accounts; maintainer position is only that "codex supports the CODEX_HOME env var" (issue #4407, closed not planned: https://github.com/openai/codex/issues/4407). **[I]** The reliable parallel-accounts pattern is one `CODEX_HOME` per account (`CODEX_HOME=~/.codex-work codex`), each logged in separately.
- **[V] Community tool:** `Ducksss/codex-profiles` (MIT, 181 stars) — macOS menu-bar app + CLI mapping named profiles to `~/.codex-<name>`, per-profile quota display with low-quota alerts, project bindings; explicitly "does not read or copy authentication file contents or ChatGPT cookies"; "not an account, OS or server-side security boundary." https://github.com/Ducksss/codex-profiles
- **[V] Demand for native multi-account:** #20500 "support multiple named accounts per app/connector" (145 reactions, open, 2026-04-30); #53247 "Support switching between multiple accounts in the Codex desktop app without logging out" (opened 2026-10-12). Switching today = `codex logout` + `codex login` within one `CODEX_HOME` **[I]**.
- **[V] Headless/portable auth:** docs recommend `scp`/ssh-forwarding or `docker cp` of `~/.codex/auth.json` into containers; admins can force `forced_login_method` / `forced_chatgpt_workspace_id`; mismatched credentials make Codex "log the user out and exit."

## 4. Sessions

- **[V] Location/format** (inspected locally): `~/.codex/sessions/YYYY/MM/DD/rollout-<ISO-timestamp>-<uuid>.jsonl`; also `~/.codex/archived_sessions/`. Line shape: `{"timestamp", "type", "payload"}`.
- **[V] Observed event types (local files, e.g. Codex Desktop 0.159.2):**
  - `session_meta`: `{id, session_id, parent_thread_id, timestamp, cwd, runtime_workspace_roots, originator, cli_version, model_provider, base_instructions, source}` — enough to index sessions without replaying them.
  - `turn_context` (one per turn); `response_item` with payload types `message | agent_message | reasoning | function_call | function_call_output | custom_tool_call | custom_tool_call_output`; `event_msg` (`task_started`, `item_completed`, `task_complete`, `token_count`); `token_usage_record`; `world_state`; `inter_agent_communication_metadata`.
  - **Token usage is in rollout files:** `token_usage_record` carries `{thread_id, turn_id, response_id, usage: {input_tokens, cached_input_tokens, cache_write_input_tokens, output_tokens, reasoning_output_tokens, total_tokens}, turn_token_usage}` — a read-only indexer can compute per-session/per-day token spend with zero network calls.
- **[V] `codex resume`:** interactive picker over "recent chat from the current repo or … across local chats"; non-interactive: `codex exec resume --last` and `codex exec resume <SESSION_ID>` (https://learn.chatgpt.com/docs/non-interactive-mode).
- **[V/I] Third-party read-only indexing:** format is undocumented-but-stable JSONL; `codex app-server generate-json-schema` can emit schemas **[V]**. No official stability guarantee — treat as de-facto API.
- **[I] Portability:** `session_meta.cwd` stores absolute paths, so resuming on another machine requires the same checkout path; copying `sessions/` along with a moved `CODEX_HOME` should make resumes portable — not officially documented **[U]**.

## 5. Usage & quota

- **[V] Limit model** (https://learn.chatgpt.com/docs/pricing, 2026-10): 5-hour rolling window **and** weekly allowance; "Local messages and cloud chats share your plan's usage allowance." Per-plan estimated local-message ranges (Plus/Standard Business: GPT-6.1 Sol 15–160 / 5h; GPT-6 Luna 350–3,000; GPT-6 Astra 5–45). "Pro plans currently have no five-hour limit." Usage depends on model choice, context, reasoning, tool use, retrieval, and caching — "prompt length alone isn't a reliable estimate"; "These estimates are not fixed message limits." Fast tier consumes at 2.5x, Ultrafast at 8x. Plus/Pro can buy credits; extra local chats possible via API key at API rates.
- **[V] In-CLI visibility:** `/status` = "Show the chat ID, context usage, and rate limits" (https://learn.chatgpt.com/codex/reference/slash-commands); `/usage` opens usage/billing settings (ChatGPT sign-in). `/statusline` widgets `five-hour-limit` and `weekly-limit` exist (maintainer-confirmed in #15281).
- **[V/U] Programmatic quota: no public CLI/API for 5h/weekly limits.** Issue #15281 "Expose full usage/limits data in CLI" (open) — commenters note scripts can't read limits, making orchestration "horrible to plan"; third-party `aistat` reads an undocumented `account/rateLimits/read` endpoint (stability **[U]**).
- **[V] Local usage data:** rollout `token_usage_record` lines + `codex exec --json` `turn.completed` usage objects — these are model tokens, not plan credits; correlation is fuzzy **[I]**.
- **[V/U] Exhaustion behavior:** no documented quota-specific exit code for `codex exec`; detect via `turn.failed`/`error` events in `--json` + stderr text **[I]**.
- **[V] Notify hook:** `notify = ["prog", …]` fires on `agent-turn-complete` with JSON `{type, thread-id, turn-id, cwd, input-messages, last-assistant-message}` — a "turn finished / possibly blocked" signal; carries no quota fields.

## 6. Extensibility surface

- **[V] `codex exec`** (https://learn.chatgpt.com/docs/non-interactive-mode): flags `--json` (JSONL), `-o/--output-last-message`, `--output-schema` (JSON-Schema-constrained final message), `--ephemeral`, `--sandbox`, `--skip-git-repo-check`, `--ignore-user-config`, `--ignore-rules`, `-` stdin sentinel. Event schema: `thread.started`, `turn.started`, `item.started/item.completed` (agent_message, command_execution, mcp_tool_call, web_search, file changes, plan updates), `turn.completed {usage}`, `turn.failed`, `error`. Resume subcommand (`--last`, `<SESSION_ID>`). `CODEX_API_KEY` inline per-run key.
- **[V] App Server** (https://learn.chatgpt.com/codex/app-server): `codex app-server` — JSON-RPC 2.0 over stdio/WebSocket/unix socket; `initialize` handshake; primitives `thread/start|resume|fork`, `turn/start|steer|interrupt`, streamed `item/started|completed`; plus `model/list`, `command/exec`, `fs/*`, MCP management, approvals (server-initiated requests), config read/write. Schema generation: `codex app-server generate-ts` / `generate-json-schema --out`. This is what powers the VS Code extension — **the strongest foundation for a supervisor app [I]**. Enterprise integrations should set `clientInfo.name` and contact OpenAI to join a known-clients list.
- **[V] MCP:** `[mcp_servers]` config, `/mcp` status; the standalone Codex MCP server binary was deprecated 2026-08-24 and **removed 2026-09-05** — app-server is the replacement (changelog).
- **[V] Hooks:** `PreToolUse` etc. in `~/.codex/hooks.json` or inline `[hooks]`; GA 2026-05-14.
- **[V] Misc:** GitHub Action `openai/codex-action@v1`; `/import` of Claude Code/Cursor setup+chats (2026-08-11 changelog — notable: a vendor shipping a cross-harness importer validates the handover thesis); `codex://` deep links (`codex://threads/new?prompt=…&path=…`).

## 7. Terms of service

Source: **OpenAI Terms of Use, effective 2026-01-01** — https://openai.com/policies/terms-of-use/ (retrieved 2026-10-12).

- **(a) Multiple accounts you own:** the terms prohibit *sharing*, not plural ownership: "You may not share your account credentials or make your account available to anyone else and are responsible for all activities that occur under your account." No "one account per person" clause **[U]** whether an explicit restriction exists elsewhere.
- **(b) CLI automation:** "Cannot" list includes "Automatically or programmatically extract data or Output" and "Interfere with or disrupt our Services, including circumvent any rate limits or restrictions or bypass any protective measures or safety mitigations." Driving the open-source CLI itself is normal use **[I]**; scraping dashboards to harvest quota numbers is closer to the prohibited line **[I]**.
- **(c) Third-party wrappers:** cannot "copy, modify, lease, sell or distribute any of our Services" or "sell or resell access to Interfaces." The CLI is Apache-2.0 (redistributable) **[I]**; business API use governed by separate Business Terms **[V]**.
- **[U/I] Enforcement:** no public OpenAI statement on multi-account Codex use; community tools in this space deliberately avoid touching credentials.

## 8. Pain points (community)

- **Quota burns / opacity:** #28879 "rate-limit cost per token jumped ~10–20x since June 16, draining the 5h budget in 2–3 prompts" — **560 reactions**; #2448 "Plus users hitting usage limits extremely quickly" (92); #30918 "70% to 100% in ~6 minutes"; #26979 "Usage limit decreasing without use"; #43201 rapid drain; #16909 CLI claims limit reached while dashboard says otherwise; #7354 variable weekly refresh dates. HN: "Ask HN: How do you feel about the new 5H usage limit in Codex?" (2026-08-27, https://news.ycombinator.com/item?id=49465304).
- **No programmatic usage surface:** #15281 — users are begging for machine-readable 5h/weekly limits and building scavengers themselves.
- **Limit-reset interruptions:** #21073 "Auto-resume CLI session when usage limit resets" (74 reactions) — validates the stuck/handover-monitor thesis.
- **Multi-account:** #20500 (145 reactions), #53247; community uses `CODEX_HOME` fan-out (codex-profiles).
- **Sessions:** #2080 session management (107, closed — delivered as `codex resume`); #8784 `codex delete <session>` (112, closed); #37967 Remote Control can't attach to live in-progress CLI threads.

## Synthesis for AI Juggler [I]

1. **Account multiplexing:** one `CODEX_HOME` per account is the only robust pattern; spawn every `codex`/`codex exec` child with its own `CODEX_HOME` env. Do not parse or proxy `auth.json` contents; just isolate directories.
2. **Quota monitoring:** no official API — best local signals are rollout `token_usage_record` lines, `exec --json` `turn.completed.usage`, `/statusline` widgets; model everything as **estimated** usage, mirroring OpenAI's own language.
3. **Control plane:** `codex app-server` (JSON-RPC, schema-generatable) is the purpose-built embedding surface — threads/turns/approvals/events; `codex exec --json` suffices for simpler supervision; `notify` hooks give turn-completion pings.
4. **Version treadmill:** profile semantics broke at 0.134.0; MCP-server binary removed 2026-09-05. Pin versions and re-verify the config reference each release.

## §9 addendum — billing/quota investigation (AJ-E603, 2026-10-15)

Append-only per the task law; supersedes nothing above.

- Live captures + AJ-E601 grades: `docs/research/evidence/billing/codex-captures.md`. Headlines: rollout `token_usage_record` (Desktop 0.162.0-alpha.17.2, two sessions ≥24 h apart) and the npm CLI 0.163.0 `exec --json` envelope are **verified local-observable** → per-turn tokens may render **Measured**; plan-credit percentage has **no sanctioned source** (the `/statusline` widgets render an undocumented `rateLimits` feed — prohibited; scraping is the ToS line) → Codex quota always renders **Estimate** with `≈` resets; renewal stays **unknown** (≥4 negative surfaces recorded).
- The API boundary nailed with dated evidence: `/v1/organization/costs` is Admin-key, org-scoped, pay-as-you-go API usage only — ChatGPT seats are a separate billing system; API billing must never be presented as subscription renewal evidence.
- Plan tier of the captured install is **unverified** (rollouts carry no plan field) — per-tier ceilings stay unverified until an authenticated capture names its tier.
