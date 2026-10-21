# Research notes: Muse Code (Meta)

Research date: 2026-10-21

> Input to the multi-Provider evaluation. Mirrors the structure of [codex.md](codex.md). Nothing was installed, no account was signed in, no binary was downloaded, and no local Muse, `~/.config`, `~/.local/share` or credential directory was read. Every claim was accessed on 2026-10-21.
>
> **Legend (per [EVIDENCE-STANDARD.md](EVIDENCE-STANDARD.md)):**
> **[Verified-primary]** the vendor's own docs, SDK, protocol schema or published package metadata say it ·
> **[Verified-secondary]** a reliable second-hand source (Homebrew API, press, a fan site) says it ·
> **[Negative-evidence]** not found by this search; the search is logged under Search log (S-n) ·
> **[Inferred]** our own reasoning; never a fact.
>
> Short URLs: `MD/` = `https://dev.meta.ai/docs/`. `SDK/` = `https://github.com/meta-models/muse-code-sdk/blob/HEAD/`.

## 0. Identification: which "Muse Code"?

| # | Candidate | Evidence | Likelihood |
|---|---|---|---|
| **1** | **Meta's Muse Code**: a terminal coding agent from Meta Superintelligence Labs, binary `muse`, launched in beta on **2026-08-05**, powered by Muse Spark 1.2 | **[Verified-primary]** Meta's launch post: "a terminal coding agent powered by Muse Spark 1.2", released in beta, dated August 5, 2026 — https://research.meta.ai/blog/introducing-muse-code-and-muse-spark-1-2. Product page https://dev.meta.ai/products/muse-code (`developer.meta.com/ai/products/muse-code/` 302-redirects there). **[Verified-secondary]** CNBC, Quartz, 9to5Mac and AppleInsider (2026-08-05) report it as Meta's first coding agent, aimed at Claude Code and Codex. | **Almost certain.** It is the only AI coding product with this exact name, and it matches AI Juggler's category (a subscription terminal agent). |
| 2 | musecodes.io | **[Verified-secondary]** Its footer says: "an unofficial fan site, not affiliated with Meta Platforms, Inc." It offers guides about candidate 1 and warns that pages advertising a "Muse Code desktop app" are not from Meta. — https://musecodes.io/ | Not a product: a fan site about #1. |
| 3 | Meta's **Muse** consumer personal-agent app (iOS/Android, then macOS) | **[Verified-secondary]** Reported as separate from Muse Code. A pricing guide says the consumer agent is distinct from "the Muse Code command-line interface developer tool". — https://www.layer3labs.io/guides/meta-muse-pricing; betanews headline "Meta launches a Muse app for macOS" (page returned 403). | Unlikely: not a coding tool. |
| 4 | **Muse Spark** models (in OpenCode Go and Zen) | **[Verified-primary]** OpenCode Go lists "Muse Spark 1.2 and 1.3 Contributor" models. — https://opencode.ai/v2/docs/console/go/ | A model, not a tool. It is relevant only as something you can reach from OpenCode/Codex. |
| 5 | Unity Muse (Muse Chat code help in the Unity Editor) | **[Verified-secondary]** Retired 2025-10-01 and replaced by Unity AI. — https://docs.unity3d.com/Packages/com.unity.muse.chat@1.1/manual/index.html | Very unlikely: retired, editor-only. |
| 6 | Third-party GUIs for #1 ("AFK Pilot" VS Code extension + desktop app) | **[Verified-secondary]** The developer says they are not affiliated with or endorsed by Meta. — https://www.productcompass.pm/p/muse-code-gui | A wrapper of #1, not a separate tool. |

**Pick: candidate 1, Meta's Muse Code.** The rest of this document is about it.

## 1. Distribution & versioning

