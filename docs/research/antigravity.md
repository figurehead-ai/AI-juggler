# Research notes: Google Antigravity

Research date: 2026-10-21

> Input for a possible AI Juggler Antigravity Provider. Covers the three
> Google-built local surfaces that share one agent harness: the
> **Antigravity App** (desktop, "Antigravity 2.0"), the **Antigravity IDE**
> (the original Nov-2025 VS Code-derived editor, now a separate app), and the
> **Antigravity CLI** (`agy`). Nothing was installed, signed in or downloaded;
> no local `~/.gemini`, Keychain or Application Support data was read. Sources
> are the vendor's site (all 130 `antigravity.google` docs/blog/product pages
> listed in its sitemap were fetched as public HTML on 2026-10-21), the vendor's
> GitHub issue tracker, Homebrew cask metadata, and the ACP registry.
>
> Legend (EVIDENCE-STANDARD.md): **[Verified-primary]** vendor docs, binary or
> vendor-owned repo say it · **[Verified-secondary]** reliable second-hand
> source (Homebrew cask, vendor-repo collaborator reply, community report) ·
> **[Negative-evidence]** not found by this search, see Search log ·
> **[Inferred]** our reasoning, never shown as fact.
> All sources accessed 2026-10-21. `AG` = `https://antigravity.google`.

**Headline facts**

- **[Verified-primary]** Gemini CLI stopped serving Google AI Pro/Ultra and free individual users on 2026-06-18; the successor is Antigravity CLI, built in Go on the shared harness and available since 2026-05-19. — Source: https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli (accessed 2026-10-21)
- **[Verified-primary]** The geminicli.com quota page banner reads "Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." — Source: https://geminicli.com/docs/resources/quota-and-pricing (accessed 2026-10-21)
- **[Verified-primary]** On 2026-05-19 "Antigravity 2.0" shipped as a new standalone agent-first desktop app with no IDE; the existing IDE became a separate app ("Antigravity IDE"). — Source: AG/blog/introducing-google-antigravity-2/ (accessed 2026-10-21)
- **[Verified-primary]** The FAQ says that wrapping `agy` inside third-party apps, or automating account rotation, "is not supported under consumer plans". The additional terms say that using third-party software to access the Service breaches the agreement. — Source: AG/docs/faq/, AG/terms/ (accessed 2026-10-21)

## 1. Distribution & versioning

### Desktop: Antigravity App (2.x, "Antigravity 2.0", internally "Hub")

- **[Verified-primary]** Antigravity is made by Google and runs on macOS 12+ (Apple Silicon and Intel), Windows 10+, Linux (glibc ≥ 2.28) and Googlebook. You install it by dragging `Antigravity.dmg` into Applications. — Source: AG/docs/getting-started/ (accessed 2026-10-21)
- **[Verified-primary]** The current version is **v2.24.0**, released 2026-10-15. The download URL pattern is `storage.googleapis.com/antigravity-public/antigravity-hub/<ver>-<build>/darwin-{arm,x64}/Antigravity.dmg`. — Source: AG/download/, AG/docs/changelog/ (accessed 2026-10-21)
- **[Verified-secondary]** The Homebrew cask is `antigravity` ("Agent orchestration platform"). It installs `/Applications/Antigravity.app`, and its uninstall stanza quits the bundle id **`com.google.antigravity`**. The cask has no `binary` stanza, so it ships no shell launcher. It uses `auto_updates true`, requires macOS ≥ 13, and uses an electron-builder livecheck with Squirrel `ShipIt` caches. — Source: https://formulae.brew.sh/api/cask/antigravity.json, https://github.com/Homebrew/homebrew-cask/blob/HEAD/Casks/a/antigravity.rb (accessed 2026-10-21)
- **[Verified-secondary]** The `antigravity` cask token moved from the IDE to the App. Its history runs 1.11.3 (2025-11-18) to 1.23.2 (2026-04-16), then 2.0.0 (2026-05-20). So before 2.0, `/Applications/Antigravity.app` and `com.google.antigravity` belonged to the VS Code-derived IDE. — Source: `gh api repos/Homebrew/homebrew-cask/commits?path=Casks/a/antigravity.rb` (accessed 2026-10-21)
- **[Inferred]** Detection: check that `/Applications/Antigravity.app` exists and that `CFBundleIdentifier == com.google.antigravity`. Read `CFBundleShortVersionString` from `Contents/Info.plist` (≥ 2 means the App; 1.x means a legacy IDE install). No CLI version command exists for the App.
- **[Verified-primary]** The App auto-updates by default. Older builds are listed at AG/releases/, and pinning needs `Update: Mode` set to manual or none. — Source: AG/releases/ (accessed 2026-10-21)

### Desktop: Antigravity IDE (standalone, VS Code-derived)

- **[Verified-primary]** Antigravity launched as an IDE on 2025-11-18. When users update to 2.0 they are asked whether to keep the IDE. Google plans to remove the Agent Manager from the IDE. — Source: AG/blog/introducing-google-antigravity/, AG/blog/introducing-google-antigravity-2/ (accessed 2026-10-21)
- **[Verified-secondary]** Thoughtworks Radar (2026-04-15) calls it "a standalone VS Code fork". **[Negative-evidence]** Google's own pages state no VS Code lineage, not found by this search (S10). — Source: https://www.thoughtworks.com/radar/tools/google-antigravity (accessed 2026-10-21)
- **[Verified-primary]** The current version is **v2.5.5** (`Antigravity IDE.dmg` on `edgedl.me.gvt1.com/.../antigravity/stable/2.5.5-<build>/`). Gemini 4 Argon is not available in the IDE, and the standalone IDE is not supported for enterprise deployments. — Source: AG/download/, AG/docs/getting-started/, AG/docs/enterprise/ (accessed 2026-10-21)
- **[Verified-secondary]** The cask `antigravity-ide` installs `/Applications/Antigravity IDE.app` with bundle id **`com.google.antigravity-ide`**. It links the binary `Contents/Resources/app/bin/antigravity-ide` as both `antigravity-ide` and `agy-ide`. The layout is VS Code-style and there is also a `~/.antigravity-ide-server` dir. — Source: https://formulae.brew.sh/api/cask/antigravity-ide.json (accessed 2026-10-21)
- **[Inferred]** Detection: check the bundle id `com.google.antigravity-ide`. The version comes from Info.plist, or presumably from `antigravity-ide --version` (VS Code convention; not documented).

