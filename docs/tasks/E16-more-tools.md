# E16 — More tools: OpenCode, Kimi, Grok, ZCode, Kiro, Hermes, Muse, Cursor and the desktop apps

> Source: the maintainer's list of 2026-10-21 (Kiro, Antigravity, Cursor,
> Grok, OpenCode, Kimi, Muse Code, Hermes; "both desktops and CLI for each,
> first research which of these are possible"), plus ZCode (SPEC §4.3) and
> tool update tracking (SPEC §10, EPICS M5-3).
>
> The research is in `docs/research/` (`opencode.md`, `kimi.md`,
> `grok.md`, `kiro.md`, `hermes.md`, `muse-code.md`, `cursor.md`,
> `antigravity.md`, `zcode.md`), labelled per the evidence standard and
> accessed 2026-10-21. Nothing was run live. Every adapter's final proof
> includes a short live run in a separate lab profile on the maintainer's
> existing accounts (D-107, AJ-E1612): agents drive it, the maintainer only
> signs each lab profile in.
>
> **Law for every task here:**
> - **Pass the conformance kit (AJ-E1521)** before merging: isolation, no
>   credential copies, pinned env, sessions, and honest usage.
> - **Never hold vendor sign-ins.** Sign-in happens in the tool's own
>   flow. The one exception is an account through another service
>   (AJ-E1601): its key is kept in the macOS Keychain and handed only to
>   that account's sessions (D-34).
> - **Respect each tool's terms.** Where a vendor bans wrappers or
>   automation, the tool is track-only, or the restriction is documented,
>   with the maintainer's ruling recorded.
> - **Never touch another tool's folders.** Grok reads `~/.claude`: set
>   `GROK_CLAUDE_*_ENABLED=0`. Hermes can import `~/.codex`: keep it away
>   from managed Codex homes.
> - **Detect by path and version,** never by binary name alone (`grok` is
>   also an unofficial CLI's name).

### AJ-E1601 Accounts on a compatible endpoint: OpenCode Go and Kimi on Claude Code and Codex | Status: DONE-EVIDENCED (live check pending, AJ-E1612) | Priority P0 | Effort S | Depends: AJ-E1521
**Goal**: Subscriptions that speak Claude Code's or Codex's protocol run through the adapters we already have.
**User story**: As a user with OpenCode Go and a Kimi membership, I add each as an account on Claude Code or Codex and juggle them like the rest.
**Requirements**:
1. An account kind, "Through another service", for Claude Code (`ANTHROPIC_BASE_URL`, plus the model) and for Codex (a `model_providers` block). Presets for OpenCode Go and Kimi follow their published guides.
2. The key SHALL be pasted once in Add account and kept in the macOS Keychain (D-34); AI Juggler hands it only to that account's sessions, as the environment variable the tool expects, and never writes it to argv, a file, a log, the index or a backup, never shows it again, and removes it with the account. The Privacy page's "no-proxy" line is reworded as D-34 says.
3. Usage: OpenCode Go is labelled an estimate, from dollar shares per 5 hours and per week. Kimi reads its windows when the source allows; otherwise the usage is unknown.
**UI/UX reasoning**: One more choice in Add account's More, with the service's mark.
**Performance budget**: n/a.
**Quality gates**: the conformance kit; a guardrail that no key reaches AI Juggler's storage or logs.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1601/manifest.md`.
**Visual testing**: Add account's new choice and the account card.
**Files to touch**: `crates/juggler-adapters/src/claude_code.rs`, `codex.rs`, `FirstRun.tsx`.
**Research basis**: `opencode.md` §subscriptions; `kimi.md` §8.
**Handover note**: D-34 ruled the Keychain (2026-10-24).

### AJ-E1602 OpenCode CLI | Status: DONE-EVIDENCED (live check pending, AJ-E1612) | Priority P0 | Effort M | Depends: AJ-E1521
**Goal**: OpenCode runs as a first-class tool with separate accounts.
**User story**: As an OpenCode user with several providers, I run its sessions beside Claude and Codex.
**Requirements**:
1. Discover `opencode` (v1 and v2, by version output) and existing data folders.
2. Isolate each account through the XDG folders (`XDG_DATA_HOME`, `XDG_CONFIG_HOME`) and `OPENCODE_CONFIG_DIR`, pinned after overrides.
3. Read sessions and their cost and tokens from `opencode.db`, read-only. Resume and continue them.
4. Use `opencode serve` (OpenAPI and events) only where it gives what the files don't. Always on 127.0.0.1, with a port per session.
5. Rules (`AGENTS.md`) and MCP join the sync (AJ-E1507).
**UI/UX reasoning**: The same as Codex: a tool card, accounts and sessions.
**Performance budget**: the session index within the Codex budget.
**Quality gates**: the conformance kit; fixtures for v1 and v2.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1602/manifest.md`.
**Visual testing**: the tool view, Add account and the session row.
**Files to touch**: `crates/juggler-adapters/src/opencode*.rs`, `state.rs` registry.
**Research basis**: `opencode.md`.
**Handover note**: The maintainer chooses v1, v2 or both; default both.

### AJ-E1603 Kimi Code CLI | Status: IN-PROGRESS (R1, R2 and R4 evidenced; R3 built 2026-11-13 under D-106 and evidenced on fixtures — limit hits keyless, the windows from `kimi web` attached or started briefly, labelled experimental; the monthly window's display and the website's privacy line left; live check pending, AJ-E1612) | Priority P0 | Effort M | Depends: AJ-E1521
**Goal**: Kimi Code runs with separate accounts and real quota windows.
**User story**: As a Kimi member, I see my 5-hour, 7-day and monthly windows and juggle Kimi accounts.
**Requirements**:
1. Discover `kimi`, and isolate each account with `KIMI_CODE_HOME`. Never set `KIMI_CODE_IDENTITY_*`.
2. Sessions come from the documented layout and `session_index.jsonl`.
3. Usage on by default (D-106): from an undocumented source that needs no key if the research pass finds one (sourced, fixture-tested, format-guarded), else `kimi web`'s usage endpoint on 127.0.0.1 with the home's `server.token`, read for that request only and never stored, logged, copied or sent elsewhere; labelled experimental; the Privacy page names the exception.
4. Interactive use only on subscription accounts: no headless runs and no auto-restart, as the guidelines require.
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; a guardrail that headless use is refused on subscriptions.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1603/manifest.md`.
**Visual testing**: the tool view and usage.
**Files to touch**: `crates/juggler-adapters/src/kimi*.rs`.
**Research basis**: `kimi.md`.
**Handover note**: The device quota per home is unknown; the live check finds out.

### AJ-E1604 Grok Build CLI | Status: IN-PROGRESS (Req.1–3 evidenced; Req.4's token reading awaits the live check of signals.json; live check pending, AJ-E1612) | Priority P1 | Effort M | Depends: AJ-E1521
**Goal**: Grok Build runs with separate accounts and never touches the user's Claude setup.
**User story**: As an X Subscription user, I run Grok sessions beside the rest.
**Requirements**:
1. Discover the official `grok` by path and version (not the unofficial CLI). Isolate with `GROK_HOME`.
2. Set `GROK_CLAUDE_*_ENABLED=0` (and the Cursor equivalents), so the user's Claude hooks and MCP servers don't run inside Grok.
3. Sessions from `~/.grok/sessions/<cwd>/<id>/` (`summary.json`, `signals.json`); resume with `--resume` and `-c`.
4. Usage: tokens are measured; the weekly pool is unknown. A 402 means the pool is exhausted.
**UI/UX reasoning**: Like Codex; the X Subscription note says the pool is shared with Cursor.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; the Claude-isolation guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1604/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/grok*.rs`.
**Research basis**: `grok.md`.
**Handover note**: —

### AJ-E1605 ZCode | Status: IN-PROGRESS (rounds 1–3 built 2026-11-13: ZCode.app ships no terminal UI, so round 3 runs it driven over its engine's own app-server on the app's own runtime; R1–R7 on fixtures and a stand-in app-server, limit hits from its store, a pack into ZCode by its own import proven live with no sign-in; sending a turn, the provider-headers reply and the reset's zone UNPROVEN until AJ-E1612) | Priority P1 | Effort L | Depends: AJ-E1521
**Goal**: ZCode, promised in the README and SPEC, ships.
**User story**: As a GLM subscriber, I juggle ZCode with my other tools.
**Requirements** (SPEC §4.3; numbered 2026-11-13 after re-checking the research on the installed app, desktop 3.15.3 / engine 0.16.9):
1. Discover ZCode by its app bundle (`ZCode.app`, `dev.zcode.app`, in `/Applications` or `~/Applications`): its engine `Contents/Resources/glm/zcode.cjs`, run by a Node.js the engine accepts (`zcode doctor`); the version from `zcode version`, never claimed verified.
2. Isolate each account in its own `.zcode` tree without moving `HOME` (a moved `HOME` would take the user's git identity and SSH keys away from ZCode's own shell tools): `ZCODE_STORAGE_DIR=<account>/.zcode` and `ZCODE_DATA_BASE_DIR=<account>` (its sign-in then lives at `<account>/.zcode/v2/credentials.json`). The default `~/.zcode` runs with both unset. Both variables are the engine's own (the beta build uses the first), not documented: a launch check confirms the engine wrote inside the account's tree, and the adapter refuses the account otherwise. The sign-in probe sees whether that file exists, never its contents.
3. Sessions from `<tree>/cli/db/db.sqlite`, read-only and never written beside (the write-ahead log joined while ZCode runs, the file opened `immutable` when it doesn't): `session` rows that are interactive, not archived and not a sub-agent's (`parent_id` empty), with title, folder and times; resumed with `--resume <sess_…>`, the folder's latest with `-c`; a running engine recognised by its own command line.
4. Usage **Computed**: `model_usage` tokens × the published credit formula (per-model multipliers, peak and off-peak), per session and account; the 5-hour and weekly pools are the plan's caps the user sets, never claimed read (no public endpoint).
5. Rules, skills and MCP join the sync (`AGENTS.md`, `skills/`, `cli/config.json` `mcp.servers`); bring-in uses a tree in place or makes a copy that never takes `v2/credentials.json`, the database, logs or rollouts.
6. Packs both ways with Claude Code and Codex through the handover engine, marked unvalidated until the live check (AJ-E1612).
7. ADR-0019: ZCode is data (its spec, its terms in AJ-E1518's register, its letter mark); no tool id outside its adapter; it passes the conformance kit (AJ-E1521); it shows in Add account, first run and its page.
**UI/UX reasoning**: Like Codex.
**Performance budget**: SQLite reads within the session budget.
**Quality gates**: the conformance kit; credit-formula fixtures; a fixture database without a write-ahead log opens and is left byte-for-byte unchanged.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1605/manifest.md`.
**Visual testing**: the tool view and usage.
**Files to touch**: `crates/juggler-adapters/src/zcode*.rs`.
**Research basis**: `zcode.md`; SPEC §4.3.
**Handover note**: Re-verify the research (it is from Stage 1) before building.

### AJ-E1606 Kiro CLI | Status: TODO (ruled, D-105) | Priority P1 | Effort M | Depends: AJ-E1521
**Goal**: Kiro CLI runs with separate profiles, within Kiro's terms.
**User story**: As a Kiro Pro user with work and personal profiles, I juggle both.
**Requirements**:
1. Ruled (D-105, 2026-11-13): full support on every plan; the vendor's wording on "third-party automation harnesses" is kept in `kiro.md` § Terms. Build against Kiro CLI V3 (`kiro-cli`; V2 behind `--v2`).
2. Isolate with `KIRO_HOME`, the documented way to keep work and personal apart. Sessions come through `kiro-cli acp` (list, load, fork), read-only. Usage is the monthly credit pool, with per-turn metering.
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1606/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/kiro*.rs`.
**Research basis**: `kiro.md`.
**Handover note**: Old Kiro CLI builds stop connecting on 2026-11-09; support V3 only.

### AJ-E1607 Hermes Agent | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1521
**Goal**: Hermes Agent runs with its own profiles, kept away from managed Codex and Claude homes.
**User story**: As a Nous Portal user, I juggle Hermes profiles.
**Requirements**:
1. Nous Research's Hermes Agent is the one meant (D-74, 2026-11-04).
2. Isolate with `HERMES_HOME` and profiles; usage from `hermes usage --json`; manage its gateway process.
3. Never let Hermes import or edit a managed `~/.codex` or Claude home (guardrail).
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; the cross-tool guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1607/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/hermes*.rs`.
**Research basis**: `hermes.md`.
**Handover note**: The Nous Portal allows one account per person (§5.1).

### AJ-E1608 Muse (Meta) CLI | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1521
**Goal**: Meta's `muse` CLI runs, API-key accounts first.
**User story**: As a Muse user, I see its 5-hour and weekly windows beside my other tools.
**Requirements**:
1. Meta's `muse` CLI is the one meant (D-74, 2026-11-04).
2. The goal is subscription accounts ($5–$200 plans), as for Claude Code and Codex. API-key accounts ship first; subscription accounts follow once a Keychain test shows sign-ins can be kept apart per account.
3. Sessions and usage through `muse serve` (JSON-RPC, MIT SDK) and `usage/read`.
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1608/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/muse*.rs`.
**Research basis**: `muse-code.md`.
**Handover note**: —

### AJ-E1609 Cursor CLI | Status: TODO (ruled, D-105) | Priority P2 | Effort L | Depends: AJ-E1521
**Goal**: Cursor's CLI runs where accounts can be kept apart and the terms allow it.
**User story**: As a Cursor user, I run Cursor agent sessions beside the rest.
**Requirements**:
1. Ruled (D-105, 2026-11-13): full support on every plan, subscription and key accounts alike; the Acceptable Use Policy's wording is kept in `cursor.md` § Terms.
2. The goal is subscription accounts (the plans Cursor sells now, under the X Subscription since 2026-10-12, and the legacy ones). Per-account isolation goes through a per-account `HOME`, or `CURSOR_API_KEY` for key accounts, chosen after a live test. Sessions and resume through `agent` with `--resume` and `stream-json`. Usage is unknown for individuals.
**UI/UX reasoning**: Like Codex; a note on the X Subscription pool shared with Grok.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1609/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/cursor*.rs`.
**Research basis**: `cursor.md`.
**Handover note**: D-105 ruled full support; no track-only fallback.

### AJ-E1613 Antigravity | Status: TODO (ruled, D-105) | Priority P2 | Effort L | Depends: AJ-E1521
**Goal**: Antigravity runs as a full tool on every plan: Google AI consumer plans, Workspace and Enterprise (D-105).
**User story**: As a user with an Antigravity work subscription, I juggle it like my other tools.
**Requirements**:
1. Ruled (D-105, 2026-11-13): full support on every plan, consumer ones included; the FAQ's wording is kept in `antigravity.md` § Terms.
2. The plan the account reports is shown, never used to limit it. Usage: `/usage` is an interactive command; a sanctioned non-interactive reading is to be found first (`agy -p /usage --output-format json` wasn't found documented, 2026-11-13).
3. Isolation: no documented variable moves the CLI's home (`~/.gemini/antigravity-cli/`), so a per-account `HOME` is tested live first.
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; a guardrail that no Antigravity token or session credential is ever read, copied or reused (the vendor's own concern, D-105).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1613/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/antigravity*.rs`.
**Research basis**: `antigravity.md`.
**Handover note**: The maintainer pointed out (2026-10-21) that Muse, Cursor, Antigravity and Kiro all have subscriptions; the aim is subscription accounts wherever terms and isolation allow.

### AJ-E1610 Desktop apps: open, keep alive, focus, and reply where a route exists | Status: TODO | Priority P1 | Effort M | Depends: AJ-E413, AJ-E414
**Goal**: Every AI desktop app on the Mac shows up in AI Juggler as an App, opens, stays alive and comes into focus like every other App, and takes replies and shows its sessions wherever a sanctioned route exists (D-105, D-66, D-106).
**User story**: As a user with the Claude app, Cursor, Kiro IDE and Antigravity open, I see them in This Mac and the rail, open or keep each one running, jump to the right window, and answer their sessions from AI Juggler where the app allows it.
**Requirements** (rewritten 2026-11-13 against D-105, D-66 and D-106; "track-only" was the old default):
1. Detect, by bundle id: the Claude app, Cursor, Kiro IDE, Antigravity and Antigravity IDE, OpenCode desktop, Kimi Code desktop and Hermes; each is an App (offered when seen running, added only by the user, D-84).
2. Open, Keep alive and focus, as every App (D-105: opening, reopening and focusing are using the app, not wrapping it): the existing desktop contract (AJ-E413–E415) and Keep alive; a profile per account only where the app itself supports one (an Electron app's `--user-data-dir`, for example), found by requirement 3's pass; the sign-in is never read, copied or touched. Kiro, Cursor and Antigravity's apps get the same full support as their CLIs (D-105).
3. Before anything is called track-only, an undocumented-route research pass per app (D-106's standing order), sourced in the research notes: where its sessions are stored, and any reply or read route — a local server, a URL scheme or deep link, IPC, an app-server or agent-server mode (as ZCode's engine has), an MCP or extension surface.
4. Sessions read, and replies offered, through every route requirement 3 finds sound (D-66: any sanctioned way to reply into a desktop app's sessions, through its adapter, re-checked every release); where none is found, the app is track-only for that part, and its page says why in one line.
**UI/UX reasoning**: The desktop path finally reaches users (ADR-0007's promise).
**Performance budget**: within This Mac's tick budget.
**Quality gates**: bundle-id fixtures; the read-only guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1610/manifest.md`.
**Visual testing**: This Mac and the rail with desktop apps.
**Files to touch**: `crates/juggler-adapters/src/desktop_*.rs`, `machine_health.rs`.
**Research basis**: all the research files above, desktop sections.
**Handover note**: Grok Bot runs in the cloud and is out of scope.

### AJ-E1611 Tool update checks | Status: DONE-EVIDENCED (the first live request is on the maintainer's checklist) | Priority P1 | Effort S | Depends: —
**Goal**: The user knows when a tool has a new version, and whether AI Juggler has verified it.
**User story**: As a user, I see "Claude Code 2.x is available; not yet verified with AI Juggler".
**Requirements**:
1. Record each tool's version. Check its release channel once a day and show the result on the tool's view.
2. Never update a tool. Show the user's update command instead (guardrail).
**UI/UX reasoning**: Calm: a line on the tool view, never a popup.
**Performance budget**: one request per tool per day.
**Quality gates**: fixtures per tool; the no-update guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1611/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/`, `commands.rs`.
**Research basis**: SPEC §10; EPICS M5-3.
**Handover note**: —

### AJ-E1612 Live checks in separate lab profiles | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1601..AJ-E1609
**Goal**: Each new tool is proven live once, on the maintainer's existing accounts in separate lab profiles (D-107).
**User story**: As the maintainer, I sign each tool's lab profile in once, all tools in one sitting; agents run the checklists and the evidence records what really happened.
**Requirements**: For each tool, a checklist covering: sign-in stays separate across two homes, a session starts and resumes, usage is read, and quota exhaustion is seen. The captures go to `docs/research/evidence/`.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the evidence standard (live, never with real users' secrets).
**Evidence/validation proof**: one capture per tool.
**Visual testing**: n/a.
**Files to touch**: `docs/research/evidence/`, `docs/internal/MAINTAINER-STEPS.md`.
**Research basis**: the evidence standard, rule 3.
**Handover note**: D-107: agents set up a separate profile or home per tool (`KIMI_CODE_HOME`, `ZCODE_*`, `KIRO_HOME`, a per-account `HOME`…) under a throwaway lab project in AI Juggler Dev, never the tool's real home; the maintainer signs them in, in one sitting; agents drive every step with guarded keystrokes, 1–2 tools at a time, reap every process. A tool not installed is listed with what installing it needs; installing waits for the maintainer's yes.
