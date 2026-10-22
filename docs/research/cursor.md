# Research notes: Cursor

Research date: 2026-10-21

> Deep dive on Cursor (Anysphere, Inc., now a SpaceX subsidiary) for AI Juggler, covering two surfaces: the **desktop app** (a VS Code fork, `Cursor.app`) and the **Cursor CLI** (`agent`, legacy name `cursor-agent`). It mirrors `codex.md`.
> Nothing was installed, signed in or executed. No local Cursor state was read. Every claim below was read from the web on the date shown.
>
> **Legend:**
> - **[Verified-primary]**: Cursor's own docs, changelog, install script, terms or pricing page say it.
> - **[Verified-secondary]**: Cursor forum staff, Homebrew metadata, or a reputable third-party reading of the bundle say it.
> - **[Negative-evidence]**: not found by this search. Each one has an entry in the Search log (S-nn).
> - **[Inferred]**: our own reasoning. It is never used as fact.
>
> Format: `- **[Label]** Claim. — Source: <url> (accessed 2026-10-21)`. Quotes are kept under 15 words.

## 0. Context that changes everything

- **[Verified-primary]** Cursor's own blog post (dated 2026-08-14) says SpaceX has acquired it: "Cursor has officially been acquired by SpaceX." — Source: https://cursor.com/blog/joining-spacex (accessed 2026-10-21)
- **[Verified-primary]** New customers start on **"The X Subscription"**, which bundles Grok, Cursor and X. Grok and Cursor share one weekly included-usage pool, and switching merges eligible Cursor and Grok accounts. Older Cursor plans (Pro, Pro+, Ultra, Teams) are now "legacy". — Source: https://cursor.com/help/the-x-subscription/overview ; https://cursor.com/docs/account/pricing/legacy-cursor-plans (accessed 2026-10-21)
- **[Inferred]** Plan names, usage windows and billing surfaces changed in 2026, and usage now shows at grok.com. Expect more churn, so re-verify billing claims every release candidate.

## 1. Distribution & versioning

### Desktop app
- **[Verified-secondary]** The Homebrew cask is `cursor` at version **3.26.13** (commit `3bfec373…`). It installs `Cursor.app` to `/Applications/Cursor.app`, has `auto_updates: true`, and requires macOS 12 or later. The download comes from `downloads.cursor.com/production/<commit>/darwin/<arch>/Cursor-darwin-<arch>.zip`. — Source: https://formulae.brew.sh/api/cask/cursor.json (accessed 2026-10-21)
- **[Verified-secondary]** The bundle id is **`com.todesktop.230313mzl4w4u92`**. It appears in the cask's zap paths, such as `~/Library/Caches/com.todesktop.230313mzl4w4u92` and `~/Library/Preferences/com.todesktop.230313mzl4w4u92.plist`. The app is a ToDesktop build that updates through ShipIt. — Source: https://formulae.brew.sh/api/cask/cursor.json (accessed 2026-10-21)
- **[Verified-secondary]** The shell command **`cursor`** is a link to `Cursor.app/Contents/Resources/app/bin/code`. Homebrew links it as `$HOMEBREW_PREFIX/bin/cursor`. Otherwise the user installs it from the editor's Command Palette (forum). — Source: https://formulae.brew.sh/api/cask/cursor.json ; https://forum.cursor.com/t/how-to-open-cursor-from-terminal/3757 (accessed 2026-10-21)
- **[Verified-primary]** The app is available for Mac, Windows and Linux (apt/dnf, or AppImage/archive). The macOS install is: download from cursor.com/download, then drag the app into Applications. — Source: https://cursor.com/help/getting-started/install (accessed 2026-10-21)
- **[Inferred]** Ways to detect it:
  - check that `/Applications/Cursor.app` (or `~/Applications/Cursor.app`) exists;
  - look up bundle id `com.todesktop.230313mzl4w4u92` with Launch Services;
  - read the version from `Contents/Info.plist` `CFBundleShortVersionString`. `cursor --version` (VS Code CLI semantics) should also work, but this is unverified.
- **[Verified-primary]** The desktop changelog lists dated feature posts with no version numbers. Recent ones:
  - 2026-10-16: Cloud Agent Canvases
  - 2026-10-06: "Remote control for local agents" from iOS
  - 2026-09-10: Cursor Projects

  Source: https://cursor.com/changelog (accessed 2026-10-21)

### CLI
- **[Verified-primary]** Install with `curl https://cursor.com/install -fsS | bash` (macOS, Linux, WSL). Native Windows uses `irm 'https://cursor.com/install?win32=true' | iex`. Then add `~/.local/bin` to PATH. Check the version with `agent --version` and update with `agent update`. The CLI auto-updates by default. — Source: https://cursor.com/docs/cli/installation (accessed 2026-10-21)
- **[Verified-primary]** The install script (read as text, not executed) does the following:
  - downloads `https://downloads.cursor.com/lab/<ver>/<darwin|linux>/<x64|arm64>/agent-cli-package.tar.gz`;
  - extracts it to **`~/.local/share/cursor-agent/versions/<ver>/`**;
  - creates **two symlinks**, `~/.local/bin/agent` (primary) and `~/.local/bin/cursor-agent` (legacy), both pointing at `…/versions/<ver>/cursor-agent`.

  The version format is `YYYY.MM.DD-<shortsha>`, currently **`2026.10.19-6bbf2c4`**. — Source: https://cursor.com/install (accessed 2026-10-21)
