# E27 — AI Juggler for Linux: its own project and releases

> Source: the maintainer, 2026-10-29 (D-52), in the maintainer's words: "Can you add in
> our global planner of all the features and future versions, special
> releases for both Windows and Linux separately." And earlier the same
> day: "we have a lot of specialized process monitoring, special desktop
> capabilities that work separately for each OS … I am not saying we will
> not build them but I feel they are massive and require their own
> projects and releases."
> Research: `docs/research/cross-platform-readiness.md` (the audit of
> 2026-10-27, read at `41f6c8c3`): the inventory C1–C16 (§2), the seams
> (§3), what each tool stores per OS (§4), the ranked plan (§5), the
> minimum Linux build (§5.3) and the questions only a lab answers (§5.5).
> Rows are cited as "audit 7.5"; line numbers in the audit are at
> `41f6c8c3`, and the lines cited below were re-read on `release/0.2` at
> `57bd760a` (2026-10-29).
> Sister epic: [E26 — AI Juggler for Windows](E26-windows-release.md),
> which holds the default owner of most shared seams (its intro's table).
>
> **Why a project of its own.** Linux is closer than Windows (audit §0:
> sockets, process groups, signals, modes, ports and PTYs already work),
> but what is missing is exactly what makes AI Juggler useful on a
> desktop: finding the desktop apps and terminals (`.desktop` files,
> Flatpak and Snap), going to a window (Wayland allows almost none of it),
> the system's keyring (Secret Service), keeping the computer awake (logind
> or the portal), notifications and the tray (which differ by desktop:
> GNOME, KDE, others), and packaging (AppImage, deb, rpm, Flatpak, each
> with a different update story and sandbox). The desktops and
> distributions differ more than Windows versions do, so the lab, the
> support matrix and the gates are this line's own. The Linux line is not
> part of the macOS 0.x releases: a macOS release never waits for Linux
> work, and never claims Linux support.
>
> **What is already cross-platform** (by reading; nothing has run as an
> installed Linux app):
> - Everything Unix: the data folder (`dirs::data_dir()` is
>   `$XDG_DATA_HOME` or `~/.local/share`, audit 1.1), Unix sockets and file
>   trust, process groups and signals, PTYs, ports (`netstat2`), the
>   dot-folder tools and Claude's `~/.claude/.credentials.json`, the single
>   instance (audit §5.3, "What already works").
> - The OS seams crate `crates/juggler-platform/` takes its Unix branch on
>   Linux (`user`, `file_trust`, `private_fs`, `local_ipc`, `system`).
> - CI: the `verify` job runs `cargo test --workspace` on Ubuntu
>   (`.github/workflows/ci.yml:89-91`), and `build-linux` runs
>   `cargo check --workspace --all-targets` and the build
>   (`ci.yml:403-429`). No Linux bundle is built and no test runs in a
>   desktop session.
>
> **What is macOS-only today on Linux** (audit §2, §5.3): credentials
> (Keychain only, audit 6.1–6.4); Go to it and focus (`/usr/bin/open`,
> `osascript`, audit 8.1–8.3); the host-app model (`.app` bundles, audit
> 7.1–7.3); held-session detection (`/usr/sbin/lsof`, audit 7.5);
> keep-awake, open at login and machine signals (audit 10.1–10.6); the
> tool environment drops `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY` (audit 4.2);
> OpenCode's `XDG_*` folders (audit 1.7); 15-character process names
> (audit 2.3); the Trash, icons and window chrome (audit 12.1–12.4);
> packaging and the update feed (audit 14.1–14.5); Mac words (audit 16.2).
>
> **The release shape is the maintainer's decision (AJ-E2701).** Not
> decided by D-52 and not invented here:
>
> | Question | Options (consequence) |
> |---|---|
> | Version numbers | (a) the same numbers as macOS, built from the same commit, with Linux's own channel position and feeds; (b) a stream of its own (collides with D-20's one stream) |
> | Channels | Linux Nightly / Beta / Stable with their own gates, or Beta and Stable only until Linux Stable |
> | Formats | AppImage (the Tauri updater updates it), `.deb` and `.rpm` (updated by a package repository, as Claude's desktop app does), Flatpak or Snap (their sandboxes hide the host's processes: see AJ-E2713) |
> | Repository | an apt / dnf repository signed with a project GPG key, or downloads only |
> | Support matrix | which distributions and versions (Tauri 2 needs WebKitGTK 4.1: Ubuntu 22.04 and later, Debian 12 and later, current Fedora), GNOME and KDE, Wayland and X11, x86_64 only or aarch64 too |
> | Order | Linux first, Windows first (E26), or both together |
>
> **Shared seams, built once.** The default owners are in E26's table.
> This epic owns `OpenFiles` (C11, AJ-E2710); for the others, the Linux
> task adds its OS's half and links back. If this epic starts first, its
> task takes the shared part and E26's records the forward link.
>
> **Law for every task here:**
> - **Behind the seams.** Linux code lives in `crates/juggler-platform/`,
>   the audited per-OS shims, or an adapter; a `cfg(target_os = "linux")`
>   in feature code is a defect. `scripts/platform.allow` only shrinks.
> - **Honest per desktop.** Where GNOME, KDE or Wayland can't do something
>   (focus another app's window, show a tray icon without an extension,
>   badge the launcher), AI Juggler says so in plain words and offers what
>   it can ("Open" instead of "Go to it").
> - **No desktop-specific branches in features.** A desktop difference
>   (GNOME vs KDE) is a capability the seam reports, never a check of
>   `XDG_CURRENT_DESKTOP` in a feature.
> - **`unsafe_code = "forbid"` stays**; D-Bus and `/proc` need none.
> - **macOS doesn't change**, except copy the platform words module now
>   draws.
> - **Real accounts, read-only.** The lab uses a throwaway Linux user and
>   lab tool profiles; no sign-ins, no Allow, no credential reads.
>
> Research links in "Research basis" that the audit did not already cite
> were not opened while writing this epic (2026-10-29); each task's first
> step re-reads them, and until then they are labelled **I**
> (`docs/research/EVIDENCE-STANDARD.md`).

## Step 0 — decide and see

