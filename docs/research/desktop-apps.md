# Research notes: desktop apps, routes before "track-only"

Research date: 2026-11-15, for AJ-E1610 requirement 3 (D-106's standing order: look for undocumented routes before calling any part track-only).

> **How this was done.** Reading only. The one installed app in scope (the Claude app) had its own `Info.plist` and the `package.json` and built script files inside its `app.asar` read as text, extracted into the session scratchpad and never run. The other apps are not on this Mac, so their facts come from the vendors' docs and public source. No app was launched. No sign-in, token, cookie or conversation was read; only the names of files in the Claude app's data folder were listed. Nothing was installed or downloaded for this note.
>
> **Labels** (as in [EVIDENCE-STANDARD.md](EVIDENCE-STANDARD.md)):
> **[Verified-primary]** the vendor's own docs, source, or the installed bundle's own files say it ·
> **[Verified-secondary]** a reliable second-hand source says it ·
> **[Inferred]** my reasoning, never a fact.
>
> **Not redone here.** Cursor, Kiro and Antigravity have their desktop sections in [cursor.md](cursor.md), [kiro.md](kiro.md) and [antigravity.md](antigravity.md) (updated 2026-11-15). This note only cross-references them (section 6) and answers the desktop questions they left open.

## 0. At a glance

| App | On this Mac? | Bundle id | Own profile per account? | Sessions shared with its CLI? | Best route found |
|---|---|---|---|---|---|
| Claude app | Yes, 5.11492.5, `/Applications/Claude.app` | `com.anthropic.claudefordesktop` | Yes: Electron's `--user-data-dir` (already used) | The Code tab runs Claude Code; its own record points at the CLI conversation | Own links into one session (`claude://...`), read its records; no reply route |
| OpenCode desktop | No | `ai.opencode.desktop` (beta `.beta`, dev `.dev`) | Not documented; Electron switch untested | Yes: one data folder with the CLI | Its own server and `opencode://` links; replies through the CLI's server, not the window |
| Kimi Code desktop | No | not published | Not documented | Yes: `~/.kimi-code` | `kimi-code://open?root=...&new=1` opens a new chat in a folder |
| Hermes desktop | No | `com.nousresearch.hermes` | Through `HERMES_HOME` (no documented launcher) | Yes: one gateway writes, all clients read | The gateway's own routes (the CLI adapter's), shown live in the app |

## 1. The Claude app

### 1.1 Install and version
- **[Verified-primary]** `/Applications/Claude.app`, bundle id `com.anthropic.claudefordesktop`, version `5.11492.5`, Electron, macOS 13 or later. Read from `Contents/Info.plist` here, 2026-11-15.
- **[Verified-primary]** Detection: `mdfind "kMDItemCFBundleIdentifier == 'com.anthropic.claudefordesktop'"` finds only `/Applications/Claude.app` (the profile launchers the user made are separate small apps, not the bundle).
- **[Verified-primary]** Already built in AI Juggler: `claude_live.rs` lists the app as a host with `--user-data-dir` as the instance switch and `claude://claude.ai/epitaxy/{id}` as its session link.

### 1.2 Profiles per account
- **[Verified-primary]** The app takes Electron's `--user-data-dir`. Its own script tests for that switch (`isPackaged && !hasSwitch("user-data-dir")`), and AI Juggler's profiles already run this way (entity-model-audit.md).
- **[Verified-primary]** A signed build **ignores** the `CLAUDE_USER_DATA_DIR` environment variable. Its log line says so: the variable "is set and signed builds ignore it". So the switch is the only route; an environment variable does nothing.
- **[Verified-primary]** The app's Code tab runs Claude Code, and `CLAUDE_CONFIG_DIR` is the folder switch for that engine (the app's script reads it in many places). A profile is therefore two things: the app's data folder and the paired Claude Code folder. AI Juggler already records this (`folder_env`).
- **[Verified-primary]** Only one launch key exists per data folder, so a second launch with the same folder hands over to the first. Two profiles run side by side because their folders differ **[Inferred]** from Electron's single-instance lock being per data folder; AI Juggler's own profile work proves it in practice.
- The sign-in is never read, copied or touched.