- **[Verified-secondary]** The rename: "The new primary entrypoint is `agent` (`cursor-agent` still works as an alias)." This comes from a forum release post dated 2026-01-09. — Source: https://forum.cursor.com/t/cursor-cli-jan-8-2026-new-commands-and-performance-improvement/148372 (accessed 2026-10-21)
- **[Negative-evidence]** No rename entry was found in Cursor's own CLI changelog by this search. The current docs use only `agent` (S-01).
- **[Verified-secondary]** The Homebrew cask `cursor-cli` (2026.10.19-6bbf2c4) links **only `cursor-agent`**, not `agent`, into `$HOMEBREW_PREFIX/bin`. Its zap paths are `~/.config/cursor-agent`, `~/.local/share/cursor-agent` and `~/Library/Logs/CursorAgent`. — Source: https://formulae.brew.sh/api/cask/cursor-cli.json (accessed 2026-10-21)
- **[Inferred]** Ways to detect it:
  - look for `agent` or `cursor-agent` on PATH, in `~/.local/bin`, or in `$HOMEBREW_PREFIX/bin`;
  - resolve the symlink into `~/.local/share/cursor-agent/versions/` to get the version without spawning;
  - run `agent --version` / `cursor-agent --version`.

  The bare name `agent` is generic and can collide with other tools, so the adapter should prefer resolving `cursor-agent`, or check that the symlink target sits under `cursor-agent/versions`.
- **[Verified-primary]** Release cadence is roughly weekly or faster. Recent versions are v2026.09.15, v2026.09.22, v2026.09.28 and v2026.10.08. — Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Negative-evidence]** No open-source repository for the CLI was found by this search. Package listings show it as proprietary (Gentoo "all-rights-reserved", AUR "LicenseRef-Cursor") (S-02).
- **[Verified-secondary]** One third-party tool says it verified Cursor 3.5.38 "using the CLI bundled in the app", which suggests the desktop app ships an agent binary. — Source: https://jazzyalex.github.io/agent-sessions/guides/cursor-agent-local-history.html (accessed 2026-10-21). Still an open question.

## 2. Configuration system

### Desktop app
- **[Verified-secondary]** Per the cask zap list, app state lives in:
  - `~/Library/Application Support/Cursor` (VS Code layout: `User/settings.json`, `User/globalStorage/state.vscdb`, `User/workspaceStorage/`)
  - `~/Library/Logs/Cursor`
  - `~/Library/Caches/com.todesktop…`
  - **`~/.cursor`**

  Source: https://formulae.brew.sh/api/cask/cursor.json (accessed 2026-10-21)
- **[Verified-secondary]** Cursor staff (Dean Rie) recommend **`cursor --user-data-dir="~/cursor-work"`** to run separate instances and say "Each instance keeps its own login." — Source: https://forum.cursor.com/t/feature-request-multi-account-support-fast-account-switching/166689 (accessed 2026-10-21)
- **[Verified-secondary]** User reports dated 2026-10-17 say:
  - `--user-data-dir` plus `--extensions-dir` keeps logins separate on macOS;
  - both instances share one Dock identity;
  - both **still share `~/.cursor`**, which holds CLI agent state, MCP config and chats.

  A duplicated or re-signed Cursor.app broke after an update. — Source: https://forum.cursor.com/t/seamless-account-switching-in-cursor/58411?page=6 (accessed 2026-10-21)
- **[Verified-primary]** These things live under `~/.cursor` regardless of the editor's user-data dir: `~/.cursor/mcp.json`, `~/.cursor/hooks.json`, `~/.cursor/worktrees/`, `~/.cursor/skills-cursor`, `~/.cursor/settings.json` (plugins), and `~/.cursor/cli-config.json`. — Source: https://cursor.com/docs/mcp ; https://cursor.com/docs/hooks ; https://cursor.com/docs/cli/reference/parameters ; https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Negative-evidence]** No Cursor doc page was found that documents `--user-data-dir`, or any env var that relocates `~/.cursor` for the editor (S-03).
- **[Verified-primary]** Enterprise or MDM hooks live at `/Library/Application Support/Cursor/hooks.json` on macOS. — Source: https://cursor.com/docs/hooks (accessed 2026-10-21)

### CLI
- **[Verified-primary]** Global config is `~/.cursor/cli-config.json`. Project config is `<project>/.cursor/cli.json`, and "Only permissions can be configured at the project level."
  - **`CURSOR_CONFIG_DIR`** sets a custom config directory.
  - `XDG_CONFIG_HOME` (Linux/BSD) moves the config to `$XDG_CONFIG_HOME/cursor/cli-config.json`.
  - Keys: `version`, `editor.vimMode`, `permissions.allow/deny`, `model`, `maxMode`, `approvalMode` (`allowlist|auto-review|unrestricted`), `sandbox.*`, `network.useHttp1ForAgent`, `attribution.*`.
  - Corrupt files are backed up as `.bad`, and some fields are CLI-managed.

  Source: https://cursor.com/docs/cli/reference/configuration (accessed 2026-10-21)
- **[Verified-primary]** Since v2026.07.06, config writes stage to a temp file and rename atomically, to survive concurrent agents. — Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Verified-secondary]** A third-party reading of the bundle (Cursor Agent 2026.09.18) says `CURSOR_CONFIG_DIR` is "read only by the config-dir function for `cli-config.json`". It does **not** move credentials, so setting it alone left the sign-in shared. — Source: https://github.com/codepawl/orglet/pull/419 (accessed 2026-10-21)
- **[Negative-evidence]** No Cursor documentation was found saying `CURSOR_CONFIG_DIR` also moves `~/.cursor/chats`, `~/.cursor/projects`, `mcp.json`, `hooks.json` or credentials (S-04).
- **[Verified-primary]** Permission rule syntax is `Shell(git)`, `Read(.env*)`, `Write(**/*.key)`, `WebFetch(domain)` and `Mcp(server:tool)`. Deny overrides allow. — Source: https://cursor.com/docs/cli/reference/permissions (accessed 2026-10-21)
- **[Verified-primary]** Rules: the CLI loads `.cursor/rules` plus `AGENTS.md` and `CLAUDE.md` at the project root. — Source: https://cursor.com/docs/cli/using (accessed 2026-10-21)

