# E26 — AI Juggler for Windows: a dedicated first release, then every release

> Source: the maintainer, 2026-10-29 (D-52): "Can you add in our global
> planner of all the features and future versions, special releases for
> both Windows and Linux separately." Earlier the same day: "we have a lot
> of specialized process monitoring, special desktop capabilities that work
> separately for each OS … I am not saying we will not build them but I
> feel they are massive and require their own projects and releases." And,
> settling the shape: "it is fine if its part of the same releases, but
> initial ones, because we have to actually build and setup VM and test it
> and build specialized capabilities for each, requires a dedicated release
> per OS." and "Once we make the initial release than every feature is
> built all the time for each OS."
> Research: `docs/research/cross-platform-readiness.md` (the audit of
> 2026-10-27, read at `41f6c8c3`): the inventory C1–C16 (§2), the seams
> (§3), what each tool stores per OS (§4), the ranked plan (§5), the
> minimum Windows build (§5.4) and the questions only a lab answers (§5.5,
> Q1–Q10). Rows are cited here as "audit 3.1"; line numbers in the audit
> are at `41f6c8c3`, and the lines cited below were re-read on
> `release/0.2` at `57bd760a` (2026-10-29).
> Sister epic: [E27 — AI Juggler for Linux](E27-linux-release.md).
>
> **Why a dedicated first release.** AI Juggler's value is OS-specific
> work: it watches every process an app or a session starts, stops whole
> process trees safely, finds the desktop apps and brings them forward,
> keeps the computer awake for overnight runs, and keeps sign-in secrets in
> the system's store. Each of those is a different set of system calls,
> rules and permissions on Windows (Job Objects, named pipes and their
> security descriptors, Credential Manager, foreground-window rules,
> `SetThreadExecutionState`, Authenticode and SmartScreen). Setting that up
> (a Windows VM and lab, the OS seams, the tests on Windows, a signed
> installer) is a project, so Windows gets **one dedicated release**: the
> bring-up, whose scope is this epic, ending in Windows' first Beta
> (AJ-E2615). Until it ships, a macOS release never waits for Windows work
> and never claims Windows support.
>
> **After the first release, Windows is in every release** (AJ-E2616). One
> version stream (D-20 stands), the same channels (Nightly, Beta, Stable)
> and the same promotion rules (D-24): every nightly builds, signs and
> publishes Windows with macOS, one promote moves every OS's build of a
> version together, and no OS lags behind. From then on every feature is
> built and tested for Windows all the time: a task is done only when it
> works on every shipped OS, or is marked unsupported there with the
> reason, on screen and in the docs (the Definition of Done,
> `docs/plan/QUALITY.md` §2).
>
> **What is already cross-platform** (by reading; nothing has run on
> Windows):
> - The core, the adapters, the storage, `sysinfo` (the one process
>   source), `portable-pty`, `notify`, `dirs`, `rfd` and the Tauri plugins
>   in use (opener, dialog, notification, global-shortcut, single-instance,
>   updater) (audit §3, "What is already cross-platform").
> - Since the audit: the OS seams crate `crates/juggler-platform/` (who
>   this process runs as, whose a file is, owner-only files, local
>   connections, the computer's and user's names, the app badge), with the
>   agent bridge, driven sessions and hand-over moved onto it (commit
>   `752066f1`). On Windows each of those seams refuses honestly today
>   (`crates/juggler-platform/src/lib.rs:12-18`).
> - Since the audit: a stop can no longer report "stopped" when the system
>   can't tell (`Liveness::Unknown`,
>   `crates/juggler-supervisor/src/tree.rs:673-684`, the non-Unix
>   `signal_pid` at `:923-924`); the platform guard lint
>   (`scripts/platform_lint.py`, allowlist `scripts/platform.allow`: 235
>   sites in 96 files on 2026-10-29, shrink-only); CI's `build-windows` runs
>   `cargo check --workspace --all-targets` and `cargo build --workspace`
>   (`.github/workflows/ci.yml:431-452`), compile only.
>
> **What is macOS-only today** (audit §2): stopping a whole tree (no Job
> Object, audit 3.3–3.6); program names (`claude.exe`, `.cmd` shims, `\`,
> audit 2.4–2.5, 4.4); the tool environment (no `USERPROFILE`,
> `SystemRoot`, `PATHEXT`, audit 4.2); `git` (audit 4.6); owner-only files
> and the local connection for the agent bridge (audit 5.1–5.5, 5.9–5.10);
> credentials (Keychain only, audit 6.1–6.4); the host-app model (`.app`
> bundles, audit 7.1–7.3); Go to it (audit 8.1–8.2); keep-awake, open at
> login and machine signals (audit 10.1–10.6); ports (audit 11.1); the
> Trash, icons and window chrome (audit 12.1–12.4); packaging, signing and
> the update feed (audit 14.1–14.5); and about 210 lines of Mac words in
> the UI (audit 16.2).
>
> **Settled by D-52: version numbers and channels.** Windows uses the
> same version stream and channels as macOS; there is no Windows version
> line, channel position or feed of its own. The dedicated first release is
> a release on that stream whose scope is the Windows bring-up, and Windows
> first reaches users on Beta. Each channel's one feed (`latest.json`) then
> carries a `windows-x86_64` entry beside the `darwin-*` ones, for the same
> version.
>
> **Still the maintainer's decision (AJ-E2601):**
>
> | Question | Options (consequence) |
> |---|---|
> | Packaging | NSIS `.exe` installer, MSI, MSIX / Microsoft Store, WinGet manifest (AJ-E2614) |
> | Signing | Authenticode with an OV or EV certificate, or a cloud signing service (AJ-E2614) |
> | Scope | x64 only first, or x64 and ARM64; Windows 10 and 11 or 11 only; WSL sessions out of the first release (audit §4: recommended no) |
> | Data folder | `%LOCALAPPDATA%` (local, never roams; the audit's recommendation) or `%APPDATA%` (Roaming, what `dirs::data_dir()` gives today); a move after the first release is a migration |
> | Order and timing | Windows first, Linux first (E27), or both together; and when the bring-up starts (settled by D-53: Windows' first release comes before 1.0, and 1.0 waits for it) |
>
> **Shared seams, built once.** Some seams are the same shape on both
> systems. Each is built once, in the task that reaches it first; the
> other epic's task adds only its OS's half and links back (the
> architecture standard: no duplication). Default owners (the OS that
> needs it most):
>
> | Seam (audit §3) | Default owner | Other half |
> |---|---|---|
> | `Home` / `AppDirs` / `ToolHome` (C1) | AJ-E2604 | AJ-E2704 (XDG) |
> | `ExeId` (C2) | AJ-E2605 | AJ-E2705 (15-character names) |
> | `ProcessControl` (C3) | AJ-E2606 (Job Object) | AJ-E2705 (cgroup escapes) |
> | `LaunchEnv` / `ToolLocator` (C4) | AJ-E2607 | AJ-E2706 |
> | A real `SecretStore` back end and an honest `ServiceKeys::store()` (C6) | AJ-E2609 | AJ-E2708 |
> | `HostId` / `InstalledApps` / `Desktop` capabilities (C7, C8) | AJ-E2611 | AJ-E2710 |
> | `OpenFiles` (C11) | AJ-E2710 (`/proc`) | AJ-E2612 |
> | The platform words and keys module (C16) | AJ-E2613 | AJ-E2712 |
> | The release descriptor `release/platforms.json` (C14) | AJ-E2614 | AJ-E2713 |
>
> If the other epic starts first, its task takes the shared part and this
> one records the forward link.
>
> **Law for every task here:**
> - **Behind the seams.** Windows code lives in `crates/juggler-platform/`,
>   the audited per-OS shims, or an adapter; never a `cfg(windows)` branch
>   in feature code. `scripts/platform.allow` only shrinks.
> - **Honest before complete.** A seam with no Windows implementation says
>   so in plain words ("isn't available on Windows yet"); it never reports
>   success, "gone" or "stopped" it can't prove.
> - **`unsafe_code = "forbid"` stays.** Win32 calls come through a vetted
>   wrapper crate (a dependency); a fourth carve-out beside
>   `juggler-scap-init`, `juggler-login-item` and `juggler-vm-stats` needs a
>   decision of its own (D-17, D-26, D-27).
> - **macOS doesn't change.** Every task here keeps macOS behaviour and its
>   gallery byte-for-byte, except copy that the platform words module now
>   draws.
> - **Real accounts, read-only.** The lab uses a throwaway Windows user and
>   lab tool profiles; no sign-ins, no Allow, no credential reads.
> - **Plain words, Windows words.** File Explorer, Recycle Bin, Task
>   Manager, Credential Manager, Settings, Ctrl; never a Mac word on
>   Windows, never an internal word on screen.
>
> Research links in "Research basis" that the audit did not already cite
> were not opened while writing this epic (2026-10-29); each task's first
> step re-reads them, and until then they are labelled **I**
> (`docs/research/EVIDENCE-STANDARD.md`).

## Step 0 — decide and see

### AJ-E2601 The Windows bring-up's decisions | Status: TODO | Priority P0 | Effort S | Depends: —
**Goal**: The maintainer's remaining choices for the Windows bring-up are recorded before its first release ships: packaging, signing, scope (architectures, Windows versions, WSL), the data folder, and which OS goes first and when. Version numbers and channels are settled by D-52: the same stream and channels as macOS.
**User story**: As the maintainer, I decide once, from one page with the consequence of each option, how AI Juggler for Windows is packaged, signed and scoped, so the agents building it never guess.
**Requirements**:
1. A decision page SHALL list each open question of the intro's table with its options, the consequence of each, one concrete example, and the tasks it unblocks.
2. The page SHALL restate D-52's settled part (one version stream, the same channels, one feed per channel, one promote for every OS, every shipped OS in the Definition of Done) and SHALL NOT reopen it.
3. The page SHALL state the default the agents recommend for each open question and label it as the agents', not the maintainer's.
4. Each answer SHALL be recorded as its own decision-log entry (the next free D-number) in the D-52 style, with the maintainer's words.
5. The data-folder answer (`%LOCALAPPDATA%` or `%APPDATA%`, audit 1.1) SHALL be recorded before AJ-E2604 starts, because moving it after a release is a migration.
6. The WSL answer SHALL say plainly what the first release does with sessions that run inside WSL (audit §4: not tracked, recommended).
**UI/UX reasoning**: The serious-choices rule (consequence, example, inline tip, docs link) applied to the maintainer's own decisions; Nielsen #6 recognition rather than recall: every option on one page with its consequence.
**Performance budget**: one page; every question answerable in one sitting (≤ 6 questions).
**Quality gates**: `python3 scripts/check_task_counts.py` (the decision log's numbering); `./scripts/terminology_lint.sh`; the decision log's format (heading, ruling, what changed, why, risk, evidence).
**Evidence/validation proof**: the recorded entries in `docs/review/decision-log.md` and a manifest at `docs/qa/evidence/AJ-E2601/manifest.md` linking each to the tasks it unblocks; pass = every question of the table answered or explicitly deferred with a date.
**Visual testing**: None: a decision record, no screen. Its effect on screens is tested in the tasks it unblocks.
**Files to touch**: `docs/review/decision-log.md`, `docs/plan/EPICS.md` (the Windows milestone row), this file.
**Research basis**: audit §5.4 items 5–6, §4 (WSL), 1.1 (`crates/juggler-app/src/state.rs:584-585`, `dirs::data_dir()` is Roaming on Windows); `docs/RELEASING.md` (one version stream, D-20; D-24); D-52; https://v2.tauri.app/distribute/ (bundle formats, VP in the audit).
**Handover note**: Don't reopen version numbers or channels: D-52 settled them (one stream, D-20; memory: SemVer strictly). "Which goes first" is shared with AJ-E2701: record it once and link both.

### AJ-E2602 CI that runs the tests on Windows, not only compiles them | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: Windows runtime behaviour is exercised on every change: the crates that claim Windows support run their tests on `windows-latest`, and the tests that can't run there say why.
**User story**: As a contributor, I see a red check when my change breaks AI Juggler on Windows, before anyone installs it.
**Requirements**:
1. `build-windows` SHALL run `cargo test` for every crate that claims Windows support (a list in `release/platforms.json` or the workflow, kept in one place), in addition to today's `cargo check --workspace --all-targets` and build.
2. Each test module that cannot run on Windows SHALL say why at its top (`#![cfg(unix)]` plus a one-line reason), and a count of such modules SHALL be printed by the job and only shrink (an allowlist like `scripts/platform.allow`).
3. The fake tool programs (`src/bin/fake_*`) SHALL build and run on Windows, so supervisor and stop tests have a Windows subject.
4. `scripts/install_tauri_cli.sh` SHALL install the CLI on Windows (or the job installs it another way), so a Windows bundle can be built in CI (audit 15.2).
5. The developer story SHALL be written: which `just` recipes work in Git Bash, and a PowerShell equivalent for the ones that don't (audit 15.3).
6. The job SHALL be required on the bring-up's branch, and on every PR to `main` and `release/*` once Windows has shipped (AJ-E2616).
**UI/UX reasoning**: Contributor experience, Nielsen #9 (help users recognise, diagnose and recover from errors): a failure names the test and the OS, and the skip list says why each module is skipped.
**Performance budget**: the Windows job ≤ 25 min on `windows-latest` with a warm cache; the test step ≤ 12 min.
**Quality gates**: `actionlint` on the workflow; the job green on a `release/0.2`-based branch; `cargo test --workspace` unchanged on macOS and Linux; the skip-list count printed and checked.
**Evidence/validation proof**: two CI runs linked from `docs/qa/evidence/AJ-E2602/manifest.md`: one green, one deliberately failing (a Windows-only assertion), proving the job fails. Pass = both behave as stated; the skip list's count recorded.
**Visual testing**: None: CI only. The visual run on Windows is AJ-E2613's.
**Files to touch**: `.github/workflows/ci.yml` (`build-windows`, `:431-452`), `scripts/install_tauri_cli.sh` (`:15-20`), `justfile`, `crates/juggler-supervisor/tests/` (the `#![cfg(unix)]` modules), `crates/juggler-adapters/src/bin/` (`fake_claude.rs`, `fake_codex.rs`, `fake_harness.rs`, `fake_desktop.rs`), `CONTRIBUTING.md`.
**Research basis**: audit 15.1–15.4, §1 (test code assumes Unix: `juggler-supervisor/tests/{supervisor,collateral,fleet_identity,node_script_tool}.rs`), §5.1 step 3, §5.4 item 8; `.github/workflows/ci.yml:431-452` (today: check and build only).
**Handover note**: The first Windows test run will be small on purpose (most supervisor tests use `/bin/sh`); grow it with the seams, don't fake it with skips. Don't run the full macOS visual suite here: it's 25 minutes and runs in its own sweep.

### AJ-E2603 The Windows lab: a throwaway user, read-only, the open questions answered | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: The questions only a real Windows machine answers (audit §5.5: Q2, Q3, Q4, Q5, Q6, Q7, Q9, Q10) are answered with evidence before the seams that depend on them are built.
**User story**: As the agent building Windows support, I know where Claude's and Codex's desktop apps keep their sessions on Windows, how `.cmd` shims spawn under ConPTY, and whether notifications report a click, from a measured lab and not from guesses.
**Requirements**:
1. The lab SHALL run on a Windows 11 machine or VM under a throwaway Windows user with lab tool profiles only; never the maintainer's real accounts, never a sign-in, never a credential read (memory: real accounts read-only).
2. `docs/qa/windows-lab.md` SHALL hold the exact steps, the before/after listing of each tool's folder, and the cleanup; runnable by the maintainer or an agent the maintainer allows.
3. Each question SHALL get an answer labelled VP / U with the capture or command output that proves it: netstat2 listeners (Q2); `PATH` vs `Path` and `.cmd` under ConPTY (Q3); the Claude desktop data folder, process names and `claude-code-sessions` layout (Q4); the Codex desktop install folder, exe name, data folder and link scheme (Q5); the Claude status line under Git Bash and PowerShell (Q6); the notification click (Q7); how Claude names a Windows project folder (Q9); OpenCode, Kimi and Grok paths and credential protection (Q10).
4. Captures SHALL be redacted with a fail-closed tool and a sidecar, as on macOS (memory: captures checked by eye); the macOS redaction tool is Swift (`scripts/qa/redact_capture.swift`), so a Windows-capable one SHALL exist first or captures stay on the lab machine.
5. The audit's §4 table SHALL be updated with each answer and its date.
**UI/UX reasoning**: n/a to the product's screens; the lab follows the evidence standard's labels so no screen ever shows a guessed path as fact (Nielsen #1, honest status).
**Performance budget**: the lab ≤ 1 day of machine time; each question ≤ 30 min to re-run from the steps.
**Quality gates**: `docs/research/EVIDENCE-STANDARD.md` labels on every answer; `just evidence` (secret scan); `python3 scripts/qa/check_captures.py` (sidecars).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2603/lab-<date>.md` plus `manifest.md`; pass = all eight questions answered VP or explicitly U with the reason, zero secrets in the store, every capture checked.
**Visual testing**: The lab's captures of the desktop apps and of a Windows notification, redacted and checked by eye; not a product capture.
**Files to touch**: `docs/qa/windows-lab.md` (new), `docs/research/cross-platform-readiness.md` (§4, §5.5), `scripts/qa/` (a Windows-capable redaction step), `docs/qa/evidence/AJ-E2603/`.
**Research basis**: audit §5.5 Q1–Q10, §4 (the Windows column), 1.10 (`%APPDATA%\Claude`, VS), 4.7 (status line, U), 9.1 (notifications only for installed apps, VP); `docs/qa/processes-lab.md` and `docs/qa/runtime-lab.md` (the lab style).
**Handover note**: Q1 (does it compile) is CI's, AJ-E2602, not the lab's. The Codex desktop app may run sessions in WSL; record what `sysinfo` sees and doesn't, don't model WSL here. Reap every process the lab starts (memory: leftover processes are mine).

## Step 1 — the seams

### AJ-E2604 Where things live: one home, one app-data folder, each tool's home | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2601
**Goal**: AI Juggler, its adapters and the tools agree on which folder is "home" and where AI Juggler's own data lives on Windows, through one resolver.
**User story**: As someone on Windows with a redirected profile folder, AI Juggler finds my `.claude` and `.codex` where the tools themselves put them, and keeps its own database in my local app data so it never roams.
**Requirements**:
1. One `Home` resolver in `juggler-platform` SHALL replace the three in use (audit 1.2–1.4): `HOME` on Unix, `USERPROFILE` on Windows, never `dirs::home_dir()` where a tool deliberately differs (Grok, audit 1.4).
2. `AppDirs::data` SHALL be the folder AJ-E2601 chose (default `%LOCALAPPDATA%\<edition>`), per edition, so the three editions stay apart (D-23).
3. Adapters SHALL own `tool_home(tool) -> ToolHome { path, env_override }` and keep their constants (`~/.claude`, `~/.codex`, `~/.kimi-code`, `~/.grok` hold on Windows, audit §4).
4. The crash reporter's user-name scrub SHALL read the Windows user (`USERNAME`) as well as `USER` (audit 2.10, a privacy fix).
5. Every path shown on screen SHALL use the system's separators (`C:\Users\…`), shortened to `%USERPROFILE%\…` where the UI shortens `~/` today.
6. Guardrail tests: a redirected `USERPROFILE`, a Git Bash `HOME` that differs, a missing home; macOS paths byte-for-byte unchanged.
**UI/UX reasoning**: Windows app design guidelines (data and settings: per-user local app data for non-roaming state, https://learn.microsoft.com/windows/apps/design/app-settings/store-and-retrieve-app-data, **I**); Nielsen #2 match the real world: the paths the user sees in File Explorer.
**Performance budget**: resolving every home and app folder at start ≤ 1 ms; zero file-system writes during resolution.
**Quality gates**: `cargo test -p juggler-platform`; `cargo test -p juggler-adapters`; `just platform` (the allowlist shrinks by the removed sites); clippy `-D warnings` on Windows and macOS.
**Evidence/validation proof**: `cargo test -p juggler-platform --test home` on `windows-latest` and macOS (CI links), and the lab machine's first start creating its data under the chosen folder (`docs/qa/evidence/AJ-E2604/manifest.md`); pass = all guardrail cases green on both, the database where the decision says.
**Visual testing**: Settings › About and the account page's folder lines on Windows, 1280×800 and 1440×900, Light and Dark, the path shortened; compared against the macOS baseline for layout (Playwright, the preview's Windows mode, AJ-E2613).
**Files to touch**: `crates/juggler-platform/src/` (a new `dirs` module), `crates/juggler-app/src/state.rs` (`:584`), `crates/juggler-adapters/src/inventory_files.rs`, `crates/juggler-adapters/src/claude_code.rs`, `crates/juggler-adapter/src/user_path.rs`, `crates/juggler-app/src/crash.rs` (`:225`), `crates/juggler-app/src/machine_health.rs`, `crates/juggler-app/src/commands.rs`.
**Research basis**: audit 1.1–1.4, 1.10, 2.10, §3 C1, §5.2 step 3; https://docs.rs/dirs/latest/dirs/fn.data_dir.html (VP in the audit: Roaming); `crates/juggler-app/src/state.rs:584-585`; `crates/juggler-app/src/crash.rs:225`.
**Handover note**: Shared seam (default owner here; Linux adds XDG in AJ-E2704). Decide the folder before the first Windows build that anyone installs: a later move is a migration with a backup.

### AJ-E2605 Programs by name on Windows: `.exe`, `.cmd`, case and separators | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2602
**Goal**: One value type answers "which program is this?" for every process and path, so `claude.exe`, `codex.exe`, `juggler-agent.exe`, npm's `.cmd` shims and `C:\…` paths are recognised wherever `claude` and `/` are today.
**User story**: As someone running Claude Code in Windows Terminal, AI Juggler shows my session as a Claude Code session on This PC, with its helpers named by role, as it does on a Mac.
**Requirements**:
1. `ExeId` (in `juggler-platform`) SHALL be built from a path or a name, split on both separators, strip `.exe` / `.cmd` / `.bat` / `.ps1`, compare case-insensitively on Windows, strip `\\?\`, and answer `app_bundle()` (macOS only; `None` elsewhere) (audit §3 C2).
2. Every hand-rolled `rsplit('/')` and `== "claude"` site in the audit's rows SHALL use it: `is_cli_process` (`crates/juggler-adapters/src/claude_code.rs:1388`), `program_name` (`crates/juggler-app/src/features/processes/roles.rs:104`), the helper identity's `starts_with('/')` (audit 2.2), the `juggler-agent` comparisons (audit 2.4).
3. The shell list SHALL include `cmd`, `powershell` and `pwsh` on Windows (audit 2.4).
4. A recorded `exe` that the system won't show (no privilege) SHALL stay `Unverifiable` and refuse a stop, never guess (audit 2.1).
5. Tests: a table of real Windows names (from the AJ-E2603 lab) and the macOS and Linux tables, all through one function.
**UI/UX reasoning**: Nielsen #2 (the program's name as Task Manager shows it, without `.exe`); the Processes view's role words stay the same on every OS (UI consistency).
**Performance budget**: classifying 2,000 processes ≤ 5 ms per tick (release); no allocation per unchanged process.
**Quality gates**: `cargo test -p juggler-platform`, `-p juggler-adapters`, `-p juggler-app --lib features::processes`; `just platform` (`slash-split` and `app-bundle` counts fall); `just tool-literals`.
**Evidence/validation proof**: the name table test on `windows-latest`, and on the lab machine a Claude Code CLI session recognised in the Processes view (`docs/qa/evidence/AJ-E2605/manifest.md`); pass = 100 % of the table, the live session recognised.
**Visual testing**: The Processes page with Windows names (preview data from the lab's table), 1280×800, Light and Dark; the role words identical to the macOS baseline.
**Files to touch**: `crates/juggler-platform/src/` (an `exe` module), `crates/juggler-adapters/src/claude_code.rs`, `crates/juggler-adapters/src/codex_live.rs`, `crates/juggler-app/src/features/processes/roles.rs`, `crates/juggler-app/src/features/processes/stop.rs`, `crates/juggler-supervisor/src/live_stats.rs`, `crates/juggler-app/src/features/conversations/places.rs`, `crates/juggler-app/src/features/pools/cards.rs`.
**Research basis**: audit 2.1–2.6, §3 C2, §5.2 step 2; `crates/juggler-adapters/src/claude_code.rs:1388`; `crates/juggler-app/src/features/processes/roles.rs:104`; the `dunce` crate (Grok's choice for `\\?\`, VP in the audit).
**Handover note**: Shared seam (Linux adds 15-character `comm` names in AJ-E2705). Don't touch `.app` matching's macOS meaning; `app_bundle()` keeps it, behind the seam.

### AJ-E2606 Stopping a whole tree on Windows: Job Objects | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2602
**Goal**: A stop on Windows ends the session's whole tree, verified, like a process group on macOS; until it does, the stop says it couldn't stop the whole group.
**User story**: As someone who stops a session, its dev server and watchers end too, and AI Juggler tells me truthfully what it stopped.
**Requirements**:
1. `ProcessControl { terminate_tree, kill_tree, state, members }` SHALL be the seam (extending `signal_pid`, `crates/juggler-supervisor/src/tree.rs:900-924`), with `state` returning `Alive / Gone / Unknown` (today's `Liveness`).
2. On Windows every process AI Juggler starts (PTY and piped) SHALL be assigned at spawn to its own Job Object; terminate SHALL ask the root to close (a console control event where it applies), wait the grace, then `TerminateJobObject` (audit §3 C3).
3. `members` SHALL list the job's processes (the job's process list), so the ledger and the reconciler see the tree as on macOS (`pgid_members`, `tree.rs:1059`).
4. A process that escaped the job (started with breakaway) SHALL be reported as "left behind", never silently missed.
5. An adopted session (AI Juggler restarted) whose job can't be reopened SHALL say "can't stop the whole group from here" and offer stopping the root only, as a separate click.
6. The ladder's labels SHALL come from one place (audit 3.9).
7. Guardrail: zero false kills: no process outside the job is ever terminated (the E5 zero-false-kill proof, on Windows).
**UI/UX reasoning**: Nielsen #1 and #9: the stop sheet states what happened per process ("exited on request", "ended by Windows after 5 s"), never "stopped" for an unknown; Microsoft Fluent's guidance on confirmation dialogs (one clear primary action, https://learn.microsoft.com/windows/apps/design/controls/dialogs-and-flyouts/dialogs, **I**).
**Performance budget**: assigning a job at spawn ≤ 2 ms; a 50-process tree gone ≤ grace + 500 ms; `members` ≤ 2 ms per job.
**Quality gates**: `cargo test -p juggler-supervisor` on Windows (a fake tool that starts grandchildren); the collateral test ported (`tests/collateral.rs`); clippy; the unsafe rule (a wrapper crate, no workspace `unsafe`).
**Evidence/validation proof**: `cargo test -p juggler-supervisor --test collateral` and a new `--test job_tree` on `windows-latest`, plus the lab's live stop of a dev server tree (`docs/qa/evidence/AJ-E2606/manifest.md`); pass = 0 false kills in 100 runs, 100 % of the tree gone, the sheet's words matching each outcome.
**Visual testing**: The stop sheet's states on Windows (stopped, ended by Windows, couldn't stop the whole group, left behind), 1280×800, Light and Dark, WCAG 2.2 AA contrast through the axe run.
**Files to touch**: `crates/juggler-supervisor/src/tree.rs`, `crates/juggler-supervisor/src/group.rs` (the Job Object seam noted at `:21-53`), `crates/juggler-supervisor/src/supervisor.rs` (`send_terminate`, `:1564-1585`), `crates/juggler-supervisor/src/stop_guard.rs`, `crates/juggler-supervisor/src/piped.rs`, `crates/juggler-app/src/features/processes/stop.rs`, `crates/juggler-supervisor/Cargo.toml`.
**Research basis**: audit 3.1–3.9, §3 C3, §5.2 step 4, §5.4 items 1–2; https://learn.microsoft.com/windows/win32/procthread/job-objects (**I**); `crates/juggler-supervisor/src/group.rs:21-53` (the documented gap); ADR-0010 (verify-or-refuse).
**Handover note**: `portable-pty`'s killer ends the root only (audit 3.3); the PTY's child must be put in the job right after spawn, before it starts its own children: measure the race on the lab machine. A process may already be in a job (Windows Terminal, some IDEs); nested jobs are allowed on Windows 8 and later, but breakaway rules differ: test it.

### AJ-E2607 Finding and starting tools on Windows: the environment, shims, ConPTY and git | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2603, AJ-E2605
**Goal**: AI Juggler finds every supported tool however it was installed on Windows (native `.exe`, npm `.cmd` shim, WinGet, scoop), probes it with a working environment, starts it in a ConPTY, and finds `git`.
**User story**: As someone who installed Codex with npm on Windows, AI Juggler finds it, shows its version, and starts a session in it without my setting anything.
**Requirements**:
1. `LaunchEnv` SHALL carry a per-OS allowlist; Windows adds `USERPROFILE`, `SystemRoot`, `APPDATA`, `LOCALAPPDATA`, `PATHEXT`, `TEMP`, `TMP`, `ProgramFiles`, `ProgramFiles(x86)`, `ProgramData`, `ComSpec` (audit 4.2).
2. `ToolLocator` SHALL resolve with `PATHEXT`, `.cmd` shims (spawned through `cmd /c` only when the tool has no `.exe`, with arguments quoted for `cmd`), and the known folders per OS (`%APPDATA%\npm`, `%USERPROFILE%\.local\bin`, `%USERPROFILE%\.cargo\bin`, scoop shims, WinGet links) (audit 4.3–4.5).
3. `git` SHALL be found in `C:\Program Files\Git\cmd` and on `PATH`, and its missing message SHALL name Git for Windows, not Apple's Command Line Tools (audit 4.6).
4. The `PATH` / `Path` key SHALL be handled case-insensitively when building a child's environment (Q3).
5. The Claude status line wrapper SHALL have a Windows form, or quota capture SHALL say it is off on Windows (audit 4.7, Q6).
6. A link that needs a privilege (audit 4.8) SHALL fall back to a copy or say why, never fail silently.
7. Sessions inside WSL SHALL be stated as not tracked in the first release (AJ-E2601).
**UI/UX reasoning**: Nielsen #9: "Codex was installed with npm, but Windows couldn't run it: Node.js isn't on this PC's Path." instead of "not found"; Windows app design writing guidance (plain, second person, https://learn.microsoft.com/windows/apps/design/style/writing-style, **I**).
**Performance budget**: locating every supported tool ≤ 300 ms; each `--version` probe ≤ 2 s, all in parallel ≤ 5 s; zero shells spawned to find a tool.
**Quality gates**: `cargo test -p juggler-adapter` (`user_path`), `-p juggler-adapters` on Windows; `just tool-literals`; `just platform`; the no-bare-tools test.
**Evidence/validation proof**: on the lab machine, each tool installed each supported way is found and its version shown; a table in `docs/qa/evidence/AJ-E2607/manifest.md`; pass = every installed way found, every missing tool's message names the fix.
**Visual testing**: The Tools page and first run's tool check on Windows, with found, found-but-broken and missing states, 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-adapter/src/user_path.rs` (`ENV_ALLOWLIST`, `:44`), `crates/juggler-adapter/src/system_tools.rs` (`GIT_PLACES`, `:21`), `crates/juggler-adapters/src/claude_code.rs` (`find_on_path`, `:1092`), `crates/juggler-adapters/src/codex.rs`, `crates/juggler-adapters/src/opencode.rs`, `crates/juggler-adapters/src/kimi_code.rs`, `crates/juggler-adapters/src/grok_build.rs`, `crates/juggler-adapters/src/quota_ingest.rs` (`:177`), `crates/juggler-adapters/src/claude_environment.rs`, `crates/juggler-supervisor/src/supervisor.rs`, `crates/juggler-app/src/worktree.rs`.
**Research basis**: audit 4.1–4.9, §3 C4, §5.2 step 5, §5.4 item 3; §4 (Claude ships `claude.exe`; WinGet `Anthropic.ClaudeCode`, VP); https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session (ConPTY, **I**).
**Handover note**: Shared seam (Linux adds `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY` in AJ-E2706). `cmd /c` with user-controlled arguments is an injection risk: quote with the `cmd` rules, and prefer the shim's real target (`node <script>`) when the shim can be read.

### AJ-E2608 Private files and local connections on Windows: owner-only ACLs and named pipes | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2602
**Goal**: What is owner-only on macOS is owner-only on Windows: AI Juggler's data, the Fleet identity key, and the agent bridge's and driven sessions' local connections, through the existing seams.
**User story**: As someone sharing a Windows PC with another user, they can't read my AI Juggler data or talk to my running AI Juggler.
**Requirements**:
1. `private_fs` SHALL make folders and files owner-only on Windows with an explicit DACL (the current user and SYSTEM only, inheritance off) and verify it on read (`crates/juggler-platform/src/private_fs.rs:7,21,54`).
2. `user` and `file_trust` SHALL have a Windows `UserId` (the user's SID) and answer whose a file is (`crates/juggler-platform/src/file_trust.rs:8,16,26`).
3. `local_ipc` SHALL be a named pipe with a security descriptor for the current user only, `PIPE_REJECT_REMOTE_CLIENTS`, and a check of the client's identity on connect (`crates/juggler-platform/src/local_ipc.rs:14-33`).
4. The agent bridge, the hook client (`crates/juggler-agentbridge/src/client.rs:93,161`), driven sessions and the hand-over SHALL run over it; until then the UI says the agent tools are off on Windows.
5. The isolation check that reports "owner-only (0700)" (audit 5.10) SHALL check the ACL on Windows and say what it checked.
6. The Fleet identity key and registry (audit 5.9) SHALL be written through `private_fs`.
7. Guardrail tests: a second local user can't open a file or connect (lab), a remote client is refused, an inherited ACL is detected and fixed.
**UI/UX reasoning**: Nielsen #1 honest status: the Privacy page's "only you can read it" line is shown on Windows only when the ACL check passes; WCAG 2.2 AA for the warning when it fails.
**Performance budget**: a named-pipe round trip ≤ 5 ms p95 locally; creating a private folder ≤ 10 ms; the start-up checks ≤ 30 ms in total.
**Quality gates**: `cargo test -p juggler-platform` on Windows; `cargo test -p juggler-app --test trust`; `just platform` (`unix-api` and `unix-socket` counts fall); the security review checklist (`docs/research/engineering-standards.md`); clippy.
**Evidence/validation proof**: CI tests on `windows-latest`, and on the lab machine a second Windows user's attempts to read the data folder and connect to the pipe, both refused (`docs/qa/evidence/AJ-E2608/manifest.md`); pass = 0 successful reads or connections by the other user.
**Visual testing**: The Privacy page and Diagnostics' isolation lines on Windows, passed and failed states, 1280×800, Light and Dark, axe clean.
**Files to touch**: `crates/juggler-platform/src/private_fs.rs`, `local_ipc.rs`, `file_trust.rs`, `user.rs`, `crates/juggler-agentbridge/src/client.rs`, `crates/juggler-app/src/features/agent_bridge/`, `crates/juggler-app/src/features/runtime/launch.rs`, `crates/juggler-app/src/handover.rs`, `crates/juggler-link/src/identity.rs`, `crates/juggler-app/src/commands.rs`, `ui/src/lib/trust-claims.json`.
**Research basis**: audit 5.1–5.11, §3 C5, §5.2 step 1, §5.4 item 4; https://learn.microsoft.com/windows/win32/ipc/named-pipe-security-and-access-rights (**I**); `crates/juggler-platform/src/local_ipc.rs:14-20` (the plan already written in the seam).
**Handover note**: A named pipe's default security descriptor lets Everyone read in some cases: set it explicitly. Don't trust the pipe's name for identity (any user can create a pipe of that name first): create it with `FILE_FLAG_FIRST_PIPE_INSTANCE` and check the server side.

### AJ-E2609 Sign-in secrets in Windows Credential Manager | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2604
**Goal**: The secrets AI Juggler keeps (keys for accounts through another service, the Figurehead device token, the Cloud backup key) live in Windows Credential Manager, and every place that saves one is honest when it can't.
**User story**: As someone on Windows who adds an account through another service, my key is kept in Credential Manager, and if it can't be, AI Juggler says so before I type it.
**Requirements**:
1. A real `SecretStore` back end for Windows Credential Manager SHALL exist (generic credentials, target name per edition as the Keychain's service is, `crates/juggler-account/src/secrets.rs:23-46`).
2. `ServiceKeys::store()` SHALL return an error up front when no store works, instead of failing at `set` with the macOS text (audit 6.3, `crates/juggler-app/src/features/service_accounts/mod.rs:53`).
3. The Figurehead device token (`crates/juggler-app/src/figurehead.rs:77`) and the vault key (`crates/juggler-account/src/vault.rs`) SHALL use it.
4. Size limits SHALL be checked (Credential Manager caps a blob's size, **U** until measured) with a plain error.
5. Probes of the tools' own logins stay in the adapters and read files first: Claude and Codex keep them in files on Windows (audit 6.5–6.6).
6. All copy that says "Keychain" SHALL say "Credential Manager" on Windows (audit 6.7, AJ-E2613).
**UI/UX reasoning**: The serious-choices rule: the add-account sheet says where the key is kept and how to remove it ("Kept in Windows Credential Manager under AI Juggler. Remove it there or with Forget this account."); Nielsen #5 error prevention.
**Performance budget**: a read ≤ 50 ms; a write ≤ 100 ms; no read on the UI thread.
**Quality gates**: `cargo test -p juggler-account` on Windows (a test target name per run, removed after); `cargo test -p juggler-app --test service_accounts`; guardrails (no secret in logs, D-37); clippy.
**Evidence/validation proof**: CI tests on `windows-latest`, and on the lab machine a test key saved, read and forgotten with Credential Manager's list before and after (`docs/qa/evidence/AJ-E2609/manifest.md`); pass = round trip exact, nothing left behind, every refusal worded.
**Visual testing**: The add-account sheet and Settings › Figurehead account on Windows (saved, can't save), 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-account/src/secrets.rs`, `crates/juggler-account/src/vault.rs`, `crates/juggler-account/Cargo.toml` (`:25`), `crates/juggler-app/src/edition.rs`, `crates/juggler-app/src/lib.rs`, `crates/juggler-app/src/features/service_accounts/mod.rs`, `crates/juggler-app/src/figurehead.rs`.
**Research basis**: audit 6.1–6.7, §3 C6, §5.2 step 6; https://docs.rs/keyring/latest/keyring/ (VP weak in the audit); https://learn.microsoft.com/windows/win32/api/wincred/nf-wincred-credwritew (**I**); D-34 (keys in the Keychain).
**Handover note**: Shared seam (Linux's Secret Service in AJ-E2708). D-34 names the Keychain; this task needs a one-line decision note that "the system's credential store" is what D-34 means off macOS, or a superseding decision.

### AJ-E2610 Each tool's files on Windows | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2603, AJ-E2604, AJ-E2607
**Goal**: Every adapter reads and writes the right files on Windows (audit §4): Claude Code, Codex, OpenCode, Kimi Code and Grok Build, their logins, sessions and per-account copies, and switching works between them.
**User story**: As someone with two Claude accounts and Codex on Windows, I see both accounts, their sessions and their usage, and I can switch a conversation between them.
**Requirements**:
1. Claude Code: `%USERPROFILE%\.claude`, `.claude.json`, `.credentials.json` (login state by presence only), `CLAUDE_CONFIG_DIR` per account; the project folder name for a `C:\…` working folder as Claude makes it (Q9, audit 1.11).
2. Codex: `CODEX_HOME`, `auth.json`, the sandbox folders (`.sandbox`, `.sandbox-secrets` never read), the keyring setting reported honestly (audit 6.6).
3. OpenCode, Kimi Code and Grok Build: their Windows folders from §4, with the lab's answers for the unknown ones (Q10).
4. The Claude desktop app's sessions SHALL be read from the folder the lab found (Q4), or the desktop app's place SHALL say "not on Windows yet".
5. The switch and hand-over SHALL map `C:\` paths and never mix separators in a transcript (audit 1.11).
6. Fixtures: one Windows fixture tree per tool, from the lab's listing with contents replaced.
**UI/UX reasoning**: Nielsen #4 consistency: accounts, sessions and usage read the same as on macOS; a tool whose Windows support is partial says what is missing on its card, plainly.
**Performance budget**: the inventory scan of five tools ≤ 500 ms with 1,000 sessions on disk; zero writes outside each account's own folder.
**Quality gates**: `cargo test -p juggler-adapters` with the Windows fixtures (on every OS); `cargo test -p juggler-app --test guardrails` (zero writes to `%USERPROFILE%\.claude`); the switch tests with Windows paths.
**Evidence/validation proof**: the fixture tests, and in the lab one switch A→B→Codex→A with lab accounts (the 0.1 switching rule), recorded in `docs/qa/evidence/AJ-E2610/manifest.md`; pass = history and context kept at each step, zero writes outside the account folders.
**Visual testing**: Accounts, a session's page and Usage with the Windows fixtures, 1280×800 and 1440×900, Light and Dark, compared against macOS baselines for layout.
**Files to touch**: `crates/juggler-adapters/src/claude_code.rs`, `claude_live.rs` (`:226-239` area), `codex.rs`, `codex_environment.rs`, `opencode.rs`, `opencode_environment.rs`, `kimi_code.rs`, `grok_build.rs`, `crates/juggler-adapter/src/handover.rs`, `crates/juggler-adapters/tests/` (fixtures), `crates/juggler-app/src/features/quota/records.rs`.
**Research basis**: audit §4 (the whole table and its consequences), 1.5–1.11, 6.5–6.6; https://code.claude.com/docs/en/authentication.md and /claude-directory.md (VP in the audit); https://learn.chatgpt.com/docs/windows/windows-sandbox (VP in the audit).
**Handover note**: The four dot-folder tools need no path logic beyond the home resolver; the risk is the desktop apps and WSL. Never read a credential file's contents to tell a login state: presence and modification time only (D-37).

### AJ-E2611 Desktop apps, Go to it, and open or reveal on Windows | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2603, AJ-E2605
**Goal**: AI Juggler knows which desktop apps and terminals run on Windows and which session each holds, and "Go to it" does what Windows allows, honestly.
**User story**: As someone with sessions in Windows Terminal, VS Code and the Claude app, I see each under its app and one click brings that app forward, or tells me to click it in the taskbar.
**Requirements**:
1. The host model SHALL become `HostId { MacBundle, WinExe { exe, aumid }, LinuxDesktop }` with `InstalledApps` and `RunningApp` (audit §3 C7); `hosts.json` SHALL gain Windows entries (Windows Terminal, PowerShell, VS Code, Cursor, JetBrains, the Claude and ChatGPT apps).
2. `Desktop` SHALL report capabilities (`can_focus_window`, `can_open_link`, `can_find_tty`); a Windows implementation SHALL bring an app forward within the foreground rules (`AllowSetForegroundWindow` from the click), and otherwise say "Windows didn't bring it forward: click it in the taskbar" (audit 8.1–8.2).
3. Open and reveal SHALL go through `tauri-plugin-opener` (File Explorer); "Open Activity Monitor" SHALL become "Open Task Manager" (audit 8.5).
4. The installed-editors list SHALL read the Start Menu's shortcuts and the App Paths registry, not `/Applications` (audit 7.3, `crates/juggler-app/src/manage.rs:944`).
5. Held-session detection without `lsof` (audit 7.5) SHALL say "unknown" on Windows unless the `OpenFiles` seam (AJ-E2710) has a Windows answer.
6. The Codex link `codex://threads/{id}` SHALL be opened only if the lab found the scheme registered (Q5).
**UI/UX reasoning**: Windows app design guidelines on the taskbar and activation (an app may not steal focus; flash the taskbar button instead, https://learn.microsoft.com/windows/apps/design/shell/taskbar, **I**); the desktop-first rule: what is open, where, one click to it, Go to it optional.
**Performance budget**: the installed-apps scan ≤ 200 ms at start, cached, re-read on change; matching 2,000 processes to hosts ≤ 3 ms per tick; Go to it ≤ 300 ms to its outcome.
**Quality gates**: `cargo test -p juggler-adapter` (`live`), `-p juggler-app --lib features::conversations`; `just platform` (`applications` and `app-bundle` fall); the hosts.json schema test.
**Evidence/validation proof**: on the lab machine, a session in each of Windows Terminal, VS Code and the Claude app shown under its app, and Go to it's outcome per app recorded (`docs/qa/evidence/AJ-E2611/manifest.md`); pass = every running host matched, every Go to it either works or says why.
**Visual testing**: Home's Open now, a session's "where it runs" line and the Go to it outcome toast on Windows, 1280×800 and 1440×900, Light and Dark; app icons per AJ-E2613.
**Files to touch**: `crates/juggler-adapter/src/live.rs` (`HostApp`, `:148-180`), `crates/juggler-app/src/features/conversations/goto.rs` (`Desktop`, `:32`; `MacDesktop`, `:383`), `crates/juggler-app/src/features/conversations/commands.rs` (`:316,335`), `crates/juggler-app/src/features/conversations/hosts.json`, `crates/juggler-app/src/features/conversations/places.rs`, `crates/juggler-app/src/activation.rs`, `crates/juggler-app/src/focus.rs`, `crates/juggler-app/src/manage.rs`, `crates/juggler-app/src/features/processes/commands.rs`, `crates/juggler-adapters/src/claude_live.rs`, `crates/juggler-adapters/src/codex_live.rs`.
**Research basis**: audit 7.1–7.7, 8.1–8.10, §3 C7–C8, §5.2 step 7; https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-setforegroundwindow (the foreground rules, **I**); https://learn.microsoft.com/windows/win32/shell/appids (AUMID, **I**); D-36, D-38.
**Handover note**: Shared seam (Linux's `.desktop` half in AJ-E2710). The `Desktop` trait is wired unconditionally to `MacDesktop` today (`commands.rs:316,335`); choose the implementation through the platform seam, not a `cfg` in the command.

### AJ-E2612 System integration on Windows: notifications, the taskbar, open at login, keep awake, machine signals and ports | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2604; live part AJ-E2614
**Goal**: The background features that make AI Juggler worth keeping open work on Windows: notifications with a working click, the count on the taskbar button and in the tray, opening at login, keeping the PC awake while sessions work, memory and power signals, and the ports a session listens on.
**User story**: As someone running overnight sessions on Windows, the PC stays awake while they work, AI Juggler opens when I sign in, and a notification click takes me to the session that needs me.
**Requirements**:
1. Notifications SHALL use the installed app's AUMID (the plugin works only for installed apps, audit 9.1); a click SHALL open the notice's subject (Q7); the E25 rules hold (off until chosen, once per state, quiet hours).
2. The app badge seam SHALL draw the count as the taskbar button's overlay icon with an accessible description (`crates/juggler-platform/src/system.rs:41-51`, today `NotYet`).
3. The tray SHALL show the count and menu (`crates/juggler-app/src/features/notifications/tray.rs`), with a tooltip, in the notification area.
4. Open at login SHALL use the per-user Run key or the plugin's mechanism, shown and reversible in Settings (audit 10.1).
5. Keep awake SHALL hold `SetThreadExecutionState` (system required, not display) while sessions work and release it when they stop (audit 10.2; `crates/juggler-supervisor/src/awake.rs:165` is the stub).
6. Memory pressure, power (battery saver) and thermal SHALL be read where Windows reports them, else stay `None` without an alert (audit 10.3–10.6); disk names SHALL read `C:` (audit 10.7).
7. Ports SHALL use `netstat2` on Windows if the lab shows it works (Q2), else stay "ports unknown on this system" (audit 11.1, `crates/juggler-supervisor/src/ports.rs:66-105`).
**UI/UX reasoning**: Windows notification design guidance (https://learn.microsoft.com/windows/apps/design/shell/tiles-and-notifications/toast-ux-guidance, **I**: concise title, one action, nothing urgent without reason); Fluent's badge guidance for the overlay; Nielsen #3 user control (each is a setting, off where E25 says off).
**Performance budget**: a notification shown ≤ 500 ms after its state; the overlay redrawn ≤ 100 ms after a count change; the awake request ≤ 1 ms; port listing ≤ 50 ms per tick at 2,000 processes.
**Quality gates**: `cargo test -p juggler-app --lib features::notifications`, `--test notification_model`, `--test system_notifications`; `cargo test -p juggler-supervisor --lib awake`, `--test machine` on Windows; clippy.
**Evidence/validation proof**: on an installed lab build: a notification clicked through, the overlay count matching the bell for 10 changes, a 2-hour unattended session with the PC kept awake (`powercfg /requests` showing AI Juggler), opening at login after a sign-out (`docs/qa/evidence/AJ-E2612/manifest.md`); pass = all four observed, the count equal 10 of 10.
**Visual testing**: Settings › Notifications and Settings › Desktop on Windows (each toggle's on, off and unavailable states), 1280×800, Light and Dark; a redacted capture of a real toast and the taskbar overlay from the lab, checked by eye.
**Files to touch**: `crates/juggler-app/src/features/notifications/mod.rs` (`on_action`, `:581`), `tray.rs`, `crates/juggler-platform/src/system.rs`, `crates/juggler-login-item/src/lib.rs`, `crates/juggler-app/src/features/app/login_item.rs`, `crates/juggler-supervisor/src/awake.rs`, `crates/juggler-app/src/features/app/awake.rs`, `crates/juggler-supervisor/src/machine.rs`, `crates/juggler-app/src/machine_alerts.rs`, `crates/juggler-supervisor/src/ports.rs`, `crates/juggler-supervisor/Cargo.toml` (`netstat2`, `:32`).
**Research basis**: audit 9.1, 10.1–10.8, 11.1, §3 C9–C11, §5.2 step 8, §5.4 item 7; https://v2.tauri.app/plugin/notification/ (VP in the audit); https://v2.tauri.app/plugin/autostart/ (VP weak in the audit); https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate (**I**); https://learn.microsoft.com/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist3-setoverlayicon (**I**); the always-on rule (memory).
**Handover note**: Dev builds show PowerShell's name on toasts (audit 9.1): test only an installed build. `juggler-login-item` is an audited unsafe carve-out on macOS; a Windows Run key needs no unsafe, so keep it out of that crate's carve-out.

### AJ-E2613 Looks and reads like Windows: window chrome, icons, the Recycle Bin, words and keys | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: On Windows, AI Juggler looks like a Windows app (title bar, caption buttons, icons, scaling) and speaks Windows (This PC, File Explorer, Recycle Bin, Task Manager, Credential Manager, Ctrl shortcuts), from one platform words module, with macOS unchanged.
**User story**: As a Windows user, every shortcut shown works as written with Ctrl, every place named is one I can find in Windows, and the window behaves like my other apps (snap layouts, caption buttons).
**Requirements**:
1. A platform module in the UI (`modifierGlyph`, `fileManagerName`, `settingsName`, `credentialStoreName`, `trashName`, `taskManagerName`, `computerName`) and its Rust twin (the `cfg!` three-way helper of `not_answering_words`, `crates/juggler-app/src/lib.rs:1054`) SHALL draw every OS word (audit §3 C16, 16.1–16.3).
2. Shortcuts SHALL show as "Ctrl+N", "Ctrl+Shift+N", "Ctrl+," on Windows (`ui/src/lib/shortcuts.ts`, `ShortcutSheet.tsx`); no shortcut SHALL be Ctrl+Alt+<letter> on Windows, because AltGr layouts type characters with it: the notification centre's ⌥⌘N (`shortcuts.ts:107`) gets a Windows key chosen and recorded here; quit is Alt+F4 (the menu's Ctrl+Q stays as a menu item) (`crates/juggler-app/src/quit.rs`).
3. "This Mac" SHALL read "This PC" on Windows (the screen, the menu, the guide); the guide page gets a Windows version.
4. The window SHALL use a Windows configuration overlay: the system title bar with the UI's header below it, or a custom title bar that keeps caption buttons, snap layouts on hover of Maximise, and a drag region (audit 12.3).
5. App icons SHALL be read at run time from each app's executable (no icons in the repo, D-38 and the app-icons ruling); the letter fallback stays (audit 12.1).
6. Moving to the Recycle Bin SHALL use the `trash` crate on Windows (audit 12.2, `crates/juggler-app/src/manage.rs:497`), so removing an account with "move its folder to the Recycle Bin" works.
7. The icon pipeline for the Dev and Nightly editions SHALL not need Swift (audit 12.4, `justfile:308`).
8. Window mirroring stays "unavailable" on Windows (audit 13.1); the UI never offers it there.
**UI/UX reasoning**: Microsoft Fluent 2 and the Windows app design guidelines: title bar (https://learn.microsoft.com/windows/apps/develop/title-bar, **I**), keyboard accelerators (https://learn.microsoft.com/windows/apps/design/input/keyboard-accelerators, **I**: show the accelerator as the platform writes it), Segoe UI Variable (already in the font stack, audit 16.4); WCAG 2.2 AA (contrast, 2.4.7 focus visible, 2.5.8 target size) and Windows contrast themes (`forced-colors: active`).
**Performance budget**: the platform module adds ≤ 2 KB to the first-load bundle (lazy-load rule); an app icon read ≤ 20 ms, cached; no layout shift when the title bar draws.
**Quality gates**: `just platform` (`mac-words` falls file by file; the module is the only place OS words live); `python3 scripts/plain_words_lint.py`; vitest for the module and `ShortcutSheet`; `pnpm --dir ui build`; the a11y gate (axe) on the touched screens.
**Evidence/validation proof**: the preview's Windows mode gallery (every screen that showed a Mac word) and a Windows-machine run of the touched screens' Playwright specs in WebView2's engine (Chromium), stored under `docs/qa/evidence/AJ-E2613/`; pass = zero Mac words on Windows captures (a text scan of every capture's DOM), axe 0 serious or critical, every listed shortcut pressed and working in the lab.
**Visual testing**: Every screen with OS words in the preview's Windows mode at 1280×800 and 1440×900, Light, Dark and a Windows contrast theme (`forced-colors`), at 100 %, 150 % and 200 % display scaling on the lab machine; compared against the macOS baselines for layout; the macOS gallery unchanged (0 diffs).
**Files to touch**: `ui/src/lib/` (a new `platform.ts`), `ui/src/lib/shortcuts.ts`, `ui/src/components/ShortcutSheet.tsx`, `ui/src/lib/hotkey.ts`, `ui/src/lib/native-menu.ts`, `ui/src/lib/app-keys.ts`, `ui/src/components/TerminalToolbar.tsx`, `ui/src/screens/ThisMacScreen.tsx`, `ui/src/lib/machine.ts`, `ui/src/lib/osAccess.ts`, `crates/juggler-app/src/lib.rs`, `crates/juggler-app/src/quit.rs`, `crates/juggler-app/tauri.conf.json` (`:16-17`), a new `tauri.windows.conf.json`, `crates/juggler-app/src/manage.rs`, `crates/juggler-app/Cargo.toml` (`trash`, `:130`), `crates/juggler-app/src/features/app_icons/render.rs`, `justfile`, `docs/guide/this-mac.md`, `scripts/platform.allow`.
**Research basis**: audit 12.1–12.4, 13.1, 16.1–16.5, §3 C12–C13, C16, §5.2 step 10; `ui/src/lib/shortcuts.ts:77-84,107` (Ctrl+Alt today); `ui/src/components/TerminalToolbar.tsx:17` (`navigator.platform`); D-50 (the keys).
**Handover note**: Shared seam (Linux's words in AJ-E2712). The preview needs a way to render the Windows words on a Mac (a preview-only platform switch), or the gallery can't show them; keep it out of the shipped build. Don't rename `ThisMacScreen` in code; rename what the user reads.

## Step 2 — ship it

### AJ-E2614 The Windows installer, signing and updates | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2601, AJ-E2604
**Goal**: A signed Windows installer that installs per user, updates itself from its feed with verified signatures, uninstalls cleanly, and is built by CI from the release descriptor.
**User story**: As a Windows user, I download AI Juggler, install it without SmartScreen warnings about an unknown publisher, and it updates itself on my channel.
**Requirements**:
1. `release/platforms.json` SHALL list each target triple, its bundle formats, its signing step and its updater key; the workflows SHALL read it instead of hard-coding macOS (audit §3 C14).
2. The bundle SHALL be the format AJ-E2601 chose (NSIS by default, per user, no admin), with `juggler-agent.exe` as an `externalBin` and the WebView2 install mode chosen (audit 14.1, 5.6).
3. Every executable and the installer SHALL be Authenticode-signed with the chosen certificate and timestamped; the signing secrets live in a GitHub Environment like Apple's.
4. The updater SHALL be built with its `zip` feature only once a version without the open advisories ships (audit 14.3, `crates/juggler-app/Cargo.toml:81`, `docs/RELEASING.md:202-205`); until then the first Windows release is download-and-reinstall and says so on its page and in About.
5. Each channel's one feed SHALL carry a `windows-x86_64` entry beside `darwin-*` for the same version (D-52: no feed of Windows' own), written by `nightly.yml` in the same run as macOS and checked by `scripts/updater_sig_check.py` and Promote's feed assertions (`.github/workflows/promote.yml:203-205`, `nightly.yml:158`).
6. The three editions SHALL install side by side with their own AUMIDs, Start Menu entries, data and credentials (D-23).
7. Uninstall SHALL remove the app and offer to keep or remove its data, plainly (the uninstall guide).
8. Fleet's local discovery SHALL be tested against the Windows Firewall prompt, and the first-run copy SHALL say why it asks (audit 14.4).
**UI/UX reasoning**: Windows app design guidelines on packaging and the first run (per-user install, no unnecessary admin prompt); Nielsen #1 for updates (the channel label, D-04/D-11); the update sheet's existing states (`docs/RELEASING.md` "How users get updates").
**Performance budget**: the installer ≤ 20 MB; install ≤ 30 s on the reference Windows machine; cold start ≤ 1.5 s after install (SPEC §1); an update check ≤ 2 s; an update install ≤ 60 s including the backup.
**Quality gates**: `python3 scripts/release_version.py --selftest`, `promotion_rules.py --selftest`, `updater_sig_check.py --selftest` with Windows cases; `signtool verify /pa /v` on every executable in CI; `cargo deny check` with the Windows dependencies; a release rehearsal (`release-rehearsal.yml`) on Windows.
**Evidence/validation proof**: a rehearsal build installed on a clean Windows VM: no unknown-publisher warning, an update from N to N+1 verified, a bad signature refused, a rollback to N, uninstall leaving nothing but the data the user chose to keep (`docs/qa/evidence/AJ-E2614/manifest.md`); pass = every step as stated, logs kept.
**Visual testing**: The installer's pages and the update sheet's states on Windows (available, downloading, installed, failed with Try again), captured on the clean VM at 100 % and 150 % scaling, Light and Dark, checked by eye.
**Files to touch**: `release/platforms.json` (new), `.github/workflows/nightly.yml`, `promote.yml`, `release-rehearsal.yml`, a new `build-windows-release.yml`, `crates/juggler-app/tauri.conf.json` (`bundle`, `:30`), `crates/juggler-app/tauri.nightly.conf.json`, `tauri.dev.conf.json`, `crates/juggler-app/Cargo.toml`, `crates/juggler-app/src/updates.rs`, `scripts/updater_sig_check.py`, `scripts/release_version.py`, `docs/RELEASING.md`, `docs/guide/install-and-updates.md`, `docs/guide/uninstall.md`.
**Research basis**: audit 14.1–14.6, 5.6, §3 C14, §5.2 step 9, §5.4 item 6; https://v2.tauri.app/distribute/windows-installer/ and https://v2.tauri.app/distribute/sign/windows/ (**I**); `.github/workflows/build-macos.yml:195-231` (the macOS overlay to mirror); `docs/RELEASING.md` (D-20, D-23, D-24).
**Handover note**: Shared seam: the release descriptor (Linux adds its rows in AJ-E2713). An OV certificate builds SmartScreen reputation slowly; the first downloads may still warn: say so in the release notes instead of promising none. Keep the updater private key and signing secrets out of the repo (memory: public repo rules).

### AJ-E2615 Windows' dedicated first release (its first Beta) | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2602, AJ-E2603, AJ-E2604, AJ-E2605, AJ-E2606, AJ-E2607, AJ-E2608, AJ-E2609, AJ-E2610, AJ-E2611, AJ-E2612, AJ-E2613, AJ-E2614
**Goal**: The dedicated first release ships Windows on the Beta channel, on the shared version stream, only when the core works on a real Windows machine: tools found, accounts imported, sessions run and are seen, stops are whole and honest, switching keeps history, and nothing claims what it can't do.
**User story**: As an early Windows user, the Beta does what its page says, and every feature that isn't there yet says so where I'd look for it.
**Requirements**:
1. A gate file `docs/qa/gate-windows-beta.md` SHALL list each criterion with its evidence link (the `gate_packet.py` format).
2. Core criteria: tools found every supported way (AJ-E2607); import of existing setups; two Claude accounts in parallel with zero writes to `%USERPROFILE%\.claude`; sessions seen in the Processes view; stops whole with zero false kills (AJ-E2606); switching A→B→Codex→A keeps history, tasks and context (the 0.1 switching rule); honest usage; keep-awake and the PC's signals (AJ-E2612).
3. Every feature not ready on Windows SHALL show "not on Windows yet" where it would be, with the reason, and the release notes SHALL list them (WSL, window mirroring, any `NotYet` seam).
4. The release SHALL go through the shared flow: the version's nightly carries its Windows build beside macOS's, and D-24's promotion rules (days of Nightly use, no open `blocker`) hold on Windows as well as macOS before the one promote to Beta moves both.
5. The maintainer's walk on a Windows machine with the lab setup (read-only on real accounts) SHALL be recorded with its findings fixed or ruled.
**UI/UX reasoning**: Nielsen #1 and #10 (help and documentation): the "not on Windows yet" lines and a Windows section in each guide page that differs.
**Performance budget**: SPEC §1's budgets on the reference Windows machine: cold start < 1.5 s, < 150 MB RSS typical, < 1 % CPU for monitoring, 60 fps with 20 live sessions.
**Quality gates**: `just evidence` (the gate's links resolve); every other E26 task `DONE-EVIDENCED` or explicitly deferred with the maintainer's word; the a11y gate on the Windows captures; the load gate run on Windows.
**Evidence/validation proof**: `docs/qa/gate-windows-beta.md` with every criterion PASS and linked; pass = 100 % of the core criteria, 0 open `blocker`.
**Visual testing**: The Windows gallery (AJ-E2613) re-run on the release candidate, 1280×800 and 1440×900, Light, Dark and a contrast theme, at 100 % and 150 % scaling; axe 0 serious or critical.
**Files to touch**: `docs/qa/gate-windows-beta.md` (new), `docs/releases/` (the release's notes, with a Windows section), `docs/guide/` (Windows sections), `README.md` (the platform row, once shipped).
**Research basis**: the 0.1 gate (memory: import, sessions, honest usage, This Mac and permissions on the real setup); `docs/qa/gate-beta.md` (the Beta gate's form); `docs/RELEASING.md` (D-20, D-24); audit §5.4 ("deferred without harm"); D-52.
**Handover note**: This is the only release whose scope is Windows; after it, Windows rides every release (AJ-E2616). Don't claim Windows in the README or the site until this gate passes and the maintainer says so (memory: no public marketing yet).

### AJ-E2616 Windows in every release: the shared flow and every shipped OS | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2615
**Goal**: After the first release, Windows ships in every release with macOS: every nightly builds, signs and publishes it, one promote moves every OS's build of a version together, Windows reaches Stable under the same rules, and every feature is built and tested on Windows as part of being done.
**User story**: As a Windows user, every new version reaches me the same day and on the same channels as on a Mac, and a feature that isn't on Windows says why.
**Requirements**:
1. `nightly.yml` SHALL build, sign and publish the Windows build of every version in the same run as macOS, from `release/platforms.json`; a failed Windows build SHALL fail the nightly, so no version ships without every shipped OS.
2. `promote.yml` SHALL promote a version only when every shipped OS's build of it is in the release, and move the channel's one feed with all of them together; `scripts/promotion_rules.py` SHALL check it.
3. A `blocker` on any shipped OS SHALL stop the promotion of that version for every OS (D-24 across OSes; the agents' reading of D-52, the maintainer to confirm).
4. The Definition of Done (`docs/plan/QUALITY.md` §2) and the reviewer checklist SHALL require every shipped OS; the PR template SHALL ask for it; CI's Windows test job (AJ-E2602) SHALL be required on every PR to `main` and `release/*`.
5. The task format (`docs/tasks/README.md`) SHALL ask every new task to name how it is tested on each shipped OS, or why it is unsupported there.
6. Windows SHALL reach Stable with the first version carrying it that the stable rules promote (14+ days on beta, no `blocker`, the beta gate signed); before that promote, the security review (the M8-2 checklist) SHALL be re-run for the Windows-specific code: ACLs, the named pipe, Credential Manager, the Job Object, the installer.
7. An update from the first Windows Beta to that Stable build SHALL be proven, with a rollback.
**UI/UX reasoning**: The release channel policy (memory: nightly as pushed, beta after real use, stable deliberate), now for every OS at once; Nielsen #1: the channel shown in About, and "not on Windows yet" with its reason where a feature is unsupported.
**Performance budget**: a nightly with Windows ≤ 30 min longer than the macOS-only run (the OS jobs in parallel); a promote ≤ 10 min; the Stable candidate within 10 % of the first release's measured budgets.
**Quality gates**: the release scripts' self-tests with multi-OS cases (`release_version.py`, `promotion_rules.py`, `updater_sig_check.py`); `actionlint`; `just evidence`; the security findings closed (a finding blocks the merge).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2616/manifest.md` linking two nightlies that published both OSes, one deliberately failed Windows build that stopped its version, one promote that moved both, and the Stable promote with the security review; pass = every step as stated, 0 open `blocker`, 0 open security findings.
**Visual testing**: The Windows gallery runs in the visual sweep for every release, beside macOS's; 0 unexplained diffs against the approved Windows baselines.
**Files to touch**: `.github/workflows/nightly.yml`, `promote.yml`, `ci.yml`, `visual-sweep.yml`, `scripts/promotion_rules.py`, `scripts/release_version.py`, `docs/plan/QUALITY.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `docs/tasks/README.md`, `docs/RELEASING.md`, `docs/qa/gate-beta.md`, `README.md`, `docs/SPEC.md` (§1's platform row).
**Research basis**: D-52 (the maintainer: "Once we make the initial release than every feature is built all the time for each OS."); `docs/RELEASING.md` (D-20, D-24, D-23); `.github/workflows/nightly.yml:158` (the feed's `platforms`), `promote.yml:203-205` (its assertions); `docs/plan/EPICS.md` M8-2.
**Handover note**: Once Windows has shipped, a Windows-only failure stops the release for every OS; that is the point (no OS lags behind). Keep the Windows jobs fast enough that this isn't felt every night, and never mark a feature "unsupported on Windows" to get a release out without the maintainer's word.