### CLI: Antigravity CLI (`agy`)

- **[Verified-primary]** Install with `curl -fsSL https://antigravity.google/cli/install.sh | bash` (macOS/Linux), `irm …/install.ps1 | iex`, or `install.cmd`. The binary is **`~/.local/bin/agy`**, and `agy --version` verifies it. Installer flags are `--skip-aliases` (don't purge legacy `agy`/`antigravity` shell aliases) and `--skip-path`. — Source: AG/docs/getting-started/, AG/docs/install/ (accessed 2026-10-21)
- **[Verified-secondary]** The Homebrew cask `antigravity-cli` (new 2026-05-19, now **1.4.4**) ships a tarball `cli_mac_{arm64,x64}.tar.gz` whose binary is named **`antigravity`**, linked as `agy`. It also has Linux builds and zaps `~/.gemini/antigravity-cli`. The winget id is `Google.AntigravityCLI` (secondary). — Source: https://formulae.brew.sh/api/cask/antigravity-cli.json (accessed 2026-10-21); https://docs.flutter.dev/ai/antigravity-cli (search result, not fetched)
- **[Verified-primary]** Versions: the docs changelog lists CLI v1.4.0 (2026-10-16) as latest. The vendor repo CHANGELOG already has **1.4.4** (committed 2026-10-21). Releases ship every 1–3 days. — Source: AG/docs/changelog/, https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md (accessed 2026-10-21)
- **[Verified-primary]** The CLI has a statically linked background self-updater that keeps state in `~/.gemini/antigravity-cli/updater/` (`update.lock`, `last_check.timestamp`). `AGY_CLI_DISABLE_AUTO_UPDATE=true` disables it. — Source: AG/docs/cli/troubleshooting/ (accessed 2026-10-21)
- **[Verified-secondary]** A vendor-repo collaborator says a read-only (`chmod 555`) binary directory also makes the updater skip, which is supported. — Source: https://github.com/google-antigravity/antigravity-cli/issues/1080 (accessed 2026-10-21)
- **[Verified-primary]** The source is closed. The GitHub repo `google-antigravity/antigravity-cli` holds only README, CHANGELOG, a demo gif and `examples/` (statusline, title), with no license. Issue #4 "open source antigravity-cli" is still open. — Source: `gh api repos/google-antigravity/antigravity-cli/contents` (accessed 2026-10-21)
- **[Verified-primary]** Relationship to Gemini CLI: Antigravity CLI is the successor, not a fork. It "took inspiration" from Gemini CLI, and `agy plugin import gemini` plus a first-launch checklist migrate extensions, settings and tokens into the keyring. — Source: AG/blog/introducing-google-antigravity-cli/, AG/docs/cli/gcli-migration/ (accessed 2026-10-21)
- **[Inferred]** Detection: run `command -v agy` and `agy --version`, and check `~/.local/bin/agy` or `$(brew --prefix)/bin/agy`. Beware a legacy shell alias `agy`/`antigravity` from 1.x IDE installs, which the installer purges unless `--skip-aliases` is set.

## 2. Configuration system

### Shared (all surfaces)

- **[Verified-primary]** The App and CLI share common settings, such as permissions set once and applied to both. — Source: AG/blog/introducing-google-antigravity-cli/ (accessed 2026-10-21)
- **[Verified-primary]** Shared customization root `~/.gemini/config/`: `config.json` (`userSettings`, e.g. `enableAdc`), `mcp_config.json`, `hooks.json`, `rules/`, `skills/`, `agents/`, `plugins/`, `workflows/`, `sidecars/`, `projects/`. Global rules can also live in `~/.gemini/AGENTS.md` or `~/.gemini/GEMINI.md`. — Source: AG/docs/install/, AG/docs/mcp/, AG/docs/hooks/, AG/docs/rules/, AG/docs/skills/ (accessed 2026-10-21)
- **[Verified-primary]** The workspace layer is `.agents/` (`mcp_config.json`, `hooks.json`, `rules/`, `rules.json`, `skills/`, `agents/`, `plugins/`, `workflows/`). The legacy `.agent/rules/` is still read. — Source: AG/docs/rules/, AG/docs/cli/gcli-migration/ (accessed 2026-10-21)
- **[Verified-primary]** Per-surface app-data dirs (`<app_data_dir>`): **App** `~/.gemini/antigravity`, **CLI** `~/.gemini/antigravity-cli`, **IDE** `~/.gemini/antigravity-ide`. — Source: AG/docs/hooks/ (accessed 2026-10-21)
- **[Verified-secondary]** Other dirs the Homebrew zap stanzas list:
  - App: `~/.antigravity`, `~/Library/Application Support/Antigravity`, `~/Library/Logs/Antigravity`, `~/Library/Preferences/com.google.antigravity.plist`, `~/Library/Caches/com.google.antigravity*`.
  - IDE: `~/.antigravity-ide`, `~/.antigravity-ide-server`, `~/Library/Application Support/Antigravity IDE`.

  — Source: Homebrew cask JSON (accessed 2026-10-21)
- **[Verified-primary]** The IDE extensions (VS Code, Visual Studio) auto-download and manage a background Antigravity CLI/server in `~/.gemini/bin`. — Source: AG/docs/changelog/ (IDE Extensions section, VS v1.0.260818.0) (accessed 2026-10-21)

### Desktop (App / IDE)

- **[Verified-primary]** The App is configured in its Settings UI (Cmd+,), and ADC is turned on with `AGY_ADC_AUTH=true` at launch or `userSettings.enableAdc` in `~/.gemini/config/config.json`. — Source: AG/docs/install/, AG/docs/settings/ (accessed 2026-10-21)
- **[Negative-evidence]** A documented env var or flag that relocates the **App's** data dirs (`~/.gemini/antigravity`, `~/Library/Application Support/Antigravity`) was not found by this search (S2, S9).
- **[Verified-secondary]** For the IDE, a forum reply (affiliation not shown) says `--user-data-dir` gives "full isolation" with its own root folder and `--profile` manages profiles within one data dir. The reply did not address sign-in isolation. — Source: https://discuss.ai.google.dev/t/how-do-user-data-dir-and-profile-work/137054 (accessed 2026-10-21)
- **[Inferred]** `--user-data-dir` (VS Code semantics) would move only `~/Library/Application Support/Antigravity IDE`, not `~/.gemini/antigravity-ide` or the shared keyring entry (§3). So it does not create an isolated account.

### CLI

- **[Verified-primary]** Config lives in `~/.gemini/antigravity-cli/settings.json`; `/config` (alias `/settings`) edits it. Documented keys: `colorScheme`, `altScreenMode`, `toolPermission`, `artifactReviewPolicy`, `notifications`, `editor(Mode)`, `allowNonWorkspaceAccess`, `enableTerminalSandbox`, `useG1Credits`, `enableTelemetry`, `verbosity`, `permissions.allow`, `statusLine`, `title`, `modelProvider`. CLI flags such as `--sandbox` override settings for the session. — Source: AG/docs/cli/reference/, AG/docs/cli/using/, AG/docs/cli/headless/ (accessed 2026-10-21)
- **[Verified-primary]** Other CLI files:
  - `keybindings.json`, `cli.log`, `cache/last_conversations.json` (workspace path → last conversation id), `cache/projects.json`, `plugins/`, `skills/` and `statusline.sh`/`title.sh`, all under `~/.gemini/antigravity-cli/`.
  - macOS Remote Control daemon log: `~/Library/Logs/antigravity-cli-daemon.log`.

  — Source: AG/docs/cli/using/, AG/docs/cli/commands/resume/, AG/docs/enterprise/, AG/docs/remote-control/, AG/docs/changelog/ (accessed 2026-10-21)
- **[Verified-primary]** Documented CLI env vars:
  - Auth: `AGY_ADC_AUTH`, `GEMINI_API_KEY`, `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_CLOUD_QUOTA_PROJECT`, `GOOGLE_CLOUD_LOCATION`.
  - Updater: `AGY_CLI_DISABLE_AUTO_UPDATE`.
  - Display and terminal: `AGY_CLI_HIDE_ACCOUNT_INFO`, `AGY_CLI_HIDE_LOGO`, `AGY_CLI_OSC7501`, `AGY_CLI_DISABLE_LATEX`, `AGY_CLI_CMD_OUTPUT_PERCENTAGE`.

  None of these relocates the home. — Source: AG/docs/install/, AG/docs/cli/troubleshooting/, AG/docs/changelog/ (accessed 2026-10-21)
- **[Negative-evidence]** A documented equivalent of `CODEX_HOME` / `GEMINI_CLI_HOME` / `CLAUDE_CONFIG_DIR` for `agy` was not found by this search (S1). Issue #155 "Add equivalent to GEMINI_CLI_HOME env variable" is open (2026-05-24). A 2026-10-19 commenter reports `agy --help` shows only `--log-file` for paths. — Source: https://github.com/google-antigravity/antigravity-cli/issues/155 (accessed 2026-10-21)
- **[Verified-secondary]** Workaround from vendor-repo collaborators: run each worker with an isolated `HOME=/path` on macOS/Linux, or `USERPROFILE` on Windows (where `HOME` is ignored), from a clean cwd. Run `HOME=… agy login` once for that home. This isolates `settings.json`, `config.json`, hooks, skills and MCP config. — Source: https://github.com/google-antigravity/antigravity-cli/issues/1080 (2026-09-23), https://github.com/google-antigravity/antigravity-cli/issues/1261 (2026-10-16) (accessed 2026-10-21)
- **[Verified-secondary]** The same Windows reply says credentials are **not** namespaced by profile: there is one `gemini:antigravity` Credential Manager entry per OS user, a worker reuses the existing sign-in, and logging out or signing in from the worker affects normal Desktop/CLI sessions. — Source: https://github.com/google-antigravity/antigravity-cli/issues/1261 (accessed 2026-10-21)

## 3. Auth & multiple accounts

### Shared

- **[Verified-primary]** There is one "unified authentication layer" across App, CLI, IDE extensions and SDK. Signing in on one local surface shares your credentials across the local environment. — Source: AG/docs/install/ (Authenticate page) (accessed 2026-10-21)
- **[Verified-primary]** Sign-in methods:
  - Personal Google Account via browser OAuth (Free, Pro, Ultra).
  - Business account via Google Cloud SSO / Workforce Identity Federation, with a license selector for Gemini Enterprise.
  - ADC (`gcloud auth application-default login` plus `AGY_ADC_AUTH=true`).
  - Gemini API key (CLI only: `modelProvider: "gemini"` plus `GEMINI_API_KEY`).

  The IDE `auth.type` values are `oauth-personal`, `oauth-business`, `gemini-api-key` and `agent-platform`. — Source: AG/docs/install/, AG/docs/enterprise/ (accessed 2026-10-21)
- **[Verified-primary]** Eligibility: personal accounts in approved geographies only, age 18 or over. Workspace accounts may have issues, and the FAQ suggests trying an @gmail.com address. — Source: AG/docs/faq/ (accessed 2026-10-21)
- **[Verified-primary]** Switching Google Cloud project or location requires logging out and back in. — Source: AG/docs/enterprise/ (accessed 2026-10-21)

### Desktop

- **[Verified-primary]** The App signs in through a browser OAuth flow that redirects back to the App, with log out under Settings > Account. — Source: AG/docs/install/, AG/docs/settings/ (accessed 2026-10-21)
- **[Negative-evidence]** A documented Keychain item name or token file for the App/IDE was not found by this search (S3).
- **[Verified-secondary]** There is no in-app account switcher: users sign out and back in, and a feature request is open (2026-07-06). Staff say conversations survive an account switch because history is stored locally. — Source: https://discuss.ai.google.dev/t/switch-account-feature-request/173874, https://discuss.ai.google.dev/t/project-and-conversation-history-retained-when-switching-accounts/172985 (accessed 2026-10-21)
- **[Verified-primary]** MCP OAuth tokens are kept in plaintext at `~/.gemini/antigravity/mcp_oauth_tokens.json`. — Source: AG/docs/mcp/ (accessed 2026-10-21)

### CLI

- **[Verified-primary]** On launch, `agy` silently reads a saved "token profile" from the OS keyring (Apple Keychain on macOS). With none found it opens the browser, and over SSH it prints a URL and asks for a pasted code. `/logout` purges keyring tokens, and `/login` exists. Headless `-p` needs a prior interactive sign-in and exits with an auth error in CI. — Source: AG/docs/install/, AG/docs/cli/headless/, AG/docs/enterprise/ (accessed 2026-10-21)
- **[Verified-primary]** The troubleshooting doc refers to "the Antigravity CLI security item" in Keychain Access, but gives no service or account name. — Source: AG/docs/cli/troubleshooting/ (accessed 2026-10-21)
- **[Verified-secondary]** A user observed the macOS Keychain generic password at **service `gemini`, account `antigravity`**. The CLI log reports "Failed to save token to keyring, falling back to file" (location undocumented). — Source: https://github.com/google-antigravity/antigravity-cli/issues/803 (comment 2026-09-22) (accessed 2026-10-21)
- **[Verified-primary]** The changelog confirms a file-storage fallback for tokens ("token removal directly to file storage when keyring storage is bypassed", v1.1.26). — Source: AG/docs/changelog/ (accessed 2026-10-21)
- **[Verified-primary]** The FAQ says automating account rotation "is not supported under consumer plans". — Source: AG/docs/faq/ (accessed 2026-10-21)
- **[Verified-secondary]** Account switching = `/logout` then sign in as account B. A regression where `/usage` kept showing account A's quota was fixed by 2026-09-25. — Source: https://github.com/google-antigravity/antigravity-cli/issues/1085 (accessed 2026-10-21)
- **[Verified-secondary]** A request for a supported auth-profile selector (`--profile`, `AGY_PROFILE`, `--data-dir`) for wrappers and multi-account workflows is open (2026-06-13) with no maintainer answer. A request for `agy whoami` / JSON status is open too. — Source: https://github.com/google-antigravity/antigravity-cli/issues/381, https://github.com/google-antigravity/antigravity-cli/issues/1069 (accessed 2026-10-21)
- **[Inferred]** On macOS, `HOME` isolation likely does **not** isolate the subscription token: the item is one fixed service/account pair, and the Windows maintainer reply confirms the equivalent there. Two isolated homes would then share or overwrite one sign-in. This is unproven for macOS until a sacrificial-account capture.
- **[Verified-primary]** API-key mode never sets up an account session, so `/logout` has no effect. Only `GEMINI_API_KEY` is read (no `GOOGLE_API_KEY`, no `.env`). — Source: AG/docs/install/ (accessed 2026-10-21)
- **[Inferred]** API-key mode is therefore per-process isolatable: one key per child env.

## 4. Sessions

### Shared

- **[Verified-primary]** Each conversation has a UUID. Its artifacts and transcript live in `<app_data_dir>/brain/<conversationId>/`, with the transcript at `.system_generated/logs/transcript.jsonl` (App `~/.gemini/antigravity`, CLI `~/.gemini/antigravity-cli`, IDE `~/.gemini/antigravity-ide`). Hooks receive `transcriptPath` and `artifactDirectoryPath`. — Source: AG/docs/hooks/ (accessed 2026-10-21)
  - **[Verified-primary]** The statusline doc's example shows a CLI `transcript_path` under `~/.gemini/antigravity/brain/…`, which contradicts the hooks page. — Source: AG/docs/cli/statusline/ (accessed 2026-10-21)
  - **[Inferred]** Read the path from the payload, never hard-code it.
- **[Negative-evidence]** A documented schema for `transcript.jsonl` lines was not found by this search; only the path is documented (S6).
- **[Verified-primary]** By default App and CLI conversations are not shared. The App can pull CLI conversations via its `@conversation` dropdown, and the CLI `/resume` picker has an "Antigravity" tab that imports App threads by cloning history, context and tool trajectories. — Source: AG/blog/introducing-google-antigravity-cli/, AG/docs/cli/commands/resume/ (accessed 2026-10-21)

### Desktop

- **[Verified-secondary]** In the 1.x IDE, users report conversations stored as binary protobuf `.pb` files in `~/.gemini/antigravity/conversations/`, with an index in VS Code-style `state.vscdb`. — Source: https://discuss.ai.google.dev/t/bug-report-agent-chat-history-not-displaying-despite-pb-files-being-present/135603 (search result; not fetched)
- **[Verified-primary]** For 2.x, the CLI changelog mentions "importing SQLite conversation from Antigravity 2.0" (v1.0.4). — Source: AG/docs/changelog/ (accessed 2026-10-21)
- **[Inferred]** The App's conversation store is SQLite (undocumented schema).
- **[Verified-secondary]** One user reports 37 GB under `~/.gemini/antigravity` (the "Hub") and 15 GB under `~/.gemini/antigravity-cli` (12 GB `brain/`, 2.7 GB `conversations/`). — Source: https://github.com/google-antigravity/antigravity-cli/issues/155 (comment 2026-10-19) (accessed 2026-10-21)
- **[Verified-secondary]** Conversation history is isolated between the App and the IDE (open bug). — Source: https://github.com/google-antigravity/antigravity-cli/issues/1040 (accessed 2026-10-21)
- **[Negative-evidence]** A documented headless or non-interactive mode for the App or IDE was not found by this search. The App has Scheduled Tasks/automations and Remote Control instead (S11).

### CLI

- **[Verified-primary]** Conversations are scoped to the launch cwd. Resume options: `agy -c` / `--continue` (most recent for this workspace, via `~/.gemini/antigravity-cli/cache/last_conversations.json`, checked against the backend); `agy --conversation <id>`; and `/resume` (aliases `/switch`, `/conversation`) with search, F2 rename and Ctrl+Delete delete. `/fork` (alias `/branch`) clones a thread, `/rewind` undoes, and on exit the CLI prints the resume command. — Source: AG/docs/cli/conversations/, AG/docs/cli/commands/resume/, AG/docs/cli/using/ (accessed 2026-10-21)
- **[Verified-primary]** Since v1.0.4 the CLI stores conversations as SQLite `.db` files (with `.db-wal`). The changelog notes file-lock fixes for concurrent CLI processes on the same database. — Source: AG/docs/changelog/ (v1.0.4, v1.1.x, v1.2.x) (accessed 2026-10-21)
- **[Verified-secondary]** The CLI keeps them in `~/.gemini/antigravity-cli/conversations/`. — Source: issue #155 comment above; https://cdn.jsdelivr.net/gh/EstebanForge/pi-ask-antigravity@main/README.md (search result; not fetched)
- **[Verified-primary]** Headless mode:
  - `agy -p/--print/--prompt "…"` with `--output-format text|json|stream-json`. The JSON envelope carries `conversation_id`, `status` (`SUCCESS|ERROR|CANCELED|INTERRUPTED|INVALID|WAITING|RUNNING`), `response`, `duration_seconds`, `num_turns` and `usage{input_tokens, output_tokens, thinking_tokens, cache_read_tokens, total_tokens}`.
  - Stream events are `init`, `step_update` (`step_type` user_input/agent_response/tool/checkpoint, `tool_info`, `subagent_info`, per-step `usage`) and `result`.
  - Other flags: `--json-schema`, `--input-format stream-json` (multi-turn over stdin), `--model`, `--effort`, `--agent`, `--continue`, `--conversation`, `--dangerously-skip-permissions`, `--sandbox`, `--print-timeout`.
  - Exit codes: 0 on success, 1 or 2 on input errors.

  — Source: AG/docs/cli/headless/ (accessed 2026-10-21)
- **[Verified-primary]** Other launch flags and subcommands: `--add-dir`, `--project`, `--new-project`, `--mode` (plan/auto), `--remote-control`, `agy models`, `agy agents`, `agy plugin …`, `agy remote-control start|status|stop`, `agy mic-serve`, `agy changelog`. — Source: AG/docs/cli/*, AG/docs/remote-control/, AG/docs/changelog/ (accessed 2026-10-21)
- **[Verified-primary]** Since v1.4.0, terminal status reporting over OSC 7501 tells a supporting terminal whether the CLI is working, waiting for input, done or failed. `AGY_CLI_OSC7501=on|off` controls it. The `title` hook gives the same state as JSON to a script. — Source: AG/docs/changelog/, AG/docs/cli/title/ (accessed 2026-10-21)
- **[Inferred]** OSC 7501 is a ready-made "stuck / waiting" signal for a terminal supervisor.

## 5. Usage & quota

### Plans (both surfaces; same account pool)

- **[Verified-primary]** Quota by plan:
  - Free: "meaningful quota, refreshed weekly" plus a weekly limit.
  - Google AI Pro: quota refreshed every five hours until the weekly limit is reached.
  - Google AI Ultra: highest quota, refreshed every five hours, highest weekly limits, plus third-party models.

  Limits track "amount of work done by the agent" and can change. BYOK for extra limits is not supported. — Source: AG/docs/plans/ (accessed 2026-10-21)
- **[Verified-primary]** Since 2026-05-19 there is one shared Gemini pool drawn down at API-price ratios, with non-Gemini models on a separate fixed limit. Ultra $100 = 5× Pro and Ultra $200 = 20× Pro in tokens (Pro $20). AI credits are overage-only. — Source: AG/blog/changes-to-antigravity-plans/ (accessed 2026-10-21)
- **[Verified-primary]** Current US prices: AI Pro $19.99, AI Ultra 5x $99.99, AI Ultra 20x $199.99 per month, with Antigravity entry / higher / highest rate limits respectively. — Source: https://gemini.google/subscriptions/ (accessed 2026-10-21)
- **[Verified-primary]** Business: Gemini Enterprise Standard/Plus seats from $30/user/month, or pay-as-you-go via API. Pooled seat credits are $10 (Standard) or $15 (Plus) per month, and third-party models are billed by tokens. Enterprise accounts have no per-model quota panel. — Source: AG/pricing/, AG/docs/enterprise/, AG/docs/changelog/ (App v2.13.0) (accessed 2026-10-21)
- **[Verified-primary]** Overages: Pro/Ultra can spend purchased AI credits when baseline quota runs out. App setting "AI Credit Overages" = Never|Always; CLI `useG1Credits`. — Source: AG/docs/plans/, AG/docs/cli/credits/ (accessed 2026-10-21)

### Desktop

- **[Verified-primary]** Baseline quota per model is shown on the Settings page (Models tab, redesigned in v2.1.4 as used vs remaining), and the model selector shows reset times. — Source: AG/docs/plans/, AG/docs/changelog/ (accessed 2026-10-21)
- **[Negative-evidence]** A documented machine-readable quota source for the App was not found by this search (S5).
- **[Verified-secondary]** The community tool `skainguyen1412/antigravity-usage` reads quota through the local IDE connection or by authenticating accounts itself. Both routes are undocumented, and the second involves handling tokens, so it is off-limits under D-06 and the ToS. — Source: https://prompts.brightcoding.dev/blog/skainguyen1412antigravity-usage-track-antigravity-ai-quotas-across-accounts (search result; not fetched)

### CLI

- **[Verified-primary]** `/usage` (alias `/quota`) refreshes quota from disk and backend and shows per-model limits and remaining amounts. `/credits` shows the AI credit balance and current-cycle use. The statusline shows remaining credits. — Source: AG/docs/cli/commands/usage/, AG/docs/cli/commands/credits/, AG/docs/cli/credits/ (accessed 2026-10-21)
- **[Verified-primary]** **Machine-readable quota without spending quota:** since v1.1.11, `agy -p "/usage"` (also `/quota`, `/credits`, `/model`) prints tab-separated records, or a structured payload under `--output-format json|stream-json`. It "without starting an agent turn, spending quota, or leaving a conversation behind". Piped/JSON output uses UTC timestamps (v1.2.15). — Source: AG/docs/changelog/ (accessed 2026-10-21)
- **[Verified-secondary]** The JSON shape is `command.data.groups[].buckets[]` with `window`, `remaining_fraction` and `reset_time`, from backend `v1internal:retrieveUserQuotaSummary`. Groups are e.g. "Gemini Models" and "Claude and GPT models", with weekly windows. The shape itself is undocumented. — Source: https://github.com/google-antigravity/antigravity-cli/issues/1045 (accessed 2026-10-21)
- **[Verified-primary]** **Status-line feed:** `statusLine: {type: "command", command: …}` in settings.json pipes a JSON payload to a script on every state change. The payload includes:
  - `quota{<bucket>: {remaining_fraction, reset_time, reset_in_seconds}}` (example bucket `gemini-weekly`)
  - `plan_tier` and `email`
  - `context_window{…}`
  - `agent_state` (idle/thinking/working/tool_use/initializing), `tool_confirmation_pending`
  - `conversation_id`, `transcript_path`, `version`

  — Source: AG/docs/cli/statusline/ (accessed 2026-10-21)
- **[Inferred]** The statusline feed is the closest analogue to Claude Code's statusline and is a documented, sanctioned local quota readout. It is per running session, and it requires writing the user's `settings.json`, which is a consent item.
- **[Verified-primary]** Local token usage per turn and session is available from `-p --output-format json/stream-json` (`usage`). These are model tokens, not plan quota. — Source: AG/docs/cli/headless/ (accessed 2026-10-21)
- **[Verified-primary]** On exhaustion, personal quota is reported immediately rather than retried (v1.3.3). With credits too low the CLI says "Your AI credits balance is too low to continue." (v1.2.15). — Source: AG/docs/changelog/ (accessed 2026-10-21)
- **[Negative-evidence]** A documented quota-specific exit code was not found by this search (S5).
- **[Verified-primary]** Gemini API-key mode is billed and limited by the Gemini API (RPD resets etc.), and the CLI stops immediately on daily or spend caps. — Source: AG/docs/changelog/ (accessed 2026-10-21)
- **[Negative-evidence]** A renewal or billing-date source was not found by this search. Same D-12 verdict as `gemini.md` §5: user-entered or unknown (S5).

## 6. Extensibility surface

- **[Verified-primary]** **MCP:** global `~/.gemini/config/mcp_config.json` and workspace `.agents/mcp_config.json`, using `{"mcpServers": {…}}` with `serverUrl` for remote servers (legacy `url`/`httpUrl` renamed). It supports DCR OAuth, `/mcp` manager in the CLI and an MCP Store in the IDE. — Source: AG/docs/mcp/, AG/docs/cli/gcli-migration/ (accessed 2026-10-21)
- **[Verified-primary]** **Rules:** `AGENTS.md` / `GEMINI.md` at each directory level, `.agents/rules/*.md` (YAML frontmatter, triggers such as `model_decision`), `.agents/rules.json`, and global `~/.gemini/{AGENTS,GEMINI}.md` or `~/.gemini/config/rules/*.md`. All rules share a 20k-token budget. — Source: AG/docs/rules/ (accessed 2026-10-21)
- **[Verified-primary]** **Skills / workflows:** skills live in `.agents/skills/` and `~/.gemini/config/skills/` (CLI also `~/.gemini/antigravity-cli/skills/`). IDE workflows "stop working on October 19, 2026" in favour of skills. — Source: AG/docs/skills/, AG/docs/ide/workflows/, AG/docs/cli/gcli-migration/ (accessed 2026-10-21)
- **[Verified-primary]** **Hooks:** `hooks.json` (workspace `.agents/`, global `~/.gemini/config/`, CLI settings.json, plugins). Events are `SessionStart`, `PreToolUse`, `PostToolUse`, `PreInvocation`, `PostInvocation` and `Stop`. JSON goes over stdin/stdout with `conversationId`, `transcriptPath` and `modelName`. — Source: AG/docs/hooks/ (accessed 2026-10-21)
- **[Verified-primary]** **Plugins / marketplace, subagents, custom agents, sidecars, Remote Control** are all documented. Remote Control works in a browser at `antigravity.google.com/r/<instance-id>`, signed in with the same Google Account, and there is a CLI daemon `agy remote-control start`. — Source: AG/docs/plugins/, AG/docs/subagents/, AG/docs/sidecars/, AG/docs/remote-control/ (accessed 2026-10-21)
- **[Verified-primary]** **SDK:** `pip install google-antigravity` (Python ≥ 3.10, Apache-2.0, v0.1.22). It authenticates with a Gemini API key, Vertex/ADC or local models, and has no documented consumer-subscription sign-in. — Source: AG/docs/sdk/overview/, AG/docs/install/, https://github.com/google-antigravity/antigravity-sdk-python (accessed 2026-10-21)
- **[Verified-secondary]** **ACP:** the ACP registry lists `antigravity-acp` v1.4.1 (Google LLC, proprietary), a separate binary `agy_acp_server.par` from `dl.google.com/agy-extensions/…`. The Zed / JetBrains "External Agents" integration uses it. Users report it does not send token usage or quota. — Source: https://github.com/agentclientprotocol/registry/blob/main/antigravity-acp/agent.json, https://github.com/google-antigravity/antigravity-cli/issues/1045 (accessed 2026-10-21)
- **[Negative-evidence]** ACP is not mentioned in the vendor docs, not found by this search (S8). Native ACP in `agy` is requested in issue #31 (open). — Source: https://github.com/google-antigravity/antigravity-cli/issues/31 (accessed 2026-10-21)
- **[Negative-evidence]** A JSON-RPC "app-server" control plane like Codex's was not found by this search. The closest are `--input-format stream-json` and Remote Control (S8).

## 7. Terms of service

Sources: **Google Antigravity Additional Terms of Service** (AG/terms/), the Google Terms of Service, and AG/docs/faq/ (all accessed 2026-10-21). Enterprise, Workspace-on-Cloud and Gemini Enterprise API-key users fall under their Cloud terms instead.

- **[Verified-primary] Third-party access:** the additional terms ban using the Service "in connection with products not provided by us". They give using OpenClaw with Antigravity OAuth as an example of a breach that "may be grounds for suspension or termination" of Antigravity and/or Gemini CLI accounts. — Source: AG/terms/ (accessed 2026-10-21)
- **[Verified-primary] Wrappers and account rotation:** the FAQ says third-party software (Claude Code, OpenClaw, OpenCode) with an Antigravity login violates the ToS and can get the account suspended. It recommends a Gemini Enterprise or AI Studio API key instead. It also says that wrapping `agy` in third-party apps, or automating account rotation, is not supported under consumer plans. Headless `agy -p` is meant for direct scripting and CI. — Source: AG/docs/faq/ (accessed 2026-10-21)
- **[Verified-secondary] Enforcement precedent:** in Feb 2026 Google suspended AI Ultra and Antigravity accounts that used OpenClaw via Antigravity OAuth (403 ToS errors). Appeals are ongoing, and a promised automated unban is unconfirmed. — Source: https://oecd.ai/en/incidents/2026-02-23-ce89 (search result; not fetched); vendor forum threads e.g. https://discuss.ai.google.dev/t/issue-with-antigravity-account-suspension-due-to-openclaw-oauth-usage/126426 (search result; not fetched); https://github.com/google-antigravity/antigravity-cli/issues/927 (accessed 2026-10-21)
- **[Verified-primary] Data:** "Interactions" are recorded and may be reviewed by Google staff, with an opt-out in settings. — Source: AG/terms/ (accessed 2026-10-21)
- **[Negative-evidence]** An explicit "one account per person" clause was not found by this search; the FAQ addresses only automated rotation (S4).
- **[Inferred]** Juggler launching the official `agy`/App binaries for the user, without touching tokens, is closer to a terminal than to OpenClaw. But "wrapping agy inside … applications" and "automating account rotation" describe Juggler's core multi-account feature almost literally. For consumer accounts this is a policy blocker, not a technical one.

## 8. Pain points (community, brief)

- Disk growth: tens of GB under `~/.gemini`, and no way to move it (#155).
- Keychain dialogs ("Keychain Not Found … 'antigravity'") during long sessions (#803). Oversized WIF tokens are rejected by Keychain (#1037).
- ToS blocks or eligibility checks that persist after appeal (#927, #1016).
- Quota confusion. "Antigravity Quota is not working as advertised" threads appear on discuss.ai.google.dev (search results).
- No wrapper-friendly auth profiles (#381) and no `whoami` (#1069).

## Synthesis / Verdict for AI Juggler [Inferred]

**Antigravity CLI (`agy`): Track-only for consumer accounts. A full adapter is possible only for API-key / Gemini Enterprise accounts. Size M (track-only), L (full).**

- What works: detection (`agy --version`), launch in the user's terminal, OSC 7501 / `title` state for "waiting / stuck", resume ids (`-c`, `--conversation`, the `last_conversations.json` map), and the session index from `brain/<id>/…/transcript.jsonl` (path documented, schema not).
- Quota: `agy -p /usage --output-format json` (documented, no quota spend; JSON shape community-observed) and the documented statusline `quota` object give a sanctioned *Measured-ish* remaining fraction plus reset time. That is better than Codex.
- Blockers:
  1. **No supported per-account isolation.** There is no `*_HOME` env var. `HOME` override is collaborator-suggested, and the subscription token appears to sit in one fixed keyring item (`gemini`/`antigravity`).
  2. **The FAQ explicitly calls wrapping `agy` and automating account rotation unsupported on consumer plans**, under a ToS with a suspension precedent.
  3. The binary is closed-source and auto-updates every few days, with a fast-moving changelog.
- API-key (`GEMINI_API_KEY` per child env) and Enterprise/ADC accounts are isolatable per process and sit under different terms. A full adapter is plausible there.

**Antigravity App (desktop 2.x): Track-only. Size S–M.**

- What works: detect `com.google.antigravity` at `/Applications/Antigravity.app` (version ≥ 2) and focus its windows. Read the conversation list only if a documented source appears: `brain/` transcripts are documented by path, while the SQLite store is undocumented.
- Blockers: no relocation flag or env var, no CLI launcher, no headless mode, one unified local sign-in, and no machine-readable quota.

**Antigravity IDE (desktop, VS Code-derived): Track-only. Size S.**

- What works: detect `com.google.antigravity-ide` / `agy-ide`. Note that pre-May-2026 installs used `Antigravity.app` / `com.google.antigravity` for the IDE.
- Limits: `--user-data-dir` is only secondary-sourced and doesn't isolate `~/.gemini/antigravity-ide` or the shared credential. Google plans to strip the IDE's Agent Manager.

**Open questions (need a sacrificial-account capture with maintainer approval):**

1. macOS: does `HOME=/x agy login` write a separate credential, or the same `gemini`/`antigravity` Keychain item? Does the CLI's file fallback land under `$HOME/.gemini`?
2. The exact JSON shape of `agy -p /usage --output-format json`, its stability, and the plan-tier field.
3. The `transcript.jsonl` line schema, and the App's SQLite conversation schema.
4. Whether Google would treat a local launcher or supervisor as "wrapping". A written answer from Google (forum or issue) before any consumer multi-account feature.
5. Does the App honor `--user-data-dir`, `AGY_ADC_AUTH` or any isolation when launched by `open -a`?
6. The App's Keychain item(s), and whether App and CLI read the same `gemini`/`antigravity` entry.

## Search log

All searches on 2026-10-21.

- **Base corpus:** all 130 URLs under `/docs`, `/blog`, `/product`, `/terms`, `/pricing`, `/download`, `/releases` and `/support` in https://antigravity.google/sitemap.xml were fetched (public HTML, Astro/Starlight server-rendered), and their `<main>` text was grepped locally. Homebrew cask API JSON and Ruby sources were read for `antigravity`, `antigravity-ide` and `antigravity-cli`. These tokens returned 404: `antigravity-app`, `google-antigravity`, `agy`, and formulae `antigravity`, `agy`, `antigravity-cli`. Issues in `google-antigravity/antigravity-cli` were searched with `gh search issues`.
- **S1: CLI home relocation env var.**
  - Grep of the corpus for `AGY_*`, `ANTIGRAVITY_*`, `GEMINI_*`, `GOOGLE_*`, `HOME`, `data dir`, `config dir`, `user-data-dir`, `profile`. Found only auth, display and updater vars. `ANTIGRAVITY_EXECUTABLE_DATA_DIR` is for sidecars and `ANTIGRAVITY_HARNESS_PATH` is for the SDK.
  - Issue searches "HOME", "config dir", "data directory", "profile". Found #155 (open), #669 (open), #381 (open), #1080 and #1261 (collaborator workarounds).
  - Result: no documented relocation variable.
- **S2: Desktop relocation / `--user-data-dir`.**
  - Corpus grep for `user-data-dir`, `--profile`, `Application Support`: no App/IDE relocation documented.
  - WebSearch "Antigravity multiple Google accounts switch account desktop app --user-data-dir". Forum threads 137054, 173874 and 172985 fetched.
- **S3: Keychain / token storage names.**
  - Corpus grep for `keychain`, `keyring`, `token`. Only the "Antigravity CLI security item" wording and `mcp_oauth_tokens.json` were found.
  - Issue search "keychain". #803 (service `gemini`, account `antigravity`, user-observed) and #1261 (Windows `gemini:antigravity`, collaborator).
- **S4: Multi-account support and one-account clauses.**
  - Corpus grep for `account`, `switch`, `multiple accounts`. The FAQ entry on account rotation was found. AG/terms/ was read in full.
  - Issue searches "multiple accounts", "switch account", "two accounts", "second account".
  - Forum feature request 173874.
  - No one-account-per-person clause found in AG/terms/ (Google ToS not re-read for this clause).
- **S5: Usage / quota machine-readable source, exit codes, renewal.**
  - Corpus grep for `/usage`, `quota`, `credits`, `exit`, `--json`, `renewal`, `billing`.
  - Found the changelog's print-mode `/usage` entries (v1.1.11, v1.2.15) and the statusline `quota` field. Found no JSON schema for `/usage`, no quota exit code, no App quota API and no renewal date.
  - Issue search "acp" led to #1045 (community-observed shape).
  - WebSearch "antigravity-usage skainguyen1412 local quota offline".
- **S6: Transcript / conversation format.**
  - Corpus grep for `transcript`, `brain`, `.pb`, `conversations/`, `sqlite`, `database`. Found paths (hooks, statusline) and changelog SQLite mentions, but no line schema.
  - WebSearch `"~/.gemini/antigravity/conversations" .pb antigravity conversation files`.
- **S7: Open source status of the CLI.** `gh api repos/google-antigravity/antigravity-cli/contents` shows no source and no license. Issue #4 is open. The Google developers blog post says nothing about the licence.
- **S8: ACP / app-server protocol.**
  - Corpus grep for `ACP`, `Agent Client Protocol`, `acp_server`, `app-server`, `JSON-RPC`: none.
  - `gh search code antigravity --repo agentclientprotocol/registry` found `antigravity-acp/agent.json`.
  - Issue search "acp": #31, #195 (duplicate of #31), #1045, #1273, #1281, #1307.
- **S9: Desktop App shell launcher binary.** The Homebrew `antigravity` cask has no `binary` stanza. Corpus grep for App CLI flags (`antigravity --`, `open -a`) found nothing.
- **S10: Google confirming VS Code lineage.** Corpus grep for `VS Code fork`, `fork`, `Windsurf`, `Code - OSS` in blog and docs found nothing. Secondary confirmation came from Thoughtworks Radar (fetched). Visual Studio Magazine returned 403.
- **S11: App / IDE headless or non-interactive mode.** Corpus grep for `headless` and `--print` outside `/docs/cli/` found only ADC "headless" auth, Remote Control daemon and Scheduled Tasks, with no App print mode.