- **[Verified-primary]** Install: `curl -fsSL https://dev.meta.ai/install.sh | sh` (macOS/Linux), or `irm https://dev.meta.ai/install.ps1 | iex` (Windows). It is a native binary named **`muse`**; verify with `muse --version`. — Source: MD/muse-code/
- **[Verified-primary]** Platforms: the docs now say macOS, Linux and Windows "from one codebase". On Windows, PowerShell replaces Bash, and voice input and session messaging are unavailable. The launch post listed only macOS/Linux. — Source: MD/muse-code/; research.meta.ai launch post
- **[Verified-secondary]** Homebrew cask **`muse-code` 1.4.11-R6667.1** ("Interactive terminal coding agent", homepage dev.meta.ai). It downloads a **closed, prebuilt binary** `muse-aarch64-macos` from `lookaside.facebook.com` and links it as `$HOMEBREW_PREFIX/bin/muse`; it needs macOS ≥ 12. Zap paths: **`~/.config/muse`, `~/.local/share/muse`**. — Source: https://formulae.brew.sh/api/cask/muse-code.json
- **[Verified-primary]** Current version 1.4.11. The changelog lists 1.4.0 to 1.4.11 with no dates on the page. The SDK "versions in lockstep with Muse Code". — Source: MD/muse-code/changelog; https://registry.npmjs.org/@muse-code/sdk (`latest` 1.4.11)
- **[Negative-evidence]** **Desktop:** no official Meta desktop app for Muse Code was found (S-1). The changelog entry for 1.4.10 does fix "Switching a conversation between the desktop app and the terminal when they run different versions of Muse Code". Which "desktop app" this means (a Meta app, or a third-party MSP client) is **unresolved**.
- **[Negative-evidence]** No open-source repository for the CLI itself was found. The public repo is the SDK/protocol (S-2).
- **[Inferred]** Mac detection: `muse` on `PATH` (installer location unverified; `$(brew --prefix)/bin/muse` for Homebrew); `muse --version`; data at `~/.config/muse` / `~/.local/share/muse`.

## 2. Configuration & local state

- **[Verified-primary]** User settings: `~/.config/muse/settings.json`, which must include `"schema_version": 1` if present. Keys include model defaults, `tui.*`, `permissions.default_profile`, tools, `mcp_servers`, `hooks` / `managed_hooks_path`, `runtime_capabilities`, `session_retention_days` (unset means keep forever), `agents.execution_capacity`, and telemetry. One-off override: `-c key=value`. — Source: MD/muse-code/configuration, MD/muse-code/extending
- **[Verified-primary]** **`XDG_CONFIG_HOME` is honoured:** user skills live in `$XDG_CONFIG_HOME/muse/skills`. The 1.4.10 sandbox fix covers "`~/.config/muse`, `~/.config/tbh`, or the same folders under `XDG_CONFIG_HOME`". — Source: MD/muse-code/extending; MD/muse-code/changelog
- **[Verified-primary]** The session protocol's `initialize` result returns **`museHome`**, "Absolute path of the muse home directory". — Source: SDK/schema/msp/msp.d.ts (`InitializeResult`)
- **[Negative-evidence]** No documented `MUSE_HOME` (or similar) variable that moves the whole home was found. `~/.local/share/muse` (Homebrew zap) is not named in the docs (S-3).
- **[Inferred]** The data dir likely follows `XDG_DATA_HOME` (the `~/.local/share/muse` default matches XDG), but this is unverified. A live probe with a throwaway home, reading `museHome` from `initialize`, would settle it.
- **[Verified-primary]** Project files: `.muse/hooks.json`; `.agents/` (memory `.agents/memory/MEMORY.md`, skills `.agents/skills/<id>/SKILL.md`). `.git`, `.muse` and `.agents` stay read-only inside the sandbox. — Source: MD/muse-code/configuration, MD/muse-code/permissions

## 3. Auth & multiple accounts

- **[Verified-primary]** First run offers "Sign in with your browser" (a Meta / dev.meta.ai single sign-on) or "Paste an API key". `/login` reopens these; `muse auth set` stores a key; `muse logout` clears the browser session and the stored key. The precedence is `META_API_KEY` env → stored key → stored browser session. MMA (Business & Enterprise) accounts must use an API key. — Source: MD/muse-code/auth
- **[Verified-primary]** **Credentials live in the macOS Keychain** ("Keep credentials in macOS Keychain across updates", 1.4.2; "Retry transient macOS Keychain reads", 1.4.0). The docs page says only that Muse Code "stores the credential for you". — Source: MD/muse-code/changelog; MD/muse-code/auth
- **[Negative-evidence]** No docs on switching accounts, multiple accounts, or how the Keychain item is named (per user or per home) were found (S-3).
- **[Inferred]** **This is the isolation blocker.** If the Keychain item is a single global entry (not keyed by config home), two isolated "homes" will share one browser sign-in, as Claude Code does on macOS. `META_API_KEY` per process does isolate **API-key** accounts (the env key comes first), so API-key accounts are possible today; subscription (browser) accounts need a live probe.