## 3. Auth & multiple accounts

### Desktop app
- **[Verified-primary]** You sign in with a Cursor account when the app first opens. — Source: https://cursor.com/help/getting-started/install (accessed 2026-10-21)
- **[Verified-secondary]** Third-party tools read the desktop's session JWT from `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb`, table `ItemTable`, key `cursorAuth/accessToken`. — Source: https://raw.githubusercontent.com/miiiiiiich/agent-walker/HEAD/docs/cursor.md (accessed 2026-10-21). **AI Juggler must not read this file** (see section 7).
- **[Verified-secondary]** Staff say:
  - "there's no native account switcher yet, and it's not on the roadmap right now" (2026-07-29);
  - it is "on our radar" but still not on the roadmap (2026-10-11);
  - "Having multiple accounts is totally fine"; the only restriction is sharing one account between users.

  Source: https://forum.cursor.com/t/feature-request-multi-account-support-fast-account-switching/166689 (accessed 2026-10-21)
- **[Verified-secondary]** The workaround is one `--user-data-dir` per account, which gives a separate login per instance. `~/.cursor` stays shared (see section 2). A community extension, "Userdata Switcher", wraps the same mechanism. — Source: https://forum.cursor.com/t/i-built-an-extension-to-run-work-and-personal-cursor-accounts-at-the-same-time-foss-no-sign-out-dance-no-hacks-uses-native-userdata-mechanism/163254 (accessed 2026-10-21)

### CLI
- **[Verified-primary]** Sign-in methods:
  - `agent login` opens a browser. `NO_OPEN_BROWSER=1` prints the URL instead, and since v2026.07.06 it can also show a QR code.
  - `agent logout`.
  - `agent status` / `whoami` / `about`, which take `--format json`.
  - API key via **`CURSOR_API_KEY`** or `--api-key`, using a user key from the dashboard.

  The docs say only that credentials are "securely stored locally". — Source: https://cursor.com/docs/cli/reference/authentication ; https://cursor.com/docs/cli/reference/parameters (accessed 2026-10-21)
- **[Verified-primary]** ACP mode also accepts `--auth-token` / **`CURSOR_AUTH_TOKEN`**. — Source: https://cursor.com/docs/cli/acp (accessed 2026-10-21)
- **[Verified-primary]** Credential storage:
  - On macOS the default store is the Keychain. v2026.06.29 says: "Run Cursor in sandboxes without macOS Keychain."
  - **`AGENT_CLI_CREDENTIAL_STORE=file`** stores credentials "unencrypted in an owner-only file".
  - v2026.08.11 refers to "the ~/.cursor folder that stores CLI credentials".

  Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Verified-secondary]** Staff (Mohit Jain, 2026-08-04) named three Keychain items, all with account **`cursor-user`**: services **`cursor-access-token`**, **`cursor-refresh-token`** and **`cursor-api-key`**. File mode writes **`~/.cursor/auth.json`**. — Source: https://forum.cursor.com/t/errsecitemnotfound-couldnt-find-your-saved-login-in-the-macos-keychain/167325 (accessed 2026-10-21)
- **[Verified-secondary]** According to a reading of the bundle (2026.09.18), the Keychain service and account names are constants. "The Keychain item is one per computer and no variable moves it."
  - The macOS file path is `join(homedir(), ".cursor", "auth.json")`.
  - Linux uses `$XDG_CONFIG_HOME/cursor/auth.json`; Windows uses `%APPDATA%\Cursor\auth.json`.
  - `AGENT_CLI_CREDENTIAL_STORE` takes `file` or `memory`.
  - `CURSOR_AUTH_TOKEN` writes to the same store.

  That project disabled per-account Cursor sign-in on macOS as a result. — Source: https://github.com/codepawl/orglet/pull/419 (accessed 2026-10-21)
- **[Verified-secondary]** One multi-account competitor (AgentsRoom) says setting `CURSOR_API_KEY` "does not switch account" once a stored session exists. It isolates accounts with one directory per account. — Source: https://agentsroom.dev/docs/multi-account.md (accessed 2026-10-21). This conflicts with the bundle reading above on macOS and is unverified.
- **[Negative-evidence]** No Cursor doc was found that describes multiple CLI accounts or profiles, or an env var that relocates CLI credentials (S-04, S-05).
- **[Negative-evidence]** No Cursor doc was found saying whether `agent login` and the desktop sign-in share credentials (S-06).
  - **[Inferred]** They are separate stores: the desktop uses `state.vscdb` and the CLI uses the Keychain or `~/.cursor/auth.json`. A third-party usage tool treats them as distinct sources and avoids mixing them. — Source: https://github.com/akitaonrails/ai-usagebar/pull/283 (accessed 2026-10-21)
- **[Inferred]** Possible isolation levers for CLI accounts on macOS, in order of preference:
  - (a) A per-account **user API key** in `CURSOR_API_KEY`. This is a documented credential, but it needs a live test on whether an existing Keychain login overrides it.
  - (b) `AGENT_CLI_CREDENTIAL_STORE=file` plus a per-account `HOME`. This works because Node `homedir()` honours `HOME`. It also moves `~/.cursor`, chats, rules and MCP config, and it affects child tools such as git, ssh and npm, so it is heavy and fragile.
  - (c) One CLI account per Mac.
