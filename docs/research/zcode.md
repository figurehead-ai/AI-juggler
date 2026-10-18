# Research notes: ZCode (Z.AI / GLM)

> Deep dive compiled 2026-10-12 as input to the [proposal](../PROPOSAL.md) and [spec](../SPEC.md).
> Combines web research with local inspection of a real installation (macOS, desktop 3.14.5 / CLI engine 0.16.9).
> Legend: **[V-W]** = verified web · **[V-L]** = verified locally (exact command) · **[I]** = inference · **[U]** = unknown.
> ZCode is newer and less documented than Claude Code / Codex — unknowns are expected and are flagged precisely.

## A. Web research

### A1. What ZCode is (Oct 2026)

- **[V-W]** ZCode is Z.ai/Zhipu's "next-gen vibe-coding tool" — a multi-agent desktop coding app, branded "GLM-5.3 official Harness." Electron app (`.dmg`/`.exe`/`.AppImage`/`.deb`/`.rpm`) for macOS (arm64/x64), Windows, Linux (beta). Site version v3.15.1. (https://zcode.z.ai/, https://zcode.z.ai/cn/docs/install)
- **[V-W]** ZCode is one of "20+ Agent Tools" covered by the **GLM Coding Plan** subscription (alongside Claude Code, Codex, Cline, Kilo Code, OpenCode, Clawdbot/OpenClaw). One subscription = one shared quota pool across all those tools. (https://z.ai/subscribe, https://docs.z.ai/devpack/overview)
- **[V-W]** Plan tiers (credits system, revised 2026-07-30): **Lite** 2,000 credits / 5h, 10,000/week; **Pro** 12,000/5h, 60,000/week; **Max** 28,000/5h, 140,000/week. Credits = `(input×in-mult + cached×cached-mult + output×out-mult) / 10,000`; GLM-5.3 multipliers 6.9/1.7/24, GLM-5.3-Flash 2.3/0.56/8. 5h credits reset 5h after consumption (rolling, like Claude); weekly resets every 7 days from purchase. (https://docs.z.ai/devpack/overview, https://docs.z.ai/devpack/notice/usage-revision)
- **[V-W]** Pricing: official docs say "from $18/month"; third-party tracker lists Lite $18 / Pro $80 / Max $168 monthly (https://codingplan.org/Coding%20Plans). Treat as soft — promos are frequent.
- **[V-W]** Off-peak billing: 50% credit rate; peak = weekdays 14:00–18:00 UTC+8, weekends all off-peak. Z.ai has run all-day off-peak promos (e.g. Sep 25–Oct 7 2026). (https://docs.z.ai/devpack/overview, https://sessionwatcher.com)
- **[V-W]** GLM Coding Plan endpoints: Anthropic-compatible `https://api.z.ai/api/anthropic` (coding plan) and OpenAI-compatible `https://api.z.ai/api/paas/v4` (standard API); CN mirror BigModel `https://open.bigmodel.cn/api/anthropic`. **[V-L]** Confirmed in `/Applications/ZCode.app/Contents/Resources/config/provider/zcode-builtin.json` (templates: `zai-api`, `zai-standard-api`, `bigmodel-api`, `bigmodel-standard-api`).
- **[V-W]** ZCode-specific features beyond a plain CLI agent: Goal mode (long-horizon planning), subagents, plugins/skills/MCP/hooks/slash commands, automations (scheduled tasks), **idle-time tasks** (queued jobs that run free during compute-idle windows without consuming plan quota — requires a personal Coding Plan), remote development, Bot Channel (invoke ZCode remotely via WeChat/Feishu/Telegram), Memory, repo Wiki, browser automation. (https://zcode.z.ai/cn/docs/ sidebar)

### A2. Official docs coverage

- **[V-W]** Docs live at `https://zcode.z.ai/cn/docs/*` (Chinese-first; EN mirror at `/en/docs/...`). Pages: welcome, install, configuration, agents, goal, browser-use, task-management, repo-wiki, memory, automations, idle-time-tasks, edit-history, remote-development, remote-control, bot-channel, subagents, plugin, skill, mcp-services, commands, hooks, usage-stats, safety-confirm, ADE-tools, keyboard-shortcuts, qa, linux-wsl, feedback.
- **[V-W]** **No standalone CLI install is documented** — install docs cover only the desktop app; no npm/brew/curl channel. The TUI/CLI engine exists inside the app bundle (see B1). **[U]** whether Z.ai plans a separately distributed CLI.
- **[V-W]** docs.z.ai (developer docs) does **not** document the ZCode harness — it covers models/APIs and the Coding Plan ("DevPack" section). (https://docs.z.ai/llms.txt)
- **[V-W]** Multi-account: **no docs page exists** (confirmed by docs-nav enumeration). Usage visibility: the app's "Usage Stats" page shows (a) local per-device session stats (tokens, sessions, active days, per-model breakdown) and (b) remote Coding Plan quota (5h pool, weekly, MCP monthly, tool calls) — local stats are per-device, not synced. In-app "quota reset cards" since 3.8.1 auto-issue extra quota under conditions (Coding Plan only). (https://zcode.z.ai/cn/docs/usage-stats)
- **[V-W / V-L]** Extensibility: plugins with marketplaces, skills (`~/.zcode/skills/`), custom slash commands (`~/.zcode/commands/`), hooks (`hooks` key in `~/.zcode/cli/config.json` or workspace `<repo>/.zcode/config.json` / `zcode.json`), MCP servers (user `mcp.servers` in `~/.zcode/cli/config.json`, fallback `~/.agents/mcp.json`; precedence CLI > env > user > workspace > system). AGENTS.md instruction files (`~/.zcode/AGENTS.md` user-level, `<repo>/AGENTS.md` or `.zcode/AGENTS.md`).
- **[V-L]** Headless: `zcode -p/--prompt` (single-shot, default mode `yolo`), `--json`, `--surface terminal|desktop`, `--mode build|edit|plan|yolo`, `--target`, `app-server` (stdio "ZCode Protocol" server — how the Electron UI and VS Code extension drive it). Sessions/resume: `--resume <sessionId>`, `-c/--continue`, in-TUI `/resume`, `/fork`, `/rewind`, `/compact`.

### A3. Terms of service — multi-account, automation, wrappers

- **[V-W]** ZCode User Agreement (https://zcode.z.ai/cn/terms, Chinese; translated):
  - **3.2:** evidence of "improper registration or improper use of multiple accounts" → ZCode may freeze/close accounts and refuse service. No definition of "improper"; no explicit per-person account cap — it targets *improper* multi-accounting. **[I]** Running 2 paid personal accounts in parallel is a grey zone, not expressly banned, but risk-controlled.
  - **3.3:** the account is for personal use only; no third-party authorization/transfers.
  - **Section 4** prohibits using the service "via deep links, page scraping, social bots, spiders or other automated means" to access unlicensed data/servers/accounts; reverse engineering ZCode's "algorithms, source code, mechanisms"; circumventing safety measures; training competing models. **[I]** This clause is web-scraping-oriented; ordinary CLI/headless use of your own account is the product's own designed usage (`-p`, app-server) — a third-party wrapper that drives ZCode is not addressed head-on.
  - Third-party API keys: ZCode is then "only a local execution channel."
- **[V-W]** GLM Coding Plan Usage Policy (https://docs.z.ai/devpack/usage-policy): "**Subscription benefits are exclusive to the subscriber: Account sharing or multi-user access is prohibited**"; the plan "may only be used within officially supported tools and products"; violations → "rate limiting, account freezing, or other restrictions"; >3 violations may ban; appeal via Plan Overview page.
- **[V-W]** FAQ: all supported tools share one subscription's quota; quota visible on the subscription management page; subscriptions non-refundable. (https://docs.z.ai/devpack/faq)
- **[U]** No English z.ai Terms of Use page with a clean "one account per person" clause was found; the authoritative documents are the ZCode 用户协议 and DevPack usage policy above.

### A4. Community pain points

- **[V-W]** July 30 2026 plan revision (prompts → credits) fragmented the user base into V1/V2/Team/legacy plans with different switch rules and mid-cycle upgrade costs — hard to mentally compare (legacy V2 Pro ~400 prompts/5h vs new 12,000 credits/5h). (https://docs.z.ai/devpack/notice/usage-revision)
- **[V-W]** 5h rolling + weekly dual caps and peak/off-peak multipliers are the recurring complaint pattern (same shape as Claude's limits). (https://sessionwatcher.com positioning: "Never Lose a Coding Session to Rate Limits")
- **[V-W]** Third-party quota trackers exist and thrive (SessionWatcher, $7–24, menu-bar, multi-account support for **Claude and Codex**, reads local usage files) but **does not support ZCode / GLM plans** — an open niche. (https://sessionwatcher.com)
- **[U]** Specific Reddit/HN threads could not be retrieved — community evidence is indirect, via docs revisions and third-party tracker positioning.

## B. Local inspection (macOS, 2026-10-12)

### B1. CLI location, version, surface

- **[V-L]** `which zcode` → not found; no shim in PATH. The CLI engine is a Node SEA-style bundle inside the app: `/Applications/ZCode.app/Contents/Resources/glm/zcode.cjs` (14.8 MB), run via `node …/zcode.cjs …`. Desktop 3.14.5 (`dev.zcode.app`, URL scheme `zcode://`); CLI reports 0.16.9; `zcode doctor` works (node v26.10.0, darwin/arm64).
- **[V-L]** `zcode --help` — commands: `app-server`, `commands`, `doctor`, `login` (Z.AI OAuth; `--no-browser` prints URL), `logout`, `plugins`, `skills`, `tui`, `version`. Notable flags: `-p/--prompt`, `--resume <sessionId>`, `-c/--continue`, `--json`, `--mode build|edit|plan|yolo`, `--surface terminal|desktop`, `--cwd`, `--attach`, `--disallowed-tools`, `--target`, `--browser-use headless`, `--locale`, `--no-browser`, `--verbose`. Subcommand `--help` just reprints top-level help (no per-subcommand docs).
- **[V-L]** Bundled extras: `Resources/tools/{ripgrep,bfs,ugrep}`, `Resources/cua-helper/ZCode Computer Use.app`, `Resources/config/provider/zcode-builtin.json` (provider templates), `Resources/glm/packages/*`.

### B2. `~/.zcode/` map

- **[V-L]** `~/.zcode/cli/` (54 MB) — CLI state: `config.json` (JSON), `db/db.sqlite` (5.4 MB + WAL), per-session data in `exec/sess_*`, `agents/`, `artifacts/sess_*`, `log/zcode-YYYY-MM-DD.jsonl` (~4 MB/day), `plugins/{cache,data,marketplaces}`, `rollout/model-io-sess_*.jsonl`, `shell-snapshots/`.
- **[V-L]** `~/.zcode/v2/` (3.6 MB) — desktop state: `credentials.json` (**0600**), `setting.json` (48 top-level keys incl. `receivePreviewUpdates`, `autoDownloadAndInstallUpdates`, `modelIoFullRetentionEnabled`, `memoryEnabled`), `provider_config.json`, `tasks-index.sqlite` (tasks, task_groups, automations, **off_peak_tasks**), `certs/`, `logs/`, `crash/`.
- **[V-L]** Config format is **JSON** throughout. `~/.zcode/cli/config.json` schema supports `mcp.servers`, `hooks`, `plugins`, and `storage.dir`/`storage.sessionDbPath` overrides. Project-level config: `<repo>/.zcode/config.json` or `<repo>/zcode.json`; user instructions `~/.zcode/AGENTS.md`.

### B3. Sessions & transcripts

- **[V-L]** Canonical session store is **SQLite**: `~/.zcode/cli/db/db.sqlite`. Tables: `session, session_entry, session_input, session_target, message, part, permission, todo, input_history, local_setting, model_usage, turn_usage, tool_usage, workflow_*, dwf_*, schema_migration`. `session` columns include `id (sess_UUID), project_id, parent_id, directory, path, title, version, summary_*, time_created/updated/compacting/archived, task_type, trace_id` — `parent_id` + `/fork` imply forking is first-class.
- **[V-L]** Raw model I/O mirroring: `~/.zcode/cli/rollout/model-io-<sessionId>.jsonl` — one JSON object per model request incl. request headers (⚠ headers may include auth → treat files as sensitive). Gated by `modelIoFullRetentionEnabled`.
- **[V-L]** Resume: CLI `--resume sess_…` / `-c`; TUI `/resume`, `/fork`, `/rewind` (checkpoint restore). `exec/sess_*/` dirs hold per-session shell snapshots + tool-call logs.

### B4. Auth

- **[V-L]** Credentials are a **plaintext JSON file** (mode 0600): `~/.zcode/v2/credentials.json` — **not** macOS Keychain (`security find-generic-password -s zcode` → not found). Keys (values redacted): `oauth:zai:access_token`, `zcodejwttoken`, `oauth:zai:user_info`, `oauth:active_provider`, and per-account entries `account-provider:coding-plan:account:zai-individual-coding-plan:account:<UUID>:api-key` — **the schema already anticipates multiple stored accounts plus an "active provider" pointer.** **[I]** Account switching = rewrite `oauth:active_provider` (and the oauth tokens) in this file.
- **[V-L]** `zcode login` = Z.AI OAuth; CLI and desktop **share** these credentials.
- **[V-L]** Env vars in the bundle (names only): `ZCODE_DATA_BASE_DIR`, `ZCODE_PERSONAL_PROVIDER_CONFIG_FILE`, `ZCODE_SESSION_ID`, `ZCODE_SKILL_DIR`, `ZCODE_PLUGIN_ROOT`/`ZCODE_PLUGIN_DATA`, `ZCODE_PRODUCTION_BASE_URL`, `ZCODE_REMOTE_HTTP_PROXY`, `ZCODE_TELEMETRY_*`, `ZCODE_RG_BINARY`/`UGREP_BINARY`/`GIT_BINARY`, plus `XDG_DATA_HOME`/`XDG_CACHE_HOME` reads.
- **[V-L, controlled experiment]** `ZCODE_DATA_BASE_DIR=/tmp/x zcode plugins list` still read `~/.zcode/...` — the var does **not** redirect plugin/config/db roots in 0.16.9. But `HOME=/tmp/y zcode plugins list` fully redirected state (created `/tmp/y/.zcode/...`). **Full-home isolation via `HOME` works; `ZCODE_DATA_BASE_DIR` alone is insufficient.**

### B5. Usage & quota

- **[V-L]** Local usage is comprehensive: `model_usage` logs per-request `provider_id, model_id, input_tokens, output_tokens, reasoning_tokens, cache_creation_input_tokens, cache_read_input_tokens, computed_total_tokens, duration_ms, time_to_first_token_ms, finish_reason, retry_count, error_type`; `turn_usage`/`tool_usage` aggregate per turn/tool. **[I]** Because plan credits are a deterministic formula of these token counts and the published multipliers, a manager app can compute exact credit burn per session/account locally — no API needed.
- **[V-L / U]** Remote quota (5h/weekly pools) is fetched by the app from zcode.z.ai/api/v1 + z.ai endpoints. **No official public quota API or CLI status command exists.**

### B6. Update mechanism

- **[V-L]** Electron with electron-updater (Squirrel.Mac): `Squirrel.framework` present; `Resources/app-update.yml` ships a placeholder (`provider: generic, url: http://localhost:8081`); runtime `setFeedURL(...)` overrides it. User toggles: `receivePreviewUpdates`, `autoDownloadAndInstallUpdates`, `skippedElectronUpdateVersions`. **[I]** CLI engine updates only with the app bundle.

### B7. Multi-account isolation assessment

- **[V-L]** Best mechanism: run the CLI with a **per-account `HOME`** (e.g. `HOME=<juggler-state>/zcode-acctA`), which provably isolates credentials, session DB, plugins, logs, and settings. The desktop app additionally keys off Electron's userData dir but shares `~/.zcode`, so parallel ZCode accounts are CLI-only (or one desktop instance per macOS user).
- **[V-L / I]** `ZCODE_DATA_BASE_DIR` is insufficient (B4 experiment); `storage.dir` in config.json gives partial isolation (sessions) without moving credentials.
- **[I]** Parallelism within one account: `app-server` (stdio, one per process) and `-p` headless runs are process-per-session; caps are server-side credits. A manager can run N sessions of one account and meter credits via `model_usage`.
- **ToS overlay (A3):** a supervisor that spawns ZCode processes with isolated HOMEs stays in "local execution channel" territory. Z.ai explicitly bans improper multi-account use, account sharing, and unsupported-tool use of the *plan*; a shipping product should rely on documented surfaces (CLI flags, app-server protocol, SQLite reads) rather than patched binaries.

## Key unknowns (honest list)

1. No public ZCode harness docs for CLI flags/env vars — all CLI knowledge here is from local `--help` and binary strings; Z.ai documents only the desktop app.
2. No documented public quota/usage HTTP API; reverse-engineering the app's own endpoints would be ToS-sensitive — out of scope for AI Juggler.
3. Production auto-update feed URL unknown (runtime `setFeedURL`).
4. Whether ZCode will ever ship as a standalone npm/brew CLI; today it is app-bundle-only.
5. Community pain-point evidence is indirect (docs revisions, third-party tracker positioning) — no quotable forum threads retrieved.

## §B8 addendum — billing/credit investigation (AJ-E604, 2026-10-15)

Append-only per the task law; supersedes nothing above.

- Live captures + per-field AJ-E601 grades: `docs/research/evidence/billing/zcode-captures.md`. Headlines: **credit burn is `Computed`-ready** (model_usage rows verified across two days ≥24 h apart; the ±0 recomputation script ships beside the packet), with the bucket→column mapping stated as `inferred` (`computed_total = input + output` pins `input_tokens` as all-inclusive — the fresh/cached split is arithmetic, not vendor-documented); **pool totals stay `Estimate`-at-best** (no sanctioned remote source; the "no public quota API" claim re-verified against docs.z.ai on 2026-10-15 — billing web page only); **renewal stays `unknown`/user-entered** (≥4 surfaces searched, negatives recorded in the packet).
- Tier table re-checked 2026-10-15 (docs.z.ai/devpack/overview): unchanged since the 2026-07-30 revision (Lite 2,000/10,000; Pro 12,000/60,000; Max 28,000/140,000).
- NEW formula detail since §A1: MCP tool calls (Web Search / Web Reader / Zread) each bill at an output multiplier of 1.2 — the recomputation script does not model these; AJ-E607 must treat sessions using MCP tools as slightly under-counted (or extend the script from tool_usage rows).
- Cap-entry wording inputs for AJ-E610 (vendor spellings): tiers "Lite", "Pro", "Max"; caps "credits per 5 hours" and "weekly credits"; burn badge "Computed"; pool badge "Estimate" until caps entered.