## 4. Sessions

- **[Verified-primary]** Sessions are append-only local event logs: "every model call, tool run, approval, and edit is appended", which makes them "replay-exact and restart-safe". `--no-session-log` turns retention off (and with it, resume). — Source: research.meta.ai launch post; MD/muse-code/interactive
- **[Verified-primary]** Resume: `/resume` (workspace-scoped), `/resume --last`, `muse resume <fork-id>`, `/fork`, `/rewind`. Headless: `muse exec "<prompt>"` with `--json` (JSONL events on stdout), `--session-id <uuid>` (continue), `--prompt-file`, `--max-model-steps`, `--allow-workspace-switch`, `--disable-approval`, `--yolo`. Exit codes: 0 done, 1 failed/cancelled, 2 usage error, 130/143 on signals. `muse export --session <uuid> --out <file>`. — Source: MD/muse-code/interactive, MD/muse-code/extending
- **[Verified-primary]** **Server/SDK:** `muse serve` runs the **Muse Session Protocol (MSP)**, JSON-RPC 2.0 with `schemaVersion` 1, a stable manifest, a fingerprint and an experimental flag. `muse schema` prints the JSON schema. The TypeScript SDK is `@muse-code/sdk` (MIT, "developer preview"); `MuseClient.spawn({museBin:"muse", args:["serve"]})` talks to a child process. A Python client `msp-py` also exists. Methods include `initialize`, `session/start|resume|fork|leave|list|read|rename|compact`, `turn/start|steer|interrupt|cancel`, `approval/decide`, `model/list`, `skill/*`, `plugin/*`, `hook/list`, `task/*`, `goal/*`, `workflow/*`, `view/subscribe`, and **`usage/read`**. — Source: MD/muse-code/; SDK/README.md; SDK/clients/sdk-ts/README.md; SDK/schema/msp/stable/manifest.json; SDK/schema/msp/msp.d.ts (`MspMethod`)
- **[Verified-primary]** `session/list` includes sessions that another Muse process (such as an open TUI) is using, but resuming them is refused while they are in use (1.4.7). `session/leave` hands a loaded session to another host (1.4.11). — Source: MD/muse-code/changelog
- **[Negative-evidence]** The on-disk path and format of session logs are not documented (S-3). **[Inferred]** They are probably under `~/.local/share/muse`. AI Juggler should index through `muse serve` `session/list` / `session/read`, not by parsing files.

## 5. Usage & quota

- **[Verified-primary]** Subscription plans (monthly): **Everyday $5** ("Send 10-50 prompts every 5 hours"), **High $15** (5× Everyday), **Power $50** (20×, adds Muse Wonder Full), **Maximum $200** (80×). Plans default to **Contributor mode** (more usage in exchange for training on your content; it can be turned off). At the limit you upgrade or wait. Extra API keys are billed pay-as-you-go. — Source: https://dev.meta.ai/products/muse-code; MD/muse-code/subscriptions
- **[Verified-primary]** API pricing per 1M tokens: `muse-spark-1.3` and `1.2` cost $1.25 in / $0.15 cached / $4.25 out; the `-contributor` variants cost $0.10 / $0.002 / $0.20. Rate limits are per team. — Source: https://dev.meta.ai/products/muse-code; MD/pricing-rate-limits
- **[Verified-primary]** **A sanctioned quota readout exists.** MSP `usage/read` "returns the last-seen 5-hour and weekly windows without spending a prompt" (1.4.0), and `usage/changed` pushes updates. The payload `SubscriptionUsage` is `{observedAtMs, tier, window:{usedPercent, resetsAtMs, windowDurationMins}, weekly:{usedPercent, resetsAtMs}}`, with "verbatim provider percentages"; it is omitted when nothing has been observed. `/usage` in the TUI shows it. `muse exec --json` emits a `run.usage` line with per-model tokens and the subscription allowance (1.4.11). — Source: SDK/schema/msp/msp.d.ts (`SubscriptionUsage`, `UsageReadResult`); MD/muse-code/changelog
- **[Inferred]** This is the best usage surface of any tool researched so far. The percentages are "last-seen" (from response headers), so they render as **Measured, as of `observedAtMs`**, not live.

## 6. Extensibility

