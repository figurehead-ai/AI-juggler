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
> includes a short live run on a spare account, which is the maintainer's
> step (AJ-E1612).
>
> **Law for every task here:**
> - **Pass the conformance kit (AJ-E1521)** before merging: isolation, no
>   credential copies, pinned env, sessions, and honest usage.
> - **Never hold vendor credentials.** Sign-in happens in the tool's own
>   flow. API keys live in that account's own tool settings, never in AI
>   Juggler (pending D-34).
> - **Respect each tool's terms.** Where a vendor bans wrappers or
>   automation, the tool is track-only, or the restriction is documented,
>   with the maintainer's ruling recorded.
> - **Never touch another tool's folders.** Grok reads `~/.claude`: set
>   `GROK_CLAUDE_*_ENABLED=0`. Hermes can import `~/.codex`: keep it away
>   from managed Codex homes.
> - **Detect by path and version,** never by binary name alone (`grok` is
>   also an unofficial CLI's name).

### AJ-E1601 Accounts on a compatible endpoint: OpenCode Go and Kimi on Claude Code and Codex | Status: TODO | Priority P0 | Effort S | Depends: AJ-E1521
**Goal**: Subscriptions that speak Claude Code's or Codex's protocol run through the adapters we already have.
**User story**: As a user with OpenCode Go and a Kimi membership, I add each as an account on Claude Code or Codex and juggle them like the rest.
**Requirements**:
1. An account kind, "Through another service", for Claude Code (`ANTHROPIC_BASE_URL`, plus the model) and for Codex (a `model_providers` block). Presets for OpenCode Go and Kimi follow their published guides.
2. The key SHALL be entered into that account's own tool settings, through the tool's own mechanism. AI Juggler never reads, copies or stores it (D-34, to be ruled).
3. Usage: OpenCode Go is labelled an estimate, from dollar shares per 5 hours and per week. Kimi reads its windows when the source allows; otherwise the usage is unknown.
**UI/UX reasoning**: One more choice in Add account's More, with the service's mark.
**Performance budget**: n/a.
**Quality gates**: the conformance kit; a guardrail that no key reaches AI Juggler's storage or logs.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1601/manifest.md`.
**Visual testing**: Add account's new choice and the account card.
**Files to touch**: `crates/juggler-adapters/src/claude_code.rs`, `codex.rs`, `FirstRun.tsx`.
**Research basis**: `opencode.md` §subscriptions; `kimi.md` §8.
**Handover note**: Blocked on D-34 for the key's home.

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

### AJ-E1603 Kimi Code CLI | Status: IN-PROGRESS (Req.1, 2 and 4 evidenced; Req.3's usage source awaits the maintainer's ruling; live check pending, AJ-E1612) | Priority P0 | Effort M | Depends: AJ-E1521
**Goal**: Kimi Code runs with separate accounts and real quota windows.
**User story**: As a Kimi member, I see my 5-hour, 7-day and monthly windows and juggle Kimi accounts.
**Requirements**:
1. Discover `kimi`, and isolate each account with `KIMI_CODE_HOME`. Never set `KIMI_CODE_IDENTITY_*`.
2. Sessions come from the documented layout and `session_index.jsonl`.
3. Usage comes from `kimi web`'s usage endpoint where it is available, labelled experimental.
4. Interactive use only on subscription accounts: no headless runs and no auto-restart, as the guidelines require.
**UI/UX reasoning**: Like Codex.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; a guardrail that headless use is refused on subscriptions.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1603/manifest.md`.
**Visual testing**: the tool view and usage.
**Files to touch**: `crates/juggler-adapters/src/kimi*.rs`.
**Research basis**: `kimi.md`.
**Handover note**: The device quota per home is unknown; the live check finds out.

### AJ-E1604 Grok Build CLI | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1521
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

### AJ-E1605 ZCode | Status: TODO | Priority P1 | Effort L | Depends: AJ-E1521
**Goal**: ZCode, promised in the README and SPEC, ships.
**User story**: As a GLM subscriber, I juggle ZCode with my other tools.
**Requirements**: as SPEC §4.3. The bundled engine runs with an isolated `HOME`. Sessions are read from its SQLite database. Credit burn is computed from `model_usage` and the published formula. Resume, and packs both ways (unvalidated until live).
**UI/UX reasoning**: Like Codex.
**Performance budget**: SQLite reads within the session budget.
**Quality gates**: the conformance kit; credit-formula fixtures.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1605/manifest.md`.
**Visual testing**: the tool view and usage.
**Files to touch**: `crates/juggler-adapters/src/zcode*.rs`.
**Research basis**: `zcode.md`; SPEC §4.3.
**Handover note**: Re-verify the research (it is from Stage 1) before building.

### AJ-E1606 Kiro CLI | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1521
**Goal**: Kiro CLI runs with separate profiles, within Kiro's terms.
**User story**: As a Kiro Pro user with work and personal profiles, I juggle both.
**Requirements**:
1. Before building: confirm that supervising the interactive CLI is allowed, given the pricing FAQ's ban on "third-party automation harnesses". Record the ruling, or ask Kiro.
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
1. Wait for the maintainer to confirm that Nous Research's Hermes Agent is the one meant, not Hermes IDE.
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
1. Wait for the maintainer to confirm Meta's `muse` is the one meant.
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

### AJ-E1609 Cursor CLI (conditional) | Status: TODO | Priority P2 | Effort L | Depends: AJ-E1521
**Goal**: Cursor's CLI runs where accounts can be kept apart and the terms allow it.
**User story**: As a Cursor user, I run Cursor agent sessions beside the rest.
**Requirements**:
1. A maintainer ruling on the Acceptable Use Policy ("automated or non-human means") first.
2. The goal is subscription accounts (legacy Pro, Pro+ and Ultra, and the X Subscription). Per-account isolation goes through a per-account `HOME`, or `CURSOR_API_KEY` for key accounts, chosen after a live test. Sessions and resume through `agent` with `--resume` and `stream-json`. Usage is unknown for individuals.
**UI/UX reasoning**: Like Codex; a note on the X Subscription pool shared with Grok.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1609/manifest.md`.
**Visual testing**: the tool view.
**Files to touch**: `crates/juggler-adapters/src/cursor*.rs`.
**Research basis**: `cursor.md`.
**Handover note**: If the ruling is no, this becomes track-only inside AJ-E1610.

### AJ-E1613 Antigravity on work plans | Status: TODO | Priority P2 | Effort L | Depends: AJ-E1521
**Goal**: Antigravity runs as a full tool for Workspace and Enterprise accounts, where the terms allow it, and stays track-only on Google AI consumer plans.
**User story**: As a user with an Antigravity work subscription, I juggle it like my other tools.
**Requirements**:
1. Before building: read the Workspace and Enterprise terms for Antigravity and record whether supervising `agy` is allowed. Consumer plans (Google AI Pro and Ultra) stay track-only, because the FAQ says wrapping isn't supported there.
2. The account kind is decided by the plan the account reports. Usage comes from `agy -p /usage --output-format json` (it spends no quota).
3. Isolation: no env var moves the CLI's home, so a per-account `HOME` is tested live first.
**UI/UX reasoning**: Consumer accounts show as track-only with one plain line explaining why.
**Performance budget**: as Codex.
**Quality gates**: the conformance kit; a guardrail that consumer accounts are never launched through AI Juggler.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1613/manifest.md`.
**Visual testing**: the tool view for both kinds of account.
**Files to touch**: `crates/juggler-adapters/src/antigravity*.rs`.
**Research basis**: `antigravity.md`.
**Handover note**: The maintainer pointed out (2026-10-21) that Muse, Cursor, Antigravity and Kiro all have subscriptions; the aim is subscription accounts wherever terms and isolation allow.

### AJ-E1610 Desktop apps, track-only | Status: TODO | Priority P1 | Effort M | Depends: AJ-E413, AJ-E414
**Goal**: Every AI desktop app on the Mac shows up in AI Juggler, so its windows can be found.
**User story**: As a user with the Claude app, Cursor, Kiro IDE and Antigravity open, I see them in This Mac and the rail, and jump to the right window.
**Requirements**:
1. Detect, by bundle id: the Claude app, Cursor, Kiro IDE, Antigravity and Antigravity IDE, OpenCode desktop, Kimi Code desktop and Hermes.
2. Track their windows and focus the right one (the existing desktop contract, AJ-E413–E415). Read their sessions only where they're stored in a documented place.
3. Never launch them with altered accounts, and never touch their sign-in.
**UI/UX reasoning**: The desktop path finally reaches users (ADR-0007's promise).
**Performance budget**: within This Mac's tick budget.
**Quality gates**: bundle-id fixtures; the read-only guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1610/manifest.md`.
**Visual testing**: This Mac and the rail with desktop apps.
**Files to touch**: `crates/juggler-adapters/src/desktop_*.rs`, `machine_health.rs`.
**Research basis**: all the research files above, desktop sections.
**Handover note**: Grok Bot runs in the cloud and is out of scope.

### AJ-E1611 Tool update checks | Status: TODO | Priority P1 | Effort S | Depends: —
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

### AJ-E1612 Live checks on spare accounts | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1601..AJ-E1609
**Goal**: Each new tool is proven live once, on accounts that don't matter.
**User story**: As the maintainer, I run a short script per tool on a spare account, and the evidence records what really happened.
**Requirements**: For each tool, a checklist covering: sign-in stays separate across two homes, a session starts and resumes, usage is read, and quota exhaustion is seen. The captures go to `docs/research/evidence/`.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the evidence standard (live, never with real users' secrets).
**Evidence/validation proof**: one capture per tool.
**Visual testing**: n/a.
**Files to touch**: `docs/research/evidence/`, `docs/internal/MAINTAINER-STEPS.md`.
**Research basis**: the evidence standard, rule 3.
**Handover note**: The maintainer's step: agents prepare the checklists; the maintainer runs them.
