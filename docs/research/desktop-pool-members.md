# Desktop app sessions as pool members: the connector routes (AJ-E3013, D-116)

Research for 0.4.6, 2026-11-15. Asked for by D-116: find every route by which
AI Juggler can add its connector (the agent-bridge MCP server, plus the
Claude Code plugin for Claude) to a user's **desktop** app, so that app's
sessions load it. Documented and undocumented routes both (the standing
order of 2026-11-13). Each route says how sure it is and how a lab proves it.

**How this was researched (all read-only).** Vendor docs read on 2026-11-15.
The CLIs' own `--help`. The Claude app's bundle (`app.asar`, its main-process
JavaScript) and the bundled Claude Code binary (its strings). The ChatGPT
app's bundle and its bundled `codex`. File *names* and non-secret *keys* in
the profile folders. No app was launched, quit or clicked; no session was
started; no token, cookie, OAuth record or Keychain item was read.

**Versions seen.** Claude app 5.11492.5 (`CLAUDE_CODE_DESKTOP_APP_VERSION`).
Its bundled Claude Code is 2.1.325 in an everyday profile and 2.1.321 in
aij-test (`<user-data>/claude-code/<version>/<hash>/claude.app`). ChatGPT.app
26.1111.21308 (bundle id `com.openai.codex`), with codex-cli
0.169.0-alpha.11.1 in `Contents/Resources/codex-cli/`.

**Labels** follow `EVIDENCE-STANDARD.md`: **VP** (the vendor's own docs,
binary or CLI output say it), **VS** (secondary), **NE** (searched, not found:
log in §9), **I** (our inference; never product fact until a lab proves it).
"Local" marks a fact observed in files on this Mac. Every source was read
on 2026-11-15.

Paths use `<user-data>` for a Claude app profile's `--user-data-dir`
(`~/Library/Application Support/ClaudeProfiles/<profile>`), `<config>` for
its `CLAUDE_CONFIG_DIR` (`~/Library/Application Support/ClaudeCodeProfiles/<profile>`),
and `<agent>` for the bridge program of the AI Juggler edition doing the
Add (for example `/Applications/AI Juggler.app/Contents/MacOS/juggler-agent`).

---

## 0. Summary

| App | Recommended Add | Guided fallback | Confidence | Blocks |
|---|---|---|---|---|
| **Claude app, Code tab** | Write AI Juggler's plugin as a folder, `<config>/skills/ai-juggler/` (it loads as `ai-juggler@skills-dir`), and optionally one allow rule in `<config>/settings.json` | The app's own **+ › Add plugins** with AI Juggler's marketplace, or `claude plugin install` from the profile's bundled CLI, with copy buttons | **High** that it loads (vendor docs, VP). **Medium** on when a running session sees it, and on the hook events in the Code tab (docs say yes, unproven here) | **Identity** (§1.1): a desktop session has no AI Juggler key. The bridge needs a new identity check, an ADR-0020 addendum |
| **Claude app, chats and Cowork** | Not a member in 0.4.6. AI Juggler doesn't track chats or Cowork tasks as sessions, and their MCP servers are started by the app, so no chat can be told apart | `claude_desktop_config.json` `mcpServers` (per profile), or a `.mcpb` extension the user installs. Kept for a later release, after the identity probe (§7.3) | **High** that the file loads. **Low** on per-chat identity | Identity per chat: not found by this research |
| **ChatGPT's Codex app** | `[mcp_servers.juggler]` in the App's `config.toml` (`~/.codex` or its `CODEX_HOME`), shared with the CLI, with per-tool `approval_mode = "approve"` | Settings › MCP servers › Add server in the app, with copy buttons | **High** that it loads (VP). **Low** on which thread is calling | **Identity per thread**: an MCP server under the Codex app can't see its thread (§4.4). Needs the lab probe first |

**What this changes in AJ-E3013's plan.**

1. **Identity is the first build item.** AI Juggler's bridge knows a
   session only by the key a launch puts in its environment (ADR-0020). A
   desktop session never gets one, so today the plugin answers every call
   with "wasn't started by AI Juggler" (`juggler-agentbridge/src/plugin.rs`).
   §1.1 proposes a check that needs no key: process ancestry plus Claude
   Code's own session registry. It must be built and get a security review
   before any desktop member works.
2. **The Code tab's route needs no edit of a shared file.** Writing one
   folder (`skills/ai-juggler/`) adds the MCP server, the pool-tasks skill
   and the inbox hook together. Remove deletes the folder. Nothing is merged
   into `.claude.json`, which every running session rewrites.
3. **Waking is possible in the Code tab, through two documented routes.**
   D-116 keeps "waking stays the honest limit (D-66)". But Claude Code
   documents two sanctioned ways in, and both reach an idle Code-tab
   session:
   - a process posting to its own session's **inbox socket**, which "starts
     a new turn" when the session is idle;
   - an **`asyncRewake` hook** that "wakes Claude immediately even when the
     session is idle".

   D-66's standing rule says to support such routes, so this needs a ruling
   (§5.3). It is not built until the maintainer rules.
4. **Chats and Cowork aren't AI Juggler sessions today.** The pool members
   from a Claude app profile are its Code-tab sessions. If the maintainer
   expects chats to join too, that is a new decision (§8, Q1).

---

## 1. What every route must solve

### 1.1 Identity: which session is calling

**Today (VP, repo).** `juggler-agent` holds no state. Its identity is
`JUGGLER_AGENT_TOKEN`, which AI Juggler puts in the environment of a session
it launches, with `JUGGLER_AGENT_SOCKET` (`wire.rs`, `client.rs`). The app
checks the peer's user (`getpeereid`), then the key (`server.rs`). The
plugin's copy (`--mcp --plugin`) serves the tools. With no key, each call
answers `CallError::NoKey` (`plugin.rs`, AJ-E1905's handover note: "a key
only with a launch"). The hook (`hook.rs`) needs `JUGGLER_REPORT_TOKEN`.

**What a Code-tab session gives its MCP servers and hooks.**

| Fact | Source | Label |
|---|---|---|
| A stdio MCP server gets the CLI's whole environment plus `CLAUDE_PROJECT_DIR`, `CLAUDE_CODE_SESSION_ID` and `CLAUDECODE=1`, minus `CLAUDE_CODE_CHILD_SESSION` | Bundled CLI 2.1.325, the stdio spawn: `Rn={...An,CLAUDE_PROJECT_DIR:…,CLAUDE_CODE_SESSION_ID:xgt(),CLAUDECODE:"1",...n.env}` | VP (binary) |
| "Set automatically to the current session ID in … stdio MCP server subprocesses … Use to correlate scripts and external tools with the Claude Code session that launched them" | https://code.claude.com/docs/en/env-vars (`CLAUDE_CODE_SESSION_ID`) | VP |
| Stdio servers inherit the shell environment unless `CLAUDE_CODE_MCP_ALLOWLIST_ENV=1` | same page | VP |
| The Code tab's CLI environment holds `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_ENTRYPOINT=claude-desktop`, `CLAUDE_CODE_HOST_SESSION_ID=local_<uuid>` (the app's record id) and `CLAUDE_CODE_MESSAGING_SOCKET` | This research ran inside a Code-tab session; names and the non-secret ids were read from its own environment | Local |
| `CLAUDE_CODE_HOST_SESSION_ID` equals the app's record `<user-data>/claude-code-sessions/<account>/<org>/local_<uuid>.json`, the record `claude_app_family.rs` already reads | Local (matched on disk) | Local |
| Claude Code keeps a **session registry**: `<config>/sessions/<pid>.json` with `pid`, `sessionId`, `cwd`, `kind`, `entrypoint` (`claude-desktop`), `hostSessionId`, `messagingSocketPath`, `status` (`busy`/`idle`), `procStart`, `name`. Beside each is a `<pid>.<hash>.key` (an inbox key: a secret, never read) | Local (key names and enum values only) | Local |

