# E15 — Spec gaps: what the specs promised and no task covered

> Source: the planned-vs-built audit of 2026-10-21, which read SPEC,
> PROPOSAL, EPICS, QUALITY, the expected workflow, the decision log, every
> ADR, the wireframes and both review reports against the code. 147 of 153
> tasks were built, but about 25 promised features had no task. The
> maintainer reviewed the list and approved it ("I agree with the full
> plan", 2026-10-21). The plan doc "AI Juggler — Built so far and E15 spec
> gaps" holds the review, the order and the versioning plan.
>
> New tools (ZCode, tool update checks, a real desktop tool, and the tools
> the maintainer added) are E16. Agent pools, messaging and task
> management are E17–E19, all written after their research.
>
> **Law for every task here:**
> - **Calm, plain words, the same patterns everywhere (D-25).** A new row,
>   sheet or toast reuses the closest existing component. Labels are short,
>   and detail sits behind disclosure.
> - **The app prepares; the user executes (ADR-0003).** Nothing here
>   switches, syncs, stops or launches by itself unless the user turned
>   that exact behaviour on.
> - **The user's own folders are theirs (ADR-0018).** No task writes to a
>   folder used where it is, except a file the user explicitly applies.
> - **Moved, never deleted.** Anything removed from disk goes to the Trash.
> - **Proof.** Every task ships tests that fail on the old code,
>   screenshots in both themes for UI, and an evidence manifest.

## Wave 1 — daily driving and the same setup everywhere (toward 0.1)

### AJ-E1501 Manage accounts: rename, remove, set default | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1024
**Goal**: An account can be renamed, removed, and made the default, without leaving AI Juggler.
**User story**: As a user who brought in five setups, I tidy them up: rename "personal-max" to "Max (personal)", remove the one I no longer use, and pick which account new sessions start with.
**Requirements**:
1. Rename SHALL change the account's label (and the plan, when the user gives one) everywhere it shows, with no restart. Duplicate labels are refused in words.
2. Remove SHALL forget an account used where it is and never touch its folder. For a copy, it asks: keep the copy's folder, or move it to the Trash.
3. Remove SHALL be refused while the account has live sessions, naming them. Its history stays readable and is marked "removed account".
4. A default account per project, and a global default, SHALL preselect in New session. Where none is set, the most recently used account is the default, as today.
5. Every change SHALL be recorded in an audit row (who, what, when).
**UI/UX reasoning**: Actions sit in the account card's menu (the existing pattern), with a confirm only for Remove.
**Performance budget**: each action < 200 ms; no rescan.
**Quality gates**: tests for both kinds of account (folder untouched, or copy moved to the Trash), live-session refusal, a duplicate label, history kept.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1501/manifest.md`.
**Visual testing**: the account menu, the remove sheet for each kind, the renamed account in the rail and on Usage.
**Files to touch**: `crates/juggler-core/src/storage/`, `crates/juggler-app/src/commands.rs`, `ui/src/screens/Profiles.tsx`, `ui/src/screens/NewSession.tsx`.
**Research basis**: Max-Depth report B-32 and §9.2; ADR-0018.
**Handover note**: Removing a copy never deletes; it uses the Trash (NSFileManager trashItem).

### AJ-E1502 Manage sessions and projects | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1014
**Goal**: Sessions and projects can be named, pinned, opened and tidied.
**User story**: As a user with twelve projects, I pin the three I'm on, rename a session to what it's doing, open a project in my editor, and forget an old one.
**Requirements**:
1. Sessions: rename (a label over the tool's own title) and pin to the top of the rail.
2. Projects: rename (a label only; the folder is never renamed), pin, open in the editor (the user's chosen editor in Settings, else Finder) and reveal in Finder. Forget hides a project until it is used again. Relocate repoints a moved folder.
3. Each action SHALL be reachable from the row's menu and from the project view, with the hidden actions from AJ-E1025 shown again once they work.
4. Pins and labels SHALL survive restarts and backups.
**UI/UX reasoning**: The same row menu as accounts (AJ-E1501). Pinned rows lead their group, each with a pin mark.
**Performance budget**: < 100 ms per action; the rail's sort stays under its existing budget.
**Quality gates**: per-action tests; relocate with a missing folder; forget then use again.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1502/manifest.md`.
**Visual testing**: rail rows with pins and labels; project-view actions.
**Files to touch**: `crates/juggler-core/src/storage/projects.rs`, `commands.rs`, `ui/src/components/WorkflowRail.tsx`, the project view in `RightInspector.tsx`.
**Research basis**: AJ-E408 Req.5; Max-Depth report §6.
**Handover note**: Reuse `ProjectRow`'s markup; don't add a second row style.

### AJ-E1503 One feedback system: toasts with undo, a quit warning, loading placeholders | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
**Goal**: Every action answers the same way, and the risky ones can be undone.
**User story**: As a user who just stopped the wrong session, I press Undo; as a user quitting with three sessions running, I'm told they keep running.
**Requirements**:
1. One toast component SHALL replace one-off result messages. It shows the result in words, with Undo where the action can be undone.
2. Stopping one session SHALL be undoable for 5 s: the stop is held, then sent. "Stop all" SHALL ask to confirm and is not undoable (the maintainer's plan, 2026-10-21). Mark reviewed, snooze, forget and remove get Undo too.
3. Quitting with live sessions SHALL say how many are running, and that they keep running, offering Quit anyway or Stop them first.
4. Lists SHALL show placeholders shaped like their rows while loading, never blank space.
5. Toasts SHALL be announced to screen readers and never steal focus.
**UI/UX reasoning**: Undo beats confirm for single actions (Max-Depth C1; the maintainer's plan).
**Performance budget**: a toast renders < 16 ms; the held stop adds exactly the undo window.
**Quality gates**: undo-before-send and send-after-window tests; the quit warning with 0, 1 and n sessions; an a11y gate on the toast.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1503/manifest.md`.
**Visual testing**: toast states; the quit sheet; placeholders for the rail, Home and Accounts.
**Files to touch**: `ui/src/components/` (Toast, Placeholder), `SwitchUndoToast.tsx` (folded in), `crates/juggler-app/src/lib.rs` (the quit request).
**Research basis**: Max-Depth report §8 and critic G1; SPEC §9.
**Handover note**: Fold `SwitchUndoToast` into the new component; keep its tests.
**Done (2026-10-21)**: `components/Toast.tsx` (`showToast`, `holdAction`, `ToastHost`), `lib/stop.ts` (the held Stop), `components/QuitSheet.tsx` with `crates/juggler-app/src/quit.rs`, and `components/Placeholder.tsx`. Req.3 as built: a terminal session runs on a terminal AI Juggler holds, so it **ends** when the app quits (closing the terminal hangs it up). The sheet says so, and that a desktop session keeps running. ⌘Q and the close button ask first. The Dock's Quit and logging out still quit without asking (macOS sends those past the menu; a follow-up). Forget and remove use this toast when AJ-E1501/E1502 land.

### AJ-E1504 Keys: a cheat sheet, inspector collapse, go-to keys | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: —
**Goal**: The keyboard reaches the main screens, and every shortcut can be looked up.
**User story**: As a keyboard user, I press ? to see every shortcut, ⌘⇧I to hide the inspector and ⌘1–4 to switch screens.
**Requirements**:
1. `?` (outside text fields) SHALL open a sheet listing every shortcut by area, generated from `lib/shortcuts.ts` so it can't drift.
2. ⌘⇧I SHALL collapse and restore the inspector, remembered per window. ⌘1–4 SHALL open Home, Accounts, Usage and Fleet.
3. No shortcut SHALL fire while typing in the terminal or a field (the existing rule).
**UI/UX reasoning**: The wireframes' global conventions.
**Performance budget**: n/a.
**Quality gates**: the sheet lists exactly the registered shortcuts; the typing-guard tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1504/manifest.md`.
**Visual testing**: the cheat sheet; the collapsed inspector.
**Files to touch**: `ui/src/lib/shortcuts.ts`, `ui/src/App.tsx`.
**Research basis**: the wireframe conventions; Max-Depth report §8.
**Handover note**: —
**Done (2026-10-21)**: `lib/shortcuts.ts` is the one registry (`SHORTCUTS`, `matches`). App's handler map is typed to cover every `owner: "app"` shortcut. Home and the inspector match through it, and `ShortcutSheet` renders it. The typing rule is general: no plain key or Ctrl-combo while typing in a field or the terminal, and ⌘ ones still fire except those a field owns (⌘Z). Two choices to note. ⌘1–6 used to switch the inspector's tabs (AJ-E406 Req.6), so they moved to ⌥⌘1–6 (amended there). The inspector *is* the work panel since ADR-0007 iteration 5, so "collapse" folds it to its header line (name, state, Stop) and keeps the open tab, which gives the terminal its room.

### AJ-E1505 Settings the SPEC promised: stuck thresholds, terminal font and scrollback | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E405
**Goal**: The thresholds SPEC §5.1 calls user-tunable are tunable.
**User story**: As a user whose builds run quietly for 10 minutes, I raise "Stuck after" so those builds aren't flagged.
**Requirements**:
1. Settings SHALL offer: Stuck after (no output for N minutes, with low CPU), the terminal font size and scrollback lines, each explained in one line, with a reset.
2. The supervisor's heuristics SHALL read the setting live, with no restart.
**UI/UX reasoning**: Settings › Sessions, behind the existing disclosure.
**Performance budget**: n/a.
**Quality gates**: the heuristic test with a changed threshold; reset.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1505/manifest.md`.
**Visual testing**: the Settings section.
**Files to touch**: `crates/juggler-supervisor/src/heuristics.rs`, `crates/juggler-app/src/settings_policy.rs`, the Settings screen.
**Research basis**: SPEC §5.1.
**Handover note**: —
**Done (2026-10-21)**: `settings_policy.rs` `session_prefs_get/set` (typed `SessionPrefsInfo`, ranges checked) is stored in app settings. The `HeuristicEngine` holds its thresholds behind a lock (`set_thresholds`), so "Stuck after" sets the no-output and transcript windows (SPEC's "same window") at the next tick, and it's applied over `heuristics.json` at launch. A per-tool override in `heuristics.json` (AJ-E1004) still wins for that tool. In the UI it's Settings › Sessions, with Terminal behind a disclosure; `TerminalView` applies font and scrollback live.

### AJ-E1506 Terminal tools | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
**Goal**: The built-in terminal does what a terminal user expects.
**User story**: As a user reading a long session, I search it, click a file path, copy a block, zoom the text, and pop the terminal out to a second screen.
**Requirements**:
1. ⌘F search with next and previous; clickable links and file paths (a path opens in the editor); copy; font size ±; jump to the bottom when scrolled up.
2. A Live / Ended / Reconnecting pill.
3. Maximize the terminal within the window, and pop it out into its own window, with the session still supervised in the main window (wireframe 05).
**UI/UX reasoning**: Wireframe 05 (approved); the xterm addons the ecosystem uses.
**Performance budget**: search over 10,000 lines < 100 ms; no frame drops while output streams.
**Quality gates**: tests per tool; the pop-out keeps one PTY.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1506/manifest.md`.
**Visual testing**: the toolbar, search, the maximized view and the pop-out window, in both themes.
**Files to touch**: `ui/src/components/TerminalView.tsx`, `ui/package.json` (xterm search and web-links addons), `crates/juggler-app/src/lib.rs` (the second window).
**Research basis**: wireframe 05; Max-Depth report §8 (terminal).
**Handover note**: Pin each addon's version, matching xterm 6.

### AJ-E1507 Rules and MCP sync across tools, with a diff preview | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E202
**Goal**: A project's rules and MCP servers can be made the same across tools and accounts in one reviewed click.
**User story**: As a user running Claude Code and Codex on one repo, I see that `CLAUDE.md` and `AGENTS.md` have drifted, pick the master, review the diff and apply it.
**Requirements**:
1. Per project, the rules files each tool reads SHALL be listed (`CLAUDE.md`, `AGENTS.md`, and later tools' own), with when each changed and whether they differ.
2. The user SHALL pick the master copy. AI Juggler shows a diff for every other file and writes only what the user applies. It never writes in the background.
3. MCP servers SHALL be shown per account and tool, and copied across on request, translated to the target tool's config format. Server env values that look like secrets are left out, and the user is told which.
4. The project view SHALL say "Rules in sync" or "2 files differ".
5. A guardrail test SHALL prove nothing is written without an apply.
**UI/UX reasoning**: One sheet, diff first (SPEC §7.5 "one click, diff-previewed").
**Performance budget**: listing < 300 ms per project; the diff < 100 ms for 2,000-line files.
**Quality gates**: fixtures for each pair of tools; the secret-filter test; the no-write guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1507/manifest.md`.
**Visual testing**: the sync sheet, the diff, and the project-view line.
**Files to touch**: `crates/juggler-handover/` (a sync module beside packs), the adapters' config writers, `commands.rs`, the project view.
**Research basis**: SPEC §7.5; EPICS M4-6; expected-workflow ("rules and guidelines need to work").
**Handover note**: The MCP formats: Claude `.mcp.json` / `settings.json`, Codex `config.toml` `[mcp_servers]`. E16 adds more.

### AJ-E1508 Copy shared settings into a new account | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1507, AJ-E1022
**Goal**: A new account starts with the user's usual MCP servers, permissions, plugins and status line.
**User story**: As a user adding a third Claude account, I tick "same as Work" and get my servers and permissions without copying files by hand.
**Requirements**:
1. Add account SHALL offer, for fresh accounts and copies, to bring over MCP servers, permissions, plugins and the status line from an account the user picks. Each item is listed and ticked by default.
2. Credentials, history and caches SHALL never be copied (a guardrail test lists the files).
**UI/UX reasoning**: Progressive disclosure: one line, "Start with the same settings as …", that expands to the list.
**Performance budget**: < 1 s.
**Quality gates**: the copy holds exactly what was ticked; the credential-file guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1508/manifest.md`.
**Visual testing**: the Add account section, collapsed and open.
**Files to touch**: the adapters' `prepare_account`, `FirstRun.tsx`.
**Research basis**: Max-Depth report (profile clone, settings sync).
**Handover note**: —

### AJ-E1509 Always on: keep working sessions going | Status: DONE-EVIDENCED | Priority P1 | Effort L | Depends: AJ-E505, AJ-E1503, AJ-E1505, AJ-E1527
**Goal**: What should stay on stays on, and comes back if it stops or crashes: a crashed session (when the user asked), the Mac while work runs, AI Juggler itself.
**User story**: As a user running an overnight job, I leave the Mac plugged in and walk away: it doesn't fall asleep mid-run, a crashed session resumes instead of ending the night, and if AI Juggler itself crashes it comes back with my sessions shown as they really are.
**Requirements**:
1. **Restart a crashed session.** Settings › Sessions has "Restart sessions that crash" (off by default), and a launch setting can override it. A session that exits with an error SHALL be started again with the same settings and resumed. Never after a quota stop, a stop the user made, or a sign-in failure. At most 3 tries in 10 minutes; then it stays ended and says why. Each try is recorded in the session's history.
2. **Keep the Mac awake while work runs.** While at least one session is working (running; not idle or waiting for the user), AI Juggler SHALL hold a macOS power assertion that prevents idle system sleep (the display may still sleep), named so `pmset -g assertions` shows "AI Juggler: N sessions working", and release it as soon as nothing is working. Settings › Sessions: "Keep the Mac awake while sessions work", on by default on power, off on battery unless the user turns that on too. It never prevents a sleep the user asks for (closing the lid, the Apple menu).
3. **No App Nap while sessions run.** While anything is supervised, AI Juggler SHALL declare an activity (`NSProcessInfo beginActivity`, user-initiated, allowing idle sleep) so its monitoring and timers aren't throttled in the background.
4. **AI Juggler stays running.** "Open AI Juggler at login" is opt-in, through `SMAppService` (a login item), offered in the welcome's Make it smoother card and in Settings. If AI Juggler quits unexpectedly (a crash, not the user quitting), it SHALL be started again (a launchd agent with KeepAlive on an unsuccessful exit, or the login item's relaunch) and pick up its sessions. On that restart, the sessions it was supervising are shown as they are: re-attached if still alive; otherwise ended with the reason and, where requirement 1 allows, resumed.
5. **Honest about it.** A small row on This Mac says what is keeping the Mac awake and why, with its setting one click away.
**UI/UX reasoning**: ADR-0003 (revised 2026-10-22): each behaviour is one the user turned on, or a default they can see and change; none touches an account or starts new work. One row on This Mac, the settings in Settings › Sessions (D-25: depth on demand).
**Performance budget**: taking or releasing the assertion is off the UI thread and under 10 ms; no polling beyond the existing 5 s tick.
**Quality gates**: tests for every never-restart case and for the 3-in-10-minutes limit; the power assertion taken and released (`pmset -g assertions` in a test, or a seam); a crash test: the app killed with SIGKILL comes back up with its sessions shown correctly; nothing left running after any test (reaper, `pgrep` clean).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1509/manifest.md`.
**Visual testing**: the settings, the welcome's login-item row, the This Mac row (awake and not), and a session's history with a restart.
**Files to touch**: `crates/juggler-supervisor/src/lifecycle.rs`, launch settings, Settings › Sessions, a macOS power module, the login item, `SmootherCard.tsx`, This Mac.
**Research basis**: SPEC §1 and §5.4; EPICS M2-7; the maintainer's "always on" request (2026-10-22); Apple: IOKit power assertions, `NSProcessInfo` activities, `SMAppService`.
**Handover note**: Widened from "Auto-restart a crashed session (opt-in)" (2026-10-22); the ID is kept so the counts don't change. Built (2026-10-22): Req.1–3, Req.4's crash relaunch (a watchdog of the app's own binary rather than launchd's KeepAlive, which covers only processes launchd started) and Req.5; see the evidence manifest. Req.4's login item: the audited `juggler-login-item` crate (D-26, approved by the maintainer 2026-10-22); its live registration needs an installed build.

### AJ-E1510 System notifications | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E611, AJ-E1408
**Goal**: What needs the user reaches them outside the window, when they want it to.
**User story**: As a user in another app, I get a macOS notification when a session waits for me, and clicking it opens that session.
**Requirements**:
1. Optional macOS notifications SHALL be offered for: waiting for you, stuck, quota reached, finished, and This Mac tight. Off by default, with a choice per kind.
2. Quiet hours and digest settings (AJ-E1012) SHALL apply. Each kind is sent at most once per session per state.
3. Clicking a notification SHALL bring the app forward on that session.
**UI/UX reasoning**: Calm by default; the user picks what may interrupt them.
**Performance budget**: n/a.
**Quality gates**: rate and quiet-hours tests; routing on click. Granting the permission is the maintainer's step on a real Mac.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1510/manifest.md`.
**Visual testing**: Settings › Notifications.
**Files to touch**: `crates/juggler-app/Cargo.toml` (the Tauri notification plugin), `alerts.rs`, the Settings screen.
**Research basis**: Max-Depth report §8 (OS notifications).
**Handover note**: Tests never grant the macOS permission themselves.

### AJ-E1511 Open a context pack in any tool | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E202, AJ-E208
**Goal**: A saved pack can start a new session in any tool and account.
**User story**: As a user who exported a pack last week, I open it and continue the work in Codex.
**Requirements**:
1. Open pack SHALL validate the file before anything is written and show what it holds, and what redaction withheld.
2. The user picks the tool, account and project; the session starts primed with the pack, labelled with its fidelity.
3. The pack's lineage SHALL be recorded in the switch graph.
**UI/UX reasoning**: The same summary as the switch preview.
**Performance budget**: validation < 2 s; it inherits the E2 budgets.
**Quality gates**: round trips for every pair of tools; malformed-pack refusals.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1511/manifest.md`.
**Visual testing**: the open sheet with its withheld list.
**Files to touch**: `crates/juggler-handover/src/pack.rs`, `commands.rs`, `SwitchFlow.tsx`.
**Research basis**: ADR-0009; AJ-E1011 (its first half).
**Handover note**: The colleague side of AJ-E1011 stays demand-gated.

### AJ-E1524 A welcome Home for new installs | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1014
**Goal**: Someone who has just installed AI Juggler is welcomed, sees what it does for them, and is encouraged to bring their accounts in.
**User story**: As a new user, I open AI Juggler and Home tells me in a few warm lines what it's for, that it found Claude Code and Codex on my Mac, and what it will do for me, and it invites me to find my accounts.
**Requirements** (the maintainer's request, 2026-10-21):
1. Home SHALL show it whenever there are no accounts (also after first run's Not now), replacing today's small "Bring in your accounts to start." block. Usage's and Fleet's empty states point back to it in one line.
2. Content: a headline that says what AI Juggler is for; a line from the scan ("We found Claude Code and Codex on this Mac", or "No AI coding tools found yet" with install links); 3–4 one-line benefits with icons (every session in one list with what needs you first; how much each account has left, and carrying on in another; what your Mac is running; setups used where they are); a trust line (never stores your passwords or keys, never sits between you and the tool). D-25 plain words, no codes.
3. Actions: primary "Find my accounts" (first run), secondary "Add a fresh account", and a quiet "How it works" disclosure with three steps. No new screens.
4. Design: generous spacing, the three-balls mark as a calm hero with one gentle animation that honours reduced motion, and benefit cards in Home's card style. Both themes, both densities, 390–1440 px. Existing tokens only. Before and after screenshots go in the PR.
5. Accessibility: one h1, a sensible reading order, the animation decorative, every action reachable by keyboard; the a11y gate is clean.
**Quality gates**: vitest (zero accounts shows it, one account hides it; the found-tools line matches the scan; each action opens the right flow); new visual baselines; walked by hand as a brand-new user with a throwaway HOME.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1524/manifest.md`.
**Files to touch**: `ui/src/screens/Home.tsx`, a `Welcome` component, `Usage.tsx`, `FleetScreen.tsx`.
**Handover note**: Tool names come from the catalog (AJ-E1525), never from ids.
**Done (2026-10-21)**: `components/Welcome.tsx` (`Welcome`, `WelcomePointer`) on Home's no-accounts branch. The found line comes from the install probe, and the lead names the first-run tools from the catalog. "Add a fresh account" opens Add account on its fresh step (`FirstRun startAt`). The mark rises in once (`.welcome-rise`) and doesn't move at all under reduced motion. Before and after screenshots are in the evidence folder.

### AJ-E1525 ToolSpec: tools as data | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
**Goal**: ADR-0019 migration step 1: every layer reads a tool's facts from its adapter's spec, and no screen or command branches on a tool id.
**Requirements**:
1. Each adapter SHALL supply a `ToolSpec`: id, display name, vendor, mark, install link, sign-in words, kinds, the layout's first-run groups, rules files, flow flags, terms notes. The catalog composes it with the adapter's capabilities and isolation keys.
2. One `tools_catalog` command SHALL serve the specs.
3. Display names, install links, sign-in words, tool groups and the first-run tool list SHALL move out of `commands.rs`, `FirstRun.tsx`, `providers.ts` and `App.tsx` into the specs.
4. A tool-literal lint in `just verify` and CI SHALL fail on a tool-id literal outside the adapters, adapter data and tests, starting from an allowlist that only shrinks.
5. A test SHALL render first run and Add account from a catalog holding an unknown fake tool.
**Quality gates**: the catalog tests (every tool complete, an unknown one neutral); the lint's self-test; the unknown-tool render test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1525/manifest.md`.
**Files to touch**: `crates/juggler-adapter/src/spec.rs`, the adapters, `crates/juggler-app/src/features/tools/`, `ui/src/lib/tools.ts`, `FirstRun.tsx`, `providers.ts`, `App.tsx`, `scripts/tool_literal_lint.py`.
**Research basis**: ADR-0019; ENGINEERING-GUIDE A2, A4.
**Done (2026-10-21)**: `ToolSpec` in `juggler-adapter/src/spec.rs` (domain data; `HarnessAdapter::spec()` defaults to the neutral spec); Claude Code's and Codex's specs in their adapters. `features/tools/` serves `tools_catalog` (async), composing each spec with the adapter's capabilities and isolation keys. On the UI side, `lib/tools.ts` holds the catalog once (loaded before the first render) and `providers.ts` reads it. First run's groups, default labels, desktop-app words, install commands and tool list come from the specs. `scripts/tool_literal_lint.py`, in `just verify` and CI, starts from an allowlist of 150 backend and bridge-mock sites in 15 files that only shrinks; the UI screens hold none. Not moved yet: the handover pair data (`fidelity.rs`, `validation.rs`), the usage pipeline's per-tool sources and the bridge's preview mocks, all allowlisted.

### AJ-E1526 Tool and company marks everywhere | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1525
**Goal**: Wherever a tool or its company is named, its mark sits beside the name.
**Requirements** (the maintainer's request, 2026-10-21):
1. Each ToolSpec names its mark, and a vendor mark where it differs.
2. Marks come from the `simple-icons` package (CC0, version-pinned) where the brand is in it; otherwise from the vendor's press kit, only where its guidelines allow third-party apps to use it. Each mark's source, licence and guideline link go in `docs/research/brand-marks.md`. A forbidden or missing mark falls back to a neutral letter mark.
3. A mark is never altered beyond its guidelines. The default rendering is monochrome in `currentColor`; brand colour only where the guidelines require it.
4. One `ToolMark` component everywhere a tool or company is named: Add account and first run, account cards, rail rows, the inspector's tool chip, the tool view's header, Usage, This Mac's tool lines. The mark id comes from the catalog; no per-tool branches.
5. Third-party marks live in their own assets folder, excluded from the MIT licence; TRADEMARKS.md says other companies' names and logos belong to their owners, identify compatibility only, and imply no endorsement. Bundled locally; nothing is fetched at runtime.
**Quality gates**: every catalog tool resolves to a mark or the fallback; an unknown tool gets the fallback; marks are decorative beside the visible name; re-rendered baselines with notes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1526/manifest.md`.

## The 0.1 blockers — the core verifiably working on a real multi-account Mac

The maintainer's rule after testing 0.0.12 on their own Mac (2026-10-21):
0.1 doesn't ship until the core works on a real multi-account setup. These
come before the rest of Wave 1's polish. Any task that shows sessions,
projects or usage carries an oracle comparison in its evidence; a hand
walk alone isn't enough.

### AJ-E1531 A realistic replica for walks and tests | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
**Goal**: Every fix is walked and tested on a Mac shaped like a heavy real user's, without touching anyone's real data.
**User story**: As a developer, `just replica` gives me a throwaway HOME with six Claude profiles, Codex, months of history and live sessions, and the app opens on it.
**Requirements**:
1. A generator (`scripts/replica.py build`, `just replica`) SHALL build a throwaway HOME with real-shaped files and no real content:
   - `~/.claude` plus six profiles, with settings (two with a status line of their own), `.claude.json` and fake credentials;
   - `~/.codex` and `~/.codex-work`;
   - months of transcripts across 13+ git projects, including three clones of one repo (`acme-web`, `acme-web2`, `acme-web3`) and agent worktrees under `<repo>/.claude/worktrees/<name>`, some since removed;
   - Codex rollouts with `token_count` `rate_limits`;
   - project icons of every kind AJ-E1532 reads, including an oversized one and a "logo" that isn't an image;
   - shell aliases pointing at the profiles.
   It is deterministic for a seed, and it refuses a HOME that isn't empty and isn't a replica.
2. `scripts/replica.py live` SHALL run, while a walk goes on:
   - live `claude` and `codex` sessions: the fake harness's `write` mode under those names, appending real-shaped lines;
   - a Claude desktop stand-in, and a ChatGPT-like helper that isn't a Claude tool;
   - optional memory pressure.
   Each of these exits when the walk ends.
3. The generator writes its own ground truth (`~/.replica/truth.json`): accounts, projects, conversations per account and project, worktrees and live sessions. An oracle such as `scripts/qa/real_truth.py` is checked against it.
4. `just replica-walk` opens AI Juggler Dev on the replica HOME. Every hand walk on release/0.1 uses it from now on.
5. (Moved to AJ-E1529, Req.8.)
**Quality gates**: the generator's selftest (determinism, the refusal, the shape of each file the app reads); the app's scan finds every replica account, checked against `truth.json`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1531/manifest.md`.
**Files to touch**: `scripts/replica.py`, `crates/juggler-adapters/src/bin/fake_harness.rs` (the `write` mode), `justfile`.
**Handover note**: Never point it at a real HOME. Real-data checks are the QA session's, read-only, with redacted counts.

### AJ-E1529 Sessions started outside AI Juggler | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1531, AJ-E1502
**Goal**: The rail shows every session on each account, not only the ones AI Juggler started.
**User story**: As someone with six sessions open across my accounts, I open AI Juggler and see all of them, live ones first, under their projects.
**Requirements** (the maintainer's real-setup test, 2026-10-21):
1. Each account's recent conversations SHALL be listed from its own files (Claude `projects/**/*.jsonl`, Codex `sessions/**/rollout-*.jsonl`): title, project, last activity and tokens. AI Juggler never writes to those folders.
2. A conversation is live when a running tool process uses that account (This Mac's detection of tools AI Juggler didn't start) and its transcript was written recently.
3. They SHALL show in the rail under their project, with a quiet "outside AI Juggler" mark, read-only: no stop and no cleanup (D-08). Actions: open it in its app or terminal where possible, and "Continue in AI Juggler" (a resume launch in that account, on the user's click).
4. The rail SHALL update when the history backfill finishes, through a typed event, with no reload.
5. Worktrees map to their repo, even after they're removed:
   - from the transcript's recorded cwd under `<repo>/.claude/worktrees/<name>`, when `<repo>` has a `.git`;
   - otherwise from the transcript's `gitBranch` and the repo's known worktrees.
   They merge into the repo's group, and AI Juggler's own agent worktrees never show as projects.
6. A project's view SHALL list its conversations, including those started outside AI Juggler.
7. The rail stays within its budget at 50+ sessions.
8. Playwright journeys that show sessions, projects or usage read a fixture made from the replica by these readers, never hand-written (from AJ-E1531).
**Quality gates**: the counts match an oracle. AJ-E1529 merges only when the app's projects, conversations and live sessions match `scripts/qa/real_truth.py` on the replica, and on the maintainer's real data. The real-data pass is run read-only by the QA session, with redacted counts and mismatches only.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1529/manifest.md`, with both oracle comparisons.
**Files to touch**: `crates/juggler-app/src/features/conversations/`, `crates/juggler-adapters` (each adapter's conversation reader), `ui/src/components/WorkflowRail.tsx`, the project view.

### AJ-E1532 Telling projects and sessions apart | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1529, AJ-E1526
**Goal**: With many projects, each is recognisable at a glance.
**User story**: As someone with fifteen projects, I tell acme-web from payments-api by its mark and colour before I read the name.
**Requirements** (the maintainer's request, 2026-10-21):
1. A project mark, decided by the backend and served as data (ADR-0019):
   - (a) the project's own icon, found read-only: a Tauri, Electron or Xcode app icon, `public/favicon.*`, `logo.*` or `icon.*` at the root, or `.github/*logo*`. It must be at most 512 KB and a known image type, rendered locally;
   - (b) otherwise a two-letter monogram on a colour from an OKLCH palette of about 12 well-separated hues, AA in both themes, stable per project.
   The user may override the colour, emoji or letters from the project's menu.
2. Session rows show the project mark, or a coloured stripe with the mark when a group is folded, plus the tool mark and the account dot. The same components serve the rail, Home, History and the inspector (D-25).
3. Clones of one repo share its mark, with a small number.
4. Account avatars, at the maintainer's request ("which account, which profile is serving that session right now, or that project"). The avatar is round, in the account's colour, with 1–2 initials from its label and its name as a tooltip.
   - **Session rows**: the serving account's avatar sits beside the tool mark, and changes when the session switches accounts. A session started outside AI Juggler gets the avatar of the account AJ-E1529 matched.
   - **Project rows**: up to 3 stacked avatars of the accounts that worked there, then "+N". An account serving a live session there gets a ring, the same live cue sessions use.
   - **Elsewhere**: the same component on the Accounts cards, the page headers (AJ-E1533) and, later, the menu bar.
   - **Colour and shape**: avatar colours come from the project palette, but the avatar is round and the project mark is a rounded square, so the two are never confused.
   - **Accessibility**: the avatar is decorative, and the row's accessible name includes the account ("on Work Max").
**Quality gates**: palette contrast and a minimum ΔE between neighbouring hues; icon discovery on fixtures, refusing oversized and odd files; vitest; baselines with 15+ projects; walked on the replica.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1532/manifest.md`.

### AJ-E1530 Usage for sessions outside AI Juggler | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1529
**Goal**: Each account's window % is measured even when its sessions weren't started by AI Juggler.
**User story**: As someone who mostly starts Claude in my own terminal, I let AI Juggler read my usage once, and Runway shows my real window.
**Requirements**:
1. Offer one user-applied change per Claude account, "Let AI Juggler read your usage". It adds AI Juggler's status-line reader to the account's own settings, chained after any status line already there. The change is previewed as a diff, applied only on click, and removable.
2. Until then: tokens are shown as Measured, and the window % says "Not measured yet", never a made-up figure.
3. Codex limits come from its rollouts' `rate_limits` (the bug-fix branch's parser, merged from main).
**Quality gates**: the diff preview; apply and remove on the replica leave every other setting byte-for-byte; the oracle comparison of tokens and % on the replica.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1530/manifest.md`.

### AJ-E1527 Make it smoother: the optional permissions on the welcome | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1524
**Goal**: A new user sees the optional permissions early, as a reminder that the experience is better with them, never as a requirement.
**Requirements** (the maintainer's request, 2026-10-21):
1. The welcome Home has a "Make it smoother" card listing the optional permissions (jump to a session's own window; a live preview of each session), each with its state and its Allow or Open System Settings action. It uses the same rows as first run's access step, from one shared component (`OsAccessRows`).
2. It is never required. "Not now" hides it until something needs a permission, and then only that row shows, in context.
3. After the welcome, while a permission is still missing and not dismissed, Home shows a small, dismissible reminder card ("Live previews are off. Turn on"). It is never a modal.
4. Screen Recording's "Reopen AI Juggler" state, from the bug-fix branch's state module, comes through the same component.
**Quality gates**: vitest; baselines; walked as a brand-new user on the replica.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1527/manifest.md`.

### AJ-E1528 A This Mac page | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1403, AJ-E1409
**Goal**: "See what's using it" opens a page for it, rather than scrolling.
**Requirements** (the maintainer's request, 2026-10-21):
1. A This Mac screen in the shell registry. "See what's using it" and This Mac's header open it; it is also reachable from More.
2. From the existing read model, with nothing new measured:
   - the Mac's state, in words and meters;
   - what's using it, grouped by tool, then by session or window, with tools AI Juggler didn't start marked;
   - heavy sessions, with links;
   - recommendations and recent alerts.
   Controls appear only for what AI Juggler owns (D-08).
3. The same rows, meters and type as Home's panel, with detail behind disclosure (D-25). Home's panel stays a summary and links to the page.
4. The alert subject comes from the bug-fix branch's single function, never a second one.
**Quality gates**: vitest and Playwright navigation; a11y; baselines in both themes and densities; an oracle comparison of the process groups on the replica; a hand walk under memory pressure, read-only.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1528/manifest.md`.

### AJ-E1533 One page model | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1529
**Goal**: Every item opens the same kind of page, and a fact never disagrees with itself.
**Requirements** (the coordinator's screen-by-screen review against wireframes 01–09, 2026-10-21):
1. One shell for every item type, registered per type (ADR-0019). An identity header carries:
   - the type label (Session, Project, Account, Tool, This Mac);
   - the mark and the name;
   - a breadcrumb to the parent;
   - 2–3 type facts.
2. Per-type tab sets, with no empty tabs:
   - **Session**: the terminal as the body. Tabs: Usage (tokens, cost, window share and pace, with Quality merged in), Processes, and History only when it has events. Actions live in the header and its ⋯ menu.
   - **Project** (wireframe 03): the header has the path, git branch, ahead/behind, dirty, Open in Finder or editor, and New session here. The body is the sessions list (live, then recent, across accounts and tools, including those from AJ-E1529). Tabs: Overview (worktrees with the shared-checkout warning, accounts used, sessions ever), Usage (by account and tool), and Rules & MCP (AJ-E1507). Quality and Processes go.
   - **Account** and **Tool** pages, reached from their names on Accounts and from Tools under More.
3. One source per fact, so the header and the tabs agree. These disagree today:
   - "2 helpers" vs "No subprocesses";
   - "CPU 4%" vs "collecting";
   - project Usage says none while its session has 19k tokens;
   - project Processes counts 0 sessions while 1 runs.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1533/manifest.md`. It holds before and after screenshots of every page at 1280 and 1024 in both themes, walked on the replica, and checked against the wireframe.

### AJ-E1534 Orientation | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1533
1. The rail shows a clear selection for every row type (a filled row, an accent bar and `aria-current`), scrolled into view.
2. The top-nav highlight comes from the current page. Today History showed Accounts, and a session page showed Home.
3. Tab memory is per type, never carried from a session to a project. Today the bar showed Processes while the page showed Overview.
4. Each type has its own mark shape:
   - project: a rounded square (AJ-E1532);
   - session: a status dot with a terminal glyph;
   - account: a round avatar with initials;
   - tool: the tool's mark (AJ-E1526).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1534/manifest.md`, as for AJ-E1533.

### AJ-E1535 Plain words and consistency | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
1. Remove "COMP", "tok", "F1", "transplant", "switch-back ↺", "root: … copy" and "imported sessions: 0", and New session's footer "No folder picked — No launch settings…". A footer shows only where it applies.
2. A finished session's row shows no account "% left".
3. This Mac's "under 1 MB" for a Claude session: find out what it measures, and fix it or say it plainly.
4. New session opens without delay.
5. A plain-words lint in CI: a banned list, with an allowlist that only shrinks.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1535/manifest.md`.

### AJ-E1536 History you can scan | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1533
1. Each row shows a title or the first prompt, the tool, account and project marks, and opens the session's page.
2. Delete moves into the row's ⋯ menu.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1536/manifest.md`.

### AJ-E1537 Context menus and ⋯ menus from one action registry | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1502, AJ-E1533
**Goal**: A desktop user right-clicks anything and gets the actions that matter, with the same items in the ⋯ menu and from the keyboard.
**Requirements** (the maintainer's request, 2026-10-21):
1. One action registry (ADR-0019). Each item type declares its actions once: session, project, account, tool, history entry and This Mac row (pools and tasks later). Each action has an id, a label, an icon, when it's enabled and why not, whether it's destructive, a shortcut and a handler. Features contribute actions; no screen keeps its own copy of a menu.
2. Three triggers, one menu:
   - right-click opens a native macOS menu (Tauri's menu popup);
   - a ⋯ button on each row and in each page header;
   - the context-menu key or Shift+F10 on a focused row.
   The ⋯ and keyboard menus may be in-app, with the same items in the same order.
3. The items per type:
   - **Session**: Open; Focus its window or terminal; Continue on another account; Rename; Pin; Copy resume command; Reveal its project; Stop (destructive, with Undo, AJ-E1503).
   - **Project**: Open; New session here; Open in editor; Reveal in Finder; Copy path; Rename; Pin; Forget; Set default account.
   - **Account**: Open; Start a session; Sign in; Edit launch settings; Rename; Set as default; Remove.
   - **Tool**: Open; Check for update; Its accounts.
   - **History entry**: Open; Replay; Continue; Archive; Delete (destructive).
   - **This Mac row**: Go to the session. A tool AI Juggler didn't start says so and has no actions.
   A disabled item shows why: a tooltip in the in-app menu, a reason line in the native one.
4. Destructive items sit last, in their own group, and confirm or undo (AJ-E1503). Sessions started outside AI Juggler get only read-only actions and "Continue in AI Juggler" (D-08).
**Quality gates**: the same items for every trigger; keyboard access; a11y (`role=menu`, focus return); a snapshot of each type's menu; a disabled item's handler never runs. Right-click checked by hand on the replica's rows.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1537/manifest.md`.

### AJ-E1538 Search the words of sessions started outside AI Juggler (opt-in) | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1536
**Goal**: History's search finds words said in any session, including the ones started outside AI Juggler, for people who choose to index them.
**Context** (P20, the coordinator's option (c), 2026-10-22): today History finds sessions started outside AI Juggler by their title, project, account, tool, branch and date. Only AI Juggler's own sessions have their words indexed. History says so in its description and when nothing matches.
**Requirements**:
1. Off by default. Settings › Privacy offers "Search the words of every session", with the index's size on this Mac before and after it is built, and a switch.
2. The index is built in the background from each tool's own files (read-only), newest first and incrementally, as AJ-E1529's index reads them. Turning it off deletes the index.
3. Search shows the matching words in context for outside sessions, as it does for AI Juggler's.
**Trade-offs** (why it is opt-in):
- **Disk:** on the maintainer's Mac, about 6.6 GB of Codex rollouts alone; the index would likely take 0.5–1 GB.
- **A copy of all conversation text:** it would sit in AI Juggler's data folder, beside the tools' own files.
- **Privacy:** that copy is one more place holding everything said, including what the tools keep and later delete (it is removed with the tool's file and when the switch is turned off).
**Quality gates**: the size shown before turning it on matches the built index within 10%; turning it off leaves no index behind; the first-pass speed of AJ-E1529 is unchanged with it on.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1538/manifest.md`.

### AJ-E1544 LiveStatsCollector in its own module | Status: DONE-EVIDENCED | Priority P2 | Effort S | Depends: AJ-E2011
**Goal**: `juggler-supervisor/src/supervisor.rs` fits its size budget again.
**Context** (the coordinator, 2026-10-22): main's #126 grew supervisor.rs past its cap, and main has no size budget, so the cap was re-based at that merge (2848 → 2962 lines). Fix batch 4 (#130) changes the same collector code; splitting it before #130 is in release/0.1 would only make conflicts.
**Requirements**:
1. Once #130 has merged into release/0.1, `LiveStatsCollector` and its helpers move into their own module of `juggler-supervisor`, behaviour unchanged.
2. supervisor.rs's cap in `scripts/size_budget.allow` goes back down to its new size.
**Quality gates**: `just verify` (with `load_budgets`) passes unchanged; the size budget passes with the lower cap.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1544/manifest.md`.

## Switching you can rely on — a 0.1 core

The maintainer (2026-10-22): "One of the main features that must work in 0.1.* is switching and migrating sessions between different accounts of the same provider and in between totally different providers." The journey that must work: a simple coding task on Claude account A, switched to Claude account B, then to Codex, then back to A, with the chat history, tasks, context and project state carried at every step. Before a switch, AI Juggler checks the target's MCP servers, skills and the rest, and offers to set up what's missing; the user sees everything and approves each item.

What the code does today (mapped 2026-10-22 on release/0.1): a same-tool switch copies the transcript and resumes it, but the source keeps running (the freeze choice is dropped, `pipeline.rs` `let _ = freeze`), and Claude's `<session>/subagents/` folder makes the copy fail. A cross-tool switch passes only user and assistant text as one command-line argument (96 KiB), dropping tool calls, results, tasks and older turns, and a switch back from it probably carries nothing. Nothing detects MCP servers, skills, commands, subagents, rules or settings. No pair has ever run live: every fidelity-register cell is UNVALIDATED and nothing can fill one. Research: `docs/research/switching-capabilities.md`.

### AJ-E1539 A live switching lab | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E808
**Goal**: The real tools prove each switch, and the proof fills the fidelity register.
**Requirements**:
1. An ignored, env-gated test (`crates/juggler-app/tests/switch_lab_live.rs`, `just switch-lab`) runs in AI Juggler Dev's data model over a throwaway data folder, in a new throwaway project (a git repo under the user's Dev folder or a temp folder), with real Claude Code and Codex on accounts the person running it names. The maintainer allowed their own accounts on 2026-10-22, in a lab project only.
2. The journey: a small coding task on Claude A (headless `claude -p --session-id`, a file plus its test, a task list); switch A→B with AI Juggler's own switch code; continue on B; switch B→Codex; continue; switch back to A. After each switch a probe asks something only the earlier conversation can answer (a name chosen in step 1, the last test result, the open tasks) and checks the answer, the files and the task list.
3. Each step records the tool versions, the commands, the probe and its answer (redacted), and writes the register cell's verdict (`run: live`). A failure records a drop note.
4. Before and after snapshots (paths, sizes, mtimes, never contents) of every account folder involved. The only differences allowed are the lab sessions' own files and the changes the user approved in the preflight (AJ-E1542).
5. It never signs in, never reads credentials, and never starts anything outside the lab folder.
**Quality gates**: the harness fails a step it can't verify (never "pass by default"); the snapshot diff is part of the evidence.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1539/manifest.md`, plus the cells in `docs/qa/evidence/AJ-E808/cells/`.

### AJ-E1540 Same-tool switches done right | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1539
**Goal**: Claude→Claude and Codex→Codex continue exactly where the conversation was, and the source stops cleanly.
**Requirements**:
1. Freeze is real on a local switch: "Finish this turn" waits for the source to go idle, then stops it; "Stop now" stops it at once. Only then is the transcript copied. A source started outside AI Juggler is never stopped; the switch says the original keeps running and the copy is a separate continuation.
2. Claude's whole session travels: the `<id>.jsonl`, its `<id>/` folder (`subagents/`, `tool-results/`), its `file-history/<id>/` and its task list (`tasks/`), into the target's matching places, without overwriting anything.
3. The target resumes in the source's working folder (never the target profile's default folder) and is linked to its transcript, so a later switch or switch-back finds it.
4. Codex: the rollout resumes by id in the other CODEX_HOME (the lab proves it); a compressed rollout (`.jsonl.zst`) is decompressed into the copy rather than refused.
**Quality gates**: the register's claude→claude and codex→codex cells PASS from the lab; a test with a real-shaped subagents folder.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1540/manifest.md`.

### AJ-E1541 Cross-tool continuity | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1539
**Goal**: Moving to Codex, or back to Claude, continues the work, not just the chat.
**Requirements**:
1. A handover file replaces the single command-line argument: written in AI Juggler's data folder for the switch (`switches/<edge>/HANDOVER.md` plus `state.json`), and the target is started with a short prompt that points at it.
2. It carries: the conversation (newest first within a budget the target can read, with older turns summarised by count and topic, never silently dropped), what each tool call did and its result in one line, the files changed (from git, with the diff stat), the task list mapped onto the target's (Claude tasks ↔ Codex's plan), the working folder and branch, and the rules files in play.
3. The new session is linked to its own transcript as soon as it exists, so switching back finds what happened while away.
4. Switch-back from another tool resumes the original with the away delta, including the files changed and the task list's state.
5. What can't carry (subagents, Claude-only tools, thinking) is listed in the switch screen before the user confirms.
**Quality gates**: the register's claude→codex, codex→claude and switch-back cells PASS from the lab; the handover file never holds a secret the redaction policy removes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1541/manifest.md`.

### AJ-E1542 The capability preflight | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1507
**Goal**: Before a switch, the user sees what the target account lacks and sets it up in one reviewed step.
**Requirements**:
1. Detection, by parsing files (never by starting a tool or an MCP server), for both sides: MCP servers (Claude user, local and project scopes; Codex `config.toml` and trusted project layers), skills, custom commands and prompts, subagents, user-level rules (CLAUDE.md, `rules/`, AGENTS.md), plugins, and the project's rules files.
2. The switch screen lists what the target is missing, per item: set it up (translated to the target's format), skip, or "needs you" (an OAuth login or a value to fill in). Items are ticked by default only when the copy is exact and safe.
3. Secrets are never copied: literal secret values in env or headers become a named variable the target must provide, and the screen says which. OAuth tokens never move; the screen offers the tool's own login.
4. Each write is previewed, made through the tool's own command where one exists (`claude mcp add-json`, `codex mcp add`) or as a new file that never overwrites, recorded in the switch's manifest, and undone from the same place.
5. ADR-0018 is amended: these are user-approved writes into the target account, like AJ-E1530's.
6. Things with no equivalent (claude.ai-synced skills and connectors, plugins across tools, SSE servers on Codex) are listed as "can't be set up there", with the reason.
**Quality gates**: a no-write guardrail test without an approval; translation fixtures per pair; the secret filter test; the lab's journey runs with one MCP server and one skill on A only, and checks they're offered and set up on B and Codex.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1542/manifest.md`.

### AJ-E1543 One switch screen | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1540, AJ-E1541, AJ-E1542
**Goal**: The least work for the user, with everything in view.
**Requirements**:
1. One screen per switch: where it goes, what carries over and what doesn't (in plain words), the capability checklist from AJ-E1542, the freeze choice, and a single confirm.
2. Progress in steps the user understands, and a clear result: "Continued on B", with switch back and undo.
3. A pair the lab has proven says so; one it hasn't stays labelled Beta. Never "lossless".
4. Walked by hand on the replica and once on the real lab, with the three questions (the user's problem solved, clear without explanation, consistent).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1543/manifest.md`.
## Wave 2 — usage you can act on, Fleet and support

### AJ-E1512 Sessions left and a burn-down chart | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E608
**Goal**: Usage answers "how much more can I do?" in the units people plan with.
**User story**: As a user mid-afternoon, I see "about 4 sessions left in this window", and when it runs out.
**Requirements**:
1. Each account SHALL show sessions left in the window, from the user's own average session cost, with the forecast's confidence label.
2. A burn-down chart per account: used over the window, the forecast to exhaustion, and the reset time.
**UI/UX reasoning**: The three walls (Max-Depth personas): time-to-exhaust, per-session burn, sessions left.
**Performance budget**: chart render < 50 ms.
**Quality gates**: tests on fixture histories; the confidence label carried through.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1512/manifest.md`.
**Visual testing**: Usage in both themes and densities.
**Files to touch**: `crates/juggler-core/src/quota_forecast.rs`, `ui/src/screens/Usage.tsx`.
**Research basis**: EPICS M3-5; Max-Depth report §3.
**Handover note**: —

### AJ-E1513 Prepare the pack before quota runs out | Status: DONE-EVIDENCED | Priority P2 | Effort S | Depends: AJ-E611, AJ-E205
**Goal**: When quota runs out, the switch takes seconds.
**User story**: As a user near my limit, the switch is ready when I need it.
**Requirements**:
1. Crossing the quota warning SHALL stage the context pack in the background, refreshed as the session goes on. Nothing switches by itself.
2. A staged pack SHALL be thrown away when the session ends or the window resets.
**UI/UX reasoning**: Invisible until used; then "Ready to continue on Work".
**Performance budget**: staging stays off the UI thread, within E2 budgets.
**Quality gates**: the no-auto-switch guardrail; cleanup.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1513/manifest.md`.
**Visual testing**: the ready state in the limit dialog.
**Files to touch**: `crates/juggler-handover/src/pipeline.rs`, `alerts.rs`.
**Research basis**: SPEC §7.
**Handover note**: —

### AJ-E1514 Budget caps | Status: DONE-EVIDENCED | Priority P2 | Effort M | Depends: AJ-E609
**Goal**: A user who pays per client sees spending against a cap.
**User story**: As a consultant, I set $300 a month for a client's projects and hear about it at 80%.
**Requirements**:
1. An optional monthly cap in USD, per account or per group of projects.
2. Needs you SHALL warn at 80% and 100%. Nothing is stopped.
**UI/UX reasoning**: Inferred demand (the agency persona); built only if the maintainer keeps it in.
**Performance budget**: n/a.
**Quality gates**: thresholds; the never-stop guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1514/manifest.md`.
**Visual testing**: the cap on Usage and in Needs you.
**Files to touch**: `cost_ledger.rs`, `Usage.tsx`.
**Research basis**: Max-Depth report (Priya; inferred).
**Handover note**: Pools (E17) may replace "group of projects".

### AJ-E1515 Fleet: start a session on another Mac, and usage per machine | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E705
**Goal**: The paired Macs act like one place to work.
**User story**: As a user with a Mac Studio and a laptop, I start a session on the Studio from the laptop, and see each machine's usage.
**Requirements**:
1. New session SHALL offer the paired machines; the launch goes over the existing link (the node side handles `LaunchRecipe`).
2. Usage and This Mac's fleet view SHALL filter and total by machine.
**UI/UX reasoning**: One extra picker; nothing new to learn.
**Performance budget**: remote launch to tracked < 5 s on a LAN.
**Quality gates**: loopback tests over two nodes; offline refusal in words.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1515/manifest.md`.
**Visual testing**: the machine picker; per-machine totals.
**Files to touch**: `crates/juggler-link/`, `commands.rs`, `NewSession.tsx`, `Usage.tsx`.
**Research basis**: EPICS M7-2 and M7-4; PROPOSAL §6.6.
**Handover note**: The two-Mac proof stays the maintainer's (AJ-E710).

### AJ-E1516 Crash report sender and native crash dumps | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1107
**Goal**: Crash reports the user agreed to actually reach us.
**User story**: As a user who chose "Ask", I review a report after a crash and send it.
**Requirements**:
1. A sender for the scrubbed reports, to the address compiled into release builds only, honouring Never / Ask / Always and the daily limits.
2. Native crash dumps for crashes outside Rust, scrubbed the same way.
**UI/UX reasoning**: The existing preview-before-send.
**Performance budget**: no cost when Never.
**Quality gates**: the scrub corpus; limits; nothing sent in debug builds.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1516/manifest.md`.
**Visual testing**: unchanged screens.
**Files to touch**: `crates/juggler-app/src/crash.rs`.
**Research basis**: AJ-E1107.
**Handover note**: The service and its address are the maintainer's step; the code waits behind it.

### AJ-E1517 Export diagnostics | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: —
**Goal**: A bug report comes with what we need, and nothing it shouldn't have.
**User story**: As a beta tester, I export diagnostics, look them over and attach them.
**Requirements**:
1. One zip with logs, versions, settings and the schema version, scrubbed like crash reports and previewed before saving.
2. No transcripts, credentials or paths outside `~` placeholders.
**UI/UX reasoning**: Settings › About & updates, next to Report a bug.
**Performance budget**: < 3 s.
**Quality gates**: the scrub corpus run over the zip.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1517/manifest.md`.
**Visual testing**: the preview.
**Files to touch**: `crates/juggler-app/src/`, the Settings screen.
**Research basis**: SPEC §8.
**Handover note**: —

### AJ-E1518 A trust page and vendor terms on Add account | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: —
**Goal**: People can see what AI Juggler touches and what the vendors allow.
**User story**: As a security-minded user, I read one page on what is read, stored and sent; as a new user, I see the tool's terms before adding an account.
**Requirements**:
1. One trust page, in the app and in `docs/`, saying what AI Juggler reads, stores and sends, and what it never touches. Each line is checked against a guardrail test.
2. Add account SHALL link the tool's terms, with one line on what several accounts mean under them.
**UI/UX reasoning**: PROPOSAL §5.7.
**Performance budget**: n/a.
**Quality gates**: every trust claim maps to a test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1518/manifest.md`.
**Visual testing**: the page; the terms line.
**Files to touch**: `docs/`, `ui/src/screens/`.
**Research basis**: PROPOSAL §5.7; Max-Depth report (trust page).
**Handover note**: Vendor terms are research claims; label them per AJ-E810.

### AJ-E1519 Migration guides and importers | Status: DONE-EVIDENCED | Priority P2 | Effort M | Depends: AJ-E307
**Goal**: People coming from other tools get their setup across.
**User story**: As a Claude Squad user, I import my sessions' worktrees and keep going.
**Requirements**:
1. Guides for Vibe Kanban, Crystal, Claude Squad and cc-switch.
2. Importers for Vibe Kanban and Claude Squad setups, through the existing degraded-import path (AJ-E307).
**UI/UX reasoning**: Import from another tool, in Add account's More.
**Performance budget**: inherits AJ-E307.
**Quality gates**: a fixture per importer.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1519/manifest.md`.
**Visual testing**: the import path.
**Files to touch**: `crates/juggler-app/src/onboarding/`, `docs/guide/`.
**Research basis**: Max-Depth report §12 (GTM).
**Handover note**: —

## Wave 3 — robustness and quality

### AJ-E1520 Subtask count and the agent subtask tree | Status: TODO | Priority P2 | Effort M | Depends: AJ-E401
**Goal**: The status row's subtask field (I-08) means something.
**User story**: As a user, I see that a session spawned four subagents and what each is doing.
**Requirements**:
1. Fill I-08 from the transcript's agent calls (Claude's Task tool, Codex's equivalent).
2. Show the tree in the inspector.
**UI/UX reasoning**: Detail behind disclosure.
**Performance budget**: parsing within the transcript budget.
**Quality gates**: fixture transcripts.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1520/manifest.md`.
**Visual testing**: the row and the tree.
**Files to touch**: the transcript parsers, `commands.rs` (I-08), `RightInspector.tsx`.
**Research basis**: SPEC I-08.
**Handover note**: —

### AJ-E1521 Robustness: a database busy timeout and an adapter conformance kit | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: —
**Goal**: No read fails during a long write, and every adapter keeps the same rules.
**User story**: As a maintainer adding a tool, I run one suite that says whether the adapter is safe.
**Requirements**:
1. The core storage SHALL set a busy timeout (the stats store already does).
2. One conformance suite every adapter SHALL pass: isolation, no credential copies, pinned env, refusing option-like ids, sessions, and usage honesty.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the suite runs for Claude Code, Codex and the fake.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1521/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-core/src/storage/`, `crates/juggler-adapters/tests/`.
**Research basis**: Max-Depth report §5 (conformance kit).
**Handover note**: E16's adapters must pass it. Built: the kit is `crates/juggler-adapters/tests/conformance/mod.rs`; a new adapter adds a `Subject` and one `conformance!` line in `adapter_conformance.rs`. It found and fixed three defects (Claude Code's copy carried `.credentials.json`; the shell line brought back a removed key from Launch settings; the fake broke five rules). A tool whose pinned variables point at different folders overrides `pinned_env`.

### AJ-E1522 Quality debt | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E803
**Goal**: The open quality targets are met.
**User story**: As a maintainer, every gallery family is pixel-checked, and the adapters meet their coverage target.
**Requirements**:
1. Screenshot checks for the remaining gallery families, each made deterministic first.
2. Adapter test coverage from 77.9% to 85% (AJ-E210).
3. The end-to-end test with two concurrent sessions (AJ-E306).
**UI/UX reasoning**: n/a.
**Performance budget**: the visual gate stays under 10 minutes on CI.
**Quality gates**: the gates themselves.
**Evidence/validation proof**: the AJ-E803, E210 and E306 manifests.
**Visual testing**: n/a.
**Files to touch**: `ui/tests/visual/`, `crates/juggler-adapters/tests/`.
**Research basis**: AJ-E803, E210 and E306.
**Handover note**: Done in three PRs (2026-10-23). The gallery: one state per family in the visual gate's `gallery` project, 90 families and 137 baselines, pinned clocks first, and a state that compares nothing fails (`docs/qa/evidence/AJ-E803/manifest.md`). Adapter coverage 87.0 % (#167, the AJ-E210 manifest). Two concurrent sessions end to end, which found and fixed duplicate session ids on concurrent launches (#164, the AJ-E306 manifest).

### AJ-E1523 North-star metric: weekly juggling loops completed | Status: DONE-EVIDENCED | Priority P2 | Effort S | Depends: AJ-E1103
**Goal**: We can tell whether AI Juggler does its job.
**User story**: As the maintainer, I see how often a session hit a limit and carried on elsewhere.
**Requirements**:
1. Count a "juggling loop" (a limit reached, then the work continued on another account or tool) in opt-in usage statistics and in the user's own stats.
2. Document it in the public metrics document.
**UI/UX reasoning**: Personal stats only; never gamified.
**Performance budget**: n/a.
**Quality gates**: the counting tests; the stats guardrails.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1523/manifest.md`.
**Visual testing**: personal stats.
**Files to touch**: `crates/juggler-stats/`, `docs/usage-stats.md`.
**Research basis**: Max-Depth report (north star).
**Handover note**: —