- **[Verified-primary]** The SDK's `Cursor.auth.login()` mints a 90-day user API key and stores it in `~/.cursor/sdk/auth.json`, a separate file. "does not read credentials from a local Cursor app installation". `Cursor.me()` returns `userEmail` and related fields for a key. — Source: https://cursor.com/docs/sdk/typescript (accessed 2026-10-21)

## 4. Sessions

### Desktop app
- **[Verified-secondary]** Agent transcripts are stored at `~/.cursor/projects/<encoded-workspace-path>/agent-transcripts/<conversation_id>/<conversation_id>.jsonl`. A staff reply confirms they are local only:
  - "There is no server copy."
  - "Writing transcripts to disk is not atomic."
  - There is "no supported way to recover lost transcripts".

  Source: https://forum.cursor.com/t/chat-history-gone-after-pc-restart-agent-transcripts-files-emptied-how-to-recover/158251 ; https://forum.cursor.com/t/cursor-cli-past-chats-not-showing-up/152450/20 (accessed 2026-10-21)
- **[Verified-secondary]** The folder name comes from the project's absolute path, so it changes when the project moves. The project index sits in `globalStorage/state.vscdb` under key `glass.localAgentProjects.v1`. Older chats (and side-pane chats) are stored in `state.vscdb` table `cursorDiskKV`. — Source: https://forum.cursor.com/t/allow-storing-conversation-history-inside-the-project-directory/160515 ; https://github.com/kenn-io/agentsview/issues/1515 (accessed 2026-10-21)
- **[Verified-primary]** Hook payloads carry `conversation_id`, `generation_id`, `transcript_path` ("null if transcripts disabled"), `model`, `cursor_version`, `workspace_roots` and `user_email`. The env vars `CURSOR_TRANSCRIPT_PATH` and `CURSOR_PROJECT_DIR` are also set. — Source: https://cursor.com/docs/hooks (accessed 2026-10-21)
- **[Verified-secondary]** The transcript JSONL holds user and assistant text plus tool_use name and input. It lacks tool results, timestamps and token usage. — Source: https://forum.cursor.com/t/richer-agent-transcripts-lifecycle-data-for-observability-langfuse-stop-hooks/166592 ; https://github.com/masuda-masuo/ashiato/issues/87 (accessed 2026-10-21)
- **[Negative-evidence]** No Cursor doc was found that gives the transcript path or a schema (S-07). The conversation-search help page mentions only a "local search index". — Source: https://cursor.com/help/ai-features/conversation-search (accessed 2026-10-21)
- **[Verified-primary]** The `cursor://anysphere.cursor-deeplink/prompt?text=…` deeplink pre-fills a chat prompt, and the user must confirm it. Command and rule deeplinks also exist. No "open conversation X" link is documented. — Source: https://cursor.com/docs/reference/deeplinks (accessed 2026-10-21)

### CLI
- **[Verified-primary]** Resume options:
  - `agent ls` opens a picker;
  - `agent resume` resumes the latest chat;
  - `--continue` is an alias for `--resume=-1`;
  - `--resume [chatId]` resumes a given chat;
  - `/resume` works in the TUI;
  - `agent create-chat` creates an empty chat and prints its ID.

  Since v2026.07.06 the pickers show chats from all workspaces. — Source: https://cursor.com/docs/cli/reference/parameters ; https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Verified-primary]** Headless mode:
  - `-p/--print` with `--output-format text|json|stream-json`, and `--stream-partial-output`;
  - without `--force`/`--yolo`, "changes are only proposed, not applied";
  - `--trust` skips the workspace-trust prompt;
  - `--workspace <path>`, `-w/--worktree [name]` (under `~/.cursor/worktrees/<repo>/<name>`), `--model`, and `--mode plan|ask`.

  Source: https://cursor.com/docs/cli/headless ; https://cursor.com/docs/cli/reference/parameters (accessed 2026-10-21)
- **[Verified-primary]** `json` output emits one object `{type:"result", subtype, is_error, duration_ms, duration_api_ms, result, session_id, request_id}`. On failure it writes stderr and emits no JSON. `stream-json` emits these events:
  - `system/init` (`apiKeySource`, `cwd`, `session_id`, `model`, `permissionMode`)
  - `user`
  - `assistant`
  - `tool_call` started/completed (`call_id`)
  - `result`

  "Session IDs remain consistent throughout a single agent execution." — Source: https://cursor.com/docs/cli/reference/output-format (accessed 2026-10-21)
- **[Negative-evidence]** No documented exit codes for headless failures were found (S-08).
- **[Verified-secondary]** Staff say "The CLI writes to `~/.cursor/chats`" and that CLI and IDE history "don't sync with each other on any OS." — Source: https://forum.cursor.com/t/cursor-cli-past-chats-not-showing-up/152450/20 (accessed 2026-10-21)
- **[Verified-secondary]** The store layout is `~/.cursor/chats/<workspace-hash>/<chat-uuid>/store.db` (SQLite) plus `meta.json` (`cwd`, `createdAtMs`, `updatedAtMs`, `schemaVersion:1`).
  - The `store.db` tables are `blobs(id,data)` and `meta(key,value)`.
  - The `meta` table's row is hex-encoded JSON with `agentId`, `latestRootBlobId`, `name` and `mode`.
  - Blobs are content-addressed; some are JSON messages and some are protobuf.

  Observed on cursor-agent 2026.09.10. — Source: https://github.com/masuda-masuo/ashiato/issues/87 ; https://jazzyalex.github.io/agent-sessions/guides/cursor-agent-local-history.html (accessed 2026-10-21)