### 1.3 Sessions
- **[Verified-primary]** Where: `<data folder>/claude-code-sessions/<account>/<org>/local_<id>.json`, one record per Code session, read by AI Juggler already (title, folder, archived, last activity, pointer to the CLI conversation). The CLI conversation itself is a `projects/*.jsonl` file in the paired Claude Code folder.
- **[Verified-primary]** The data folder also holds `remote-control-state.json`, `local-agent-mode-sessions`, `git-worktrees.json` and `side-session-notes` (names only; not read).
- Chat and Cowork conversations are not Claude Code sessions: they live on claude.ai and in the app's web storage, not in files AI Juggler can read.
- **[Verified-primary]** Docs: the desktop page says the app does not see "sessions you started from the terminal" in its list; resuming one that the app also runs risks two writers. https://code.claude.com/docs/en/desktop

### 1.4 Routes (read from the bundle, section by section)
- **Own links, `claude://`** (`CFBundleURLSchemes` is `claude`; a second scheme `msauth.com.anthropic.claudefordesktop` is only for a sign-in return).
  - **[Verified-primary]** Documented by the vendor: `claude://claude.ai/new` (`q`), `claude://claude.ai/chat/<id>`, `claude://claude.ai/project/<id>`, `claude://code/new` (`q`, `folder`), `claude://cowork/new`. A folder in a link is untrusted and always asks the user first. https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link
  - **[Verified-primary, undocumented]** In the bundle's own link handler: `claude://code/continue?session=last|local_<id>`, `claude://code/needs-input[?session=...]` (the sessions waiting for you), `claude://code/search`, and `claude://code/<id>` for cloud ids (`session_`/`cse_`). `continue` finds a non-archived local session by id and opens its page.
  - **[Verified-primary, undocumented]** The app's own sidebar link to one session, `claude://claude.ai/epitaxy/local_<id>`, is what AI Juggler's Go to it already sends. It is a generic in-app address, so it keeps working only as long as the app's route name does.
  - **[Verified-primary]** The `continue` and `needs-input` links first check an "entry points" switch inside the app and are ignored when it is off. I could not tell from the files whether the switch comes from a setting or from the vendor's server. Community reports say session links are sometimes gated or open a blank window (https://claudeissues.com/issue/85042-make-the-gated-code-session-deep-link-reachable-so-external-tools-can-focus-a-ru). [Inferred] Treat each as best effort: send, then verify the right page by looking, and fall back to plain focus.
  - AI Juggler sends a link to the process of the right profile only (an Apple Event to its pid), so the right account's window gets it. No password or token is in any link.
- **Local servers and sockets.**
  - **[Verified-primary]** The only listeners in the app's script are a sign-in return on `127.0.0.1` during a login, and a socket for its browser bridge (the Chrome add-on). Neither takes sessions or prompts.
  - Debug and remote-control switches: an environment variable `CLAUDE_CDP_AUTH` exists but is for the vendor's own test harness; using the debugger port would be driving the window and is outside D-66. Not pursued.
- **AppleScript.** **[Verified-primary]** No scripting definition (`.sdef`, `OSAScriptingDefinition`, `NSAppleScriptEnabled`) in the bundle. The app only declares it may send Apple Events to others.
- **MCP and extensions.** **[Verified-primary]** The app has its own MCP and desktop-extension surface (`.mcpb`, `.dxt`, skills). It is for giving Claude tools, not for letting another program send the user's turn.
- **Claude Code's desktop hand-off.** **[Verified-primary]** `claude --desktop` hands a CLI session to the app by the `claude://resume?session=<id>` link (AI Juggler's `handoff_link`); the app records what it took under `claude-code-sessions`. It imports one conversation; it does not focus a live one.
- **Cross-session messaging.** **[Verified-primary]** Claude Code sessions, including those the Desktop runs, can bind an inbox socket, and "Claude Code tells B's Claude that the message came from another session, not from you". It cannot answer a permission prompt. https://code.claude.com/docs/en/cross-session-messaging.md. [Inferred] It is the wrong voice for the user's own reply, and a held message in a Desktop session waits for a dialog the app cannot show; kept off, as D-66 decided.
- **Remote Control.** **[Verified-primary]** The vendor's own way to reply without being at the app is Remote Control from claude.ai/code or the phone. That is the user's own sign-in in another Anthropic client, not a route for AI Juggler.

### 1.5 Terms
- **[Verified-primary]** The consumer terms restrict access by "automated or non-human means" except with an API key or where the vendor permits it. https://www.anthropic.com/legal/consumer-terms (quoted from claude-code.md §7).
- D-105's reading applies: opening, reopening and focusing the user's own app is using it. Typing into it or driving its window is the line, so no reply route is built into the Claude app.