### AJ-E2701 The Linux line's decisions | Status: TODO | Priority P0 | Effort S | Depends: —
**Goal**: The maintainer's choices for the Linux line are recorded before anything ships: version numbers and channels, formats and the repository, the support matrix, and whether Linux or Windows goes first.
**User story**: As the maintainer, I decide once, from one page with each option's consequence, how AI Juggler for Linux is versioned, packaged and supported, so the agents building it never guess.
**Requirements**:
1. A decision page SHALL list each open question of the intro's table with its options, the consequence of each, one concrete example, and the tasks it unblocks.
2. The page SHALL state the agents' recommended default for each, labelled as the agents', not the maintainer's.
3. Each answer SHALL be recorded as its own decision-log entry (the next free D-number) in the D-52 style, with the maintainer's words.
4. The support matrix SHALL name each distribution, version, desktop and display server the Beta promises, and say plainly what happens elsewhere ("may work; not tested").
5. The Flatpak and Snap answer SHALL be made knowing their sandboxes hide the host's processes from the app (AJ-E2713).
6. "Which goes first" SHALL be recorded once, shared with AJ-E2601.
7. `docs/RELEASING.md` SHALL gain a "Linux" section once decided.
**UI/UX reasoning**: The serious-choices rule (consequence, example, inline tip, docs link) applied to the maintainer's own decisions; Nielsen #6 recognition rather than recall.
**Performance budget**: one page; ≤ 8 questions, answerable in one sitting.
**Quality gates**: `python3 scripts/check_task_counts.py`; `./scripts/terminology_lint.sh`; the decision log's format.
**Evidence/validation proof**: the recorded entries in `docs/review/decision-log.md` and `docs/qa/evidence/AJ-E2701/manifest.md` linking each to the tasks it unblocks; pass = every question answered or deferred with a date.
**Visual testing**: None: a decision record, no screen.
**Files to touch**: `docs/review/decision-log.md`, `docs/RELEASING.md`, `docs/plan/EPICS.md` (the Linux milestone row), this file.
**Research basis**: audit §5.3, 14.1–14.2, §4 (Claude desktop on Linux: beta, Debian-based only, apt, VP); https://v2.tauri.app/distribute/ (VP in the audit: deb, rpm, AppImage, Snap, Flatpak, AUR); https://v2.tauri.app/start/prerequisites/ (WebKitGTK 4.1, **I**); `docs/RELEASING.md:1-31` (D-20, D-24).
**Handover note**: Don't pick version numbers here (memory: SemVer strictly; one version stream today). A distribution the Beta doesn't promise is not a bug report blocker; say so on the release page.

### AJ-E2702 CI that proves Linux as a desktop app | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: CI tests what Linux users run: the packaged app, in a desktop session with D-Bus, a keyring and a notification server, on more than one distribution, not only `cargo test` on a headless Ubuntu runner.
**User story**: As a contributor, I see a red check when my change breaks the Linux app's keyring, notifications or package, before anyone installs it.
**Requirements**:
1. A Linux job SHALL run the D-Bus-dependent tests inside `dbus-run-session` with a headless keyring (gnome-keyring unlocked with a throwaway password) and a notification server, so the Secret Service, Notifications and inhibit seams are tested for real.
2. A container matrix SHALL build and run the tests on the support matrix's distributions (AJ-E2701; at least the oldest Ubuntu and the newest Fedora promised).
3. A package smoke SHALL install the built AppImage and `.deb` in a clean container, start the app headless (Xvfb or a headless Wayland compositor), and check it reaches its first screen.
4. The UI's Playwright run SHALL include a WebKit project on Linux for the touched screens, because the Linux app draws with WebKitGTK.
5. The jobs SHALL be required for the Linux line's `release/*` branches (AJ-E2701).
**UI/UX reasoning**: Contributor experience, Nielsen #9: a failure names the distribution, the desktop service and the test.
**Performance budget**: the Linux desktop-session job ≤ 20 min with a warm cache; each distribution container ≤ 15 min; the package smoke ≤ 5 min.
**Quality gates**: `actionlint`; each job green on a `release/0.2`-based branch; a deliberately broken Secret Service test proving the job fails.
**Evidence/validation proof**: CI runs linked from `docs/qa/evidence/AJ-E2702/manifest.md` (one green per job, one deliberate failure); pass = every job behaves as stated.
**Visual testing**: The WebKit project's captures of the touched screens on Linux, compared with the Chromium baselines for layout only (font metrics differ; differences reviewed by eye, not failed by pixel).
**Files to touch**: `.github/workflows/ci.yml` (`verify`, `build-linux`, `:403-429`), a new Linux desktop-session job, `ui/playwright.config.ts`, `justfile`, `CONTRIBUTING.md`.
**Research basis**: audit 15.1, §5.3 item 1; `.github/workflows/ci.yml:89-91` (tests already run on Ubuntu), `:403-429`; https://specifications.freedesktop.org/secret-service-spec/latest/ (**I**); https://specifications.freedesktop.org/notification-spec/latest/ (**I**).
**Handover note**: Don't move the macOS visual suite to Linux: its baselines are Chromium on macOS. The headless keyring's password is a fixture, never a real one; mark it so the secret scan doesn't trip and nobody reuses it.