- **[Verified-primary]** Changelog notes:
  - January 2026: "transcripts persist to disk for tooling and hooks".
  - February 2026: "headless transcripts write Claude Code-compatible JSONL".
  - April 2026: headless errors are recorded in transcripts.

  Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Verified-primary]** Since v2026.08.26, `agent persist` keeps a session running after you disconnect, with `/detach` and `agent persist attach`. — Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Verified-primary]** Prefixing a message with `&` hands the chat to a Cloud Agent, which you can continue at cursor.com/agents. — Source: https://cursor.com/docs/cli/overview (accessed 2026-10-21)
- **[Inferred]** For read-only indexing, use `~/.cursor/projects/*/agent-transcripts/*/*.jsonl` (simple, Claude-compatible, lossy) for content and `~/.cursor/chats/*/*/meta.json` for cwd, timestamps and names. Do not decode the protobuf blobs. Neither format has a stability promise.

## 5. Usage & quota

### Plans (both surfaces share one account and pool)
- **[Verified-primary]** Current plans under The X Subscription:

  | Plan | Monthly | Annual | Usage |
  |---|---|---|---|
  | Free | $0 | — | Limited |
  | Plus | $8 | $80/yr | Base |
  | Premium | $30 | $300/yr | 4x Plus |
  | Super | $100 | $1,000/yr | 15x Plus |
  | Ultra | $200 | $2,000/yr | 50x Plus |
  | Teams | $30/$100/$200 per seat | — | Matches seat tier |
  | Enterprise | Custom | — | Pooled |

  - "one weekly pool of usage for Cursor models", shared across the editor Agent, CLI, Cloud Agents, Grok Bot and the Grok apps.
  - Third-party models (Claude, GPT and others) are billed at API list price from prepaid credit, plus the $0.25/M Cursor Token Rate on Teams.
  - Usage and the reset date appear in editor settings and at grok.com/dashboard.

  Source: https://cursor.com/docs/models-and-pricing ; https://cursor.com/pricing ; https://cursor.com/help/models-and-usage/usage-limits (accessed 2026-10-21)
- **[Verified-primary]** Legacy plans:

  | Plan | Price |
  |---|---|
  | Hobby | Free |
  | Pro | $20 |
  | Pro+ | $60 |
  | Ultra | $200 |
  | Teams Standard | $40/user |
  | Teams Premium | $120/user |
  | Start | ₹649 (India) |

  They use **two monthly pools** ("Cursor Models", "Other Models") plus opt-in on-demand spend. The page gives no dollar figures for included usage. — Source: https://cursor.com/docs/account/pricing/legacy-cursor-plans (accessed 2026-10-21)
- **[Negative-evidence]** No 5-hour window and no numeric rate limits for individual plans were found. Dollar sizes of the weekly pool were not found either (S-09).

### Desktop app
- **[Verified-primary]** Remaining usage and the next reset show in editor settings, and the editor notifies you when usage runs out. — Source: https://cursor.com/help/models-and-usage/usage-limits (accessed 2026-10-21)
- **[Verified-secondary]** Third-party tools read usage from the undocumented `cursor.com/api/dashboard/export-usage-events-csv`, using the `WorkosCursorSessionToken` cookie built from the desktop JWT. — Source: https://raw.githubusercontent.com/miiiiiiich/agent-walker/HEAD/docs/cursor.md (accessed 2026-10-21). **This is off-limits for AI Juggler**: it means reading a credential and using an undocumented endpoint.

### CLI
- **[Verified-primary]** Since v2026.07.13, `/usage` "shows your account's included-usage meters with Auto and API breakdowns", plus on-demand spend, plan name and reset date. — Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Negative-evidence]** `/usage` is missing from the slash-command reference page (S-10). It is interactive only, and no machine-readable form was found.
- **[Verified-primary]** v2026.06.09 added token counts to the status display. February 2026 added "Per-turn input/output/cache token totals and a `request_id` in `stream-json` output". March 2026 added "Per-turn token usage and a stable session ID" to hook payloads. — Source: https://cursor.com/docs/cli/changelog (accessed 2026-10-21)
- **[Negative-evidence]** The output-format reference itself defines no usage or token fields (S-10). The field names must be captured from a live run before use.
- **[Negative-evidence]** No local usage file was found for either surface (S-11).

### APIs
- **[Verified-primary]** The **Admin API** (`api.cursor.com`, Basic auth with an `admin:*` key) is for Enterprise teams only. It offers:

  | Endpoint | Returns | Limits |
  |---|---|---|
  | `POST /teams/daily-usage-data` | Per-user request counts | Range of 30 days or less |
  | `POST /teams/spend` | Current-cycle `spendCents` | — |
  | `POST /teams/filtered-usage-events` | Per-event `tokenUsage` and `chargedCents` | 60/min |

  Cursor recommends polling at most once an hour. — Source: https://cursor.com/docs/api ; https://cursor.com/docs/account/teams/admin-api (accessed 2026-10-21)
- **[Verified-primary]** Per-agent usage APIs:
  - The Cloud Agents API (all plans, beta) has `GET /v1/agents/{id}/usage`, which returns token usage per run (tokens, not cost).
  - The SDK's `agent.getUsage()` returns tokens plus `cost{rawCostCents, chargedCents}`. Local usage is "enabled per account" and still rolling out.

  Source: https://cursor.com/docs/cloud-agent/api/endpoints ; https://cursor.com/docs/sdk/typescript (accessed 2026-10-21)