## 2. OpenCode desktop

- **[Verified-primary]** Bundle id `ai.opencode.desktop`; beta `ai.opencode.desktop.beta`; dev `ai.opencode.desktop.dev`; product names `OpenCode`, `OpenCode Beta`, `OpenCode Dev`; Electron. Source: `packages/desktop/electron-builder.config.ts`, branch `dev`. https://github.com/anomalyco/opencode
- Not installed here, so no `Info.plist` was read and no version is known.
- **Profiles.** **[Verified-primary]** The app starts the OpenCode server itself as a helper process, sets a random `OPENCODE_SERVER_PASSWORD`, and leaves `XDG_DATA_HOME` alone, so the app and the CLI share `~/.local/share/opencode` (opencode.md §2). **[Inferred]** Electron's `--user-data-dir` should give a separate window state, but the sessions and sign-in stay in the shared XDG folder, so a second profile does not separate accounts unless the XDG variables are moved too. Not documented; live check.
- **Sessions.** **[Verified-primary]** One SQLite file, `opencode.db`, shared with the CLI (opencode.md §4). Readable without the app.
- **Routes.**
  - **[Verified-primary]** URL scheme `opencode://`, registered by beta and prod builds at start; the only link the source tests are `opencode://console/authorized?window=...` (a sign-in return). The vendor documents no route that opens a session. Links reach one window through a queue in the app. https://github.com/anomalyco/opencode (`deep-links`, `lifecycle`). I could not read the page-side handler (the code search was rate-limited), so a session or folder link is **unknown**, not absent.
  - **[Verified-primary]** The app can point at another server (a "default server URL" setting, opencode.md §2) and takes a second launch as a "second instance" that forwards links to the first window.
  - **[Verified-primary]** The helper server's password is random and held in the app's own process environment, so AI Juggler must not attach to it (reading another process's environment is off-limits). `opencode serve` on its own is documented, with a spec at `/doc`, events, and a prompt route (opencode.md §4).
  - **[Inferred]** The sanctioned reply route is the CLI adapter's: AI Juggler starts its own `opencode serve` for the account, points the user's app at it only if the user sets it, and the app shows the same sessions because the database is shared. Whether the open window refreshes a session written by another server is unknown; live check.
- **AppleScript.** Unknown (app not installed). **[Inferred]** none; Electron apps rarely ship one.
- **Terms.** **[Verified-primary]** The terms bar a process that "automatically or programmatically extracts data or Output". https://opencode.ai/legal/terms-of-service (opencode.md §7). The client is MIT; opening and focusing it is ordinary use.

## 3. Kimi Code desktop

- **[Verified-primary]** Official, closed source; macOS, Windows, Linux; 1.3.1 is the newest changelog entry read. Bundle id and app name on disk are **not published** anywhere found. https://www.kimi.com/code/docs/en/kimi-code-desktop/changelog.html
- **Where data lives.** **[Verified-primary]** Settings, sign-in, sessions and logs in `~/.kimi-code/` (shared with the CLI); the app's own state in `~/Library/Application Support/kimi-code-app/`. https://www.kimi.com/code/docs/en/kimi-code-desktop/settings-and-extensions.html
- **Profiles.** Whether the app honours `KIMI_CODE_HOME`, and any multiple-account support, are **not documented**. [Inferred] It is Electron-like (an `-app` support folder, an updater cache), so `--user-data-dir` may work for window state, but the sign-in sits in the shared home. Live check with an installed copy.
- **Routes.**
  - **[Verified-primary]** `kimi app [path]` is a thin wrapper. Its source (`apps/kimi-code/src/cli/sub/app.ts`, `MoonshotAI/kimi-code`) opens `kimi-code://open?root=<folder>&new=1` with the system's opener. "An existing workspace opens a new draft instead of restoring its previous session." So the link opens a **new chat in a folder**, not an existing session. https://moonshotai.github.io/kimi-code/en/reference/kimi-command.md
  - **[Verified-primary]** Remote Control (1.1.0): "control the local Agent from another device via a link or QR code". That is the user's own other device, not a route for AI Juggler.
  - **[Verified-primary]** The changelog mentions a "local embedded server" (a 1.0.3 security fix) but no port, path or token. Its own sessions list adds an "occupancy prompt for a session already open in another client" (1.1.1). **[Inferred]** The app is a client of an engine that a second client (the CLI, `kimi web`) can also attach to; D-106's `kimi web` usage route is already the sanctioned local server.
  - No sessions link, no AppleScript, no reply route found. The extension surface (plugins, skills, MCP in `mcp.json`, hooks in `config.toml`) feeds the agent, not the user's turn.
- **Terms.** **[Verified-primary]** Kimi Code subscriptions are "for interactive use only" and bar non-interactive automation (kimi.md §7, D-105). A typed reply into the user's own window is interactive use, but driving it from another program is not documented as allowed; no route is built.

## 4. Hermes desktop

- **[Verified-primary]** Hermes (Nous Research) Desktop: Electron, Apple Silicon only. The macOS bundle id is `com.nousresearch.hermes`, from the docs' permission-reset commands. Install path and version are not stated; not installed here. https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/desktop.md
- **Profiles.** **[Verified-primary]** Desktop follows `HERMES_HOME` (default `~/.hermes`) and each profile has its own config, skills and sessions; `hermes -p <profile> webapp` opens the browser version on one profile. No documented flag or launcher binds one Desktop window to one home, and the `hermes desktop` command has no `--user-data-dir` (flags are `--hermes-root`, `--cwd`, and build options). **[Inferred]** Launching the app with `HERMES_HOME` set in its environment is the likely route; live check.
- **Sessions.** **[Verified-primary]** In `state.db` under the home; the profile's gateway is the only writer and Desktop, CLI and TUI are clients (hermes.md §4, §10.3).
- **Routes.** **[Verified-primary]**
  - `hermes://skill/install?identifier=...` installs a skill after the user confirms. The only scheme the docs name; it must **not** be used by AI Juggler (installing is a change to the user's setup).
  - Desktop talks to the gateway over `/api/ws` with one-use tickets; `hermes serve` (port 9119 in the docs) is the backend; the browser version is at `127.0.0.1:9120`; `HERMES_DESKTOP_REMOTE_URL` points Desktop at a remote backend.
  - The gateway, ACP, the TUI gateway and the API server on `127.0.0.1:8642` are the sanctioned ways to read and answer a session. They belong to the CLI adapter (hermes.md §10.5–10.6). A reply sent there appears in Desktop because the gateway is the one writer.
  - **Desktop plugins** (`$HERMES_HOME/desktop-plugins/<id>/plugin.js`) "run with the app's full authority" and are not sandboxed. AI Juggler never installs one.
  - No AppleScript was mentioned.
- **Terms.** **[Verified-primary]** The Nous Portal terms bar access "by means of programs that send them automatic enquiries or requests" and allow one account per person (hermes.md §7, §10.7). Running the user's own Hermes is the product's purpose; rotating accounts is not.

## 5. What each route allows (actions table)

Legend: Yes = route found and sound; Maybe = found but gated, undocumented or untested live; No = none found (track-only for that part).

| Action | Claude app | OpenCode desktop | Kimi Code desktop | Hermes desktop |
|---|---|---|---|---|
| Detect by bundle id | Yes | Yes (id from source; check on a real install) | No (id unknown; detect by folder `kimi-code-app`) | Yes (id from docs; check on a real install) |
| Open for the right account | Yes (`--user-data-dir` plus `CLAUDE_CONFIG_DIR`) | Maybe (shared data folder; XDG variables) | Maybe (shared home) | Maybe (`HERMES_HOME` in the launch environment) |
| Keep alive | Yes (relaunch the same profile) | Yes (the app; a second launch hands over) | Yes (open the app) | Yes (app; its gateway outlives it) |
| Focus the right window | Yes (pid of the profile; the exact window by Accessibility if allowed) | Yes (app focus); exact window Maybe | Yes (app focus) | Yes (app focus) |
| Open one given session | Maybe (`claude://claude.ai/epitaxy/local_<id>`, `continue?session=`) | Maybe (route unknown) | No (new chat in a folder only) | No found |
| New chat in a folder | Yes (`claude://code/new?folder=`, asks the user) | Maybe | Yes (`kimi-code://open?root=...&new=1`) | No found |
| List sessions | Yes (its records) | Yes (`opencode.db`) | Yes (shared `~/.kimi-code`, per the CLI adapter) | Yes (`state.db` or the gateway) |
| Read a session | Yes (the CLI conversation file) | Yes (database) | Yes (shared home) | Yes (database or gateway) |
| Reply from AI Juggler | No (D-66; the inbox socket is the wrong voice) | Maybe, through the CLI's own server, not the window | No | Maybe, through the gateway (the CLI adapter's route) |

## 6. The other three desktop apps (cross-references, nothing redone)

| App | Bundle id (where read) | What is settled | Still open for the desktop |
|---|---|---|---|
| Cursor | `com.todesktop.230313mzl4w4u92` ([cursor.md](cursor.md) §1) | Separate sign-in per `--user-data-dir`; sessions in `~/.cursor` (shared); hidden `cursor desktop` bridge reported once ([cursor.md](cursor.md) second pass §3) | Whether the bridge exists in the installed build and takes a reply: needs the real app |
| Kiro | `dev.kiro.desktop`, inferred ([kiro.md](kiro.md) §1) | The CLI's own app is retired in 3.0; the IDE is a VS Code fork | IDE isolation by `--user-data-dir`: live check ([kiro.md](kiro.md) gaps) |
| Antigravity | `com.google.antigravity`, IDE `com.google.antigravity-ide` ([antigravity.md](antigravity.md) update §1) | `agy acp` is sanctioned, on Workspace and API plans only (D-105 addendum) | `--user-data-dir` for the App: live step (e) |

All three are VS Code or Electron shells, so the same pattern as section 1 applies: a data-folder switch for a separate sign-in, open, keep alive and focus by pid, and the vendor's own CLI route for replies.

## 7. Track-only, and why

One line each; each page of AI Juggler says the same.
- **Claude app, reply into a session** (Code tab, Chat, Cowork): the only sanctioned ways are the user's other Anthropic clients (Remote Control), the plugin and permission-relay routes that D-66 dropped, and the inbox socket, which delivers in another session's voice. Open the exact session instead.
- **Claude app, Chat and Cowork sessions in the list:** they are not files on the Mac.
- **OpenCode desktop, per-account profile:** one shared data folder, no documented switch.
- **OpenCode desktop, session link:** the page-side handler was not read; treat as not available until a live check finds one.
- **Kimi Code desktop, session link and reply:** only "new chat in a folder" exists; no bundle id.
- **Hermes desktop, session link:** none documented; reply goes through the gateway, not the window.
- **All four, driving the window** (typing, clicking, the debugger port): excluded by D-66 and the vendors' terms.

## 8. Live-check needs (for AJ-E1612)

1. **Claude app (installed):** send `claude://code/continue?session=local_<id>` and the epitaxy link to a lab profile's pid and see the right session open; test with the "entry points" switch in both positions if the app has a setting; confirm a second launch with a new `--user-data-dir` stays separate.
2. **OpenCode desktop (not installed):** install in a lab home with the maintainer's yes; check bundle id, version, `--user-data-dir` plus XDG separation, whether an `opencode://` link opens a folder or session, and whether the window refreshes a session written by another server.
3. **Kimi Code desktop (not installed):** read its `Info.plist` for bundle id, check `kimi-code://` handling, whether `KIMI_CODE_HOME` or `--user-data-dir` separates the sign-in, and where the embedded server's record sits.
4. **Hermes desktop (not installed, Apple Silicon):** check bundle id, launch with `HERMES_HOME` for a lab home, and whether a gateway reply shows live in the window.
5. **Cursor, Kiro, Antigravity:** as in their own notes.
6. **Every release (D-66):** re-read the vendors' link pages and terms; re-read each app's `CFBundleURLTypes`.

## Search log

- `mdfind` by bundle id for all four apps; `ls /Applications`: only the Claude app is present (2026-11-15).
- The Claude app's `Info.plist` and extracted script files: searched for URL handlers, listeners, `user-data-dir`, `CLAUDE_*` variables and sockets. No scripting definition found.
- https://code.claude.com/docs/en/desktop (first 200,000 characters), cross-session-messaging, the support page on links; one WebSearch on the `claude://` scheme.
- OpenCode: `electron-builder.config.ts`, `constants.ts`, the desktop README, `deep-links` source, docs for the web and troubleshooting pages. GitHub code search stopped on its rate limit before the page-side link handler was read.
- Kimi: `kimi app` source and tests, the desktop changelog and settings pages, the `kimi` command reference.
- Hermes: the Desktop user guide (all of it), `apps/desktop/package.json` (no app id there; the build config is in a file not read).