**Proposed check: attested desktop identity (I, a design for the security
review).**

1. The bridge starts with no key. It finds its edition's socket on its own:
   `<data>/agent/bridge.sock` for the edition whose bundle it runs from. The
   socket path is fixed per data folder (`agent_bridge::socket_path`).
   Because AI Juggler writes the connector itself, the config names that
   edition's `<agent>` by its absolute path. No search script is needed.
2. Its first call claims an identity: `CLAUDE_CODE_SESSION_ID`,
   `CLAUDE_CODE_HOST_SESSION_ID`, `CLAUDE_CONFIG_DIR`, and its own pid.
3. The app checks the claim against the operating system, not the
   environment:
   - the peer's pid (macOS `LOCAL_PEERPID`, to add beside `getpeereid`);
   - its parent chain (at most two steps: a hook runs under `sh`) reaches a
     pid `P`;
   - `<config>/sessions/P.json` exists, with `sessionId` and `hostSessionId`
     equal to the claim and `procStart` matching `P`'s start time (this
     guards against a reused pid);
   - `<config>` is the folder of an App AI Juggler knows, and that session
     is in a pool.
4. The app then gives that bridge process a per-session key in memory, as a
   launch would, and every later call uses it. Talk rules and levels stay
   where they are (D-28).

Why it's sound: only a process whose parent is that Claude Code process can
pass, and the registry is Claude Code's own record of it. Same-user
processes are inside the threat model already (ADR-0020's socket checks).
The security review must cover pid reuse, the `sh` step in hooks, and a
process that outlives its session. The docs say stdio MCP servers are
"long-lived and outlive the session that spawned them"
(env-vars, `CLAUDE_CODE_CHILD_SESSION`, VP). So the check runs on the first
call and again whenever the registry entry changes.

**Not for Codex.** A Codex app thread's MCP server is a child of the one
`codex app-server` that serves every thread (`codex_live.rs`, VP repo). Its
parent says nothing about the thread (§4.4).

### 1.2 Talk rules and levels

No change. Every call still goes through the app, so the pool's rules and
level apply at the bridge as for terminal members (D-28, D-116).

### 1.3 Tool approval prompts

A session in a pool calls AI Juggler's tools often. Launched sessions
pre-allow them (`--allowedTools` for Claude, per-tool `approve` for Codex,
D-30), because "the pool's level is the real check". The desktop equivalents:

- **Claude Code tab:** an allow rule in `<config>/settings.json`. The docs
  say "Permission rules, allowed tools, and other settings in `settings.json`
  apply to Desktop sessions", and `--allowedTools` "No per-session equivalent.
  Permission rules in settings files still apply" (desktop page, VP). A
  plugin server's tools are named `mcp__plugin_<plugin>_<server>__<tool>`
  "in permission rules" (plugins/components, VP). The entry would be
  `"mcp__plugin_ai-juggler_ai-juggler"` (a server-level rule; I until the lab
  shows it matches every tool). Without the rule, each first call shows the
  app's permission card, and the user can allow it there.
- **Codex:** `default_tools_approval_mode = "prompt"` with
  `tools.<tool>.approval_mode = "approve"`. These keys are VP (learn.chatgpt.com
  MCP page), and the 2026-10-26 lab proved them (`pools-messaging-tasks.md`
  §Lab, VP lab).

### 1.4 "Connected", and how a guide is seen to be done

For every route, the signal that a member is live is the same: its first
attested bridge call. For the Code tab there is also an earlier,
file-level signal: the folder or entry exists, and a session's registry
entry appears after it. AI Juggler can show "Added; joins at its next
start". No polling is needed: the registry and the config file are read
when the canvas is opened, and the bridge pushes "connected" (D-116's
performance budget).

---

## 2. Claude app, Code tab

**How the Code tab runs Claude Code (VP, app binary 5.11492.5).** Each local
Code session runs the profile's bundled CLI through the Agent SDK with
`settingSources:["user","project","local"]` and `hooks` from the app's own
permission broker. It also passes `deniedMcpServers` for reserved names
(`index.chunk-D1yJ0UIX.js`). One launch path, a worktree session marked by
the launcher, narrows it to `settingSources=["user"]` (same chunk). So
**user-scope** routes reach every Code session, and project or local ones
may not. The CLI inherits the app's environment: `CLAUDE_CONFIG_DIR` comes
from how the app was started (Local).

The docs agree. Under "Shared configuration": "MCP servers configured in
`~/.claude.json` or `.mcp.json` work in both". "Hooks and skills defined in
settings apply to both". "Settings in `~/.claude.json` and
`~/.claude/settings.json` are shared"
(https://code.claude.com/docs/en/desktop, VP). Hook events: "Claude Code
fires the same hook events wherever it runs: sessions in the terminal, IDE
extensions, the Desktop app, and cloud sessions"
(https://code.claude.com/docs/en/hooks, VP). Plugins: "A plugin you install
at user scope in the terminal, the desktop app's local sessions, or the VS
Code extension is available in the other two on that computer, because all
three read the same settings files"
(https://code.claude.com/docs/en/plugins, VP).

Under a profile, every `~/.claude…` path above is `<config>/…`. The
everyday profiles hold no user MCP servers today, so there is no local
proof yet that one loads in the Code tab (Local: every
`<config>/.claude.json` has an empty `mcpServers`).

### Route A: AI Juggler's plugin as a folder in `skills/` (recommended)

- **File:** `<config>/skills/ai-juggler/`. It needs a
  `.claude-plugin/plugin.json`, and it loads as `ai-juggler@skills-dir`.
- **Source.** "`@skills-dir` — You saved a plugin directory that has a
  `.claude-plugin/plugin.json` under `~/.claude/skills/` … The manifest's
  `defaultEnabled`, unless a settings file sets `"<name>@skills-dir"`."
  "Neither the trust check nor these component limits apply to a plugin
  under your own `~/.claude/skills/`." Skills-directory plugins load in
  place and are never copied
  (https://code.claude.com/docs/en/plugins/loading, VP). The bundled CLI's
  own help agrees: `plugin init` "Scaffold a new plugin at
  ~/.claude/skills/<name>/ (auto-loads next session as <name>@skills-dir)"
  (`claude plugin init --help`, 2.1.325, VP).
- **Format (minimal).** One manifest carries the server, the hooks and the
  skill folder. Inline `mcpServers` and `hooks` in `plugin.json` both load
  (plugins/components, VP).

  `<config>/skills/ai-juggler/.claude-plugin/plugin.json`:
  ```json
  {
    "name": "ai-juggler",
    "version": "0.4.6",
    "description": "AI Juggler's pool connector: this profile's sessions read and send their pool's messages, shared docs and tasks while AI Juggler runs.",
    "mcpServers": {
      "ai-juggler": { "command": "<agent>", "args": ["--mcp", "--plugin"] }
    },
    "hooks": {
      "SessionStart":     [{ "hooks": [{ "type": "command", "command": "<agent>", "args": ["hook", "SessionStart"] }] }],
      "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "<agent>", "args": ["hook", "UserPromptSubmit"] }] }],
      "PostToolUse":      [{ "hooks": [{ "type": "command", "command": "<agent>", "args": ["hook", "PostToolUse"] }] }],
      "Stop":             [{ "hooks": [{ "type": "command", "command": "<agent>", "args": ["hook", "Stop"] }] }]
    }
  }
  ```
  `<config>/skills/ai-juggler/skills/pool-tasks/SKILL.md`: the existing
  skill. Tools show as `mcp__plugin_ai-juggler_ai-juggler__<tool>`, and the
  server as `plugin:ai-juggler:ai-juggler` in `/mcp`.
- **Who approves, and when.** The user, once, on AI Juggler's consent card,
  with the diff preview (D-116). Claude Code shows no approval for a
  plugin under the user's own `skills/` (VP, above). Tool calls follow
  §1.3.
- **When a running session sees it.** "Changes to settings or to disk don't
  reach this layer until you run `/reload-plugins` or start a new session"
  (plugins/loading, VP). In the Code tab a session "starts" when the app
  spawns its CLI. That is a new session, or an old one whose CLI the app
  restarts. When the app does this for an idle session is **I**; lab step
  A8 measures it. A restart is safe when every session of the profile reads
  `status: "idle"` in the registry (Local).
- **Remove, exactly.** Delete `<config>/skills/ai-juggler/`, but only while
  each file's sha256 is as written (`Undo::RemoveCreated`, VP repo). If a
  file changed, Remove refuses and names it. Then remove the allow rule
  from `settings.json` if Add wrote one (`Undo::RemoveSetting` with its
  fingerprint). A session that already loaded it keeps it until it
  restarts or reloads.
- **Risks.**
  - **Org policy.** Managed `strictPluginOnlyCustomization`,
    `strictKnownMarketplaces` (without `{ "source": "skills-dir" }`) or
    `blockedMarketplaces` skip the folder **with no error**
    (plugins/loading, VP). Detection: no attested call arrives, and
    `/status` lists "Enterprise managed settings". The guide then says so.
  - **Name clash.** A marketplace install of `ai-juggler` takes precedence,
    and the folder copy shows "Not loaded" (plugins/loading, VP). That's
    harmless: both are ours.
  - **The switch and inventory.** `claude_inventory.rs` lists `skills/<name>`
    as the user's skills, and the switch preflight copies skills between
    accounts. AI Juggler's own folder must be recognised as its connector,
    offered as such, and never copied as a user skill.
  - **Hidden files.** `inventory_files::copy_tree` leaves out hidden files
    on purpose (review of #132, VP repo). `.claude-plugin/` is hidden, so
    the connector needs its own writer.
  - **Versions.** The folder names one edition's `<agent>`. An upgrade
    rewrites it, and Remove checks hashes, so AI Juggler must re-record
    them.
- **Confidence: high** that it loads and that no prompt appears at load
  (VP docs). **Medium** for Code-tab specifics until lab A runs.

### Route B: plugin installed from a marketplace (guided fallback)

- **Files written by Claude Code itself.**
  - `<config>/settings.json`: `enabledPlugins["ai-juggler@ai-juggler"]: true`
    and `extraKnownMarketplaces`.
  - `<config>/plugins/known_marketplaces.json` and `installed_plugins.json`.
  - A GitHub marketplace is copied under `plugins/marketplaces/` and
    `plugins/cache/`. A local-path marketplace "has no copy here" and loads
    in place (plugins/loading, VP).
- **How.** In the app: **+ › Add plugins** (or **Plugins › Browse plugins**)
  in a local session (desktop page, VP). From the profile's bundled CLI:
  `CLAUDE_CONFIG_DIR=<config> <bundled claude> plugin marketplace add <source>`
  then `… plugin install ai-juggler@ai-juggler -s user`
  (`claude plugin install --help`: `--scope` "user" by default, `--json` for
  a machine-readable result; VP). AJ-E1905 (9c1e36562 on release/0.5) moves
  the marketplace to the repo's root for `/plugin marketplace add
  figurehead-ai/AI-juggler`.
- **Approval.** The app's own plugin dialog, or the user's command.
- **Seen by sessions.** Next start or `/reload-plugins` (VP).
- **Remove.** `claude plugin uninstall ai-juggler@ai-juggler -s user`, and
  `claude plugin marketplace remove ai-juggler` (VP help).
- **Risks.**
  - It edits `settings.json`, which the user may also edit.
  - Running the CLI against a live profile folder adds a second writer.
  - A GitHub source needs the network.
  - The published plugin has a top-level `bin/`, which "claude.ai and Cowork
    don't install" (plugins/components, VP), and which puts `juggler-mcp`
    on Claude's Bash `PATH`. Move the script out of `bin/` before
    publishing.
- **Confidence: high** (VP docs). This is the guide's route: the user does
  it in the app, and AI Juggler sees it done (§1.4).

### Route C: user-scope MCP server in `.claude.json`

- **File and key.** `<config>/.claude.json`, top-level
  `mcpServers.ai-juggler` (user scope). "Stored in `~/.claude.json`"
  (https://code.claude.com/docs/en/mcp, VP).
- **Format.** Written by Claude Code's own command:
  `CLAUDE_CONFIG_DIR=<config> <bundled claude> mcp add-json -s user -- ai-juggler '{"type":"stdio","command":"<agent>","args":["--mcp","--plugin"]}'`
  (`claude mcp add-json --help`, VP). This is what the switch preflight
  already does for a missing server (`claude_inventory::add_mcp`, VP repo),
  so D-116's "reused, not built twice" is met.
- **Approval.** None at load. User and local scopes don't prompt; only
  project `.mcp.json` servers do (mcp page, VP). Tool calls follow §1.3.
- **Seen by sessions.** Next start (I). The docs describe reconnecting a
  configured server, not adding one mid-session.
- **Remove.** `claude mcp remove -s user -- ai-juggler`, run only while the
  entry's fingerprint is unchanged (existing `Undo::Command` with
  `unless_changed`, VP repo).
- **Risks.**
  - `.claude.json` is rewritten by every running session (it holds
    `projects`, stats and caches). An outside write races them, and a
    second CLI process on a live profile folder is what the profile
    hand-off note warns against.
  - It adds the MCP server only: no skill, no hook.
- **Confidence: high** that it loads (VP). Use only where Route A is blocked
  by policy, and only with the app quit.

### Route D: local scope (`projects["<path>"].mcpServers` in `.claude.json`)

- Same file. Per project folder. The `local` setting source may be skipped
  in worktree launches (§2 intro).
- **Not recommended:** per project, and it has the same race as Route C.

### Route E: the project's `.mcp.json`

- **File and key.** `<project>/.mcp.json` `mcpServers`.
- **Approval.** "In `claude -p` runs, Agent SDK sessions, and cloud sessions
  … Claude Code loads project servers without asking" (mcp page, VP). The
  Code tab is an Agent SDK session, so this route loads without a prompt
  (I, until a lab shows it).
- **Not recommended:** it writes into the user's repository, a file they
  may commit. The Code tab shows `.mcp.json` as view-only (desktop page,
  VP).

### Route F: `claude_desktop_config.json` (reaches the Code tab too)

- **Source.** "The Desktop app loads MCP servers from
  `claude_desktop_config.json` into local Code tab sessions, alongside
  servers from `~/.claude.json` and `.mcp.json`" (desktop page, VP).
- **Not for the Code tab.** The app starts these servers itself, and hands
  them to a Code session through its MCP coordinator (`createAllServers`,
  with `sessionType:"ccd"` for Code sessions; app binary, VP). Their parent
  is the app, not the session's CLI, so the §1.1 check can't place them (I).
  They would also duplicate Route A's tools. See §3 for chats and Cowork.

### Route G: hooks in `<config>/settings.json` (no plugin)

- The inbox hook could live in the user's settings instead of the plugin.
  User-settings hooks apply in desktop sessions (hooks page, VP).
- **Not recommended:** it edits a file the user owns, and adds nothing over
  the plugin's hooks. Keep it as a fallback if a policy blocks plugins but
  not hooks.

### Route H: session-only loading (`--plugin-dir`, `--mcp-config`, `CLAUDE_CODE_PLUGIN_DIRS`)

- The Code tab takes no per-launch flags (desktop page, VP: "Flags not listed
  have no desktop equivalent").
- `CLAUDE_CODE_PLUGIN_DIRS` loads `@inline` plugins from the environment
  (plugins/loading, VP). AI Juggler starts profiles with `open … --env`, so
  it could set it with no file written. The app's code names the variable
  (it reads it from settings `env`, and lists plugin-related variables in
  the app's environment when deciding SSH hosts' plugin hooks; app binary,
  VP). Whether a local Code session's CLI receives it is **I**.
- **Not recommended:** it works only when AI Juggler started the app, so it
  breaks whenever the user starts the profile another way. Note it as an
  option for AI Juggler-started profiles.

### Recommendation: Code tab

- **Add.** Route A: write `<config>/skills/ai-juggler/` (MCP server, inbox
  hook, pool-tasks skill). Show the allow rule from §1.3 as a checked option
  on the same card ("…and lets them use AI Juggler's pool tools without
  asking each time; what they may do is still the pool's level").
- **Card wording.** "Add AI Juggler's connector to [App · profile] so its
  sessions can talk in this pool". The diff preview lists the folder's files
  and the one `settings.json` line.
- **After Add.** "New sessions in this profile join at once; open sessions
  join when they next start." Offer **Restart [App]** only when the
  registry shows every session idle.
- **Guide** (when the folder can't be written, or the user prefers): Route
  B in the app (**+ › Add plugins** › add marketplace › install, user
  scope), with copy buttons for the marketplace source. Detection: the first
  attested call.

---

## 3. Claude app, chats and Cowork

### Route F: `claude_desktop_config.json` per profile

- **File and key.** `<user-data>/claude_desktop_config.json`, top-level
  `mcpServers`. The app reads it at `app.getPath("userData")`, so a profile
  started with `--user-data-dir` has its own file (app binary: `X_()` joins
  `userData` with `"claude_desktop_config.json"`; VP). Local: each profile
  folder has one, today with only `preferences` and `coworkUserFilesPath`.
- **Format (minimal).** Merge one key and keep `preferences` and everything
  else byte for byte:
  ```json
  { "mcpServers": { "ai-juggler": { "command": "<agent>", "args": ["--mcp", "--plugin"] } } }
  ```
  An entry that fails the app's schema is skipped with a dialog: "Some MCP
  servers couldn't be loaded" (app binary, VP).
- **Reach.** Chats on this profile (the standard local MCP route). Cowork
  sessions too: the Cowork session build passes the local servers through
  `createAllServers` and drops one only when all of its tools are switched
  off (`pn("local:<name>", enabledMcpTools)`; app binary, VP). The Code tab
  as well (Route F above).
- **Approval.** At load: none. The file is the user's own config. Each tool
  call in a chat asks the user, "Allow once" or "Always allow" (**I**:
  well-known app behaviour, not re-read in docs today). The app only offers
  a lasting "Allow persistent tool approvals" setting for third-party
  deployments (`mcpPersistentAlwaysAllowEnabled`, scopes `["3p"]`; app
  binary, VP).
- **When sessions see it.** After the app restarts. The docs say, for a
  server's env: "Quit Desktop, set it in that server's `env` block, then
  reopen Desktop" (desktop page, VP).
- **Remove.** Delete `mcpServers.ai-juggler` if its fingerprint is unchanged
  (the existing `config_edit` merge and unmerge, VP repo), with the app quit.
- **Risks.**
  - The app also writes this file (its `preferences`), so the edit must be
    done with the app quit.
  - **Identity:** the app starts the server, and the server sees no chat or
    Cowork id. The app's code keeps per-session builds (`buildBySession`),
    but whether each chat gets its own process, and what environment it
    carries, is **I** (probe F in §7.3).
- **Confidence: high** that it loads (VP). **Low** for membership.

### Route X: a Desktop Extension (`.mcpb`, formerly `.dxt`)

- The user installs a bundle from the app's own dialog. The app stores it
  under `<user-data>/Claude Extensions/` with its settings in `Claude
  Extensions Settings/` (folder names from the app binary, VP; none
  installed in any profile here, Local).
- It is a **guided** route: the user's own click in the app is the consent.
  Same reach and identity questions as Route F.
- **Confidence: medium**. The mechanism is VP; using it for AI Juggler is
  untested.

### Route P: a plugin on the claude.ai account (Cowork, chats)

- **Customize › Plugins › Add › Upload plugin** installs a plugin for the
  whole account. Cowork loads its hooks and local MCP servers when the
  session runs on this computer. Chats ignore local servers and hooks
  (https://claude.com/docs/plugins/platform-support, VP).
- It reaches every device on that account, and syncs into Claude Code as
  `@synced` (plugins/loading, VP), so it is wider than one App. A top-level
  `bin/` makes it "Can't be installed" (VP).
- **Guided only.** AI Juggler can't do it, and shouldn't.

### Recommendation: chats and Cowork

Not part of 0.4.6 membership. AI Juggler reads the Code tab's records only
(`claude_app_family.rs`); it has no chat or Cowork session to put in a pool.
Run probe F (§7.3) in the 0.4.6 lab so the next release starts from facts.

If the probe shows a per-chat identity, the Add is Route F, with the app
quit. If it doesn't, the honest wording is "Claude app chats can use AI
Juggler's tools as the profile, not as a pool member".

---

## 4. ChatGPT's Codex app

### 4.1 Where its config is

- "The ChatGPT desktop app, Codex CLI, and IDE extension share this
  configuration". MCP servers live in `config.toml` (default
  `~/.codex/config.toml`) and in a project's `.codex/config.toml` "only for
  trusted projects" (https://learn.chatgpt.com/docs/extend/mcp?surface=app,
  VP).
- The app honours `CODEX_HOME` from its environment, resolving it at start,
  and `CODEX_ELECTRON_USER_DATA_PATH` for its own data (ChatGPT.app
  `early-bootstrap` chunk, VP binary). So an App with its own home is
  `open -n -a ChatGPT --env CODEX_HOME=<home> …` (I until lab C1).
- Local: `~/.codex/config.toml` already holds `[mcp_servers.<name>]` tables
  written by the app itself, one with `command` inside ChatGPT.app. So the
  app writes and reads this table.

### Route K: `[mcp_servers.juggler]` in the App's `config.toml` (recommended Add)

- **File and key.** `<CODEX_HOME>/config.toml`, table `[mcp_servers.juggler]`.
  Keys: `command`, `args`, `env`, `env_vars`, `cwd`, `enabled`,
  `enabled_tools`, `disabled_tools`, `startup_timeout_sec`,
  `tool_timeout_sec`, `required`, `default_tools_approval_mode`,
  `tools.<tool>.approval_mode` (learn.chatgpt.com MCP page, VP).
- **Format (minimal).**
  ```toml
  [mcp_servers.juggler]
  command = "<agent>"
  args = ["--mcp", "--plugin"]
  enabled_tools = ["whoami", "pool_roster", "set_status", "send_message", "read_inbox", "ack", "share_doc", "read_doc", "tasks_list", "task_get", "task_create", "task_claim", "task_update", "task_assign"]
  default_tools_approval_mode = "prompt"

  [mcp_servers.juggler.tools.whoami]
  approval_mode = "approve"
  # … one table per tool in enabled_tools
  ```
  The name `juggler` matches the launch overrides (`codex_bridge.rs`). A
  session AI Juggler launches therefore overrides the same keys with `-c`
  (args, `env_vars`), and the tools are never listed twice.
- **Writer.** Either `codex mcp add juggler -- <agent> --mcp --plugin`
  (Codex's own writer; `codex mcp add --help`, VP), or the existing append
  with backup (`codex_inventory.rs`: "Appends `[mcp_servers.<name>]` …
  after a backup", `Undo::RemoveAppended`, VP repo). The approval keys need
  the append either way.
- **Approval.** At load: none. Per tool: `approve` skips the prompt (VP docs;
  VP lab 2026-10-26).
- **When sessions see it.** After Settings › MCP servers › **Restart** in the
  app (docs, VP), which presumably restarts its app-server; threads loaded
  before that are **I**.
- **Remove.** `codex mcp remove juggler` (VP help), or remove the appended
  block exactly (`RemoveAppended`).
- **Risks.**
  - The file is shared with every terminal `codex` on that home, so those
    sessions also get the server, outside AI Juggler's launches. With the
    identity check they answer "not in a pool", which is honest.
  - Codex rewrites `config.toml` (it keeps `projects`, `plugins` and
    `hooks.state` there), so edit with the app quit, or through `codex mcp`.
- **Confidence: high** that it loads (VP). **Low** for membership (§4.4).

### Route L: a Codex plugin (`codex plugin add`)

- Codex has plugins and local marketplaces. `config.toml` holds
  `[marketplaces.<name>] source_type = "local"` and
  `[plugins."<name>@<market>"] enabled = true` (Local; `codex plugin --help`,
  VP). "Plugins can also bundle hooks" (learn.chatgpt.com hooks page, VP).
- A later, tidier package for Codex (server, skill and hooks together). Not
  needed for 0.4.6.

### 4.4 Identity: why it is the blocker for Codex

- The Codex app runs every thread in one `codex app-server`
  (`codex_live.rs`, VP repo). An MCP server's parent is therefore that
  shared process.
- Codex starts MCP servers with a small default environment (HOME, LOGNAME,
  PATH, SHELL, USER, `__CF_USER_TEXT_ENCODING`, LC_ALL, TMPDIR, TZ and the
  CA variables), plus what `env_vars` forwards (codex binary strings,
  `rmcp-client`, VP; `env_vars` "to allow and forward", VP docs).
- `CODEX_THREAD_ID` appears only in the shell-command environment (codex
  binary strings beside `exec_command` and the shell environment policy, VP),
  so it isn't available to an MCP server.
- Two candidates, both **I**:
  1. **Request metadata.** The binary has "missing MCP request metadata
     object" and a `codex/connectionLease`. Codex may send a `_meta` object
     with tool calls, and probe C2 reads its keys.
  2. **The shell route.** A Codex thread's shell commands carry
     `CODEX_THREAD_ID`. The bridge's command-line mode (`juggler-agent
     tasks list`, AJ-E1902 point 2) run from the thread's shell could prove
     its thread by that value plus ancestry (shell, then app-server). That
     makes the MCP server optional for Codex.
- Codex hooks get `session_id` (the thread) on stdin, need the user's trust
  in Settings › Hooks, and can add `additionalContext` on `SessionStart`
  and `UserPromptSubmit`. "Finishing a background hook doesn't start a new
  turn" (https://learn.chatgpt.com/docs/hooks, VP). Local: config.toml
  already has a `[hooks.state."<file>:session_start:0:0"] trusted_hash`
  entry, which is the trust record.

### Recommendation: Codex app

- **Add:** Route K with the identity probe passed. Until then, **guide
  only**:
  - Settings › MCP servers › Add server (name `juggler`, STDIO, the command
    and args to copy), then **Restart**;
  - the approval lines to paste into `config.toml`.
- **Membership for Codex app threads waits for probe C2.** If `_meta`
  carries the thread, the Codex member works like the Code tab. If not, the
  shell route (candidate 2) is the fallback, and its tools are commands in
  the thread's shell.
- Waking: none. A Codex hook can't start a turn (VP).

---

## 5. What an inbox hook can do, per app

### 5.1 Claude Code tab (through the plugin's hooks)

These are the events the existing `juggler-agent hook <event>` already
handles (`wire::HOOK_EVENTS`, `hook.rs`). The desktop identity check (§1.1)
replaces the report key.

| Event | Use | Source |
|---|---|---|
| `SessionStart` | Attest identity once. Say "you're in pool P as Member; call `read_inbox`" as `additionalContext` "at the start of the conversation". Runs again on resume | hooks page, VP |
| `UserPromptSubmit` | A facts-only note before the turn ("2 unread from Planner"). Default timeout 30 s on this event. Output capped at 10,000 characters | hooks page, VP; note format VP repo |
| `PostToolUse` | The same note mid-turn, so a busy member hears new messages between tool calls | VP repo (2026-10-26 lab), hooks page |
| `Stop` | Report idle. Used for the waker below | hooks page, VP |
| `Notification`, `PermissionRequest` | Report "needs you" (AJ-E2306); never answer | VP repo |

### 5.2 Waking an idle Code-tab session (two documented routes, needs a ruling)

1. **The session's inbox socket.** Each Code-tab session binds one
   (`CLAUDE_CODE_MESSAGING_SOCKET`; Local: present in this Code-tab
   session's environment, and `messagingSocketPath` in every desktop
   registry entry). The docs say:
   - "When the receiving session is idle, Claude Code starts a new turn
     with the message."
   - "Read this section when … you want a script or hook to post into a
     session."
   - **Own-child messages**: "when no `crossSessionInbound` value applies,
     Claude Code delivers a message it verifies came from the session's own
     child processes, such as a hook or Bash command posting back to its
     own session's socket". On macOS this is verified "only while the
     posting process is still running", or by `CLAUDE_CODE_MESSAGING_TOKEN`
     in an auth line.

   (https://code.claude.com/docs/en/cross-session-messaging, VP.)

   The line format, from the CLI's own debug hint (2.1.325 strings, VP
   binary):
   ```
   {"type":"auth","token":"$CLAUDE_CODE_MESSAGING_TOKEN"}
   {"type":"user","message":{"role":"user","content":"…"}}
   ```
   The bridge's MCP server is a long-lived child of the session, so it can
   post a facts-only notice ("AI Juggler: 1 new pool message from Planner;
   read it with read_inbox"). Such a message can't approve anything or
   change settings (same page, "How a session treats an incoming message",
   VP).

   Unproven: whether a stdio MCP server's environment holds the socket and
   token. The docs name hooks and Bash. The spawn copies the CLI's
   environment (§1.1), but which values are in it at that moment is
   unknown. Lab A9 checks.
2. **An `asyncRewake` hook.** "an `asyncRewake` hook that exits with code 2
   wakes Claude immediately even when the session is idle". `timeout` is
   enforced on it (hooks page, VP). For example, a `Stop` hook that waits
   (bounded) for a pool message, then exits 2.

Both put words into the session only through Claude Code's own channels,
not the app's window, so ADR-0003 holds (I). But D-116 says "Waking stays
the honest limit (D-66)", and D-66's standing rule says "if we do stumble
upon desktop app that does support some kind of standard or protocol to
reply we should support it". So this needs the maintainer's ruling. If the
answer is yes, route 1 is preferred: it lives in the connector, needs no extra
process per turn, and is rate-limited by Claude Code.

**Sealed pools.** A sealed launch sets `crossSessionInbound: "refuse"`
(`claude_bridge.rs`). That refuse would also drop AI Juggler's own wake.
And for a desktop member, sealing means writing that key into the
profile's `settings.json`, which affects every session of the profile.
Open question Q4.

### 5.3 Claude chats and Cowork

- Chats: no hooks (platform-support: Hooks "Ignored" in Chat, VP).
- Cowork: plugin hooks load when installed on the account (VP), but
  AI Juggler doesn't track Cowork tasks. Out of scope.

### 5.4 Codex app

- `SessionStart` and `UserPromptSubmit` notes, as `additionalContext`, once
  the user trusts the hook (VP).
- No waking: "Finishing a background hook doesn't start a new turn" (VP).
- A hook's `session_id` could also give Codex identity a footing: a
  `SessionStart` hook registers the thread (I).

---

## 6. Things the build must change (from this research)

1. **Desktop identity in the bridge and app** (§1.1). This is an ADR-0020
   addendum. It needs `LOCAL_PEERPID`, the registry check and an in-memory
   key, and the security review comes first.
2. **The plugin.**
   - Write the connector folder with absolute `<agent>` paths (no search
     script, no top-level `bin/`).
   - Change `plugin.rs`'s no-key branch: it should attest, not refuse.
   - Keep `ALREADY_GIVEN` for launched sessions.
3. **A connector writer** with `Undo::RemoveCreated`. It can't reuse
   `copy_tree`, which skips hidden files.
4. **Inventory and switch.** Recognise `skills/ai-juggler/` (and
   `mcpServers.juggler` in Codex) as AI Juggler's connector. Offer it in the
   switch preflight as "AI Juggler's connector", never as a user skill.
5. **Canvas.**
   - "gets messages on its next turn" (D-116 point 3).
   - "Added; joins at its next start" between Add and the first attested
     call.
   - Idle or busy from the registry (Local fields).
6. **Cherry-pick 9c1e36562** (AJ-E1905 packaging), keeping only its plugin
   hunk of `nightly.yml` (D-116 point 6). Move `bin/juggler-mcp` before the
   plugin is published.

---

## 7. Proof plan (live checks, lab only)

Rules for every step:

- Only the sanctioned **aij-test** Claude app profile (user data
  `~/Library/Application Support/ClaudeProfiles/aij-test`, Claude Code folder
  `~/Library/Application Support/ClaudeCodeProfiles/aij-test`). Start it
  with `open -n -a Claude --env CLAUDE_CONFIG_DIR=<config> --env
  DISABLE_AUTOUPDATER=1 --args --user-data-dir=<user-data>`. Quit and
  restart it freely.
- And AI Juggler Dev's managed **Lab App** folder
  (`~/Library/Application Support/AI Juggler Dev/accounts/claude-code/claude-code-361780292000`):
  headless `claude -p` works there.
- Projects under `~/lab/` only. Guarded keystrokes (frontmost and topmost
  check). Never a real profile. Never read a key, token or credential file.
- In the aij-test app, act only through `scripts/qa/app_drive` with
  `AIJ_APP_PROFILE=aij-test` (`docs/qa/claude-app-drive.md`): it refuses any
  pid that isn't that profile's Claude app, and sends nothing to another app.
- Every step records the file snapshots (sha256 of the files the step may
  touch, before and after), the `ps` parent chain (names and pids only),
  and the bridge's log.
- At the end, reap every process started (`pgrep -f juggler-agent` is 0)
  and quit aij-test.

Evidence goes to `docs/qa/evidence/AJ-E3013/`.

### 7.0 The probe

Before routes are compared, build a probe mode of the bridge
(`juggler-agent --mcp --probe <out-file>`, test-only). It serves one tool,
`probe`, and writes to a `0600` file in the lab:

- the **names** of its environment variables, never their values, except
  `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_HOST_SESSION_ID` and
  `CODEX_THREAD_ID` (ids, not secrets);
- its pid, its parent chain;
- the MCP `initialize` `clientInfo`;
- the **keys** of any `_meta` object on each request.

The `--probe` flag is refused outside debug builds.

### 7.1 Claude Code tab, Route A (on the Lab App folder, headless, first)

1. **L1:** copy the connector folder (with the probe as its server) to
   `<lab>/skills/ai-juggler/`. Run
   `CLAUDE_CONFIG_DIR=<lab> claude plugin list --json`. Expect
   `ai-juggler@skills-dir`, enabled.
2. **L2:** `claude -p "call the probe tool" --output-format stream-json` in
   `~/lab/<project>`. Expect:
   - no approval prompt for the server;
   - the tool listed as `mcp__plugin_ai-juggler_ai-juggler__probe`;
   - the probe shows `CLAUDE_CODE_SESSION_ID` and `CLAUDE_CONFIG_DIR`, and
     whether `CLAUDE_CODE_MESSAGING_SOCKET` and `…_TOKEN` are present (names
     only);
   - its parent pid has `<lab>/sessions/<pid>.json`.
3. **L3:** add `"mcp__plugin_ai-juggler_ai-juggler"` to `<lab>/settings.json`
   `permissions.allow`. Run L2 without `--allowedTools` and in default
   mode, and confirm no permission request is made.
4. **L4:** the hooks. With `UserPromptSubmit` and `SessionStart` hooks
   printing a fixed note, confirm the note is in the transcript as a system
   reminder.
5. **L5:** Remove (delete the folder by hashes). `plugin list` no longer
   shows it, and `<lab>/settings.json` is byte-identical to its snapshot
   after the allow rule is removed.

### 7.2 Claude Code tab, Route A (on aij-test, in the app)

1. **A1:** quit aij-test. Snapshot `<config>/{settings.json,.claude.json}`
   and the list of `<config>/skills/`.
2. **A2:** in AI Juggler Dev (the build under test), make a pool with an
   aij-test Code session and a Lab CLI session. On the consent card, check
   that the diff preview lists only `skills/ai-juggler/**` and the one
   `settings.json` line. Click **Add**.
3. **A3:** check that only those paths changed (snapshot diff), and that
   the recorded hashes match the files.
4. **A4:** start aij-test. Open a **new** Code session in `~/lab/<project>`.
   Check that `<config>/sessions/<pid>.json` appears with
   `entrypoint: "claude-desktop"`, and that `ps` shows `<agent> --mcp
   --plugin` as that pid's child.
5. **A5:** in that session, ask it to call `whoami` from AI Juggler's
   tools. Record:
   - whether any prompt appeared (none expected at load; none for the tool
     with the allow rule);
   - the result naming the pool and member (the attested identity).
6. **A6:** from the Lab CLI member, `send_message` to the aij-test member.
   In aij-test, type a prompt and confirm the `UserPromptSubmit` note
   appears (the transcript has it). The member calls `read_inbox`, then
   `send_message` back, then `task_claim` on a lab task. AI Juggler shows
   each step on the canvas, and the talk rules hold (repeat with a rule set
   to Blocked: refused at the bridge).
7. **A7:** the canvas reads "gets messages on its next turn" for the idle
   aij-test member, never "can't reply".
8. **A8 (pick-up timing):** a Code session opened **before** Add. Record
   whether it has the tools after:
   - (a) a new prompt;
   - (b) `/reload-plugins` typed in it;
   - (c) quitting and reopening aij-test.

   This sets the card's wording.
9. **A9 (waking, only if ruled in):**
   - The probe from L2 says whether the bridge holds
     `CLAUDE_CODE_MESSAGING_SOCKET` and `…_TOKEN`.
   - If it does: with the aij-test session idle (registry
     `status: "idle"`), send a pool message from the Lab member. Expect the
     bridge to post one notice to its own socket, the registry to go
     `busy`, and the session to call `read_inbox`.
   - If it doesn't: try the `asyncRewake` `Stop` hook instead.
10. **A10 (Remove):** click **Remove**. The folder and the allow line are
    gone. `settings.json` is byte-identical to A1's snapshot, and
    `.claude.json` differs only in Claude's own keys (`numStartups`,
    `projects` stats). A new session has no AI Juggler tools.
11. **A11 (refusals):**
    - Edit one file in the folder, then Remove: it refuses and names the
      file.
    - Put an unrelated `skills/<other>/` beside it, then Add and Remove:
      it is untouched.
12. **A12 (policy):** in the Lab folder only, set a `--settings` value with
    `strictKnownMarketplaces` (no `skills-dir`). Confirm the folder is
    skipped silently and AI Juggler's guide says why.

### 7.3 Claude chats and Cowork, probe F (on aij-test)

1. **F1:** quit aij-test. Merge `mcpServers.ai-juggler-probe` (the probe)
   into `<user-data>/claude_desktop_config.json`, and keep a snapshot.
2. **F2:** start aij-test. In a chat, ask it to use the probe tool, then in
   a second chat. Record:
   - the approval UI ("Allow once / Always allow");
   - one process or one per chat (pids);
   - the environment names, `clientInfo` and `_meta` keys.
3. **F3:** the same in a Cowork task on this computer.
4. **F4:** a Code session. Is the probe listed (it should be, per the
   docs), and is its parent the app or the CLI?
5. **F5:** quit aij-test, remove the key, and check the file is
   byte-identical to F1's snapshot.
6. **F6 (optional):** build the probe as a `.mcpb`. The **later live
   check** installs it through the app's own dialog on aij-test, records
   where it lands, then removes it from Settings › Extensions.

### 7.4 Codex app, probes C1 and C2

These need a lab Codex home, signed in once by the maintainer (the only
maintainer step). Its name and location follow the separate-profiles rule.

1. **C1:** start the app on it:
   `open -n -a ChatGPT --env CODEX_HOME=<lab-codex-home> --env CODEX_ELECTRON_USER_DATA_PATH=<lab-codex-ui>`.
   Confirm that `~/.codex` isn't touched (snapshot its `config.toml` hash,
   names only) and that the app shows the lab home's threads.
2. **C2:** quit the app. Append `[mcp_servers.juggler-probe]` (the probe)
   with `approval_mode = "approve"` for its tool. Start the app, then
   Settings › MCP servers › **Restart**. In two threads, call the probe.
   Record:
   - one process per thread or shared;
   - its parent (expected: `codex app-server`);
   - environment names (expected: no thread id);
   - `_meta` keys (does a thread id or connection lease appear?);
   - whether any approval prompt appeared.
3. **C3:** in one thread, ask it to run `juggler-agent --probe-shell` (a
   debug command that prints `CODEX_THREAD_ID` and its parent chain).
   Confirm the thread id matches the thread, and the ancestry reaches the
   app-server. This tests the shell route.
4. **C4:** add a `SessionStart` hook printing a fixed note (trust it in
   Settings › Hooks) and confirm the note reaches the thread. Remove both
   tables, then check the file matches its snapshot (`RemoveAppended`).

### 7.5 The newcomer walk (AJ-E3011, extended)

- A Sonnet newcomer persona walks "add a Claude app profile to a pool" on
  aij-test: the consent card, the diff, Add, then the first message.
- Then the guide path: the user installs through **+ › Add plugins**, and
  AI Juggler detects the first call.
- Opus checks each verdict.

---

## 8. Open questions

1. **Chats and Cowork as members (for the maintainer, only if they were
   expected).** AI Juggler tracks only Code-tab sessions of a Claude app
   profile. Chats and Cowork tasks can't be told apart by an MCP server
   (until F2 shows otherwise). Is "a Claude app profile's Code-tab sessions
   join pools" what D-116 meant? If chats must join, that is a new feature:
   reading chats as sessions.
2. **Waking (for the maintainer: a D-number).** Adopt Claude Code's own
   inbox socket, or an `asyncRewake` hook, to wake an idle Code-tab member
   (§5.2)? This amends D-116's "waking stays the honest limit" for the Code
   tab only. Codex can't be woken.
3. **The identity design (for the main session, then the security review).**
   The ancestry plus registry check (§1.1) as an ADR-0020 addendum. Do the
   docs' "MCP servers outlive the session" cases (resume, `/clear`) need the
   check on every call?
4. **Sealed pools with desktop members.** Sealing writes
   `crossSessionInbound: "refuse"` and denies `SendMessage`/`ListAgents` for
   the whole profile (not per session), and it would block AI Juggler's own
   wake. Offer it with that consequence named, or keep desktop members out
   of sealed pools?
5. **Pick-up timing (lab A8).** When does the app restart an idle session's
   CLI? Is `/reload-plugins` available in the Code tab? The desktop page
   says terminal-dialog commands may answer "isn't available in this
   environment".
6. **Codex identity (probe C2 and C3).** If neither `_meta` nor the shell
   route proves the thread, Codex app threads stay "watched" in 0.4.6, and
   the canvas must say why.
7. **The allow rule's form (lab L3).** Does `mcp__plugin_ai-juggler_ai-juggler`
   match every tool of the server, or must each tool be listed?
8. **Does the Code tab drop `CLAUDE_CODE_PLUGIN_DIRS`** from the app's
   environment (Route H)? Only matters if AI Juggler-started profiles are
   to skip the file write.

---

## 9. Sources and search log

**Vendor docs (VP, read 2026-11-15)**
- https://code.claude.com/docs/en/desktop (Shared configuration; "MCP
  servers from the Claude Desktop chat app"; CLI flag equivalents;
  terminal-dialog commands)
- https://code.claude.com/docs/en/mcp (scopes; project approval and SDK
  sessions; plugin servers; `requiresUserInteraction`)
- https://code.claude.com/docs/en/hooks (events; additionalContext; async,
  `asyncRewake`; workspace trust; "fires the same hook events wherever it
  runs")
- https://code.claude.com/docs/en/plugins, /plugins/loading,
  /plugins/components (skills-dir, install scopes, reload, naming,
  monitors, `bin/`)
- https://code.claude.com/docs/en/cross-session-messaging (inbox socket,
  own-child messages, `crossSessionInbound`)
- https://code.claude.com/docs/en/channels (research preview; `--channels`
  and an Anthropic allowlist are required, so **not usable** for AI Juggler
  in the Code tab)
- https://code.claude.com/docs/en/env-vars (`CLAUDE_CODE_SESSION_ID`,
  `CLAUDE_CODE_MESSAGING_SOCKET`/`_TOKEN`, `CLAUDE_CODE_CHILD_SESSION`,
  `CLAUDE_CODE_MCP_ALLOWLIST_ENV`, `CLAUDE_CONFIG_DIR`)
- https://claude.com/docs/plugins/platform-support (Chat, Cowork, Claude
  Code component table)
- https://learn.chatgpt.com/docs/extend/mcp?surface=cli and `?surface=app`
  (shared config, keys, approvals, Restart)
- https://learn.chatgpt.com/docs/hooks (Codex hook events, trust, no wake)

**Binaries and CLI output (VP, read-only)**
- Bundled Claude Code 2.1.325: `--help`, `mcp --help`, `mcp add-json
  --help`, `mcp remove --help`, `plugin --help`, `plugin install --help`,
  `plugin init --help`, `plugin uninstall --help`, `plugin marketplace add
  --help`. Strings: the stdio spawn environment; the `[uds-messaging]`
  inject hint; the skills-dir adoption messages.
- Claude.app 5.11492.5 `app.asar` (`.vite/build/*.js`, extracted to the
  session's scratchpad):
  - Code-session `settingSources`;
  - the per-profile `claude_desktop_config.json` path;
  - the MCP schema check and dialog;
  - `createAllServers` with `sessionType "ccd"`;
  - the `enabledMcpTools` filter;
  - the `persistentAlwaysAllowEnabled` scope;
  - the "Claude Extensions" folders;
  - `CLAUDE_CODE_PLUGIN_DIRS` handling.
- ChatGPT.app 26.1111.21308: bundled `codex --version`, `--help`, `mcp
  --help`, `mcp add --help`, `mcp remove --help`, `plugin --help`. Binary
  strings: MCP default environment; `CODEX_THREAD_ID` contexts; request
  metadata. `app.asar` early-bootstrap: `CODEX_HOME`,
  `CODEX_ELECTRON_USER_DATA_PATH`.

**Local observation (names and non-secret keys only)**
- The Code tab session's own environment (variable names; the two session
  ids).
- `<config>/sessions/<pid>.json` (field names and enum values).
- `<config>/.claude.json` (no user or project MCP servers in any profile).
- `<config>/settings.json` (keys only).
- `<user-data>/claude_desktop_config.json` (top-level keys).
- `<user-data>/plugin-list/…/plugins.json` (empty).
- `~/.codex/config.toml` (table headers; `[mcp_servers.*]` names;
  `hooks.state` key form).

**Repo (VP)**
- `crates/juggler-agentbridge/src/{lib,main,client,plugin,hook,wire}.rs`
- `crates/juggler-adapters/src/{claude_bridge,codex_bridge,claude_inventory,codex_inventory,inventory_files,claude_app_family,codex_live,codex}.rs`
- `crates/juggler-app/src/features/agent_bridge/{mod,server}.rs`
- `plugins/claude-code/**`
- `docs/research/{reply-and-read,pools-messaging-tasks}.md`
- `docs/tasks/{E30-pools-redesign,E19-agent-tasks}.md`
- `docs/review/decision-log.md` (D-66, D-116)
- commit 9c1e36562

**Negative evidence**
- *Per-chat identity for a `claude_desktop_config.json` server.* Searched
  the app binary for the environment a local MCP server is started with,
  and for session-id variables passed to it (`CLAUDE_*SESSION*` names;
  `createAllServers`; `laptopEnv*`). Not found by this search. Probe F
  decides it.
- *A thread id available to a Codex MCP server.* Searched the codex binary
  strings for `CODEX_THREAD_ID` and `CODEX_SESSION_ID` contexts, the MCP
  default environment list, and `_meta` and `x-codex-*` keys. Found only in
  the shell-command environment. Probe C2 decides it.
- *Hook events in the Code tab, listed by the desktop page.* Not listed
  there; the hooks page's "wherever it runs" sentence is the source.