- **[Negative-evidence]** No documented API was found that returns an **individual** plan's remaining weekly or monthly allowance (S-12).
- **[Inferred]** For AI Juggler:
  - Cursor quota always renders as **unknown or estimate**.
  - Per-turn tokens could render as Measured only after the stream-json and hook fields are captured live.
  - Renewal and reset are visible only in Cursor's or Grok's own UI.

## 6. Extensibility surface

- **[Verified-primary]** **MCP**:
  - config files are `.cursor/mcp.json` (project) and `~/.cursor/mcp.json` (global);
  - transports are stdio, SSE and Streamable HTTP;
  - OAuth is supported, with a desktop callback at `http://localhost:8787/callback`;
  - `${env:NAME}`, `${workspaceFolder}` and similar are interpolated;
  - the CLI provides `agent mcp list|list-tools|login|enable|disable` and `--approve-mcps`.

  Source: https://cursor.com/docs/mcp ; https://cursor.com/docs/cli/reference/parameters (accessed 2026-10-21)
- **[Verified-primary]** **Rules**:
  - `.cursor/rules/**/*.mdc` ("A plain `.md` file in `.cursor/rules` is ignored");
  - User Rules and Team Rules;
  - `AGENTS.md` at the root and nested;
  - the CLI also reads `CLAUDE.md`.

  The rules page itself does not mention `CLAUDE.md` or `.cursorrules`. — Source: https://cursor.com/docs/rules ; https://cursor.com/docs/cli/using (accessed 2026-10-21)
- **[Verified-primary]** **Hooks**:
  - files: `~/.cursor/hooks.json`, `<project>/.cursor/hooks.json`, Team (dashboard) and Enterprise (MDM);
  - events: `sessionStart/End`, `preToolUse/postToolUse`, `beforeSubmitPrompt`, `stop`, `afterAgentResponse`, `preCompact`, `subagentStart/Stop`, `workspaceOpen` and more;
  - exit code 2 blocks, which "matches Claude Code behavior";
  - Claude Code hooks can be loaded.

  `workspaceOpen` "Runs in the Cursor desktop app and CLI". **[Inferred]** `sessionStart`/`stop` hooks give AI Juggler a sanctioned signal for "turn finished or blocked" on both surfaces. — Source: https://cursor.com/docs/hooks (accessed 2026-10-21)
- **[Verified-primary]** **ACP**: `agent acp` runs JSON-RPC 2.0 over stdio.
  - Methods: `initialize`, `authenticate` (`cursor_login`), `session/new`, `session/load` (resume), `session/prompt`, `session/cancel`.
  - Server notifications: `session/update`, `session/request_permission`.
  - Extensions: `cursor/ask_question`, `cursor/create_plan`, `cursor/update_todos`, `cursor/task`.
  - "ACP is intended for building custom clients and integrations." Team MCP servers are not supported in ACP.

  **[Inferred]** This is the purpose-built control plane, the Cursor counterpart of `codex app-server`. — Source: https://cursor.com/docs/cli/acp (accessed 2026-10-21)