- **[Verified-primary]** MCP: `mcp_servers` in settings (`stdio` or `streamable_http`, `mode: required|optional`); `muse mcp login|logout`; `/mcp`. Hooks: 17 events (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, … `Stop`, `SessionEnd`) in `.muse/hooks.json`, user settings, or a managed file. Skills: `muse skills list|inspect|enable|install|validate|import --from claude|codex`. It also reads `~/.claude/skills` and `$CODEX_HOME/skills`. Subagents: 8 per tree by default, optional worktree isolation. — Source: MD/muse-code/extending
- **[Verified-primary]** Rules: `AGENTS.md` (from `muse init`), then `CLAUDE.md`, `.agents/AGENTS.md`, `.claude/CLAUDE.md`, checked per directory up to `.git`. — Source: MD/muse-code/configuration

## 7. Terms of service

- **[Verified-primary]** The Muse Code product page footer links "Terms of Service" to the generic Meta policy centre, https://www.facebook.com/policies_center/. — Source: https://dev.meta.ai/products/muse-code/
- **[Negative-evidence]** No Muse Code–specific clause on multiple accounts, automation or wrappers was found (S-4).
- **[Verified-primary]** The SDK exists so that third-party apps can "programmatically control Muse Code", which is an explicit invitation to wrap it. — Source: https://github.com/meta-models/muse-code-sdk (repo description)

## Verdict for AI Juggler [Inferred]

| Surface | Verdict | Size | Why |
|---|---|---|---|
| **CLI (`muse`)** | **Full adapter** (API-key accounts now; subscription accounts after the Keychain probe) | **M** | Official MSP server + SDK (sessions, turns, approvals), `exec --json`, and a sanctioned 5h/weekly `usage/read`. Unknowns: home relocation and the Keychain item scope. |
| **Desktop** | **Not identified** | — | No official Meta desktop app found. The 1.4.10 "desktop app" note is unexplained. |
| **As an account type** | Not applicable to Muse Code. Muse Spark models are reachable through OpenCode Go (see opencode.md) and Meta's Messages-compatible API | — | — |

**Blockers**

1. Credentials are stored in the macOS Keychain, and nothing is documented about multiple accounts. A live probe is needed: two `XDG_CONFIG_HOME`/`XDG_DATA_HOME` homes, two browser sign-ins, then check whether each keeps its own sign-in.
2. The session file location and format are undocumented. Use MSP only.
3. The CLI is closed source, so behaviour can only be verified from the binary.

**Open questions for the maintainer**

- Confirm you mean **Meta's Muse Code** (the `muse` CLI).
- Is supporting API-key (`META_API_KEY`) accounts first acceptable, before subscription sign-ins are proven isolatable?
- May we run a live probe with a sacrificial Meta account (EVIDENCE-STANDARD rule 3)?

## Search log (Negative-evidence)

- **S-1 Official desktop app** (2026-10-21). WebSearch "Muse Code desktop app Meta macOS download"; WebSearch "Meta Muse app Mac "Muse Code" desktop conversation terminal switch" (extended). Sources: https://dev.meta.ai/products/muse-code (no desktop mention), MD/muse-code/ (lists CLI only), the changelog (one 1.4.10 "desktop app" mention), musecodes.io FAQ (warns that "desktop app" downloads are not from Meta), and productcompass.pm (third-party GUI). No Meta-published Muse Code desktop app was found by this search.
- **S-2 CLI source** (2026-10-21). GitHub API `orgs/meta-models/repos` lists `meta-model-cookbook`, `meta-oss-cookbook`, `muse-code-sdk` and `meta-sam`. No CLI source repo was found. The Homebrew cask downloads a prebuilt binary.
- **S-3 Home relocation / sessions path / multi-account** (2026-10-21). WebSearch `"MUSE_HOME" muse code`; WebFetch of MD/muse-code/configuration, /auth, /permissions, /interactive, /extending, and the full /changelog (both halves) searching for `MUSE_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `~/.local`, "sessions are stored", "Keychain", "account". Only `XDG_CONFIG_HOME` and Keychain mentions were found; no home variable, session path or multi-account guidance was found by this search.
- **S-4 Terms** (2026-10-21). WebSearch "Meta Muse Code terms of service subscription automation multiple accounts"; WebFetch of the product page footer links and MD/pricing-rate-limits. Only the generic Meta policy centre link was found. No Muse-specific account or automation clause was found by this search.
