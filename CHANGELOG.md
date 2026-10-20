# Changelog

All notable changes to AI Juggler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Cleanup says what it is in one line instead of "Tier 0 observe ·
  verified manual kills". Pause automatic cleanup and Cleanup settings
  sit behind ⋯, and Resume is on the "paused" banner. The machine filter
  only appears once another Mac is paired. Groups name the tool ("Claude
  Code"), and alerts point to "Cleanup".
- The app says **tool** where it said "Provider": History's search,
  Settings, pack secrets, the rail's filters, the session details and the
  backup's "not included" list.
- In a window narrower than 900 px, the rail folds to its icon column by
  itself and comes back when the window is wider. Your density setting
  doesn't change; ⌘B opens the list while the window is narrow.
- Home is also the morning triage. Sessions parked until a reset have
  their own section, with Resume once the reset is reached (they no
  longer read as "hit its limit"). "Pick up where you left off" lists
  what ended this week that you haven't marked reviewed. J/K or the
  arrows move between rows, Enter opens, A runs the row's action. Triage
  and Overview, which repeated Home, are gone from More.
- A calmer app, rebuilt around what you need first (decision D-25). The
  top bar is Home, Accounts, History and Settings; Usage, Fleet and
  Cleanup are under More, and Cleanup and Fleet show up by
  themselves when they have something for you. The rail is one list of
  your projects and their sessions: search, Needs you · Running · All,
  and the rest behind a Filters button that says when one is on. A row
  shows its state in words and adds a note only when it matters (low
  quota, parked, uncommitted files, a sign-in to check, a stale value).
- The app says **session** for what it used to call an Instance — in
  every screen, message and the guide (D-25). A session's conversation
  record is a "conversation". The code keeps its names.
- Home answers "what needs me?": sessions that need you, what's running,
  each account's runway, and where you left off.
- The session view opens on where the session runs (its branch or
  folder), its state, project, account and how long; Switch account… sits
  beside Stop, and the ranked list of accounts is one click (or ⌘⇧H)
  away instead of always open. Everything else is under Details. Announcements say what happened ("waiting for your reply"),
  not rule codes.
- Accounts shows each account's runway in words — the 5-hour window
  and the week, when each resets, and when this pace would run out — plus
  what runs on it and the best account for new work. Usage leads with the
  same runway and explains itself when nothing is set up yet.
- Add account finds what's already on this Mac and isn't in AI Juggler
  yet, the same way first run does, and goes straight to a fresh account
  when there's nothing new.
- When an account hits its limit, one decision: continue now on the best
  other account, park the session until the reset (it's offered back;
  nothing restarts on its own), or choose another.
- List density moved to Settings › Appearance, with each choice
  described.
- First run, when there are no accounts yet: what's already on this Mac
  (Claude Code in the terminal and the Claude desktop app's profiles),
  brought in as copies with one button; a sign-in list with "Sign in to
  all"; then the computer access this OS needs (macOS: Accessibility and
  Automation, Screen Recording), each with why, its live status and Allow.

- The project moved to its public home,
  [figurehead-ai/AI-juggler](https://github.com/figurehead-ai/AI-juggler),
  as a clean snapshot. Issue and PR numbers mentioned in earlier docs
  (`#8`, `#37`, …) refer to the pre-public development repository.
- Open-source setup: rewritten README, CONTRIBUTING (real dev setup and
  gates), SECURITY (private reporting, scope), new SUPPORT and
  GOVERNANCE, Dependabot, privacy reminders in the issue and PR templates.
- Dependencies brought current before launch: React 19, Vite 8 (Rolldown),
  TypeScript 7, Playwright 1.64.1, axe-core 4.14; in Rust, ed25519-dalek and
  x25519-dalek 3, chacha20poly1305 0.11, getrandom 0.4 and zstd 0.14 (keys,
  signatures and backup containers are byte-compatible: the known-answer
  tests pass unchanged).

### Fixed

- The "?" at the bottom of the session list now opens the guide to what
  the icons and words mean (it opened out of sight), in the list's own
  words.
- First run says where each Claude setup is open right now (the Claude
  app or a terminal) and calls the default ~/.claude "Default"; it no
  longer guesses "Terminal" for a folder the desktop app uses.
- An account that needs its sign-in shows on Home under Needs you, with
  Sign in right there, instead of replacing Home with a separate screen.

- Session rows are readable at the WCAG contrast minimum in both themes:
  their second line and time use the stronger grey, and ended sessions
  are shown by a quieter title instead of being faded.

- Release builds ride out a short outage at Apple's signing services: a
  missing timestamp or an unanswered notary or ticket request is retried
  (up to four tries); any other signing failure still stops the build.

- A session's resource history stopped updating after five minutes, so
  its sparkline froze and the "this uses a lot" check looked at old
  numbers. It now keeps the latest five minutes.
- When a session ended, a helper it left running could drop out of the
  snapshot that protects heavy helpers from automatic cleanup. The last
  live view is now kept until the session is fully finished.
- Project names in the top bar show as written ("ai-juggler", not
  "Ai-Juggler").

- The More menu opened underneath the page, so only its first item
  could be clicked; it now paints above everything and closes on Escape
  or a click elsewhere.

- Screen readers and voice control: buttons are now named by the words they
  show (WCAG 2.5.3). Instance rows, filter chips, project tiles, remote rows
  and switch targets read their visible text first; the rest of a row's
  details is its description, so nothing is lost.

### Added

- AI Juggler now watches the Mac itself, not only its sessions (E14,
  AJ-E1401): processor and memory use, macOS's own memory-pressure level,
  swap, free space on the disks your sessions use, power and thermal
  state, sampled every 2 seconds with five minutes of history. One plain
  summary — "Plenty of room", "Getting tight" or "Under pressure", with
  the reasons — is what Home, the launch advisor and the alerts will show.
  Old swap doesn't count against you; swap that keeps growing does. Memory
  in use is the figure Activity Monitor shows.
- AI Juggler now also measures the Claude tools it didn't start (E14,
  AJ-E1402/E1403): each Claude desktop app window and each `claude` in a
  terminal, over all the processes it runs, matched to your account by the
  folder it uses. They count in "what your AI tools use" and are always
  labelled as not started by AI Juggler. They are never offered stop or
  cleanup. A session is called heavy when it stays above 90 % of a core
  or 2 GB for a minute.
- Three update channels (decision D-20): **Nightly** (built from the latest
  code every day), **Beta** and **Stable** (each promoted with the
  maintainer's approval). Choose Stable or Beta in Settings › About &
  updates; moving to a steadier channel never installs an older version.
- Settings, organised as in the approved design: a section list beside
  every policy you own — Context packs & secrets, Cleanup (with a default
  for new Accounts, off unless you choose it), Accounts & renewal dates
  (with how early to remind you), Notifications (quiet hours for the
  hourly digest and which Accounts it includes), Fleet permissions, and
  Data.
- Editions that install side by side (decision D-23): **AI Juggler** for
  everyday work, **AI Juggler Nightly** for trying the newest build, and
  **AI Juggler Dev** for development (`just dev`). Each has its own data,
  Keychain items, Fleet port and icon, so none can change another's data.
- Derived workflows for the beta (E10):
  - an identity guard so an Instance never runs on the wrong account;
  - git worktree isolation for a second Instance in a project;
  - parking a quota-blocked Instance until its reset;
  - a morning summary from a timestamped status journal;
  - a Triage view;
  - History: transcript search, archive, read-only replay, and per-Account
    retention with a dry run;
  - backup and restore of AI Juggler's own data (ADR-0016);
  - announced, verified, user-initiated updates for release builds, with
    release and rollback workflows (docs/RELEASING.md).
- Quality-signals engine (AJ-E409): per-Instance tokens/min, error rate,
  context report, session cost from a versioned in-repo pricing table
  (`crates/juggler-core/src/quality/pricing.v1.json`, **pricing table v1**
  — captured 2026-10-13 from anthropic.com/pricing; future row changes bump
  the version and land here).
- Repository bootstrap: license (MIT), contribution guide, code of conduct,
  security policy, issue/PR templates, placeholder CI.
- Deep-dive research notes for Claude Code, Codex, ZCode, and the tooling
  landscape (`docs/research/`).
- Product proposal (`docs/PROPOSAL.md`) and technical specification
  (`docs/SPEC.md`) — published for review.
- CONTRIBUTING guidance: only final artifacts belong in the repo (no
  conversation logs or exploration transcripts).
- Machine health subsystem (spec §5): system + per-session resource
  monitoring, overload alerts, launch admission advisor, crash recovery,
  and optimization recommendations.
- Fleet / multi-machine architecture (spec §11): mDNS discovery, code
  pairing, gRPC over mutual TLS, telemetry/control sync with
  credentials-never-leave-the-machine invariant; fleet milestones M6/M7
  toward 1.0.
- Release & platform policy: macOS-first signed + notarized builds
  (Apple Developer via Vuxtra), strict semver with 0.x beta channel,
  stable channel at 1.0.
- Execution plan (docs/plan/EPICS.md): epics → tasks → acceptance
  criteria for M0 through 1.0, per-milestone validation gates with
  committed gate reports and user confirmation.
- Quality standards (docs/plan/QUALITY.md): engineering standards and
  the per-task Definition of Done applied to every requirement.
- Architecture decision records (docs/adr/): Tauri 2, adapter
  architecture, automation policy, fleet transport, MIT license.
- Autonomous build runbook (docs/plan/RUNBOOK.md): the operating
  contract for unattended build runs — PR flow, DoD, gate stops, hard
  rails.

### Added (M0-1)

- Cargo workspace with the seven `juggler-*` crates; workspace lint
  baseline (clippy pedantic subset, `unwrap/expect/panic/todo` warnings,
  `missing_docs` deny, `unsafe_code` forbid); pinned toolchain
  (1.99.0, MSRV 1.99); cargo-deny policy; `just` task runner
  (`just verify` = the CI gate, locally).

### Added (M0-2)

- CI pipeline: `verify` job (fmt, clippy `-D warnings`, tests, docs with
  `-D warnings`, cargo-deny, llvm-cov coverage artifact) plus a
  cross-platform build matrix (macOS arm64 + x86_64 cross-check, Linux,
  Windows); required status checks enforced on `main`.

### Added (M0-3)

- Domain model in `juggler-core`: typed identifiers, `Account`, `Profile`,
  `SessionRecord`, session statuses (incl. `QuotaBlocked`), usage snapshots
  with confidence levels, quota windows, transcript refs; typed
  `CoreError`.
- SQLite storage: WAL + foreign keys, forward-only versioned migrations
  (newer-schema databases rejected, corrupt files degrade to
  `NotADatabase`), repositories for accounts/profiles/sessions with
  cascade deletes.
- Crash-safe `atomic_write` helper.
- Property-based round-trip tests (proptest) for every entity plus
  migration, corruption, cascade, and atomic-write edge cases.

### Added (M0-4)

- `juggler-adapter`: the `HarnessAdapter` contract — capability flags,
  discovery reports with `verified_against` version pinning, launch
  specs (interactive/headless, account-aware), the read-only
  `SessionWatcher` event interface, and an id-keyed adapter registry.
- `juggler-adapters`: the scriptable `fake-harness` binary (JSONL
  transcripts, turns/stall/burn/exit fixtures, resume) and the
  `FakeHarnessAdapter` reference implementation with a polling watcher.
- End-to-end adapter test: discover → prepare account → launch spec →
  run → watch (discovered/appended/ended) → resume.

### Added (Gate V0)

- Gate V0 validation report (docs/qa/gate-M0.md): CI matrix, coverage
  baseline meeting QUALITY.md targets, functional acceptance evidence,
  quality audit with fixes applied in-PR, and carried-forward items.
- Coverage-closing tests: repository update/delete paths, launch-spec
  constructors, registry trait-object usage, adapter degrade paths.

### Added (M0-5)

- `juggler-supervisor`: PTY (interactive) and piped (headless) spawning
  with per-profile env overlays; lifecycle event bus
  (Started/Exited with signal detail); bounded output ring buffer
  (512 KiB/session); SIGTERM→SIGKILL stop ladder; process-tree
  descendant tracking via sysinfo ppid walks; append-only supervision
  journal with crash re-adoption (`adopt_orphans`, zombies treated as
  dead); children survive supervisor/app death by design.
- Integration tests: spawn/capture/stop, external SIGKILL → journaled
  exit, drop-survival + journal re-adoption, process-tree tracking,
  PTY output capture, unknown-session errors.

### Added (M1-1)

- Tauri 2 application shell (`juggler-app`): strict CSP (self only),
  minimal core capability, generated brand icon placeholder, `app_info`
  command + `app://ready` event, typed both sides via ts-rs (Rust is
  the single source for TS types).
- Webview UI scaffold (`ui/`): React 18 + TypeScript + Vite + Tailwind
  v4; dark-first design tokens; base primitives (Button/Card/Chip/
  StatCard/QuotaRing); inline SVG icons; app shell (rail nav, status
  bar, dashboard placeholder); typed bridge with browser fallback.
- CI: `ui` job (typecheck + build) as a required check; Tauri Linux
  system dependencies for cargo jobs.

### Added (M1-2)

- Claude Code adapter: PATH + installer-default + override binary
  discovery with `--version` verification against pinned releases
  (drift warns, never fails); isolated `CLAUDE_CONFIG_DIR` account
  preparation (0700 dirs, 0600 managed-marker, idempotent); metadata-only
  login detection (credentials-file presence + macOS namespaced keychain
  probe — never reads token contents); guided vendor-login launch spec;
  interactive/headless launch specs with settings-overlay env merging.

### Changed

- Handover model upgraded from summary packs to tiered session sync:
  native resume (T1), cross-account session transplant (T2), cross-harness
  transcript replay (T3); context packs remain as preview/export format (T4).
- Explicit automation policy in spec and proposal: monitoring, alerts,
  suggestions, and staging are automated; every state-changing action is
  executed by an explicit user click.

### Added (M1-5)

- Claude Code transcript parser: version-tolerant JSONL extraction
  (uuid/parentUuid/sessionId, roles, text content, token usage) with
  per-kind counters, malformed-line skip-and-count (never panics), and
  resume dedupe by message uuid (first occurrence wins).
- Committed anonymized fixture corpus: basic conversation with
  tool_use/tool_result, subagent sidechains, resumed-session duplicates,
  and malformed damage cases.
- notify-based transcript watcher: fs events wake the watcher; a bounded
  recursive walk is the self-healing diff source (lost/coalesced fs
  events recover on the next poll); emits Discovered/MessageAppended.
- Fuzz scaffold (`fuzz/`): `transcript_parse` target exercising parser +
  dedupe on arbitrary bytes (cargo-fuzz, nightly; outside the workspace).

### Added (M1-3)

- App backend: `AppState` (SQLite storage, adapter registry, journaling
  supervisor) with per-platform data-dir resolution; 13 typed Tauri
  commands (accounts, profiles, sessions, launch/stop, login state and
  copyable vendor-login command, dashboard stats); lifecycle forwarder
  persisting session status transitions; `Profile.name` (schema v2
  migration).
- Supervisor fix surfaced by integration testing: the PTY path now
  inherits the base environment and applies the profile overlay on top
  (spec §4 env semantics).
- Launcher UI (Profiles screen): account cards with harness glyph, plan
  and login chips; per-profile rows with model chips and one-click
  Launch; add-account and new-profile modals; guided-login modal with
  copyable isolated login command (external-terminal fallback) and
  re-check; empty states; dashboard shows live counts. Typed bridge with
  plain-browser design-preview mocks.
- Integration tests (unix): two accounts launch in parallel through the
  full app stack with complete config-dir isolation and per-account
  settings overlays; write containment verified across the sandbox
  (zero writes outside the app data dir).

### Added (M1-3b)

- Smart onboarding foundation (ADR-0006, spec §12.1): adapter-level
  `scan_environment` finds existing setups — the vendor default
  (`~/.claude`), `$CLAUDE_CONFIG_DIR` overrides (other manager tools),
  and `.claude*` home-pattern dirs (manual or AI-created) — with
  metadata-only login probes, dedup, and most-useful-first ordering.
- Config import: `import_account` byte-copies a discovered setup into a
  fresh managed dir (0700 + marker), skipping rebuildable caches, with a
  2 GiB fail-fast ceiling and a read-only guarantee toward the source
  (tree-snapshot tested). Keychain-locked logins honestly report the
  one-time re-login path; file-based logins carry over.
- Login aids capped at "we open things": per-adapter `login_url`, opened
  in the system browser via tauri-plugin-opener.
- App commands `environment_scan`, `account_import` (creates a default
  `Main` profile so imports are immediately launchable), and
  `login_open_web`; typed bridge entries with browser-preview mocks.
- Integration tests: scan classification/dedup/sorting, import
  correctness + source immutability + ceiling, and the command-level
  onboarding path (import → launch → session recorded).

### Added (M1-4)

- Embedded terminal (xterm.js + PTY streaming, issue #20): tabbed
  terminal dock at the bottom of the window — drag-resizable, collapsible
  with ⌘J, layout persisted. Live sessions and guided logins stream into
  it; closing a tab's panel and reopening replays the retained tail.
- Supervisor: PTY keyboard input (`write_input`), resize (`resize_pty`),
  live output taps (ring listeners, one forwarder per terminal), and
  exit probes (`is_finished`).
- App surface: `terminal_spawn_login` (vendor login in an isolated PTY),
  `terminal_attach` (idempotent streaming), `terminal_write/resize/kill/
  snapshot`; base64 chunk events batched at a 16 ms flush — the 60 fps
  UI budget. Guided login now runs fully in-app; the login modal also
  opens the vendor's sign-in page (claude.ai) or copies the command.
- Launch flows land in the dock: clicking Launch opens a terminal tab
  attached to the interactive session.
- Tests: PTY input/resize/tap round-trip; 20 concurrent streaming
  sessions fan-out without deadlock (M1-4 perf floor); app-level
  terminal cores (echo → snapshot replay → stop ladder); login spec
  env-scoping proof.

### Changed (design iteration 4 — instance-centric IA, ADR-0007)

- Reframed the product around what it manages — **instances of AI coding
  tools (CLI today, desktop next) working on tracked projects** — instead
  of a CLI-session board.
- Domain (schema v3): `sessions.kind` (`terminal` | `desktop`) and a
  first-class `projects` table keyed by canonical root; launches touch
  their project and link the session to it — the lineage that powers
  switch/hand-over.
- UI rebuilt to two modes: left rail = **Projects / Instances / Sessions**
  (project panel: who's on it now, account history, launch-with-another-
  account; instance panel: status, kind badge, open-terminal/stop);
  global menus top-right (Overview, Accounts, Usage, Fleet) with a
  primary Add action.
- `projects_list` command with active-instance and account-history
  rollup; ts-rs `ProjectInfo`.
- Tests: projects repo roundtrip, session kind/project-link persistence,
  and the juggling integration proof — two accounts, one shared
  directory, one tracked project with both accounts in its history and
  the live instance surfaced.

### Changed (design iteration 5 — workflow navigator)

- The left rail is now the workflow itself (ADR-0007): three tabs —
  Projects / Instances / Sessions — whose live list lives inside the
  rail (collapsing to icons with ⌘B), plus a New session action
  (⌘N) anchored in the navigator.
- Selecting anything loads its panel on the right; selecting a session
  or instance embeds its **terminal directly in the panel** — the
  bottom dock is gone. Panels cross-link: session → its project,
  project → its sessions, project → launch-with-another-account (the
  one-click juggle), all drill-in-place.
- New-session modal: profile picker + optional directory (launching
  into a directory creates/reuses its project), lands selected with
  its terminal open.
- Guided logins join the Sessions rail list under "Sign-ins" and open
  in the same panel; ended items keep replayable scrollback.

### Changed (design iteration 6 — rail polish: responsive by construction)

- Tabs reflow instead of clipping: labels truncate cleanly down to a
  threshold, then fade to icon-only with tooltips (rail widths
  208–360px); collapsed shows stacked icon tabs. Nothing overflows at
  any width.
- Drag-resize restored on the rail (hairline handle that lights up on
  hover/drag, persisted width; double-click collapses), with the
  layout audit-driven at default/narrow/collapsed states.
- Tighter design-system pass on the rail: 7px row rhythm, 12.5/10.5px
  type pair, Linear-style 2px accent selection bars, focus-visible
  rings, proper aria tablist semantics, tooltips only when labels are
  hidden.
- Thin overlay-style scrollbars across rail and panels.

### Added (M1-3b Phase C — onboarding wizard)

- The zero-knowledge add-account flow (ADR-0006, spec §12.1): Detect
  (auto environment scan, findings in plain language with origin and
  sign-in state) → Identity (plain-words purpose + plan tier, or
  one-tap import of a discovered setup) → Login (sign in inside AI
  Juggler / open claude.ai / copyable command / re-check, with the
  honest keychain note when import can't carry the login) → Summary
  (what was created, what was skipped, original untouched).
- First run with zero accounts opens the wizard automatically (once
  per explicit dismissal); Accounts → Add account uses the same flow.
- Replaces the old add-account form modal; fully reviewable in a
  plain browser via mocks.

### Added (scanner: other manager tools, verified against t3.codes)

- Environment scan now covers **other manager tools' profile roots**
  (new `manager-tool` origin): t3.codes' `ClaudeCodeProfiles/` verified
  on a real machine with five live profiles. Manager-tool findings say
  plainly that the tool and its sessions are never touched.
- Login honesty for pre-2.1.56 setups: when only the legacy shared
  keychain entry exists, manager-tool imports note that a one-time
  re-login inside AI Juggler is likely.
- Operational runner (ignored test) importing real profiles into the
  app's data dir; used to import the user's five t3.codes profiles
  read-only (originals manifest-verified untouched; live session
  transcripts kept growing in the source, proving t3 undisturbed).

### Added (Gate V1 prep)

- `docs/qa/gate-M1.md`: dogfood environment prepared with the five
  imported t3.codes accounts, per-account re-login expectations (shared
  legacy keychain), acceptance checklist, and manifest re-verification
  step.
- UI honesty pass: the header chip names only Claude Code (Codex/ZCode
  ship with their adapters); quota rings say usage tracking lands in M2.

### Fixed (Gate V1 dogfood findings)

- User-stopped sessions now record `stopped`, not `failed`: portable-pty
  surfaces our stop-ladder kills as plain non-zero exit codes, so the
  supervisor tracks stop intent (`was_stopped`) and the lifecycle
  forwarder maps it to the new `SessionStatus::Stopped` (schema-neutral
  wire value `stopped`; UI chip included).
- `dogfood_v1` operational runner: the agent-run mechanics dogfood
  (two imported t3 accounts in parallel on one project, real claude
  under PTY, streaming + stop + project lineage verified; evidence in
  docs/qa/gate-M1.md).

### Added (the golden path — from the experience review)

- **Single instance**: a second launch focuses the running app (no more
  ghost windows); startup reconciles stale "running" sessions whose
  processes died with the last app run (marked stopped — journal
  adoption keeps true survivors live).
- **Your real projects, day one**: `projects_backfill` parses every
  imported transcript (cwd read from transcript content, never guessed
  from encoded dir names) and surfaces the user's actual project
  history with per-account attribution — schema v4 (`history_json`).
  On this machine: 6 real projects (two app repos, a code-graph tool, t3
  worktrees…) with account history, idempotent, run at startup.
- **Get-ready home**: any account not signed in makes the app open on a
  checklist — one click per account opens Claude's own sign-in in an
  embedded terminal; progress "N of M ready"; skippable.
- **New session without typing**: recent projects (real history) +
  native macOS folder picker + drop-a-folder-anywhere; account list
  shows sign-in state before launch (unusable accounts disabled with
  the reason, not silent).
- **The juggle (v0)**: `session_handover` — moves a session's transcript
  to another account (byte copy, same project subpath) and resumes it
  there (`claude --resume <id>`), recording lineage (source
  HandedOver → continuation with native id + transcript ref). "Continue
  on another account" chips on every session panel.
- Login-state banner on sign-in terminals; `LaunchMode::Resume` in the
  adapter contract.

### Stage 1 — plan documents rewritten to the settled decisions (D-01..D-15)

- Decision log marked **Final** on maintainer instruction (2026-10-13);
  second-opinion review corpus committed (two-pass reports, competitive
  reviews, 107-task E-catalog with evidence contracts, 27 wireframes).
- SPEC rewritten where the decisions demanded it: §3 domain
  (Provider/Account/Instance vocabulary, Switch Graph, spawn ledger,
  pack v2, honesty states), §5 (ledger-owned verified cleanup, D-08
  tiers), §6 (two clocks, D-12), §7 (unlimited bidirectional
  Switching, hybrid door, switch-back resume+delta, per-pair fidelity
  badges), §9 (Inspector-primary UX, indicator grammar, Cleanup
  Centre), §11 (Fleet at launch, D-13), §12.1 (cross-Provider
  onboarding).
- PROPOSAL re-centered (Switch → See → Clean; D-09 tagline); EPICS
  resequenced with phase frame, slip rule, and expanded gate evidence.
- ADR-0008..0013 accepted (terminology, pack v2 + user-owned
  redaction, spawn ledger, Switch Graph, two clocks, Fleet at launch);
  ADR-0003's automation invariant gains its two recorded amendments.
- README truth-fixed (shipped vs specified vs labelled-beta table);
  docs/qa/beta-exit.md defines the eight beta-exit gates.
- Billing/renewal API research recorded (docs/research/billing-apis.md):
  no sanctioned public renewal endpoints found for any Provider —
  subscription clocks stay user-entered (D-12).
- AJ-E901..E910 DONE-EVIDENCED with per-task evidence manifests and
  the Stage-1 acceptance packet (docs/qa/evidence/AJ-E910/). No
  product code changed (scope proof in the packet).

### Stage 2 begins — E8 evidence machinery (AJ-E801/E802/E806) + D-16 themes

- **Evidence store (AJ-E801)**: `docs/qa/evidence/<task-id>/` with the
  manifest template (6 fields + redaction checklist), a generated
  `_index.md`, and `scripts/evidence_check.sh` (gate-doc link
  resolution, credential-pattern secret scan, index freshness) wired
  into `just verify` and the CI verify job.
- **Visual harness (AJ-E802)**: Playwright against the production
  build with seedable bridge fixtures — all 9 SessionStatus values,
  honest-empty and ready-home sets; capture matrix 4 viewports
  (incl. Tauri's 1024×680 minimum and 390×844) × 2 themes; 41
  captures; browsable generated gallery; byte-identical determinism
  self-test (pinned clock + reduced-motion); not-yet-existing states
  recorded as NOT CAPTURED with owning task IDs, never omitted.
- **Gates (AJ-E806)**: single gate vocabulary across justfile /
  QUALITY.md / CI (verify, ui, evidence now; visual/a11y/perf named
  with owners); seeded-failure proofs for unit and visual gates
  (captured red, then green after removal); a11y seeded failure
  recorded DEFERRED → AJ-E804.
- **D-16 dual theme implemented**: Light + Dark token layer (values
  only — token names unchanged), System/Light/Dark choice persisted,
  `⌘⇧L` cycles, flash-free init, WCAG-minded light palette; both
  themes captured in the gallery.

### E1 naming (Phase A + B1 + B2) — Providers/Accounts/Instances begin

- **AJ-E101**: ADR-0008 confirmed Accepted + Amendment A-1 recorded
  (identifier semantics: `SessionId` stays the *Instance* id; new
  `ConversationId` newtype wraps the *native* conversation id; wire
  and storage columns unchanged).
- **AJ-E102 (Phase A — UI copy)**: user-facing copy swept to the
  charter vocabulary ("provider:", "Launch settings", honest cwd
  placeholder); gallery re-captured green in both themes.
- **AJ-E103 (Phase B1 — domain aliases)**: `ProviderId = HarnessId`
  and `ProviderAdapter` additive aliases; zero wire change.
- **AJ-E104 (Phase B2 — Instance/Session split)**: `SessionRecord` →
  `InstanceRecord` (old name aliased one minor version);
  `native_id: Option<String>` → `conversation: Option<ConversationId>`
  (DB column and values untouched — proven by a raw-SQL v4-fixture
  round-trip test, user_version unchanged); `InstanceStatus` alias;
  `launch_settings()` accessor pending AJ-E105's field rename;
  `SessionInfo.conversation_id` exposes the conversation in the
  view-model; 83 tests green.

### E1 Phase B3 (AJ-E105) — Launch settings demotion

- `Profile` → `LaunchRecipe` canonically (`Profile`/`LaunchRecipeId`
  aliases retained one minor version); stored values untouched
  (alias round-trip test green).
- Accounts screen: per-Account "Start on <label>" primary (disabled
  with reason while signed out) + "Edit launch settings" section,
  collapsed by default, with the spec's helper copy; recipe rows show
  "Default" for the auto-created Main. No new window.alert.

### E1 Phase B4 (AJ-E106) — schema v5

- Forward-only V5 migration: `accounts.harness → provider_id`,
  `sessions → instances` (+ `native_id → conversation_id`,
  `profile_id → launch_recipe_id`), `profiles → launch_recipes` —
  pure `ALTER … RENAME` inside the per-version transaction;
  user_version 4→5 exactly once on open; zero stored values
  rewritten (proven by an in-place v4-file upgrade test with real
  values); downgrade still fails `SchemaTooNew`. Repositories query
  only the new names.

### E1 Phase B5 (AJ-E107) — bridge + generated-types rename

- DTOs: `ProviderInfo` / `LaunchRecipeInfo` / `InstanceInfo` canonical
  (`HarnessInfo`/`ProfileInfo`/`SessionInfo` deprecated aliases one
  version; `#[serde(alias = "harness")]` keeps old payloads
  deserialising). Wire fields: `provider`, `launch_recipe_id`,
  `conversation_id`.
- Tauri commands gain canonical names (`providers_list`,
  `launch_recipes_list`, `instances_list`) with legacy names registered
  as one-version aliases; arguments/returns unchanged.
- ts-rs regenerated (never hand-edited beyond the generated re-export
  alias files); bridge exposes canonical functions with `@deprecated`
  legacy aliases; all UI call sites on canonical names; gallery green
  end-to-end with the new wire fields.

### Renamed (E1, AJ-E101–E108) — terminology per ADR-0008

Old → new: harness → **Provider**; profile (as account) → **Account**;
profile (as launch config) → **Launch settings** (`LaunchRecipe`
internal); session (running process) → **Instance**; handover →
**Switch**. Stored data and wire IDs are unchanged; old databases
migrate automatically (schema v5). Deprecated aliases (Rust types,
serde fields, Tauri command names, bridge functions, TS types) remain
for one version. The terminology lint gate (`just terminology`) now
enforces the vocabulary in SPEC/PROPOSAL/EPICS and JSX copy.