- **[Verified-primary]** **Cursor SDK**: `@cursor/sdk` (Node 22.13 or later) and a Python SDK. It has `local` (agent loop runs in-process; inference stays on Cursor's servers) and `cloud` runtimes. It needs a user or service-account API key and bills to that key's plan. The `cursor-sdk-bridge` serves `sdk.v1` protobuf over Connect on loopback for other languages. — Source: https://cursor.com/docs/sdk/typescript ; https://cursor.com/docs/sdk/bridge (accessed 2026-10-21)
- **[Verified-primary]** **Cloud Agents API**: `POST/GET /v1/agents`, `/v1/agents/{id}/runs`, SSE `/runs/{runId}/stream`, cancel, archive and artifacts, with Basic or Bearer auth. It is "not a standalone model-inference or chat-completions API." — Source: https://cursor.com/docs/api ; https://cursor.com/docs/cloud-agent/api/endpoints (accessed 2026-10-21)
- **[Verified-primary]** Other features: Agents Window (local, cloud and SSH agents with worktrees), remote control of local agents from iOS (2026-10-06), `agent worker` self-hosted cloud workers, GitHub Actions, and Origin (code hosting). — Source: https://cursor.com/docs/agent/agents-window ; https://cursor.com/changelog ; https://cursor.com/docs/cli/reference/parameters (accessed 2026-10-21)

## 7. Terms of service

Sources:
- Terms of Service, last updated 2026-09-03 (Anysphere, Inc.): https://cursor.com/terms-of-service
- Acceptable Use Policy, last updated 2026-08-11: https://cursor.com/acceptable-use-policy

Both accessed 2026-10-21.

- **[Verified-primary]** **(a) Multiple accounts**: neither document has a one-account-per-person clause. The AUP bans "creating fake accounts" to mislead. Staff: "Having multiple accounts is totally fine"; the only restriction is sharing one account between users (Verified-secondary, forum 166689).
  - **[Negative-evidence]** No explicit multi-account clause was found (S-13).
- **[Verified-primary]** **(b) Account sharing**: ToS §3 says "solely responsible for maintaining the confidentiality of your account and password". The AUP intro covers others using the Service under your account.
- **[Verified-primary]** **(c) Automation**: the AUP prohibits "Accessing the Service through automated or non-human means, whether through a bot, script, or otherwise", with no API exception stated. It also bars "using bots to access, reverse engineer, decompile…".
  - **[Inferred]** This wording conflicts with Cursor's own docs, which promote `-p` headless use "for scripts, CI, and automation", ACP and the SDK. Launching the official CLI or ACP as the user is the vendor-documented path. Anything that scrapes dashboards or replays tokens is clearly on the wrong side. Ask Cursor for written clarification before marketing automation of Cursor.
- **[Verified-primary]** **(d) Metering**: the AUP prohibits "Manipulating, circumventing, or interfering with Anysphere's subscription, billing, or usage-metering systems" and "circumventing any rate limits or restrictions".
  - **[Inferred]** Rotating accounts to escape usage limits could be read as circumvention. AI Juggler should present switching as a user choice and never automate rotation on exhaustion without the user's say.
- **[Verified-primary]** **(e) Wrappers and reselling**: ToS §1.5(iii) bars "rent, lease, lend, or sell the Service", and the AUP bars "leasing, selling, reselling". Wrappers are not named. The AUP also bars "Providing services that encourage others to violate this AUP". X Subscription purchases are authorised only directly through X Subscription Corp.
- **[Verified-primary]** **(f) Reverse engineering**: ToS §1.5(i) bars reverse engineering, disassembly and decompiling, except where law forbids the restriction.
  - **[Inferred]** Decoding `store.db` protobuf or reading the bundle's credential code sits close to this line. Read only documented or plain-JSON artifacts, and never credential stores.
- **[Verified-primary]** **(g) Termination**: Cursor may suspend or discontinue the Service without notice (ToS §9).

## 8. Pain points (community, brief)

- **[Verified-secondary]** Multi-account is a long-running request: "Seamless Account Switching in Cursor" (active 2026-10-17) and "[Feature Request] Multi-account support" (active 2026-10-11). Staff say it is not on the roadmap. — Source: https://forum.cursor.com/t/seamless-account-switching-in-cursor/58411 ; https://forum.cursor.com/t/feature-request-multi-account-support-fast-account-switching/166689 (accessed 2026-10-21)
- **[Verified-secondary]** macOS Keychain failures (`errSecItemNotFound`, exit 44 or 154) and logins that fail over SSH. Staff suggest file credential mode or a local tmux session. — Source: https://forum.cursor.com/t/errsecitemnotfound-couldnt-find-your-saved-login-in-the-macos-keychain/167325 ; https://forum.cursor.com/t/cursor-cli-login-via-ssh-mosh/149045 (accessed 2026-10-21)
- **[Verified-secondary]** Lost chat history: transcripts emptied after a reboot, the sidebar empty while transcripts sit on disk, and history tied to the absolute path. `state.vscdb` grows to tens of GB. — Source: https://forum.cursor.com/t/chat-history-gone-after-pc-restart-agent-transcripts-files-emptied-how-to-recover/158251 ; https://forum.cursor.com/t/agents-sidebar-empty-222-transcripts-still-on-disk-windows/162757 ; https://forum.cursor.com/t/cursor-state-vscdb-25gb/153661/2 (accessed 2026-10-21)
- **[Verified-secondary]** Competitors already ship Cursor multi-account:
  - AgentsRoom uses one `CURSOR_CONFIG_DIR` per account and has no quota reading;
  - orglet disabled it on macOS;
  - t3code bills per-instance `CURSOR_API_KEY`, and its usage scan misses those instances.

  Source: https://agentsroom.dev/docs/multi-account.md ; https://github.com/codepawl/orglet/pull/419 ; https://github.com/pingdotgg/t3code/issues/15906 (accessed 2026-10-21)

## Synthesis / Verdict for AI Juggler [Inferred]

### CLI (`agent` / `cursor-agent`): **Full adapter, conditional. Size L** (M if the API-key path passes a live test)
- **Launch:** spawn `cursor-agent` (resolved path), interactive or `-p --output-format stream-json`. Use `agent acp` as the structured control plane, the counterpart of codex app-server: `session/new` and `session/load`, permission requests, and streamed updates.
- **Sessions:** index `~/.cursor/projects/*/agent-transcripts/*/*.jsonl` and `~/.cursor/chats/*/*/meta.json` read-only. Resume with `--resume <chatId>`. Use hooks (`stop`, `sessionStart`) for turn and blocked signals.
- **Usage:** per-turn tokens from stream-json and hooks, once captured live. Plan quota stays **unknown**: `/usage` is interactive only, and the Admin API is Enterprise-only.
- **Isolation, the blocker:** `CURSOR_CONFIG_DIR` moves only `cli-config.json`. macOS Keychain items are fixed (`cursor-access-token`/`cursor-user`), and file mode is `$HOME/.cursor/auth.json`. Candidates:
  1. A per-account user API key in `CURSOR_API_KEY`. This is documented, but AI Juggler would have to hold the key in its own Keychain, and override behaviour needs a test.
  2. A per-account `HOME` with `AGENT_CLI_CREDENTIAL_STORE=file`. This is heavy and has side effects.
  3. Ship the CLI as single-account on macOS, as orglet did.
- **Blockers:**
  - no documented credential relocation;
  - the AUP's "automated or non-human means" wording;
  - no sanctioned individual-quota source;
  - the session formats are undocumented.
- **Open questions:**
  - Does `CURSOR_API_KEY` take precedence over a stored Keychain login?
  - Does `CURSOR_CONFIG_DIR` move `chats/` and `projects/`?
  - What are the exact stream-json usage field names?
  - What does `agent status --format json` return (email, plan)?
  - Is a CLI bundled inside Cursor.app?
  - Does the brew `cursor-cli` install lack `agent`?

### Desktop app (Cursor.app): **Track-only. Size S–M**
- **Detect:** bundle id `com.todesktop.230313mzl4w4u92` at `/Applications/Cursor.app`; version from Info.plist.
- **Focus:** focus windows by bundle id. Open a folder with `cursor <path>`. Pre-fill a prompt through the `cursor://anysphere.cursor-deeplink/prompt` deeplink, which the user must confirm.
- **Sessions:** read sessions read-only from `~/.cursor/projects/*/agent-transcripts`, shared with Agents Window chats. Never read `state.vscdb`, which holds the auth JWT.
- **Optional multi-account launch (M):** launch `cursor --user-data-dir=<per-account> --extensions-dir=<per-account>`, as staff recommend. Login is per instance, but `~/.cursor` (MCP config, transcripts, CLI state) stays shared and all instances share one Dock identity. Label this "separate login, shared Cursor folder", not full isolation.
- **Not feasible:** usage or quota (no sanctioned local or individual API source), and driving the editor's agent programmatically (no documented local API).
- **Open questions:**
  - How reliable is `--user-data-dir` on current macOS builds? Reports are mixed.
  - Does the single-instance lock allow two user-data dirs at once from `open -n`?
  - Do transcripts from different user-data dirs collide under the shared `~/.cursor/projects`?

### Cross-cutting
- Plans and billing were restructured after the SpaceX acquisition, with weekly pools under The X Subscription and monthly pools on legacy plans. Model both, and show the plan as "unknown" unless the user states it.
- Pin and re-verify every release candidate: the CLI ships weekly, and the paths come from forum or third-party sources, not docs.

## Search log

All searches and fetches were run on 2026-10-21.

- **S-01 (CLI rename entry):** WebSearch `Cursor CLI "cursor-agent" renamed to "agent" changelog`. Fetched https://cursor.com/docs/cli/changelog twice (asking for rename, alias and cursor-agent mentions) and https://cursor.com/docs/cli/installation and /overview. The rename appears only in forum post 148372.
- **S-02 (CLI source repository):** WebSearch `Cursor CLI agent source code open source GitHub repository license`. The results showed only Gentoo, AUR, a sandbox report and wrappers. No Cursor-published CLI source repository was found.
- **S-03 (editor relocation docs):** WebSearch `Cursor editor two accounts at same time --user-data-dir separate instance forum` and `cursor docs "Install 'cursor' command"` (cursor.com only). Fetched https://cursor.com/help/getting-started/install, /help/getting-started/migrate-vscode, /llms.txt (sitemap scan) and /help/account-and-billing/profiles. `--user-data-dir` appears only in forum staff replies.
- **S-04 (CLI config dir scope and credential relocation):** Fetched https://cursor.com/docs/cli/reference/configuration, /authentication, /parameters and /changelog (asking for CURSOR_CONFIG_DIR, HOME and auth.json). WebSearch `"CURSOR_CONFIG_DIR" cursor agent auth.json keychain` (extended) and `forum.cursor.com CLI multiple accounts CURSOR_CONFIG_DIR switch account agent login`. No vendor statement on what else CURSOR_CONFIG_DIR moves.
- **S-05 (CLI multi-account docs):** Same pages as S-04, plus https://cursor.com/help/integrations/cli and https://cursor.com/cli. No mention of multiple accounts or profiles.
- **S-06 (CLI and desktop login sharing):** WebSearch `forum.cursor.com cursor cli "agent login" separate from IDE login share authentication editor`. Fetched https://cursor.com/docs/cli/reference/authentication and https://cursor.com/help/integrations/cli. Neither says either way.
- **S-07 (transcript path and schema in docs):** Fetched https://cursor.com/docs/hooks, /help/ai-features/conversation-search, /docs/agent/agents-window and /docs/cli/using. WebSearch `cursor agent transcripts ~/.cursor/projects agent-transcripts jsonl location`, `cursor-agent chats stored ~/.cursor/chats store.db sqlite` and `"cursor-agent" "~/.cursor/chats"`. Paths appear only in the forum and in third-party sources.
- **S-08 (headless exit codes):** Fetched https://cursor.com/docs/cli/headless and /docs/cli/reference/output-format. Only zero versus non-zero is shown.
- **S-09 (rate limits, 5-hour windows, pool size):** Fetched https://cursor.com/pricing, /docs/models-and-pricing, /help/models-and-usage/usage-limits and /docs/account/pricing/legacy-cursor-plans. There are multipliers only: no dollar sizes, no RPM, no 5-hour window.
- **S-10 (`/usage` in the reference; usage fields in output-format):** Fetched https://cursor.com/docs/cli/reference/slash-commands (no `/usage`, `/status` or `/quota`) and /docs/cli/reference/output-format (searched for "usage", "token", "inputTokens" and "cache": none). The changelog does mention both features.
- **S-11 (local usage file):** Same pages as S-09 and S-10, plus https://cursor.com/docs/cli/changelog (searched for "usage") and the third-party store.db analysis (ashiato #87: no usage in store.db). None found.
- **S-12 (individual-plan quota API):** Fetched https://cursor.com/docs/api, /docs/account/teams/admin-api, /docs/cloud-agent/api/endpoints and /docs/sdk/typescript. The Admin, Analytics and Usage APIs are Enterprise-team only. The SDK and Cloud APIs report per-agent tokens and cost, not plan allowance.
- **S-13 (multiple-accounts clause in terms):** Fetched https://cursor.com/terms-of-service (searched for "multiple", "one person", "share your account" and "automated means") and https://cursor.com/acceptable-use-policy twice. WebSearch `cursor.com acceptable use policy Anysphere`. No explicit clause.