### AJ-E2703 The Linux lab: a throwaway user, read-only, the open questions answered | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: The questions only a real Linux desktop answers are answered with evidence before the seams that depend on them: the Claude and Codex desktop apps' folders and process names (Q4, Q5), notification clicks (Q7), the AppImage mount and file trust (Q8), `netstat2` with IPv6 (Q2), and GNOME and KDE's tray, badge, focus and inhibit behaviour.
**User story**: As the agent building Linux support, I know where the Claude desktop beta keeps its sessions on Ubuntu and what GNOME on Wayland lets an app do, from a measured lab.
**Requirements**:
1. The lab SHALL run on Ubuntu (GNOME, Wayland and X11 sessions) and on Fedora or KDE as the support matrix says, under a throwaway Linux user with lab tool profiles only; no sign-ins, no Allow, no credential reads.
2. `docs/qa/linux-lab.md` SHALL hold the exact steps, the before/after listing of each tool's folder, and the cleanup.
3. Each answer SHALL be labelled VP or U with its capture or output: Claude desktop data folder (`~/.config/Claude` or another), process names and session layout (Q4); Codex desktop on Linux (Q5); `ActionInvoked` on a notification click under GNOME and KDE (Q7); `trusted_executable` and the bridge inside an AppImage mount (Q8); `netstat2` on IPv4 and IPv6 (Q2); the tray without and with the AppIndicator extension; window activation on Wayland and X11; the logind and portal inhibitors.
4. Captures SHALL be redacted with a fail-closed tool and a sidecar (the macOS tool is Swift, `scripts/qa/redact_capture.swift`; a Linux-capable one first, or captures stay on the lab machine).
5. The audit's §4 table SHALL be updated with each answer and its date.
**UI/UX reasoning**: n/a to the screens; the evidence labels keep a guessed folder from ever showing as fact (Nielsen #1).
**Performance budget**: the lab ≤ 1 day of machine time; each question ≤ 30 min to re-run.
**Quality gates**: evidence-standard labels on every answer; `just evidence`; `python3 scripts/qa/check_captures.py`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2703/lab-<date>.md` and `manifest.md`; pass = every question answered VP or U with the reason, zero secrets, every capture checked.
**Visual testing**: The lab's captures of GNOME's and KDE's notifications, tray and the desktop apps, redacted and checked by eye; not product captures.
**Files to touch**: `docs/qa/linux-lab.md` (new), `docs/research/cross-platform-readiness.md` (§4, §5.5), `scripts/qa/`, `docs/qa/evidence/AJ-E2703/`.
**Research basis**: audit §5.5 Q2, Q4, Q5, Q7, Q8; §4 (the Linux column: Claude desktop beta, Debian-based only, data folder U; Codex Linux desktop U); `docs/qa/processes-lab.md`, `docs/qa/runtime-lab.md` (the lab style).
**Handover note**: The Claude desktop beta keeps its login in the desktop keyring (audit §4, VP): the lab must not open the keyring's items, only note that they exist. Reap every process the lab starts.

## Step 1 — the seams

### AJ-E2704 Where things live on Linux: the XDG folders | Status: TODO | Priority P1 | Effort S | Depends: AJ-E2604
**Goal**: AI Juggler and the adapters honour the XDG base directories the user set, for its own data and for the tools that follow them (OpenCode).
**User story**: As someone who keeps config in a custom `XDG_CONFIG_HOME`, AI Juggler finds OpenCode where OpenCode itself does, and keeps its own data under my `XDG_DATA_HOME`.
**Requirements**:
1. The shared `Home` / `AppDirs` resolver (owner AJ-E2604) SHALL, on Linux, use `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` and `XDG_RUNTIME_DIR` with the spec's defaults; relative values are ignored, as the spec says.
2. OpenCode's data, config and state SHALL be read from its `XDG_*` folders first, then the defaults (audit 1.7, `crates/juggler-adapters/src/opencode.rs`).
3. Logs and the crash reports SHALL live under the state folder, caches under the cache folder, each per edition (D-23).
4. Per-run sockets SHALL prefer `XDG_RUNTIME_DIR` over the temporary folder (`crates/juggler-app/src/handover.rs:112-113` uses `temp_dir()`).
5. Tests: custom and unset XDG values, a relative value ignored, macOS unchanged.
**UI/UX reasoning**: XDG Base Directory spec (https://specifications.freedesktop.org/basedir-spec/latest/, **I**); Nielsen #2: the folders the user already set.
**Performance budget**: resolution ≤ 1 ms at start; zero writes during resolution.
**Quality gates**: `cargo test -p juggler-platform`, `-p juggler-adapters` (OpenCode); `just platform`.
**Evidence/validation proof**: the tests on Linux CI, and on the lab machine OpenCode found under a custom `XDG_DATA_HOME` (`docs/qa/evidence/AJ-E2704/manifest.md`); pass = all cases green, OpenCode's sessions listed.
**Visual testing**: Settings › About's data folder line and OpenCode's account page on Linux, 1280×800, Light and Dark (preview's Linux mode, AJ-E2712).
**Files to touch**: `crates/juggler-platform/src/` (the `dirs` module, Linux half), `crates/juggler-adapters/src/opencode.rs`, `crates/juggler-adapters/src/opencode_environment.rs`, `crates/juggler-app/src/state.rs`, `crates/juggler-app/src/handover.rs`, `crates/juggler-app/src/crash.rs`.
**Research basis**: audit 1.1, 1.7, §3 C1; §4 (OpenCode's `XDG_*`, VP carried); `crates/juggler-app/src/state.rs:584-585`; `crates/juggler-app/src/handover.rs:92-113`.
**Handover note**: The shared resolver is AJ-E2604's; build only the Linux half here unless this epic starts first. Moving the data folder after a Linux release is a migration: settle it before the first installable build.

### AJ-E2705 Processes on Linux: names, `/proc`, reapers and stopping escaped children | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2605
**Goal**: AI Juggler names and verifies Linux processes correctly (15-character names, `/proc/<pid>/exe`, user-session reapers) and a stop catches children that left the session's process group.
**User story**: As someone on Linux whose dev server double-forks, stopping the session ends the server too, and the Processes view names every helper correctly.
**Requirements**:
1. The shared `ExeId` (owner AJ-E2605) SHALL, on Linux, prefer the executable's file name over the 15-character `comm`, and treat a `" (deleted)"` suffix as the same program updated in place (audit 2.1, 2.3).
2. A process re-parented to a user-session reaper (`systemd --user`, not pid 1) SHALL read as left behind, as `ppid == 1` does today (audit 2.7).
3. A child that called `setsid` and left the group SHALL be found by ancestry from the ledger's root (or by the session's cgroup, if AJ-E2701 chooses a per-session `systemd-run --user --scope`) and offered for stop, never stopped silently.
4. Processes inside a Flatpak or Snap sandbox (another PID namespace) SHALL show as the app's and never be stopped by a pid AI Juggler can't verify.
5. Tests: names longer than 15 characters, a replaced binary, a reaper, a double fork; the zero-false-kill proof on Linux.
**UI/UX reasoning**: Nielsen #2 and #9: the Processes view's role words are the same on every OS; a child outside the group says "started by this session, outside its group" with Stop as its own click.
**Performance budget**: naming 2,000 processes ≤ 5 ms per tick; reading ancestry ≤ 3 ms per tick; no `/proc` scan beyond what `sysinfo` already does.
**Quality gates**: `cargo test -p juggler-supervisor`, `-p juggler-app --lib features::processes` on Linux; `tests/collateral.rs` on Linux; clippy.
**Evidence/validation proof**: the tests on Linux CI and, in the lab, a double-forking dev server stopped whole with a record of each process (`docs/qa/evidence/AJ-E2705/manifest.md`); pass = 0 false kills in 100 runs, 100 % of the tree gone.
**Visual testing**: The Processes page with long names, a reaped helper and an outside-group child, 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-platform/src/` (the `exe` module, Linux half), `crates/juggler-supervisor/src/live_stats.rs`, `crates/juggler-supervisor/src/tree.rs`, `crates/juggler-supervisor/src/group.rs`, `crates/juggler-supervisor/src/zombies.rs`, `crates/juggler-app/src/features/processes/roles.rs`, `crates/juggler-app/src/features/processes/stop.rs`.
**Research basis**: audit 2.1, 2.3, 2.7, 2.9, 3.5–3.6, §3 C2–C3; https://man7.org/linux/man-pages/man5/proc.5.html (**I**); https://docs.kernel.org/admin-guide/cgroup-v2.html (`cgroup.kill`, **I**); ADR-0010.
**Handover note**: Process groups already work on Linux; don't replace them. A cgroup per session is a design choice for AJ-E2701 (it changes how sessions start); propose it, don't slip it in.

### AJ-E2706 Finding and starting tools on Linux | Status: TODO | Priority P0 | Effort S | Depends: AJ-E2607
**Goal**: AI Juggler finds every tool however it was installed on Linux and starts it with the environment Linux tools need (the display, the session bus, XDG folders).
**User story**: As someone with Claude Code in `~/.local/bin` and Codex from Linuxbrew, AI Juggler finds both and the sessions it starts can open a browser for me.
**Requirements**:
1. The shared `LaunchEnv` (owner AJ-E2607) SHALL, on Linux, keep `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY`, `DBUS_SESSION_BUS_ADDRESS` and `XDG_RUNTIME_DIR` for probes and sessions (audit 4.2).
2. The login shell fallback SHALL be the user's shell from the password database, then `/bin/sh`, never a missing `/bin/zsh` (audit 4.3).
3. The known folders SHALL include `~/.local/bin`, the npm prefix, `/home/linuxbrew/.linuxbrew/bin` and `~/.linuxbrew/bin` (audit 4.5).
4. `git`'s missing message SHALL name the distribution's package manager, not Apple's Command Line Tools (audit 4.6).
5. Tests: each install way found in a fixture tree; the environment's keys exactly the allowlist.
**UI/UX reasoning**: Nielsen #9: "Claude Code isn't on the Path AI Juggler reads from your login shell. Check that `claude` runs in a new terminal."; GNOME HIG's writing style (plain, short, https://developer.gnome.org/hig/guidelines/writing-style.html, **I**).
**Performance budget**: locating every tool ≤ 300 ms; each `--version` probe ≤ 2 s, all in parallel ≤ 5 s; one login shell per start, at most.
**Quality gates**: `cargo test -p juggler-adapter` (`user_path`), `-p juggler-adapters` on Linux; `just tool-literals`; `just platform`.
**Evidence/validation proof**: the lab's table of tools found each way and a session opening a browser sign-in page from inside a session started by AI Juggler (no sign-in done) (`docs/qa/evidence/AJ-E2706/manifest.md`); pass = every way found, the browser opened.
**Visual testing**: The Tools page and first run's tool check on Linux (found, broken, missing), 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-adapter/src/user_path.rs` (`:35-62`), `crates/juggler-adapter/src/system_tools.rs`, `crates/juggler-adapters/src/claude_code.rs`, `codex.rs`, `opencode.rs`, `kimi_code.rs`, `grok_build.rs`, `crates/juggler-app/src/worktree.rs`.
**Research basis**: audit 4.2–4.6, §3 C4, §5.3 item 4; `crates/juggler-adapter/src/user_path.rs:44-59` (the allowlist and folders today).
**Handover note**: `DBUS_SESSION_BUS_ADDRESS` and `DISPLAY` let a tool reach the user's desktop; that is the point (sign-in in a browser) but keep the allowlist explicit and reviewed (SECURITY-GUIDELINES C2).

### AJ-E2707 Private files and local connections on Linux, packaged | Status: TODO | Priority P1 | Effort S | Depends: AJ-E2703
**Goal**: The Unix promises (owner-only files, a socket only this user can reach, programs run only from trusted places) hold for the packaged Linux app, including inside an AppImage mount.
**User story**: As someone running the AppImage, the agent tools and hooks keep working after I restart AI Juggler, and nobody else on the machine can use them.
**Requirements**:
1. `trusted_executable` SHALL accept the bridge program inside the AppImage's own mount only when the mount is the running app's (`$APPIMAGE` and `/proc/self/exe` agree), and refuse any other `/tmp` path (audit 5.4, Q8).
2. Hook commands written into a tool's settings SHALL point to a stable copy of `juggler-agent` in the app's data folder (owner-only, verified by hash at start), never into the AppImage mount, whose path changes on every run (audit 5.6).
3. Sockets SHALL live under `XDG_RUNTIME_DIR` (AJ-E2704) when it is set.
4. Tests: a mount path from another user refused; the stable copy replaced when the app updates; the hook still answering after a restart.
**UI/UX reasoning**: Nielsen #1: the agent tools' status line in Settings says "working" only after the bridge answered.
**Performance budget**: the start-up copy and hash check ≤ 20 ms; a socket round trip ≤ 5 ms p95.
**Quality gates**: `cargo test -p juggler-platform`, `-p juggler-app --test trust` on Linux; the package smoke (AJ-E2702) with the AppImage.
**Evidence/validation proof**: the lab's AppImage run: hooks answering before and after a restart, another user's connection refused (`docs/qa/evidence/AJ-E2707/manifest.md`); pass = both observed.
**Visual testing**: Settings' agent tools status on Linux (working, not answering), 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-platform/src/file_trust.rs`, `crates/juggler-platform/src/local_ipc.rs`, `crates/juggler-app/src/features/agent_bridge/mod.rs` (`socket_path`, `:95`), `crates/juggler-app/src/features/runtime/launch.rs`, `crates/juggler-app/src/handover.rs`, `crates/juggler-adapters/src/claude_environment.rs`.
**Research basis**: audit 5.4, 5.6, 5.11, §5.3 item 2; `crates/juggler-platform/src/file_trust.rs:16-46`; https://docs.appimage.org/packaging-guide/environment-variables.html (`$APPIMAGE`, **I**).
**Handover note**: The stable copy is a program AI Juggler runs from a tool's settings; it must be owner-only and verified, or it becomes a way in. A `.deb` install has a fixed path and doesn't need the copy; keep one code path anyway.

### AJ-E2708 Sign-in secrets in the Secret Service | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2609
**Goal**: AI Juggler's secrets live in the desktop's keyring through the Secret Service (GNOME Keyring, KWallet, KeePassXC), and with no keyring it refuses in plain words before the user types a secret.
**User story**: As someone on KDE, my key for an account through another service is kept in KWallet; on a minimal desktop with no keyring, AI Juggler tells me before I paste it.
**Requirements**:
1. The shared real `SecretStore` (owner AJ-E2609) SHALL have a Secret Service back end on Linux, one collection item per secret, labelled per edition.
2. A locked keyring SHALL be unlocked only by the desktop's own prompt, never by AI Juggler asking for the password.
3. With no Secret Service, `ServiceKeys::store()` SHALL refuse up front: "There's no keyring on this computer to keep the key in. Install GNOME Keyring or KWallet, or add this account another way." A plain-file fallback exists only if AJ-E2701 or a decision says so.
4. The Figurehead device token and the vault key SHALL use it.
5. Copy that says "Keychain" SHALL name the desktop's keyring on Linux (AJ-E2712).
**UI/UX reasoning**: The serious-choices rule (where it's kept, how to remove it); GNOME HIG on not asking for passwords the system asks for; Nielsen #5 error prevention.
**Performance budget**: a read ≤ 100 ms with the keyring unlocked; no D-Bus call on the UI thread; a locked keyring's prompt doesn't block the window.
**Quality gates**: `cargo test -p juggler-account` in AJ-E2702's desktop-session job; `cargo test -p juggler-app --test service_accounts`; guardrails (no secret in logs, D-37).
**Evidence/validation proof**: the desktop-session CI job and the lab on GNOME and KDE: a test key saved, read, forgotten, checked in Seahorse and KWallet Manager; a machine with no keyring refusing before input (`docs/qa/evidence/AJ-E2708/manifest.md`); pass = round trip exact, nothing left, the refusal shown.
**Visual testing**: The add-account sheet (saved, no keyring, locked) on Linux, 1280×800, Light and Dark.
**Files to touch**: `crates/juggler-account/src/secrets.rs`, `crates/juggler-account/src/vault.rs`, `crates/juggler-account/Cargo.toml`, `crates/juggler-app/src/features/service_accounts/mod.rs`, `crates/juggler-app/src/figurehead.rs`, `crates/juggler-app/src/lib.rs`.
**Research basis**: audit 6.1–6.4, §3 C6, §5.3 item 3; https://specifications.freedesktop.org/secret-service-spec/latest/ (**I**); https://docs.rs/keyring/latest/keyring/ (VP weak in the audit); D-34.
**Handover note**: An encrypted-file fallback (the audit's suggestion) is a security decision with a key-storage problem of its own; don't build it without a decision. D-34 names the Keychain: AJ-E2609's note about "the system's credential store" covers Linux too.

### AJ-E2709 Each tool's files on Linux | Status: TODO | Priority P0 | Effort S | Depends: AJ-E2703, AJ-E2704
**Goal**: Every adapter reads the right files on Linux (audit §4), including the Claude desktop beta and the Codex desktop app if the lab finds them, and switching works between tools.
**User story**: As someone with two Claude accounts and Codex on Ubuntu, I see both accounts, their sessions and usage, and I can switch a conversation between them.
**Requirements**:
1. Claude Code: `~/.claude`, `.credentials.json` (presence only), `CLAUDE_CONFIG_DIR` per account; no Keychain logic on Linux (audit 6.5).
2. Codex: `CODEX_HOME`, `auth.json`, the keyring setting reported honestly (audit 6.6).
3. OpenCode (XDG, AJ-E2704), Kimi Code, Grok Build: their Linux folders (§4).
4. The Claude desktop beta's sessions SHALL be read from the folder the lab found (Q4), or its place SHALL say "not on Linux yet"; the same for Codex's desktop app (Q5).
5. One switch A→B→Codex→A with lab accounts SHALL keep history, tasks and context (the 0.1 switching rule).
6. Fixtures: a Linux fixture tree per tool from the lab's listing, contents replaced.
**UI/UX reasoning**: Nielsen #4 consistency with macOS; a tool's card says what's missing on Linux, plainly.
**Performance budget**: the inventory scan of five tools ≤ 500 ms with 1,000 sessions on disk; zero writes outside each account's folder.
**Quality gates**: `cargo test -p juggler-adapters` with the Linux fixtures; `cargo test -p juggler-app --test guardrails`; the switch tests.
**Evidence/validation proof**: the fixture tests and the lab's switch, recorded in `docs/qa/evidence/AJ-E2709/manifest.md`; pass = history and context kept at each step, zero writes outside the account folders.
**Visual testing**: Accounts, a session's page and Usage with the Linux fixtures, 1280×800 and 1440×900, Light and Dark.
**Files to touch**: `crates/juggler-adapters/src/claude_code.rs`, `claude_live.rs`, `codex.rs`, `codex_live.rs`, `opencode.rs`, `kimi_code.rs`, `grok_build.rs`, `crates/juggler-adapters/tests/` (fixtures), `crates/juggler-app/src/features/quota/records.rs`.
**Research basis**: audit §4 (Linux column), 1.5–1.10, 6.5–6.6; https://code.claude.com/docs/en/desktop-linux.md (VP in the audit); https://code.claude.com/docs/en/authentication.md (VP in the audit).
**Handover note**: The Claude desktop beta is Debian-based only and doesn't update itself (audit §4): its version can lag the CLI's; read its files defensively. Never read a credential file's contents (D-37).

### AJ-E2710 Desktop apps, Go to it, open files and reveal on Linux | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2611, AJ-E2703
**Goal**: AI Juggler knows the Linux desktop apps and terminals and which session each holds, opens and reveals through the desktop, and says "Open" where Wayland won't let it bring a window forward.
**User story**: As someone with sessions in GNOME Terminal, Konsole and VS Code, I see each under its app, and one click opens it, or tells me why it can't.
**Requirements**:
1. The shared `HostId` model (owner AJ-E2611) SHALL gain `LinuxDesktop { desktop_id, exec, wm_class, sandbox }`, read from Desktop Entry files in the XDG data folders, including Flatpak's and Snap's exports (audit 7.1).
2. `hosts.json` SHALL gain Linux entries (GNOME Terminal and its server process, Ptyxis, Konsole, kitty, Alacritty, WezTerm, VS Code, Cursor, JetBrains), matched by `ExeId` (audit 7.2).
3. `Desktop` on Linux SHALL report `can_focus_window` false on Wayland (open the app with an activation token where the desktop supports it, else "Open"), and on X11 only where the lab showed it works (audit 8.2–8.3).
4. Open and reveal SHALL go through `tauri-plugin-opener` and the FileManager1 interface for reveal; "Activity Monitor" SHALL become the desktop's system monitor or be hidden (audit 8.5).
5. **`OpenFiles`** (this epic owns it) SHALL read `/proc/<pid>/fd` for the transcript a process holds, replacing `/usr/sbin/lsof` (audit 7.5, `crates/juggler-app/src/features/conversations/scan.rs:68-71`); the macOS implementation stays behind the same seam.
6. The terminal of a session SHALL be found from `pts/N` as macOS finds `ttysN` (audit 8.2).
**UI/UX reasoning**: GNOME HIG (don't steal focus; notifications and the activation token for bringing a window forward, https://developer.gnome.org/hig/, **I**); Desktop Entry spec (https://specifications.freedesktop.org/desktop-entry-spec/latest/, **I**); the desktop-first rule: Go to it optional, "Open" when it can't.
**Performance budget**: reading Desktop Entry files ≤ 150 ms at start, cached, re-read on change (inotify); `/proc/<pid>/fd` per candidate ≤ 1 ms; matching 2,000 processes ≤ 3 ms per tick.
**Quality gates**: `cargo test -p juggler-adapter` (`live`), `-p juggler-app --lib features::conversations` on Linux; `just platform` (`usr-bin`, `applications`, `app-bundle` fall); the hosts.json schema test.
**Evidence/validation proof**: in the lab (GNOME Wayland, GNOME X11, KDE), a session in each listed terminal and editor shown under its app, Open or Go to it's outcome per app recorded (`docs/qa/evidence/AJ-E2710/manifest.md`); pass = every running host matched, every outcome as reported by `Desktop`'s capabilities.
**Visual testing**: Home's Open now, a session's "where it runs" line and the Open / Go to it outcome on Linux, 1280×800 and 1440×900, Light and Dark; app icons per AJ-E2712.
**Files to touch**: `crates/juggler-adapter/src/live.rs`, `crates/juggler-app/src/features/conversations/goto.rs`, `commands.rs`, `hosts.json`, `places.rs`, `scan.rs`, `crates/juggler-app/src/activation.rs`, `crates/juggler-app/src/focus.rs`, `crates/juggler-app/src/manage.rs`, `crates/juggler-app/src/features/processes/commands.rs`, `crates/juggler-platform/src/` (an `open_files` module).
**Research basis**: audit 7.1–7.7, 8.1–8.9, §3 C7–C8, C11, §5.3 item 5; `crates/juggler-app/src/features/conversations/scan.rs:68-71`; `crates/juggler-app/src/features/conversations/goto.rs:32,383`; https://wayland.app/protocols/xdg-activation-v1 (**I**).
**Handover note**: GNOME Terminal runs every tab under one `gnome-terminal-server` process, not the launcher: match the server. Flatpak apps run in their own PID namespace; a session inside one may be invisible to `sysinfo` (record it, don't model it).

### AJ-E2711 System integration on Linux: notifications, the tray, open at login, keep awake, machine signals and ports | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2703, AJ-E2704
**Goal**: The background features work on the Linux desktops in the support matrix: notifications whose click opens the subject, the count in the tray (and on the launcher where the desktop supports it), opening at login, keeping the computer awake, memory and power signals, and ports.
**User story**: As someone running overnight sessions on Ubuntu, the computer stays awake while they work, AI Juggler opens when I sign in, and a notification click takes me to the session that needs me.
**Requirements**:
1. Notifications SHALL use the Notifications spec through the plugin, with a default action; a click (`ActionInvoked`) SHALL open the subject (Q7); the E25 rules hold.
2. The tray SHALL use StatusNotifierItem / AppIndicator; where the desktop shows no tray (GNOME without the extension), Settings SHALL say so and the window's bell remains the place.
3. The app badge seam SHALL use the launcher-entry count on docks that support it, else stay `NotYet` with its reason (`crates/juggler-platform/src/system.rs:41-51`).
4. Open at login SHALL write an XDG Autostart entry (`~/.config/autostart`), shown and reversible in Settings (audit 10.1).
5. Keep awake SHALL take a logind `sleep:idle` inhibitor (or the portal's Inhibit) while sessions work and release it when they stop (audit 10.2, `crates/juggler-supervisor/src/awake.rs:165`).
6. Memory pressure SHALL read PSI (`/proc/pressure/memory`), power saver from power-profiles-daemon, thermal from the system's zones, else `None` without an alert; the alert copy names the system, not macOS (audit 10.3–10.6, `crates/juggler-app/src/machine_alerts.rs:261-263`).
7. Ports SHALL be checked on IPv4 and IPv6 (Q2); the global shortcut SHALL use the GlobalShortcuts portal on Wayland or say it's unavailable (audit 8.9).
**UI/UX reasoning**: GNOME HIG notifications guidance (https://developer.gnome.org/hig/patterns/feedback/notifications.html, **I**); the freedesktop Autostart spec (**I**); Nielsen #3 user control (each a setting, off where E25 says off); WCAG 2.2 AA for every setting's unavailable state.
**Performance budget**: a notification shown ≤ 500 ms after its state; the tray redrawn ≤ 100 ms after a change; the inhibitor taken ≤ 50 ms; PSI read ≤ 1 ms per tick; port listing ≤ 50 ms per tick at 2,000 processes.
**Quality gates**: `cargo test -p juggler-app --lib features::notifications`, `--test notification_model`, `--test system_notifications`, `cargo test -p juggler-supervisor --lib awake`, `--test machine` in AJ-E2702's desktop-session job.
**Evidence/validation proof**: in the lab on GNOME and KDE: a notification clicked through; the tray count equal to the bell for 10 changes; a 2-hour unattended session with `systemd-inhibit --list` showing AI Juggler; opening at login after a sign-out (`docs/qa/evidence/AJ-E2711/manifest.md`); pass = all observed on both desktops, or the unavailable state shown where the desktop can't.
**Visual testing**: Settings › Notifications and Settings › Desktop on Linux (on, off, unavailable on this desktop), 1280×800, Light and Dark; redacted lab captures of GNOME's and KDE's notification and tray, checked by eye.
**Files to touch**: `crates/juggler-app/src/features/notifications/mod.rs`, `tray.rs`, `crates/juggler-platform/src/system.rs`, `crates/juggler-login-item/src/lib.rs`, `crates/juggler-app/src/features/app/login_item.rs`, `crates/juggler-supervisor/src/awake.rs`, `crates/juggler-app/src/features/app/awake.rs`, `crates/juggler-supervisor/src/machine.rs`, `crates/juggler-app/src/machine_alerts.rs`, `crates/juggler-supervisor/src/ports.rs`, `crates/juggler-app/src/lib.rs`.
**Research basis**: audit 8.9, 9.1, 10.1–10.8, 11.1, §3 C9–C11, §5.3 items 6–7; https://www.freedesktop.org/wiki/Software/systemd/inhibit/ (**I**); https://docs.kernel.org/accounting/psi.html (**I**); https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/ (**I**); the always-on rule (memory).
**Handover note**: The CI job already installs `libayatana-appindicator3-dev` (`ci.yml:410-413`), so the tray compiles; whether it shows depends on the desktop. A Flatpak build would need the Background and Inhibit portals instead of autostart files and logind: one more reason AJ-E2713's format decision comes first.

### AJ-E2712 Looks and reads like Linux: window chrome, icons, the Trash, words and keys | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2613
**Goal**: On Linux, AI Juggler fits the desktop: window decorations that match it, icons from the icon theme, the Trash, Ctrl shortcuts, and Linux words (Files, Settings, the keyring, This computer), from the shared platform words module.
**User story**: As a GNOME user, the window has the close button where my other apps have it, the shortcuts shown are the ones that work, and every place named is one I can find.
**Requirements**:
1. The shared platform words module (owner AJ-E2613) SHALL have Linux words: "This computer", "Files" or "your file manager" (from the desktop the seam reports), "Settings", the keyring's name, "System Monitor", "Trash".
2. Shortcuts SHALL show as "Ctrl+N", "Ctrl+Shift+N", "Ctrl+,", "Ctrl+Q", "Ctrl+?" (GNOME's keyboard-shortcuts key); keys the desktop takes (Super combinations; Ctrl+Alt+arrows on some desktops, used today by the Open now reorder, `ui/src/lib/shortcuts.ts:82-84`, `docs/guide/whats-open.md:56-57`) SHALL be checked in the lab and changed where taken.
3. The window SHALL use a Linux configuration overlay: server-side or GTK decorations with the UI's header below, or no decorations with a header bar that follows the desktop's button layout and supports drag, double-click to maximise and the window menu (audit 12.3).
4. App icons SHALL come from the icon theme named by each Desktop Entry's `Icon=` at run time (no icons in the repo, D-38); the letter fallback stays (audit 12.1).
5. Moving to the Trash SHALL use the `trash` crate per the Trash spec (audit 12.2).
6. Window mirroring stays unavailable on Linux (audit 13.1).
**UI/UX reasoning**: GNOME HIG (header bars, keyboard shortcuts, writing style, https://developer.gnome.org/hig/, **I**); KDE HIG for KDE (**I**); WCAG 2.2 AA (contrast, 2.4.7 focus visible, 2.5.8 target size) and the desktop's high-contrast and large-text settings.
**Performance budget**: the Linux words add ≤ 1 KB to the first load; an icon-theme lookup ≤ 20 ms, cached; no layout shift when decorations draw.
**Quality gates**: `just platform` (`mac-words` falls); `python3 scripts/plain_words_lint.py`; vitest for the module; `pnpm --dir ui build`; the a11y gate on the touched screens.
**Evidence/validation proof**: the preview's Linux mode gallery and a WebKit run of the touched screens on Linux (AJ-E2702), stored under `docs/qa/evidence/AJ-E2712/`; pass = zero Mac words in any Linux capture's DOM, axe 0 serious or critical, every listed shortcut pressed and working in the lab on GNOME and KDE.
**Visual testing**: Every screen with OS words in the preview's Linux mode, 1280×800 and 1440×900, Light and Dark; on the lab machine at 100 %, 125 % (fractional) and 200 % scaling under GNOME Wayland and KDE, with GNOME's high-contrast setting; macOS gallery unchanged (0 diffs).
**Files to touch**: `ui/src/lib/` (`platform.ts`, the Linux half), `ui/src/lib/shortcuts.ts`, `ui/src/components/ShortcutSheet.tsx`, `ui/src/lib/native-menu.ts`, `ui/src/screens/ThisMacScreen.tsx`, `ui/src/lib/machine.ts`, `crates/juggler-app/src/lib.rs`, `crates/juggler-app/src/quit.rs`, a new `crates/juggler-app/tauri.linux.conf.json`, `crates/juggler-app/src/manage.rs`, `crates/juggler-app/src/features/app_icons/`, `docs/guide/this-mac.md`, `docs/guide/whats-open.md`, `scripts/platform.allow`.
**Research basis**: audit 12.1–12.4, 13.1, 16.1–16.5, §3 C12, C16, §5.3 items 5 and 8; https://specifications.freedesktop.org/icon-theme-spec/latest/ (**I**); https://specifications.freedesktop.org/trash-spec/latest/ (**I**); D-50.
**Handover note**: WebKitGTK's fonts and metrics differ from Chromium's (the system font is Cantarell, Ubuntu or Noto): check for clipped labels at every scale, not just pixels. Don't branch on `XDG_CURRENT_DESKTOP` in the UI: the seam reports the file manager's name.

## Step 2 — ship it

### AJ-E2713 Linux packages, signing and updates | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2701, AJ-E2614
**Goal**: The formats AJ-E2701 chose are built by CI from the release descriptor, verified, and kept up to date: the AppImage through AI Juggler's updater, `.deb` and `.rpm` through a signed repository if chosen.
**User story**: As a Linux user, I install AI Juggler the way my distribution expects, and it stays up to date on my channel.
**Requirements**:
1. The shared release descriptor `release/platforms.json` (owner AJ-E2614) SHALL list the Linux triples, formats, signing and updater key; the workflows read it.
2. The AppImage SHALL carry `juggler-agent` as an `externalBin` for the Linux triple and update through the updater with a verified signature; the feed carries the Linux key in the shape AJ-E2701 chose (audit 14.1–14.2).
3. `.deb` and `.rpm` (if chosen) SHALL declare their dependencies (WebKitGTK 4.1, the AppIndicator library), and update through an apt / dnf repository signed with the project's key; the in-app updater SHALL say "updated by your package manager" for them instead of offering Install.
4. Flatpak and Snap SHALL not be offered unless the decision accepts what their sandboxes cost: no view of the host's processes without host access, a portal for every desktop feature; the decision record says why.
5. The three editions SHALL install side by side (desktop ids, data, keyring items, D-23).
6. Uninstall SHALL leave nothing but the data the user chose to keep.
**UI/UX reasoning**: GNOME HIG and the Desktop Entry spec for the app's entry (name, icon, categories, `StartupWMClass` matching the window so the dock groups it); Nielsen #1 for updates (the channel label, D-04/D-11).
**Performance budget**: the AppImage ≤ 120 MB, `.deb` ≤ 25 MB; cold start ≤ 1.5 s on the reference Linux machine (SPEC §1); an update check ≤ 2 s; an AppImage update ≤ 60 s including the backup.
**Quality gates**: the release scripts' self-tests with Linux cases (`release_version.py`, `promotion_rules.py`, `updater_sig_check.py`); `lintian` on the `.deb`, `rpmlint` on the `.rpm`; `cargo deny check`; the package smoke (AJ-E2702); a release rehearsal on Linux.
**Evidence/validation proof**: a rehearsal on clean Ubuntu and Fedora VMs: install, update N→N+1 verified, a bad signature refused, rollback to N, uninstall (`docs/qa/evidence/AJ-E2713/manifest.md`); pass = every step as stated, logs kept.
**Visual testing**: The update sheet's states on Linux (available, downloading, installed, failed with Try again; "updated by your package manager" for packages), 1280×800, Light and Dark, checked by eye on the VMs.
**Files to touch**: `release/platforms.json`, `.github/workflows/nightly.yml` (`:158`, the feed), `promote.yml` (`:203-205`), `release-rehearsal.yml`, a new `build-linux-release.yml`, `crates/juggler-app/tauri.conf.json`, `tauri.nightly.conf.json`, `tauri.dev.conf.json`, `crates/juggler-app/src/updates.rs`, `scripts/updater_sig_check.py`, `docs/RELEASING.md`, `docs/guide/install-and-updates.md`, `docs/guide/uninstall.md`.
**Research basis**: audit 14.1–14.5, §3 C14, §5.3 item 2; https://v2.tauri.app/distribute/appimage/, /debian/, /rpm/, /flatpak/ (**I**); §4 (Claude's desktop app ships as an apt repository, VP); https://docs.flatpak.org/en/latest/sandbox-permissions.html (**I**).
**Handover note**: The AppImage bundles WebKitGTK and is large; a too-old host glibc fails at start: build on the oldest distribution promised. Keep the repository's signing key and the updater key out of the repo.

### AJ-E2714 The first Linux Beta | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2702, AJ-E2703, AJ-E2704, AJ-E2705, AJ-E2706, AJ-E2707, AJ-E2708, AJ-E2709, AJ-E2710, AJ-E2711, AJ-E2712, AJ-E2713
**Goal**: The first Linux Beta ships only when the core works on the promised distributions and desktops: tools found, accounts imported, sessions run and are seen, stops whole and honest, switching keeps history, and every missing feature says so.
**User story**: As an early Linux user, the Beta does what its page says on my distribution, and what isn't there yet says so where I'd look for it.
**Requirements**:
1. A gate file `docs/qa/gate-linux-beta.md` SHALL list each criterion with its evidence link (the `gate_packet.py` format).
2. Core criteria on each promised distribution and desktop: tools found (AJ-E2706); import; two Claude accounts in parallel with zero writes to `~/.claude`; sessions seen in the Processes view; stops whole with zero false kills (AJ-E2705); switching A→B→Codex→A keeps history, tasks and context; honest usage; keep-awake (AJ-E2711); secrets in the keyring or a plain refusal (AJ-E2708).
3. Every feature not ready on Linux or on a given desktop SHALL say so where it would be, and the release notes SHALL list them by desktop.
4. The line's Beta rule (days of Nightly use, no open `blocker`) SHALL hold as AJ-E2701 set it.
5. The maintainer's walk on a Linux machine with the lab setup (read-only on real accounts) SHALL be recorded, its findings fixed or ruled.
**UI/UX reasoning**: Nielsen #1 and #10: "not on Linux yet" and "not on this desktop" lines, and a Linux section in each guide page that differs.
**Performance budget**: SPEC §1's budgets on the reference Linux machine: cold start < 1.5 s, < 150 MB RSS typical, < 1 % CPU for monitoring, 60 fps with 20 live sessions in WebKitGTK.
**Quality gates**: `just evidence`; every E27 task `DONE-EVIDENCED` or deferred with the maintainer's word; the a11y gate on the Linux captures; the load gate run on Linux.
**Evidence/validation proof**: `docs/qa/gate-linux-beta.md` with every criterion PASS and linked; pass = 100 % of the core criteria on every promised distribution and desktop, 0 open `blocker`.
**Visual testing**: The Linux gallery (AJ-E2712) re-run on the release candidate in WebKit, 1280×800 and 1440×900, Light and Dark, at 100 % and 200 %; axe 0 serious or critical.
**Files to touch**: `docs/qa/gate-linux-beta.md` (new), `docs/releases/`, `docs/guide/`, `README.md` (the platform row, once shipped).
**Research basis**: the 0.1 gate (memory: import, sessions, honest usage, This Mac and permissions on the real setup); `docs/qa/gate-beta.md`; `docs/RELEASING.md` (D-24); audit §5.3 ("deferred without harm").
**Handover note**: Don't claim Linux in the README or the site until this gate passes and the maintainer says so (memory: no public marketing yet).

### AJ-E2715 Linux Stable | Status: TODO | Priority P1 | Effort M | Depends: AJ-E2714
**Goal**: Linux Stable opens only after the Beta has been used for real on Linux, with its findings fixed and the stable rules met for this line.
**User story**: As a Linux user who wants a steady app, I install from the Stable channel knowing it has been through weeks of real use on my kind of desktop.
**Requirements**:
1. A gate file `docs/qa/gate-linux-stable.md` SHALL hold each criterion with evidence.
2. The Beta SHALL have been on its channel for the time AJ-E2701 set (D-24's 14 days is the default) with no open `blocker`.
3. Every Beta finding SHALL be fixed or ruled; every "not on Linux yet" line built or kept with the maintainer's word.
4. An update from the oldest Linux Beta in use to Stable SHALL be proven for each format, with a rollback.
5. The security review SHALL be re-run for the Linux-specific code: the AppImage trust rule and the stable bridge copy, the Secret Service, the repository signing.
**UI/UX reasoning**: The release channel policy (stable is deliberate); Nielsen #1: the channel in About.
**Performance budget**: as AJ-E2714, measured again; no regression > 10 % against the Beta's numbers.
**Quality gates**: `just evidence`; the full Linux jobs; the security findings closed.
**Evidence/validation proof**: `docs/qa/gate-linux-stable.md`, every criterion PASS and linked; pass = 100 %, 0 open `blocker`, 0 open security findings.
**Visual testing**: The Linux gallery re-run on the Stable candidate; 0 unexplained diffs against the Beta's approved baselines.
**Files to touch**: `docs/qa/gate-linux-stable.md` (new), `docs/RELEASING.md`, `docs/releases/`, `README.md`, `docs/SPEC.md` (§1's platform row).
**Research basis**: `docs/RELEASING.md` (D-24); `docs/plan/EPICS.md` M8-2; D-52.
**Handover note**: Linux Stable is independent of macOS's and Windows's; never pass one line's gate on another's evidence.
