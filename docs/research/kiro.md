# Research notes: Kiro (AWS)

Research date: 2026-10-21

> Input to the multi-Provider evaluation; mirrors the structure of [codex.md](codex.md). Nothing was installed, no account was signed in, and no local Kiro, AWS or credential directory was read. Every claim below was accessed on 2026-10-21 unless stated otherwise.
>
> **Legend (per [EVIDENCE-STANDARD.md](EVIDENCE-STANDARD.md)):**
> **[Verified-primary]** the vendor's own docs, source or published package metadata say it ·
> **[Verified-secondary]** a reliable second-hand source (Homebrew, a community tool's docs, a vendor-repo issue by a user) says it ·
> **[Negative-evidence]** not found by this search; the search is logged in the Search log (S-n) ·
> **[Inferred]** our own reasoning; never a fact.
>
> Surfaces: **Desktop** = Kiro IDE (a VS Code–based editor). **CLI** = Kiro CLI (`kiro-cli`), the renamed successor of Amazon Q Developer CLI (`q`). Kiro also ships Web, Mobile (preview) and **Crew** (a 24/7 agent product); those are out of scope apart from mentions.
> Short URLs: `KD/` = `https://kiro.dev/docs/`. `QSRC/` = `https://github.com/aws/amazon-q-developer-cli/blob/15cc8f3cd18c4272925ce1c7053268eedff1ea0a/crates/chat-cli/src/` (the last `main` commit of the predecessor's open-source repo, read via the GitHub API).

## 1. Distribution & versioning

### The rename (CLI)

- **[Verified-primary]** Amazon Q Developer CLI was rebranded to Kiro. Kiro CLI shipped 2025-11-17. Q CLI with auto-update on became Kiro CLI on 2025-11-24; with it off, users run `q update`. — Source: KD/upgrade-guides/migrating-from-q/ and https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/upgrade-to-kiro.html (accessed 2026-10-21)
- **[Verified-primary]** The entry point is `kiro-cli` (also `kiro-cli chat`). The old `q` and `q chat` still work, but the docs recommend `kiro-cli`. — Source: KD/upgrade-guides/migrating-from-q/ (accessed 2026-10-21)
- **[Verified-primary]** Kiro CLI is **closed source**, under the "AWS Intellectual Property License". Q CLI was Apache-2.0. — Source: KD/upgrade-guides/migrating-from-q/; README of https://github.com/aws/amazon-q-developer-cli (accessed 2026-10-21)
- **[Verified-primary]** The old repo's README says it is "no longer being actively maintained" and gets only critical security fixes. The repo is not archived. Its last push was 2026-08-24 and its latest release is `v1.19.7` (2025-11-17). The GitHub API reports Apache-2.0, while the README says MIT plus Apache-2.0. — Source: https://github.com/aws/amazon-q-developer-cli; `gh api repos/aws/amazon-q-developer-cli` (accessed 2026-10-21)
- **[Verified-primary]** Q CLI's own source names its binaries `q` (CLI) and `qchat` (chat). — Source: QSRC/util/consts.rs (`CLI_BINARY_NAME`, `CHAT_BINARY_NAME`) (accessed 2026-10-21)
- **[Verified-primary]** Rolling back means `kiro-cli uninstall`, then installing Q CLI v1.19.7 (`Amazon Q.dmg` on macOS). — Source: KD/upgrade-guides/migrating-from-q/ (accessed 2026-10-21)
- **[Verified-primary]** Connection deadlines: IDE builds before 0.11.133 and CLI builds before 1.28.2 stop connecting on 2026-11-09. IDE 0.11.133–0.12.291 and CLI 1.28.2–2.5.0 stop on 2027-01-15. — Source: https://kiro.dev/downloads/ (accessed 2026-10-21)
- **[Verified-secondary]** A user comment says AWS is ending support for the Amazon Q VS Code extension in favour of Kiro IDE. — Source: https://github.com/kirodotdev/Kiro/issues/7136 (accessed 2026-10-21)

### Desktop (Kiro IDE)

- **[Verified-primary]** Kiro IDE runs on macOS (Intel and Apple Silicon), Windows 10/11 (x64, ARM64) and Linux (x86_64, ARM64). It updates itself in the background, and "Check for Updates..." is in the Kiro menu on macOS. — Source: KD/getting-started/installation/ (accessed 2026-10-21)
- **[Verified-primary]** Kiro IDE is "Built on Visual Studio Code's open source foundation", is rebased periodically onto stable VS Code, and uses the Open VSX extension registry. — Source: KD/upgrade-guides/migrating-from-vscode/ (accessed 2026-10-21)
- **[Verified-primary]** The macOS app path is `/Applications/Kiro.app`. The shell launcher is `kiro` (`kiro .` opens a folder, and `kiro --locate-shell-integration-path <shell>` exists). — Source: KD/ide/troubleshooting/, KD/ide/setup/ (accessed 2026-10-21)
- **[Verified-secondary]** The Homebrew cask `kiro` is at 1.4.13. It installs `Kiro.app` to `/Applications/Kiro.app` and links `Kiro.app/Contents/Resources/app/bin/code` as `kiro`, which is the VS Code launcher layout. It sets `auto_updates true` and needs macOS ≥ 13. Its zap list is `~/Library/Application Support/Kiro`, `~/Library/Preferences/dev.kiro.desktop.plist`, `~/Library/Saved Application State/dev.kiro.desktop.savedState`, and rmdir `~/.kiro`. The DMG comes from `prod.download.desktop.kiro.dev`. — Source: https://formulae.brew.sh/api/cask/kiro.json (accessed 2026-10-21)
- **[Inferred]** The bundle identifier is **`dev.kiro.desktop`**, going by the preference-plist and saved-state names. To detect the app, read `CFBundleIdentifier` and `CFBundleShortVersionString` from `/Applications/Kiro.app/Contents/Info.plist`. `kiro --version` should work, as it does for VS Code forks. The vendor docs don't state the bundle id (S-8).
- **[Verified-primary]** The IDE's latest release in the changelog is 1.4.10 (2026-10-20). 1.0.x shipped in late August 2026, and releases come about every few days. — Source: https://kiro.dev/changelog/ide/ (accessed 2026-10-21)
- **[Verified-primary]** The downloads page still listed "latest" as 0.12.200 on this date, which contradicts the changelog (1.4.10) and Homebrew (1.4.13). — Source: https://kiro.dev/downloads/ (accessed 2026-10-21). **[Inferred]** The page is stale; read the version from the installed bundle, not from this page.

### CLI (Kiro CLI)

- **[Verified-primary]** Install with `curl -fsSL https://cli.kiro.dev/install | bash`. It supports macOS, Windows 11 (PowerShell), and Linux with glibc 2.34+ or a musl variant. — Source: KD/cli/, KD/getting-started/installation/, https://kiro.dev/downloads/ (accessed 2026-10-21)
- **[Verified-secondary]** The macOS curl installer puts `kiro-cli` in `~/.local/bin`, and it updated an existing Q CLI install in place. — Source: https://dev.classmethod.jp/en/articles/kiro-cli-macos-install-guide/ (accessed 2026-10-21)
- **[Verified-secondary]** The Homebrew cask `kiro-cli` is at 2.33.0 and comes from `desktop-release.q.us-east-1.amazonaws.com/2.33.0/Kiro%20CLI.dmg`. It installs **`/Applications/Kiro CLI.app`** and links `Kiro CLI.app/Contents/MacOS/kiro-cli`. On uninstall it removes the launchd job `com.amazon.codewhisperer.launcher`. Its zap list is `~/.kiro`, `~/.local/bin/kiro-cli`, `~/.local/bin/kiro-cli-chat`, `~/Library/Application Support/kiro-cli`, `~/Library/Caches/com.amazon.codewhisperer`, `~/Library/Caches/dev.kiro.cli`, `~/Library/LaunchAgents/com.amazon.codewhisperer.launcher.plist`, `~/Library/Preferences/dev.kiro.cli.plist` and `~/Library/WebKit/com.amazon.codewhisperer`. There is no Homebrew *formula* (S-11). — Source: https://formulae.brew.sh/api/cask/kiro-cli.json (accessed 2026-10-21)
- **[Inferred]** The CLI app's bundle id is **`dev.kiro.cli`**, going by its plist names. There are two binaries, `kiro-cli` and `kiro-cli-chat`. Detection order: `kiro-cli` on PATH, then `~/.local/bin/kiro-cli`, then `/Applications/Kiro CLI.app/Contents/MacOS/kiro-cli`. A legacy `q`/`qchat` may still be on PATH.
- **[Verified-primary]** `kiro-cli --version` / `-V` prints the version, and `kiro-cli version --changelog[=all|=x.y.z]` prints the changelog. `kiro-cli doctor --format json` and `kiro-cli update [-y]` exist. — Source: KD/reference/cli-commands/ (accessed 2026-10-21)
- **[Verified-primary]** The changelog's latest release is 2.32.0 (2026-10-15), with releases every 1–4 days (2.24.0 on 2026-09-23 through 2.32.0). — Source: https://kiro.dev/changelog/cli/ (accessed 2026-10-21). Homebrew already lists 2.33.0.
- **[Verified-primary]** Two agent engines ship side by side. **V2** is the 2.x default; **V3** is an "early release" chosen with `--v3` or setting `chat.agentEngine` = `v3`. V3's session store is "not backward-compatible" with V2. — Source: KD/cli/v3/ (accessed 2026-10-21)
- **[Verified-primary]** On macOS the CLI writes logs to `$TMPDIR/kiro-log/kiro-chat.log`. `KIRO_CHAT_LOG_FILE` and `KIRO_LOG_LEVEL` override it. — Source: KD/cli/setup/, KD/cli/acp/ (accessed 2026-10-21)

## 2. Configuration system

### Shared `~/.kiro` (both surfaces)

- **[Verified-primary]** Global scope is `~/.kiro/` and project scope is `<project>/.kiro/`. Both apply to IDE and CLI. — Source: KD/configuration/ (accessed 2026-10-21)
- **[Verified-primary]** File map (global / project):
  - MCP: `~/.kiro/settings/mcp.json` / `.kiro/settings/mcp.json`
  - Permissions: `~/.kiro/settings/permissions.yaml` / `~/.kiro/workspace-roots/<hash>/permissions.yaml` (per user, outside the repo)
  - Command sandbox: `~/.kiro/settings/sandbox.json` / `.kiro/settings/sandbox.json`
  - Agents, steering, skills, hooks and workflows: each under its own folder, `~/.kiro/<agents|steering|skills|hooks|workflows>/` and `.kiro/…`
  - Powers: `~/.kiro/powers/`
  - Specs: `.kiro/specs/`
  - CLI settings: `~/.kiro/settings/cli.json`

  — Source: KD/configuration/ (accessed 2026-10-21)
- **[Verified-primary]** Precedence: MCP is Agent > Project > Global. A deny in any scope wins. Agents are Project > Global. Steering, skills and hooks merge across scopes. — Source: KD/configuration/ (accessed 2026-10-21)
- **[Verified-primary]** The CLI reads `.amazonq/` in projects without changing it, and `.kiro` wins when both exist. At install it copies Q's `~/.aws/amazonq/{mcp.json,prompts,cli-agents,rules}` once into `~/.kiro/{settings/mcp.json,prompts,agents,steering}`. — Source: KD/upgrade-guides/migrating-from-q/ (accessed 2026-10-21)

### CLI: relocation

- **[Verified-primary]** **`KIRO_HOME`**: "Set an absolute path to keep separate Kiro profiles, such as work and personal." In V3, it overrides the global Kiro directory for configuration, sign-ins, sessions and agent state. Relative paths, the home directory, a filesystem root or one of its ancestors fall back to `~/.kiro`. — Source: KD/reference/settings/ (accessed 2026-10-21)
- **[Verified-primary]** KIRO_HOME must be absolute. Existing MCP sign-ins are copied into it once, on first use. — Source: KD/configuration/ (accessed 2026-10-21)
- **[Verified-primary]** In 2.31.0 (2026-10-13), Kiro "extends `KIRO_HOME` across V3 configuration and state". — Source: https://kiro.dev/changelog/cli/ (accessed 2026-10-21)
- **[Verified-secondary]** Open bugs on KIRO_HOME (maintainer-acknowledged):
  - #10417 (2.13.0, macOS): V3 interactive chat ignores KIRO_HOME, while V2 and `--v3 mcp list` honour it. **[Inferred]** Possibly fixed by 2.31.0.
  - #11348: skills, agents, steering and hooks discovery is not consistently KIRO_HOME-aware.

  — Source: https://github.com/kirodotdev/Kiro/issues/10417, https://github.com/kirodotdev/Kiro/issues/11348 (accessed 2026-10-21)
- **[Verified-secondary]** A second variable, **`KIRO_DATA_DIR`**, moves the platform data directory. A Windows 2.31.1 report says it is honoured for runtime assets, `kas/` and knowledge bases, but **`data.sqlite3` is still written live under `%LOCALAPPDATA%\Kiro-Cli`**. A maintainer (CONTRIBUTOR) labelled it a bug. The reporter quotes docs saying `data.sqlite3` lives in that directory. — Source: https://github.com/kirodotdev/Kiro/issues/11960 (accessed 2026-10-21). **[Negative-evidence]** No KIRO_DATA_DIR doc page was found by this search (S-5).
- **[Verified-primary]** The macOS platform data directory is `~/Library/Application Support/kiro-cli/`; it holds `knowledge_bases/`, for example. — Source: KD/cli/experimental/knowledge-management/ (accessed 2026-10-21)
- **[Verified-primary]** Other environment variables:
  - Credentials and timeouts: `KIRO_API_KEY` (§3), `KIRO_HEADLESS_WORKFLOW_TIMEOUT_SECS`
  - Debugging and sessions: `KIRO_ACP_RECORD_PATH` (JSONL of TUI ACP traffic), `KIRO_SKIP_BINARY_PINNING` (stops the per-session binary copy)
  - Display: `KIRO_NO_PROGRAM_STATUS`, `KIRO_NO_HYPERLINKS`, `KIRO_NO_PROGRESS`, `KIRO_NO_SYNCHRONIZED`, `KIRO_SCREEN_READER`, `KIRO_ASCII_MODE`, `NO_COLOR`

  — Source: KD/reference/settings/, KD/cli/headless/, KD/cli/setup/, KD/cli/terminal-ui/ (accessed 2026-10-21)
- **[Verified-primary]** Settings are managed with `kiro-cli settings [KEY] [VALUE]`, `settings list [--all]` and `--format json`. Relevant keys:
  - Engine and defaults: `chat.agentEngine`, `chat.defaultAgent`, `chat.defaultModel`, `chat.ui` (`tui` | `classic`)
  - Sessions and history: `chat.historyMode` (`session` | `global`), `chat.sessionDashboard.scope`, `chat.sessionDashboard.indexResponses`
  - Notifications and telemetry: `chat.enableNotifications`, `telemetry.enabled`

  — Source: KD/reference/settings/, KD/reference/cli-commands/ (accessed 2026-10-21)
- **[Verified-primary]** Predecessor (Q CLI 1.19.x): global config lived in `~/.aws/amazonq/…`. Settings and the SQLite DB lived at `dirs::data_local_dir()/amazon-q/{settings.json,data.sqlite3}`, which is `~/Library/Application Support/amazon-q/` on macOS. The source shows no environment variable to relocate the data directory. — Source: QSRC/util/paths.rs (`settings_path`, `database_path_static`) (accessed 2026-10-21)

### Desktop: relocation

- **[Negative-evidence]** Not found by this search: any Kiro doc saying whether the IDE honours `KIRO_HOME`, or documenting `--user-data-dir` / `--extensions-dir` for Kiro IDE (S-1).
- **[Verified-secondary]** CodeBurn's Kiro provider says the IDE's newer session store under `~/.kiro/sessions` honours KIRO_HOME. — Source: https://raw.githubusercontent.com/getagentseal/codeburn/main/docs/providers/kiro.md (accessed 2026-10-21)
- **[Inferred]** The VS Code launcher layout (`Resources/app/bin/code` → `kiro`) suggests the standard Code OSS flags `--user-data-dir` and `--extensions-dir` work. Together with `KIRO_HOME` in the launch environment, that could isolate a second IDE instance. Neither is proven, and the IDE's per-user state still sits under `~/Library/Application Support/Kiro` by default (cask zap).

## 3. Auth & multiple accounts

### Sign-in methods

- **[Verified-primary]** Providers: GitHub, Google, AWS Builder ID, AWS IAM Identity Center, external IdP (Okta, Entra ID), and an API key for the CLI in CI or headless use. — Source: KD/getting-started/authentication/ (accessed 2026-10-21)
- **[Verified-primary]** Q CLI supported only Builder ID and IAM Identity Center. Kiro added GitHub and Google. Q Developer Pro subscriptions keep working in Kiro. — Source: KD/upgrade-guides/migrating-from-q/ (accessed 2026-10-21)
- **[Verified-primary]** IDE: you sign in from the welcome screen through a browser and sign out with "Kiro: Sign Out" in the command palette. — Source: KD/getting-started/authentication/ (accessed 2026-10-21)
- **[Verified-primary]** CLI login and identity commands:
  - Login: `kiro-cli login`, with `--social <google|github>`, `--identity-provider <URL>`, `--region`, `--license <pro|free>` and `--use-device-flow`. Device flow works for Builder ID, IdC, Google and GitHub, but not for an external IdP.
  - Sign-out: `kiro-cli logout` clears credentials but keeps agents, conversations and settings.
  - Identity: `kiro-cli whoami --format json` shows the active method.

  — Source: KD/getting-started/authentication/, KD/reference/cli-commands/ (accessed 2026-10-21)
- **[Verified-primary]** API key: available to Pro, Pro+, Pro Max and Power, and admins must enable it on managed subscriptions. Keys are made at app.kiro.dev. Set `KIRO_API_KEY` and run `kiro-cli chat --no-interactive "…"`. API keys work **only in non-interactive mode** and use subscription credits. Precedence is (1) an active `kiro-cli login` browser session, (2) `KIRO_API_KEY`, (3) a sign-in prompt. — Source: KD/getting-started/authentication/, KD/cli/headless/ (accessed 2026-10-21)
- **[Verified-primary]** IdC sessions time out after 8 hours by default. — Source: KD/ide/troubleshooting/ (accessed 2026-10-21)

### Where tokens live

- **[Verified-primary]** Predecessor (Q CLI): OIDC tokens sat in **SQLite** `data.sqlite3`, table `auth_kv` (key/value), not in the macOS Keychain. Keys included `codewhisperer:odic:token` and `codewhisperer:odic:device-registration`, plus state keys `auth.idc.start-url`, `auth.idc.region` and `api.codewhisperer.profile`. The Builder ID start URL was `https://view.awsapps.com/start` (us-east-1). The MCP OAuth cache was `~/.aws/sso/cache`. — Source: QSRC/database/sqlite_migrations/005_auth_table.sql, QSRC/database/mod.rs, QSRC/auth/builder_id.rs, QSRC/auth/consts.rs, QSRC/util/paths.rs (accessed 2026-10-21)
- **[Verified-secondary]** Since kiro-cli 2.16, the CLI keeps sole custody of the login in its SQLite state store, and `kiro-cli login` no longer writes `~/.aws/sso/cache`. Third parties get a token through `kiro-cli chat _ get-kas-token`. The CLI refreshes the token itself, and a second refresher of the rotating token causes `invalid_grant`. Switching the active account can kill a token before it expires. — Source: https://pkg.go.dev/github.com/cplieger/vibekit/internal/kiroauth (v0.8.1, 2026-09-07) (accessed 2026-10-21)
- **[Verified-secondary]** The CLI's macOS DB is `~/Library/Application Support/kiro-cli/data.sqlite3`. — Source: https://pypi.org/project/kiro-cli-history/ (0.4.1, 2026-10-06); corroborated by the cask zap path and #11960 (accessed 2026-10-21)
- **[Verified-primary]** V3 sign-ins live under KIRO_HOME (§2). — Source: KD/reference/settings/ (accessed 2026-10-21). **[Inferred]** V2 sign-ins (in `data.sqlite3` in the platform data directory) are therefore not separated by KIRO_HOME alone. Whether V3 still reads or writes the shared `data.sqlite3` on macOS is **unknown**. #11960 shows the DB being written outside the relocated directory on Windows.
- **[Verified-secondary]** Older IDE builds kept the token at `~/.aws/sso/cache/kiro-auth-token.json` (`accessToken`, `refreshToken`, `expiresAt`, `clientIdHash`, `region`) with a `<clientIdHash>.json` registration file next to it. — Source: https://pkg.go.dev/github.com/cplieger/vibekit/internal/kiroauth, https://docs.ccs.kaitran.ca/providers/kiro (accessed 2026-10-21). **[Negative-evidence]** No vendor doc on IDE token storage was found by this search (S-2), and it is **unknown** whether 1.x builds still use this path.
- **[Negative-evidence]** Not found by this search: any vendor doc giving the CLI's token storage location or Keychain item names (S-4).

### Multiple accounts

- **[Verified-primary]** For the CLI, the vendor explicitly gives KIRO_HOME as the way to "keep separate Kiro profiles, such as work and personal" (§2). This is the closest equivalent of `CODEX_HOME`. — Source: KD/reference/settings/ (accessed 2026-10-21)
- **[Verified-primary]** Kiro "profiles" in the IDE and enterprise sense are AWS region/account profiles (one per AWS account per Region), not user identities. Admins can disable device flow per profile. — Source: KD/ide/troubleshooting/, KD/getting-started/authentication/ (accessed 2026-10-21)
- **[Verified-secondary]** There is no in-app account switcher for the IDE. Users must sign out and back in.
  - #6061 "Easily switch accounts" (open, 2026-02-26) is a feature request for this.
  - #9908 (closed as a duplicate of #7136) asks for parallel billing profiles.

  — Source: https://github.com/kirodotdev/Kiro/issues/6061, https://github.com/kirodotdev/Kiro/issues/9908 (accessed 2026-10-21)
- **[Verified-secondary]** Some users who switched accounts later reported suspensions:
  - #7339: "temporarily suspended" `AccessDeniedException` after switching Builder ID accounts once, following usage limits. Support was referred to AWS.
  - #10043: suspended after the first prompt.

  — Source: https://github.com/kirodotdev/Kiro/issues/7339, https://github.com/kirodotdev/Kiro/issues/10043 (accessed 2026-10-21). **[Inferred]** Kiro has automated abuse detection (§7). Multi-account use to stretch free quotas is exactly what such systems flag.

## 4. Sessions

### CLI

- **[Verified-primary]** Sessions save after every turn and are keyed by directory path. Starting in a folder can load that folder's history. Session ids are UUIDs. A session can be active in only one process, otherwise "Session is active in another process (PID …)". Local sessions don't sync across machines. — Source: KD/cli/chat/session-management/, KD/cli/chat/ (accessed 2026-10-21)
- **[Verified-primary]** The docs say sessions save to "a local SQLite database in `~/.kiro/`". — Source: KD/cli/chat/session-management/ (accessed 2026-10-21). **[Inferred]** This conflicts with the per-version layout below. Treat the on-disk layout as undocumented.
- **[Verified-primary]** CLI v2 stored ACP sessions in `~/.kiro/sessions/cli/` as `<session-id>.json` (metadata) and `<session-id>.jsonl` (event log). **V3 storage is "server-owned"**: clients should not read or write the files and should use ACP `session/list` / `session/load`. — Source: KD/cli/acp/, KD/cli/v3/acp-migration/ (accessed 2026-10-21)
- **[Verified-secondary]** The storage generations are:
  - CLI 1.x: `data.sqlite3` table `conversations`
  - CLI 2.x TUI: `conversations_v2`
  - 2.x classic: `~/.kiro/sessions/cli/*.json{,l}`
  - V3: `~/.kiro/sessions/<workspace-hash>/sess_*/session.json` + `messages.jsonl`

  Credits are recorded only in the 2.x JSONL and V3 stores. — Source: https://pypi.org/project/kiro-cli-history/ (accessed 2026-10-21)
- **[Verified-secondary]** The CLI JSONL is one `{version, kind, data}` event per line, with credits in `metering_usage` in the companion `.json`. The V3/IDE v2 store also has `publish.cursor` and `snapshots/`. Its `messages.jsonl` lines are `{id, timestamp, payload:{type,…}}`, credits are in `usage_summary.promptTurnSummaries[].usage`, and **there are no token counts**. — Source: https://raw.githubusercontent.com/getagentseal/codeburn/main/docs/providers/kiro.md (accessed 2026-10-21)
- **[Verified-primary]** Resume and management commands:
  - Resume: `kiro-cli chat --resume` (newest in this directory), `--resume-picker`, `--resume-id <ID>` (local or cloud)
  - List, delete, browse: `--list-sessions`, `--delete-session <ID>`, `--sessions` (V3 dashboard)
  - In chat: `/chat new|resume|save|load`, `/session-id`, `/sessions [clean]`. `/chat save` writes a zip in V3 and JSON in 2.x.
  - On exit, Kiro prints the session id and resume command.

  — Source: KD/cli/chat/session-management/, KD/reference/cli-commands/, KD/reference/slash-commands/ (accessed 2026-10-21)
- **[Verified-primary]** Headless runs use `kiro-cli chat --no-interactive "<prompt>"` (or stdin), with these flags:
  - Tool trust: `--trust-all-tools`, `--trust-tools=<ids>`
  - Agent and model: `--agent`, `--model`, `--effort`
  - Output: `--output-format stream-json` (JSON Lines, V2 or V3)
  - Run control: `--require-mcp-startup` (exit 3), `--sandbox`, `--cloud --repo owner/repo`

  Exit codes are 0, 1 (general, including auth), 3 and 4. There is no mid-session input. — Source: KD/cli/headless/, KD/reference/exit-codes/ (accessed 2026-10-21)
- **[Verified-primary]** Cloud sessions (`--cloud`) run in a managed sandbox in us-east-1 and are stored in the account's cloud store. — Source: KD/cli/chat/session-management/, https://kiro.dev/faq/ (accessed 2026-10-21)

### Desktop

- **[Verified-primary]** Sessions open as tabs, with History and an execution history you can search, restore and delete. "Export Conversation" produces Markdown (in the Chat docs) or a read-only zip of `<sessionId>/session.json`, `messages.jsonl` and `sub-executions/*.jsonl` (in Export docs), which cannot be imported back. — Source: KD/ide/chat/, KD/ide/chat/chat-export/ (accessed 2026-10-21)
- **[Verified-primary]** IDE 1.0 brought a new session storage format, and 0.x sessions are migrated on open. Agent Focus Mode (experimental) runs several sessions in parallel. — Source: KD/ide/whats-new-v1/ (accessed 2026-10-21)
- **[Verified-primary]** IDE, CLI, Web and Mobile all use ACP to reach "the same unified agent harness". — Source: KD/acp/ (accessed 2026-10-21)
- **[Verified-secondary]** On macOS the IDE stores sessions under `~/Library/Application Support/Kiro/User/globalStorage/kiro.kiroagent/`, in these layouts:
  - legacy `<workspace-hash>/<execution-id>.chat`
  - extensionless files since about February 2026
  - `workspace-sessions/<base64url(path)>/<uuid>.json`
  - the newer v2 store in `~/.kiro/sessions/<workspace-hash>/sess_<id>/`

  — Source: https://raw.githubusercontent.com/getagentseal/codeburn/main/docs/providers/kiro.md (accessed 2026-10-21)
- **[Verified-secondary]** A user reports `kiro.kiroagent` session directories of 1–2 GB each, 13 GB+ in total. — Source: https://github.com/kirodotdev/Kiro/issues/5469 (via search result; accessed 2026-10-21)
- **[Negative-evidence]** Not found by this search: any vendor doc giving the IDE's on-disk session location, or a headless or resume CLI flag for the IDE (S-3).

## 5. Usage & quota

- **[Verified-primary]** Plans (per month) and credits:

  | Plan | Price | Credits/month |
  |---|---|---|
  | Free | $0 | 50 |
  | Pro | $20 | 1,000 |
  | Pro+ | $40 | 2,000 |
  | Pro Max | $100 | 5,000 |
  | Power | $200 | 10,000 |

  - Add-on credits cost $0.04 each and are billed at once, in packs from $5 (125 credits). They expire after 12 months.
  - Teams (2–500 users) pay overages at $0.04 per credit. Enterprise overages are off by default.
  - First upgrade gives a $20 credit (social login or Builder ID only).
  - GovCloud costs about 20% more.

  — Source: https://kiro.dev/pricing/, KD/billing/ (accessed 2026-10-21)
- **[Verified-primary]** The limit model is **one monthly credit pool**, with no 5-hour or weekly window:
  - Credits reset at the start of each billing month and don't roll over.
  - Credits are "consumed fractionally based on each request" and are spent the same way from IDE, CLI and Web.
  - Hooks and workflows also use credits.
  - Model multipliers apply: a task costing X in Auto costs 1.3X in Sonnet 4.6.
  - Free users are "subject to rate limits".

  — Source: https://kiro.dev/pricing/, KD/billing/ (accessed 2026-10-21)
- **[Verified-primary]** Usage is shown "at least every 5 minutes" in the subscription dashboard. The IDE path is gear → Account & Billing → Manage Subscription, which opens a browser portal. A low-credit notification fires at 80% of monthly credits, controlled by the "Notifications: Billing" setting. — Source: https://kiro.dev/pricing/, https://kiro.dev/faq/, KD/billing/subscription-portal/, KD/billing/proactive-usage-notifications/ (accessed 2026-10-21)
- **[Verified-primary]** In the CLI, `/usage` shows "billing and credits information": a panel with usage limits, a progress bar and the credit balance. Non-interactive runs print the full breakdown. `/context show` gives a per-file token breakdown. There is no persistent status line showing credits. — Source: KD/reference/slash-commands/, KD/cli/terminal-ui/ (accessed 2026-10-21)
- **[Verified-primary]** **ACP (V3) carries usage:** `session_info_update` with `_meta.kiro.kind` set to `context_usage` (`usagePercentage`) or `turn_completion` (`promptTurnSummaries` turn metering, elapsed time, status). There is an **`_kiro/account/getUsage`** extension method when advertised, and `_kiro/error/rate_limit`. — Source: KD/cli/v3/acp-migration/ (accessed 2026-10-21). **[Inferred]** This is the sanctioned, structured route to plan usage for a supervisor that spawns `kiro-cli acp`. The response shape is **unknown** until a live capture.
- **[Verified-secondary]** There is no non-interactive `kiro-cli usage --format json`: #8477 (open, 2026-05-14) is a feature request for it, and #9330 is a duplicate. A community workaround in #8477 reads the CLI's SQLite auth and calls the `GetUsageLimits` service API directly. — Source: https://github.com/kirodotdev/Kiro/issues/8477, https://github.com/kirodotdev/Kiro/issues/9330 (accessed 2026-10-21). **[Inferred]** AI Juggler must not do this, because it reads credentials (codex.md §9 precedent).
- **[Negative-evidence]** Not found by this search: a public usage or quota REST API for individual subscribers, a quota-specific exit code, or an IDE status-bar credit meter (S-6, S-9, S-10).
- **[Verified-secondary]** Local per-turn **credits** (not tokens) are in the 2.x JSONL `metering_usage` and the V3 `promptTurnSummaries[].usage`. Third-party tooling converts them at $0.04 per credit. — Source: https://raw.githubusercontent.com/getagentseal/codeburn/main/docs/providers/kiro.md (accessed 2026-10-21). **[Inferred]** Per-turn credits could be shown as Measured after an AJ-E601 live capture. The remaining monthly balance stays an Estimate unless `_kiro/account/getUsage` is proven.

## 6. Extensibility surface

- **[Verified-primary]** **ACP** (both surfaces' harness):
  - Transport: `kiro-cli acp` speaks JSON-RPC 2.0 over stdio, protocol version 1.
  - V3 launch: `kiro-cli acp --agent-engine=v3`, plus `--auth-method=client` if the client supplies tokens via `_kiro/auth/getAccessToken`. Without that flag, V3 uses the user's Kiro CLI login.
  - V3 methods: `initialize`, `authenticate`, `session/new|list|load|fork|prompt|cancel|set_mode|set_config_option`.
  - Kiro extensions are advertised in `agentCapabilities._meta.kiro.extensionMethods` (e.g. `_kiro/session/rename`, `_kiro/mcp/status`, `_kiro/hooks/list`, `_session/steer`).
  - Clients: Zed and JetBrains, and "custom integrations" are a named client type.

  — Source: KD/acp/, KD/cli/acp/, KD/cli/v3/acp-migration/ (accessed 2026-10-21)
- **[Verified-primary]** **MCP** config is `~/.kiro/settings/mcp.json` and `.kiro/settings/mcp.json` (a `mcpServers` object), plus inline `mcpServers` in agents. It supports stdio and HTTP/SSE in both IDE and CLI, with `kiro://` install links. The CLI manages servers with `kiro-cli mcp add|remove|list|import|status`. — Source: KD/mcp/, KD/configuration/, KD/reference/cli-commands/ (accessed 2026-10-21)
- **[Verified-primary]** **Steering / AGENTS.md:**
  - Folders: `.kiro/steering/` and `~/.kiro/steering/` (global steering is IDE-only, per the table).
  - Inclusion modes: always, fileMatch, manual, auto.
  - **AGENTS.md** is read from `~/.kiro/steering/`, the workspace root and subdirectories, always included, in both IDE and CLI.
  - Predecessor: Q CLI loaded `AmazonQ.md`, `AGENTS.md` and `README.md` by default.

  — Source: KD/steering/, QSRC/util/paths.rs (accessed 2026-10-21)
- **[Verified-primary]** **Hooks** are JSON v1 files in `.kiro/hooks/` (and `~/.kiro/hooks/` in CLI V3 only). Triggers include `PostFileSave`, `PreToolUse`, `PostToolUse` and `Stop` (V3 also renamed `agentSpawn` to `SessionStart`). Actions are `command` (session JSON on stdin; exit 2 blocks a PreToolUse) or `agent`. — Source: KD/hooks/, KD/cli/v3/, KD/reference/exit-codes/ (accessed 2026-10-21). **[Inferred]** A `Stop` command hook could give a "turn finished" ping like Codex `notify`, but writing it means AI Juggler changes the user's hook config. That is only acceptable inside an AI Juggler-owned KIRO_HOME.
- **[Verified-primary]** **Custom agents** live at `~/.kiro/agents/` and `.kiro/agents/` as `.json` or `.md` with identical fields (`name`, `prompt`, `model`, `tools`, `permissions`, `resources`, `includeMcpJson`, …). They can run as subagents, and `kiro-cli agent list|create|edit|validate|set-default` manages them. Skills, Powers, Workflows and Specs exist too. — Source: KD/custom-agents/, KD/configuration/, KD/reference/cli-commands/ (accessed 2026-10-21)
- **[Verified-primary]** Terminal integration: OSC 7501 program status shows when Kiro waits for approval, and the terminal title reflects agent state (streaming, pending approval, error). — Source: KD/cli/terminal-ui/ (accessed 2026-10-21). **[Inferred]** This is a cheap "blocked/waiting" signal if AI Juggler hosts the PTY.
- **[Negative-evidence]** Not found by this search: an SDK, or an app-server-style daemon beyond ACP (S-6).

## 7. Terms of service

Governing documents: AWS Customer Agreement plus AWS Service Terms §50 (AI Services, last updated 2026-10-13), plus the AWS Intellectual Property License (2025-10-27). The IDE and CLI are licensed as "AWS Content". — Source: https://kiro.dev/license/, https://aws.amazon.com/service-terms/, https://aws.amazon.com/legal/aws-ip-license-terms/ (accessed 2026-10-21)

- **[Verified-primary]** **(c) Third-party wrappers:** the pricing FAQ says use "through third-party automation harnesses (such as OpenClaw)" that route requests "outside of Kiro's native interfaces is not permitted". The FAQ lists the native interfaces as IDE, CLI, Web, Crew, ACP-compatible IDEs and CI/CD automation. — Source: https://kiro.dev/pricing/, https://kiro.dev/faq/ (accessed 2026-10-21). **[Inferred]** AI Juggler launching the vendor's own `kiro-cli` (TUI or `acp`) keeps requests inside a native interface, unlike a proxy that replays tokens against the backend. No vendor ruling exists on supervisors, so this is the main ToS open question.
- **[Verified-primary]** **(a) Multiple accounts:** "For team usage, each developer needs their own subscription" (no sharing). — Source: https://kiro.dev/pricing/ (accessed 2026-10-21). **[Negative-evidence]** Not found by this search: an explicit "one account per person" rule or a statement allowing several personal accounts (S-7).
- **[Verified-primary]** **(b) Automation:** CI/CD automation is allowed ("automation in software development (ex: reviews during CI/CD)"), and `KIRO_API_KEY` plus headless mode is the sanctioned route. — Source: https://kiro.dev/faq/, KD/cli/headless/ (accessed 2026-10-21)
- **[Verified-primary]** **Abuse detection:** Amazon Bedrock abuse detection applies to Kiro (§1.24.1, §50.14.2). Free Tier inputs may be kept up to 60 days to detect Agreement violations. AI Content from Free and individual subscribers may be used to improve the service unless an AWS Organizations opt-out is set (§50.3). A Kiro subscription bought via the Stripe portal is contracted with Amazon Web Services, Inc. (§50.14.1). — Source: https://aws.amazon.com/service-terms/ (accessed 2026-10-21)
- **[Verified-primary]** **(d) Reverse engineering and modification:** the AWS IP License forbids modifying, distributing, tampering with or making derivative works of AWS Content, and forbids sublicensing. — Source: https://aws.amazon.com/legal/aws-ip-license-terms/ (accessed 2026-10-21). **[Negative-evidence]** An explicit reverse-engineering clause was not found in that page by this search (S-12). **[Inferred]** Reading undocumented SQLite or session stores is not modification. Reading `auth_kv` or calling private service APIs with harvested tokens would be on the wrong side, as with Codex.

## 8. Pain points (community, brief)

- **Account switching:** sign-out/sign-in only, with no switcher (#6061, #6933 auto-login to the previous account, #9976). Some users were suspended after switching (#7339, #10043).
- **No programmatic usage:** #8477, #9330. Users build tray meters by scraping the SQLite auth.
- **KIRO_HOME and data-directory leaks:** #10417, #11348, #11960, #11585 (enterprise compliance).
- **Churn:** Q to Kiro rename; V2 to V3 engine with incompatible session stores; the IDE 0.x to 1.x session format change; three-plus CLI session storage generations. Releases come every few days on both surfaces.
- **Disk bloat:** IDE `kiro.kiroagent` sessions grow to many GB (#5469).

Sources: https://github.com/kirodotdev/Kiro/issues (listed numbers; accessed 2026-10-21). The repo `kirodotdev/Kiro` is issues-only: no licence, 4.4k stars.

## Synthesis / Verdict for AI Juggler

All verdicts are **[Inferred]**.

### CLI (`kiro-cli`): **Full adapter — feasible, size M (L if V2 must be supported too)**

- **Isolation:** spawn every `kiro-cli` child with its own absolute `KIRO_HOME`. The vendor documents this for "separate Kiro profiles, such as work and personal", and in V3 it covers sign-ins, sessions and state. Pin `--v3` (or `chat.agentEngine=v3` in the per-account `cli.json`). Also consider a per-account `KIRO_DATA_DIR` so `data.sqlite3` and `knowledge_bases/` don't join accounts, but #11960 shows it leaking on Windows.
- **Identity check:** run `kiro-cli whoami --format json` per home, which never touches token files.
- **Control plane:** use `kiro-cli acp --agent-engine=v3` (JSON-RPC, capability-negotiated, explicitly open to "custom integrations"). It gives `session/list|load|fork`, turn metering in `session_info_update`, and `_kiro/account/getUsage`. Headless `--no-interactive --output-format stream-json` is the simpler route. The interactive TUI can be launched in a PTY, with `--resume-id` for handover.
- **Usage:** a single monthly credit pool, which is simpler than Codex's windows. Per-turn credits are local (V3 turn metering); the balance is available via ACP `getUsage` if it proves out; otherwise it is an Estimate.
- **Blockers / risks:**
  1. On macOS it is unproven whether V3 sign-ins really leave `~/Library/Application Support/kiro-cli/data.sqlite3` when `KIRO_HOME` is set. If not, two accounts share or clobber one login. This needs a live test with sacrificial accounts (maintainer action).
  2. Open KIRO_HOME bugs (#10417, #11348).
  3. ToS: the "third-party automation harnesses … not permitted" clause needs a reading that a supervisor launching native `kiro-cli` is allowed. Consider asking AWS.
  4. Suspension reports after account switching.
  5. Closed source, so all path facts beyond the docs are secondary and drift every few days.
- **Open questions:**
  - The `_kiro/account/getUsage` response shape and whether it includes the reset date and plan.
  - Whether V3 `~/.kiro/sessions/<hash>/sess_*/` may be read read-only, given the docs say "don't depend on the layout". Prefer ACP `session/list`.
  - Whether an ACP client may keep the user's CLI login (the default) while running several homes.
  - How `KIRO_API_KEY` precedence works when a home also has a browser login.

### Desktop (Kiro IDE): **Track-only — size S (detection and windows); session indexing M and fragile**

- **Feasible:** detect `/Applications/Kiro.app` (`dev.kiro.desktop`, version from Info.plist), focus or raise its windows, launch with a folder via `kiro <path>`, and optionally index sessions read-only from `~/.kiro/sessions/<hash>/sess_*` (shared with the V3 harness format) or from the `kiro.kiroagent` globalStorage (undocumented, several formats).
- **Not recommended yet:** multi-account IDE launches via `--user-data-dir`, `--extensions-dir` and `KIRO_HOME`. None of these is documented for Kiro, IDE token storage is undocumented, and there is no vendor account switcher.
- **Blockers:** undocumented storage, and the IDE 1.x storage migration.
- **Open questions:**
  - Does the IDE honour `KIRO_HOME`?
  - Where do 1.x builds keep the sign-in, and is it shared with the CLI?
  - Does `--user-data-dir` give a separate sign-in?

### Both surfaces
- Configuration (`~/.kiro`, steering, MCP, agents, AGENTS.md) is shared between the IDE and the CLI. Per-account `KIRO_HOME` therefore also splits the user's steering, agents and MCP config. The adapter should seed or symlink shared config into each home, or document the split. This is the same trade-off as `CODEX_HOME`.

## Search log

Every search ran on 2026-10-21. "Pages checked" means opened with WebFetch, `gh` or curl. A search snippet alone is never counted as a source.

- **S-1 (IDE relocation: KIRO_HOME, --user-data-dir, --extensions-dir)**
  - Queries: `Kiro IDE multiple accounts "--user-data-dir" two accounts at once`; `kiro-cli switch account multiple accounts login profiles KIRO_HOME work personal`; `gh search issues --repo kirodotdev/Kiro "user-data-dir"`, `"KIRO_HOME"`.
  - Pages checked: KD/ide/setup/, KD/ide/troubleshooting/, KD/upgrade-guides/migrating-from-vscode/, KD/configuration/, KD/reference/settings/, https://kiro.dev/changelog/ide/.
  - Result: none documents IDE support for KIRO_HOME or `--user-data-dir`.
- **S-2 (IDE token storage)**
  - Queries: `"kiro-auth-token.json" ~/.aws/sso/cache Kiro IDE`.
  - Pages checked: KD/getting-started/authentication/, KD/ide/troubleshooting/, KD/ide/setup/, https://pkg.go.dev/github.com/cplieger/vibekit/internal/kiroauth.
  - Result: no vendor statement; only third-party sources.
- **S-3 (IDE session storage location, IDE headless or resume flags)**
  - Queries: `Kiro IDE "globalStorage" kiro.kiroagent sessions chat history location`.
  - Pages checked: KD/ide/chat/, KD/ide/chat/chat-export/, KD/ide/whats-new-v1/, KD/ide/troubleshooting/, the CodeBurn provider doc.
  - Result: the export format is documented, but the on-disk location is not.
- **S-4 (CLI token storage: path and Keychain names)**
  - Queries: `kiro-cli "data.sqlite3" Application Support conversations`.
  - Pages checked: KD/getting-started/authentication/, KD/reference/settings/, KD/reference/cli-commands/, KD/configuration/, KD/cli/setup/; predecessor source QSRC/* (read for comparison).
  - Result: the vendor docs give no Kiro CLI token path or Keychain item names.
- **S-5 (KIRO_DATA_DIR documentation)**
  - Queries: `kiro.dev KIRO_DATA_DIR "data.sqlite3"`; `"KIRO_DATA_DIR" kiro-cli environment variable`.
  - Pages checked: KD/reference/settings/, KD/configuration/, KD/cli/setup/, KD/cli/chat/configuration/ (moved to /docs/configuration/), KD/cli/experimental/knowledge-management/, KD/crew/system/, issue #11960.
  - Result: the variable appears only in the issue, which quotes docs this search did not locate.
- **S-6 (programmatic usage: CLI command, REST API, SDK)**
  - Queries: `gh search issues --repo kirodotdev/Kiro "usage api"`, `"credits usage cli"`.
  - Pages checked: KD/billing/, KD/billing/subscription-portal/, KD/billing/proactive-usage-notifications/, KD/reference/cli-commands/, KD/reference/slash-commands/, https://kiro.dev/pricing/, https://kiro.dev/faq/, issues #8477 and #9330.
  - Result: no `kiro-cli usage` command and no public usage API. ACP `_kiro/account/getUsage` was found (KD/cli/v3/acp-migration/).
- **S-7 (multiple accounts in terms)**
  - Queries: `Kiro terms of service AWS Customer Agreement Kiro service terms multiple accounts`.
  - Pages checked: https://kiro.dev/pricing/, https://kiro.dev/faq/, https://kiro.dev/license/, https://aws.amazon.com/service-terms/ (characters 0–200,000, covering §1 and §50), https://aws.amazon.com/legal/aws-ip-license-terms/.
  - Result: no explicit rule either way on one person holding several accounts.
- **S-8 (bundle identifiers in vendor docs)**
  - Pages checked: KD/getting-started/installation/, KD/ide/troubleshooting/, https://kiro.dev/downloads/; the Homebrew casks `kiro` and `kiro-cli` (JSON API).
  - Result: no vendor-stated `CFBundleIdentifier`; the ids are inferred from the cask zap plist names.
- **S-9 (quota-specific exit code)**
  - Pages checked: KD/reference/exit-codes/, KD/cli/headless/.
  - Result: only codes 0, 1, 3 and 4; nothing for credits or rate limits.
- **S-10 (IDE credit meter or status bar)**
  - Pages checked: KD/ide/chat/, KD/ide/whats-new-v1/, KD/billing/subscription-portal/, KD/billing/proactive-usage-notifications/, https://kiro.dev/changelog/ide/.
  - Result: only the browser subscription portal and an 80% notification.
- **S-11 (Homebrew formula or legacy cask)**
  - Checks: `https://formulae.brew.sh/api/formula/kiro-cli.json` returned HTTP 404; `https://formulae.brew.sh/api/cask/amazon-q.json` returned no JSON.
  - Result: CLI distribution via Homebrew is the `kiro-cli` cask only.
- **S-12 (reverse-engineering clause)**
  - Pages checked: https://aws.amazon.com/legal/aws-ip-license-terms/ and https://kiro.dev/license/.
  - Result: no explicit reverse-engineering clause found in either. The AWS Customer Agreement itself was not fetched.
