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
**User story**: As a user who brought in five setups, I tidy them up: rename "acme-max" to "Max (personal)", remove the one I no longer use, and pick which account new sessions start with.
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
5. (D-47, 2026-10-27.) A session another app runs (the Claude app, the Codex app, a terminal) SHALL be pinnable too, from its row's menu and from the pin shown on the row's hover and focus; its pin is AI Juggler's own record (`app_settings` `rail_order`, by the rail's key `c:<account>/<conversation>`), never written to the app's files. Pinned rows lead their group in every order.
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
**Done (2026-10-21)**: `components/Toast.tsx` (`showToast`, `holdAction`, `ToastHost`), `lib/stop.ts` (the held Stop), `components/QuitSheet.tsx` with `crates/juggler-app/src/quit.rs`, and `components/Placeholder.tsx`. Req.3 as built: a terminal session runs on a terminal AI Juggler holds, so it **ends** when the app quits (closing the terminal hangs it up). The sheet says so, and that a desktop session keeps running. ⌘Q asks first. The close button asked first too until D-112 (0.4.5): the close button and ⌘W now hide the window, nothing quits and every session keeps running; the Dock icon, the menu-bar item or opening AI Juggler again brings it back. Since D-113 (0.4.5) the Dock's Quit, Activity Monitor's and a script's `quit` ask as ⌘Q does (AI Juggler answers the quit Apple event itself); logging out, restarting and shutting down still quit without asking, by design. Forget and remove use this toast when AJ-E1501/E1502 land.

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

### AJ-E1538 Search the words of sessions started outside AI Juggler (opt-in) | Status: DONE-EVIDENCED | Priority P2 | Effort M | Depends: AJ-E1536
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
7. D-51 (2026-10-28): this project's notes (the tool's own memory, Claude Code's `projects/<project>/memory/`) are offered as one more row when the target lacks some, never ticked for the user; a copy adds only what's missing, keeps a note that differs as it is, merges the index without repeats, writes private files confined to the notes folder, and is undone from the same place. A tool that keeps no notes gets no offer, and What's carried says they stay with their own account. Evidence: `crates/juggler-adapters/tests/claude_memory.rs`, `crates/juggler-app/tests/switch_memory.rs`, `ui/src/components/switch-notes.test.tsx`.
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

### AJ-E1516 Crash report sender and native crash dumps | Status: DONE-EVIDENCED | Priority P2 | Effort M | Depends: AJ-E1107
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
**Handover note**: The service and its address are the maintainer's step; the code waits behind it. Built: `crash_send.rs` (sends `queued/` over HTTPS, 5 a day, as stored), compiled only when `build.rs` sees a release build with `AI_JUGGLER_CRASH_ENDPOINT` (`cfg(crash_sender)`), started from `lib.rs`; `crash_native.rs` turns macOS's `.ips` report of this app's own crash into a kept report at the next launch. Setting the address: add it to the release workflow, then update the Privacy line and `crash_reports_address_reaches_only_the_release_build` together.

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

### AJ-E1520 Subtask count and the agent subtask tree | Status: DONE-EVIDENCED | Priority P2 | Effort M | Depends: AJ-E401
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
**Research basis**: SPEC I-08; Codex's multi-agent handlers and rollout policy (read 2026-10-23, in the manifest).
**Handover note**: Built: `juggler_adapters::subtasks` (the record and the tail reader), each tool's lines in its parser (`claude_transcript::subtask_line`, `codex_transcript::subtask_line`), `features::subtasks` (the tick, the row's count, `subtasks_list`), `SubagentsSection.tsx` and the rail row's note. The count is the subagents still running; the list shows them all. A new tool adds its `subtask_line` and nothing else. Left: when a Claude subagent started in the background ends (check live), and Codex's depth-2 subagents.

**0.2 (2026-10-27, the maintainer: "build the Subagents view into 0.2.0"; then "all of these features need to work for all of the apps we support … it should all follow adapter strategy")**:
3. Every adapter SHALL answer whether its tool's subagents can be listed (`HarnessAdapter::subagent_support`: supported, naming the records it reads, or not supported with the reason) and list a session's from the tool's own records (`subagent_reader`): id, type, task, status (running/done/failed/stopped/unknown, D-06), started and ended, parent and depth, background, its own transcript. Read-only (D-28/D-37), for every session the app shows, a desktop app's included where it shares the CLI's records. The conformance kit (AJ-E1521 check 11) holds every adapter, and every future one, to it.
4. Commands: a paged list (`subagents_list`, `paging.rs` keyset) and a subagent's conversation (`subagents_conversation`, the session's own last-turns reader), redacted (D-14).
5. A **Subagents** tab on the session page, shown only when the tool supports it and the session started some; rows in the task row's shape, nested, newest first, a skeleton at once; a row opens its conversation in the same chat view, with a way back; the rail's "+N agents" opens the tab.
6. (2026-10-27, D-48, the maintainer: "Yes, fold the Overview's older Subagents list into the new Subagents tab and retire the old one.") The tab SHALL be the only list: the session notes' older disclosure and `subtasks_list` are retired, and a session that started some shows one quiet line, "N subagents · M running › Subagents", that opens the tab (wireframe screen 05). The row's count (I-08, `subtask_count`) SHALL come from the same adapter readers as the tab, for every tool that lists subagents: live sessions only, each reader refreshed within its own budget at most every 2 s off the rail's path, `null` until read and once the session ends.

**0.2 handover note**: Built: `juggler_adapter::subagents` (the contract), one `*_subagents.rs` per adapter (Claude Code: `subagents/` meta + transcripts and task notifications; Codex: child rollouts by `parent_thread_id`, both multi-agent shapes; Kimi Code: `state.json` agents + `subagent.*` wire records; Grok Build: `subagents/<id>/meta.json`, nested through child sessions; OpenCode: child sessions in the store, read-only), `features::subagents` (readers kept for the pages open last, keyset tree order, redaction, running vs unknown), `SubagentsTab.tsx`, `useSubagents.ts`, the rail link, kit check 11 with `tests/subagent_fixture/`. Evidence: the manifest's 0.2 section. Left: Kimi, Grok and OpenCode subagents' conversations (when the session page reads those tools' own), and a live lab check of those three.

**Req.6 handover note (2026-10-27)**: The old list is retired and the count has one source. Removed: `features::subtasks` (its tick, `subtasks_list`, its DTOs and generated types), `SubagentsSection.tsx`, `lib/ipc/subtasks.ts`, the dev-bridge route and the IPC allow line. `features::subagents::count` follows the live sessions on its own 2 s tick: each one's reader (shared with its tab: `Readers` keeps the tab's last 8 and every live session's, and drops a session's when it ends) is refreshed once a pass within its budget, and a count is published once its reader has caught up; rows read only what was published. It counts what the tab calls running (nested and background ones included, which the old count left out). The rail's "+N agents" no longer falls back to child processes. `juggler_adapters::subtasks` stays: it is the Claude Code and Codex readers' start-and-end log (its old-only `label` and `running()` are gone). The session's notes keep one line to the tab (`SessionNotes.tsx`, counts from the page's one `useSubagentCounts` read). Evidence: the manifest's "One Subagents view" section.

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
**Handover note**: Done in three PRs (2026-10-23). The gallery: one state per family in the visual gate's `gallery` project, all 91 families and 141 baselines, one fixed clock first, and a state that compares nothing fails (`docs/qa/evidence/AJ-E803/manifest.md`). Adapter coverage 87.0 % (#167, the AJ-E210 manifest). Two concurrent sessions end to end, which found and fixed duplicate session ids on concurrent launches (#164, the AJ-E306 manifest).

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

### AJ-E1545 Welcome leads into a guided setup wizard (find your apps, permissions one card each with live grant detection, keep it running, the tour), and every brand mark is the real app icon | Status: IN-PROGRESS (0.5.1: built; open: R3 PL1 C9 and the critique round (the permission clips), R7 critique/baseline round) | Priority P0 | Effort L | Depends: AJ-E1524, AJ-E1527
**Found** by the maintainer testing the 0.5.0 nightly (2026-11-23): the Welcome left a newcomer on their own, "Make it smoother" sat on it as a card, and the logo on it was the small three-dot IconJuggler rather than the app's real icon.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) A newcomer on first launch, often running several AI subscriptions, who knows neither AI Juggler's words nor macOS's privacy settings. "Get started" promises a guided path with an end; the step indicator says how long. The app icon must look like the one in the Dock, so the window reads as this app.
(2) Each step says what it does and why in one sentence before any control. Each permission card shows the dialog macOS will show and what to click. Bot's step names both AIs, the size (1.8 GB), the free disk, and a recommended choice. Nothing assumes they know "profile", "instance", "TCC" or "model".
(3) Setup and first value in one place: apps found, access granted, Bot ready, kept running. It replaces the Welcome's scattered cards (Make it smoother moves in), never a second copy of Settings: each step links to where it lives later.
(4) A step with nothing to do here (no apps found, no permission this setup can use, no AI possible) says so in one line with Skip, or is left out (D-127). Every step has Skip for now, and the wizard can be left and resumed.
(5) Accessibility, Automation per target app, Notifications, the login item, app-data access for the tools' folders, the Apple helper's first start: each primed on its card before Allow (PAT-OSP, AJ-E1547's gate).
Step 1, "Your apps" (the coordinator's reasoning, 2026-11-23): (1) people think in apps, not "setups" or "profiles", and expect each app they use once, with its own icon; (2) one row per recognisable app, Desktop and Terminal as chips, plain status words, no badges or monospace paths up front; (3) it replaces 0.5.0's "Find my apps" wholly (its "Desktop apps" section merges into the rows); (4) tools installed but not set up are a collapsed, unticked group; (5) reading the installed apps' icons and folders is primed by AJ-E1547; (6) in 0.5.0 the same profile appeared twice looking unrelated, "Bring in 13" didn't match "11 setups", not-set-up tools were ticked by default (creating folders), and a terms wall scared people; providers' terms stay a closed disclosure the user opens.
Step 2, "Sign in": (1) a newcomer signing in expects the provider's own page, not a terminal; (2) one primary ("Open <provider>'s sign-in page"), the code large, one line on what to check; (3) the terminal stays reachable under Show details, not repeated; (4) a sign-in that can't be read keeps the terminal with a plain line; (5) opening the browser goes through AJ-E1547's gate; (6) in 0.5.0 raw terminal output with Find, A−/A+ and Maximize read as technical and risky.
(6) Watch for: a system dialog appearing before its card; Allow showing nothing because macOS's dialog went behind the window; a step that can't be skipped; a download starting without a click; "done" before the grant is seen; the brand mark looking unlike the Dock icon.
**Requirements**:
1. The Welcome leads with "Let's get you set up"; its primary "Get started" opens a step-by-step wizard (rich mode of the design system: sticky header and footer, a step indicator). Nobody is left on their own.
2. **Step 1, "Your apps"** (the coordinator's redesign of 0.5.0's "Find my apps", 2026-11-23): title "Which AI apps do you use?", with "We found these on your Mac. Tick the ones AI Juggler should show. Nothing is copied or changed: they keep working exactly as now." Grouped by tool (Claude, Codex, Grok Build, ZCode, Muse Code…), each group's header with the tool's official icon read at run time from the installed app (a CLI-only tool uses its vendor's app icon if installed, else a terminal glyph). One row per thing the user recognises, with a Desktop chip and/or a Terminal chip: a desktop profile and its Claude Code folder are ONE row ("Claude app · <profile>" [Desktop], its sessions' folder as a detail, [Terminal] when terminal use of that folder is seen), paired by AJ-E2327's evidence, never by name; the default Claude app and `~/.claude` are one row, "Claude app (main)"; the Codex app and CLI sharing `~/.codex` are one row with both chips. No separate "Desktop apps" section. Status in plain words ("Signed in", "Used today", "Not signed in: you can sign in later"), no Guessed/Likely/Found badges; an uncertain pairing shows only "Is this right? Change". "Installed, not set up yet" is a collapsed group, unticked, saying what ticking does. "Use them where they are" is the default, said once in the subtitle; "Make separate copies" moves under More options. No terms wall and no terms note opened for the user (the maintainer's correction, 2026-11-23: "It is not our responsibility to shove that in user face"): one collapsed disclosure at the screen's foot, "Providers' terms", closed by default, nothing ever opens it automatically (not even with two Accounts of one provider ticked); opened, it shows a line per provider with its link (PAT-SER-7). A sticky footer: "Bring in N apps" with N exactly the ticked rows; secondary "Look again", "Add from another folder…", "Set up a new app instead"; select-all per group. The step indicator: Your apps · Sign in · Mac access · Meet Bot · Keep running.
3. Step 2, "Let AI Juggler work with your Mac": each permission its own card (what it does, a short clip of the effect, "macOS will ask you; optional; change it later in Settings"); Allow opens the system prompt and the step sees the grant live (✓); Skip for now always there. Only the permissions this setup can use (D-127). "Make it smoother" leaves the Welcome: its card becomes this step.
4. Step 3, Keep it running: the login item and keep awake, by the always-on rules, only where they apply.
5. **Bot, front and centre (the maintainer's amendment, 2026-11-23).** Bot (E38) is the lead card of the Welcome's "What it does for you": full width above the four, its own loop (asking Bot about your sessions and getting a cited answer, fixture data) and a "Beta" chip. A wizard step "Meet Bot", after the permissions and before Keep it running, says in plain words what Bot does (answers about your sessions, usage and apps; cites every answer; never acts) and sets it up by the router's own availability: Apple's model ready → one click "Turn on Bot"; no Apple model → the local model offer with its real size and the free disk, started only on the user's explicit yes (D-131, the disk rules); no engine possible → one line saying what it needs, and Skip, never a broken option (D-127). Skip for now always there; Settings › Models can do it later.
6. The last step offers the guided tour (AJ-E1546).
7. Every large brand mark is one SVG of the real app icon (three balls on the arc, lavender top, indigo left, cyan right, with the glow; `crates/juggler-app/icons/icon.png`); small sizes a simplified arc and balls that stays the same mark.
8. **Step 2, "Sign in"** (the coordinator's usability call, 2026-11-23): a tool whose sign-in is a device code or a URL shows a friendly card, not its terminal: "Open <provider>'s sign-in page" as the primary, the code large with Copy, one line "Check the page shows this code, then approve", and a calm "Waiting for you to approve…" that turns into ✓ Signed in when it is seen. The live terminal moves under "Show details", collapsed. A tool whose sign-in can't be read keeps its terminal, with one plain line above it. Each row has the tool's official icon and a Terminal chip. The header says "Sign in to N apps. Each opens its own sign-in page; AI Juggler never sees your password". Each tool's sign-in output is read in its adapter (no per-tool branch outside adapters), with a fixture test per tool; opening the browser is ours, so AJ-E1547's gate applies to any prompt it can raise.
**Quality gates**: `just keep-going verify-static`; unit tests for the steps, the live grant and Skip; the a11y gate; an Opus side-by-side critique in both themes against the design system and the mock-ups before any baseline; a Sonnet newcomer-persona walk of first launch on a fresh Dev data folder, through "Meet Bot"'s three branches (Apple ready; the local offer, declined, no real download; no engine).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1545/manifest.md`.

### AJ-E1546 A guided tour on the real UI, offered once, and short clips for every "What it does for you" card and "How it works" step | Status: IN-PROGRESS (0.5.1: built; open: R3 the a11y gate and PL1 C13, R4 PL1 C15) | Priority P1 | Effort L | Depends: AJ-E1545
**Found** by the maintainer testing the 0.5.0 nightly (2026-11-23): nothing showed a newcomer around, and the "What it does for you" and "How it works" words had nothing to show the effect.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) A newcomer just after setup, or an existing user after an update. "Take the tour" promises a short, skippable walk over the real screens, with Bot as the guide; "New in 0.5.x" promises only what changed.
(2) Each stop names the thing and says what it is for in plain words. A clip shows the effect where words aren't enough. Back, Next, Skip, the progress and Esc are always visible.
(3) It teaches where things are, once, on the real UI rather than in a separate help page. Clips are the same assets the Welcome's cards and How it works use, no second set.
(4) A stop whose target isn't on screen (an empty state) shows its clip instead. Bot's stop shows "Set up Bot" when no AI is ready (D-144), and "Ask Bot about this" is hidden then. A finished tour is never offered again.
(5) None: it calls no system API. Clips play inline and muted.
(6) Watch for: a coach mark pointing at nothing; focus lost when the tour ends; a clip autoplaying with sound or under Reduce Motion; the tour offered again after it was finished; a step that only works with an AI set up.
**Requirements**:
1. The tour is offered at the end of setup and from Help › "Take the tour": coach marks anchored by element id to the real UI (Open now, Needs you, a session page, usage and runway, Continue in another app and Move, Pools, Processes, Settings). A step whose target isn't on screen (an empty state) shows its clip instead. Back, Next, Skip, a progress bar, Esc ends it.
2. The steps live in a registry, each with a feature id and the version it came in; completion is stored per id. A completed tour is never offered again; after an update, only the new features' steps are offered, once, as a calm "New in 0.5.x · Show me".
3. Keyboard and a focus trap, reduced motion honoured, the a11y gate.
5. **Bot is the tour's guide (the maintainer's amendment, 2026-11-23).** Coach marks carry Bot's mark and voice; the first stop is the Bot panel. Every step is scripted, so the tour works with no engine; "Ask Bot about this" shows on a step only when Bot is ready.
4. Clips: a 3–6 s loop for every "What it does for you" card and every "How it works" step; muted, inline, paused off screen, a still poster under prefers-reduced-motion; lazy-loaded, about 400 KB or less each, in both themes. Recorded from generic fixtures only (never real names), with what is already installed (Playwright's cached ffmpeg); anything that needs a download is asked first.
**Quality gates**: `just keep-going verify-static`; unit tests for the registry, per-id completion and the "New in" offer; the a11y gate; clip size budget; an Opus side-by-side critique in both themes; the newcomer-persona walk through the tour, covering Bot's three branches (Apple ready; the local offer, declined, no real download; no engine).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1546/manifest.md`.

### AJ-E1547 No system prompt the user wasn't told about first: one registry of every OS prompt, priming in setup, one just-in-time gate every trigger goes through | Status: IN-PROGRESS (0.5.1: built; open: R5 PL1 K1-K4 and Z1) | Priority P0 | Effort L | Depends: AJ-E1545, AJ-E3822
**The maintainer's rule (2026-11-23)**: AI Juggler never raises a system prompt the user wasn't told about first ("users will think it's a bug or their Mac is crashing").
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone, the first time AI Juggler needs something from macOS. They expect our app, not macOS, to tell them what is coming and why, so a system dialog never feels like a crash or a bug.
(2) Our sheet names the dialog in macOS's own words, shows a picture of it, says which button to click and what happens if they decline. Afterwards it says plainly whether access was granted, and if not, where to change it later.
(3) One gate and one registry instead of each feature asking on its own. Setup primes what this setup needs; the gate covers anything later.
(4) An entry this setup can't use is never primed or shown (D-127). Background work never prompts: it waits, or raises a calm needs-you item.
(5) This item is the answer for every other item: the registry lists each prompt, its trigger and its words.
(6) Watch for: a dialog appearing with no sheet before it (the walk logs each one); a sheet with no dialog after it, because access was already granted; Not now treated as a decline forever; the same sheet shown twice.
**Requirements**:
1. **One registry of every OS prompt AI Juggler can cause**, built by code audit and Apple's documentation, never by resetting privacy settings (no `tccutil`: a security setting). Each entry: id, the code path that triggers it, what the system shows in its words, why we need it, what to click, what happens when declined. Expected: Accessibility; Automation per target app (System Events, Terminal, iTerm and the others we script); Notifications; the login item's banner; "would like to access data from other apps" (app-data protection when reading the tools' folders); Documents, Desktop and Downloads for projects there; "differs from previously opened versions" (AJ-E3822); whatever the Apple-model helper or the local model can raise. Undocumented ones researched too.
2. **Priming in setup**: AJ-E1545's "Let AI Juggler work with your Mac" step primes every entry this setup needs, each card showing what the system will show (a picture of the dialog) and what to click.
3. **One just-in-time gate**, an OS seam (`juggler_platform`; Windows UAC and SmartScreen fit later), that every triggering code path goes through. An entry not primed yet gets our own sheet first ("macOS will now ask… here's why, click X", Continue and Not now) before the call; afterwards the outcome is detected and said (granted, or declined with how to change it later). Bot's first open shows "Setting up Bot" before its helper starts.
4. **Enforcement**: a lint or test so a prompt-triggering API (TCC-guarded calls, AppleScript to other apps, notification authorisation, SMAppService, a helper's first start) can't be called outside the gate.
5. **Validation**: the newcomer-persona walk notes every system dialog that appears; one with no priming sheet before it is a blocker.
**Quality gates**: `just keep-going verify-static` with the new lint; registry tests (every entry complete, every gated path named); the gate's sheet and outcome tests; the a11y gate; an Opus critique in both themes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1547/manifest.md` with the registry and the walk's dialog log.

### AJ-E1548 The 0.5.0 usability audit's findings, fixed in 0.5.1: one name per thing, no rows nobody can use, every mark and limit said plainly | Status: DONE (0.5.1) | Priority P1 | Effort M | Depends: AJ-E3823
**Found** by the PAT-USE audit of every feature 0.5.0 shipped (`docs/qa/0.5.1-usability-audit.md`, Opus, 2026-11-23): 28 findings, none a blocker. The 21 below change words or looks only and go in under the continuous-train rule. Seven change behaviour (#2, #5, #9, #13, #14, #15, #27); the coordinator ruled on them (requirement 10), so all are built in 0.5.1.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Daily users of Bot, Models, Watches, Home's feed and digest, the bell and header, the Floor, Processes and Unlink this computer, who read each name as meaning one thing everywhere.
(2) Every term means one thing (Finishes, Needs you, room, sessions not "agents"), every mark is explained (the level marks, the three "mark all" actions), and every limit shows only when it matters.
(3) The same surfaces, made clearer; nothing added, no second door.
(4) Rows nobody can use are hidden until a model passes (#3), and the not-yet-shipped toggle is hidden (#18), both D-127.
(5) Keychain access for the device token (#16) and the first watch alert's notification permission join AJ-E1547's registry, primed at the Watches form's Watch button.
(6) Watch for: a count that disagrees with its list (#1); one switch with two names (#6); "Plenty of room" next to "1 account low" (#11); twin rows that can't be told apart (#20).
**Requirements** (the audit's numbers):
1. #1 the Bot page's memory count matches its list (one read, refreshed on Remember and Forget; a test), confirmed on the 0.5.1 build.
2. #3 #4 #7 Models and AI › Jobs list only jobs a model can do here today, with one quiet line for the rest; Attention hints leaves AI › Jobs; the Models lead says only what ships (with AJ-E3823).
3. #6 one name for Bot's written-answers switch in Bot and Settings (with AJ-E3823).
4. #8 "Finishes" means one event across Watches, Notifications and the feed, with a vocabulary test; #10 the bell says "Needs you · N"; #28 Processes' tab says "Needs you".
5. #11 the header's machine chip names its subject ("This Mac: fine"), "room" kept for Accounts; #12 the level marks read plainly and the legend draws exactly the rail's marks.
6. #16 Keychain access in AJ-E1547's registry (done there: `secrets.rs:quietly` is the one door, `keychain_lint` keeps it so); the first watch alert's notification permission primed at the Watches form's Watch button (`await osPromptRequest("notifications")` before saving; on Not now the watch is still saved). AJ-E1547's three calls left to route, each in `scripts/os_prompts.allow` (which only shrinks): `conversations/goto.rs` `Real::osascript_with` through `os_prompt::script(…, Presence::Background, …)`; `goto.rs` `Real::send_link_to` and `places/handoff.rs` `Real::send` through `os_prompt::send_link(APP_LINKS, os_prompt::user(), pid, link)`; their allowlist lines removed.
7. #17 one machine word in the Unlink sheet (`machine-words.ts`); #18 the not-yet-shipped toggle hidden.
8. #19 each "mark all" says what it clears; #20 the digest counts sessions and names twins as everywhere else; #21 the feed's scope picker says what it narrows; #22 "While you were away" and "the digest" kept apart in the notes and the guide.
9. #23 #24 the Floor says what it is (tooltip, a first-open line) and names its floors by what is on them; #25 the watch limit shows only near it; #26 Bot's memory says which watches it counts and links to them.
11. **Providers' terms are never shown open** (the maintainer's correction, 2026-11-23; PAT-SER-7): every place the terms text appears (the switch and move dialogs, adding an Account, an App's terms, Settings, the in-app docs pages) shows it collapsed behind "Providers' terms", or as a link, never as an open block, and nothing opens it automatically. A test that each such place renders it closed.
10. **The seven behaviour changes, ruled by the coordinator (design calls, 2026-11-23; none reverses a D-ruling):**
   - #15: Settings › Privacy gets "Check for configuration updates" (on by default, D-129) with a line saying what it fetches and when (AJ-E2023 Req 3, built here); Privacy also gets lines for Bot's history and notes and for watches ("kept on this Mac; Clear deletes them"); each line tested.
   - #5: the AI job switches live on Models beside the model they use; the Jobs sub-pane is dropped.
   - #9: the bell holds Needs you, the latest few events and "See all on Home"; the 30-day history lives only in Home's feed.
   - #13: on session pages the account banner keeps its facts and Remind/Snooze; the side column's "Switch account…" is the door (D-143); the rail's row stays.
   - #14: "Remind me" makes a one-time "Room again" watch, shown on Watches with Edit and Remove; the button says a watch is made.
   - #2: Bot's "This is wrong" stays and does something real: marking an answer wrong removes it from the conversation's context so follow-ups don't build on it; the answer collapses to "Marked wrong: Bot won't use it in this conversation", with Undo and Ask again; nothing else is stored. Tests for the context removal and Undo.
   - #27: while a pool's setup is open its checklist owns the rules; the header's dropdown and the lead card come back when setup is done.
   Their usability reasoning: (1) each surface's user expects one door per action and one history; (2) each control says what it makes (a watch, a context change); (3) the duplicates go (#5, #9, #13, #27); (4) n/a; (5) none new; (6) a thumbs-down that did nothing, a reminder nowhere to be seen, a fetch nobody was told about.
**Quality gates**: `just keep-going verify-static`; a test per wording rule (vocabulary, counts, hidden rows) and per behaviour change; the visual gate with Opus's critique in both themes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1548/manifest.md`, one row per finding.

### AJ-E1549 No white flash, ever: the window, the page and every loading boundary paint the theme's base colour from the first frame | Status: IN-PROGRESS (0.5.1: built; open: R3 the wizard sign-in terminal placeholder, SignInCard.tsx:32) | Priority P1 | Effort S | Depends: none
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23): a very short full-window white flash at random moments, among them the import wizard's sign-in step with a live terminal panel on screen. The coordinating session found no WebContent crash, reload or WebGL context loss in the system log (the wizard's state survived), so it is a paint gap, not a restart; the main window sets no background colour and nothing sets the webview's, so the native window is white behind the page.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone, in both themes; a white flash in a dark app reads as a glitch or a crash.
(2) Nothing to explain: the fix is that it never happens.
(3) A calm, trustworthy app; no new surface.
(4) Every loading state is a skeleton inside the layout, never an empty root (instant and paged).
(5) None.
(6) The flash itself: a sensible person thinks the app restarted or broke.
**Requirements**:
1. The native window's background is the theme's base colour, set in the configuration and again at run time whenever the theme changes (dark and light).
2. `html` and `body` take their background from the tokens in `index.html`'s inline CSS, with both `prefers-color-scheme` cases, before any script runs.
3. No Suspense fallback or screen boundary renders an empty root: each renders a skeleton inside the layout.
4. The specific trigger is root-caused (candidates: a lazy screen's empty fallback while the page background lives on an inner element; the theme class applied after first paint; the embedded terminal's canvas being re-created; the Floor's surface reconfiguring) and fixed where it is.
**Quality gates**: `just keep-going verify-static`; a Playwright check sampling frames while switching screens, stepping through the wizard and opening the terminal panel, in the dark theme, failing on any frame mostly near white; a unit check that `index.html` carries the background before its scripts.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1549/manifest.md`.

### AJ-E1550 Calm surfaces: separation by tone, never bright borders; a bright line only for focus, hover or real attention (PAT-SURF, app-wide) | Status: IN-PROGRESS (0.5.1: built; open: R4 critique/baseline round) | Priority P0 | Effort L | Depends: none
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23): "what is with all of these highlighted borders… it looks ugly… does that look like peace and calm?", with the Claude app's Code view as the reference (no visible borders, panels told apart by tone, the selected row a soft fill). Their rule: "Only time you can ever use a bright border is on highlight or mouse over or to really really bring attention to the user." Seen: the selected tab and the App/Project/Account segmented control outlined; "This session" boxed; every card outlined (Conversation, Where it runs, Runway, Apps that can take it over); "Switch account…" bordered; the "Suggested" target card ringed in accent; the rail's selected row outlined.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone, all day: AI Juggler is a calm monitoring surface left open beside the user's real work; they expect it to recede, like the Claude app's Code view.
(2) Hierarchy comes from tone and type, so nothing needs explaining; what is selected or needs them stands out because little else does.
(3) Calm monitoring is tone hierarchy, not lines: the same screens, quieter, and attention kept for what deserves it (calm attention, D-117).
(4) n/a.
(5) None.
(6) Lines everywhere make every box look equally urgent; an accent ring on "Suggested" reads as an alert.
**Requirements**:
1. Research first, written into the design system: how Linear, Vercel Geist, Radix Themes, Apple's macOS HIG and the Claude and ChatGPT desktop apps separate surfaces; then the tokens (PAT-SURF): `border.subtle`, a near-invisible hairline (about 6–8 % of the foreground on dark, 8–10 % on light) used only where tone can't separate; no outline on cards or sections, separation by three or four surface steps (base, raised, overlay) and very soft shadows on floating layers only (menus, popovers, dialogs); selected (tabs, segmented controls, rail rows, the subagent tree) a soft fill and stronger text, never an outline; hover a subtle fill step; an accent border or ring only for `:focus-visible` and rare true-attention states (a Needs-you item, a destructive confirm), never for "Suggested" (a small tag); inputs and secondary buttons a fill, not a bright stroke, the ring on focus; `prefers-contrast: more` brings stronger borders back.
2. Applied app-wide through the tokens and the shared components (Card, Section, Tabs, Segmented, Button, Input, List row, the rail), every screen.
3. A lint: no component uses the accent or bright border colours outside the focus, hover or attention tokens.
4. Baselines (gallery and Happo) re-shot once, at the end of the 0.5.1 batch, after an Opus side-by-side against the Claude app's reference in both themes; every Happo diff verified by eye.
5. **Balance, not monochrome (the maintainer's amendment, 2026-11-23: "You keep going from one extreme to another. I want balance… Choose select colours and keep using them to always mean one same thing"; PAT-COLOR).** Calm tones down bright colour and keeps colour where it carries meaning. Seen on the 0.5.0 Nightly: greyed App icons looked broken; Runway's "Out" was plain white text over an empty white bar; every usage bar was white. A semantic vocabulary, one hue one meaning everywhere: critical red (out, blocked, failed); amber (needs you, running low); green (healthy, done, ready, signed in); teal (working, live, activity); brand violet (primary actions, selection, navigation); neutral greys for the text hierarchy only; the eight identity colours (Apps, Accounts) only on small identity marks (dots, avatars, chart series), never for status. Each semantic hue has a soft step (calm states: a muted tint and text) and a strong step (attention: the chip, the bar fill, the rail mark). Runway's bar follows its state: green while healthy (above about 50 %), a soft neutral-teal in between, amber when low (at or under 25 %, `LOW_LEFT`, everywhere: D-145), red with an "Out · resets <day time>" chip when out (D-145) (and Out still raises the header's attention count); unknown or not measured, a neutral dashed bar and plain words. App icons are always in full colour: idle and not running are shown by the status dot (hollow grey not running, soft idle, teal working), muted words ("Not running") and row order, never by desaturating the icon. The palette validator (the bundled dataviz `validate_palette.js`, run locally) checks the identity slots and the semantic set in both themes for colour-vision separation and contrast; status never relies on colour alone (a chip word or icon goes with it). A lint: status colours only through semantic tokens, identity tokens only on identity marks, no grayscale filter on icons. Usability: "can a newcomer tell, in one glance, which account is out, which is low and which is fine?", scored in the persona walk.
**Quality gates**: `just keep-going verify-static` with the new lint; the a11y gate (focus visible, contrast, Increase Contrast); the visual gate once; Opus's critique in both themes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1550/manifest.md` with the research table and before/after captures.

### AJ-E1551 Nothing moves after it appears: every column, header, rail and badge has its place from the first paint (layout shift near zero) | Status: DONE (0.5.1) | Priority P1 | Effort M | Depends: none
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23): opening a session, the left subagent tree appears a split second later and pushes the whole layout over.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone opening a session or switching screens; they expect what they aimed at to stay where it is.
(2) Nothing to explain: the page holds still.
(3) Instant and paged: a skeleton at the final size on the first frame, the real content filling it.
(4) A column with nothing in it is decided from cached data before the first paint, never by a late fetch.
(5) None.
(6) A layout that jumps makes people click the wrong thing and feels unfinished.
**Requirements**:
1. The session page's columns are reserved from the first paint, the subagent tree as a skeleton at its final width; whether a column shows is decided from cached data, not after a fetch.
2. A sweep of every screen for the same pattern (side columns, headers, rails, badges that mount late), each fixed the same way.
3. A Playwright layout-shift check (a cumulative layout shift budget of about 0) on opening a session, switching tabs and opening Home.
**Quality gates**: `just keep-going verify-static`; the layout-shift check; the visual gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1551/manifest.md`.

### AJ-E1552 Nothing loads forever: one loading primitive (loading, ready, empty, error, stale), a deadline on every resource, an empty state wherever there is a skeleton | Status: DONE (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2327
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23): "We need much more reliable loading and delayed loading systems. We can't leave them in these forever loading states." On Accounts (filter "Out"), one Account card's Apps row stayed a skeleton bar for over a minute and never resolved; the other cards showed their App rows. The coordinating session's suspicion, to be confirmed: that Account has no added App on the fresh install (its desktop app sat in "Running, not added", AJ-E2327) and the card treats an empty list as still loading.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone, on every screen; a skeleton promises "coming in a moment".
(2) A skeleton shows only while something really loads; after about 3 s it says "Still loading…"; after its timeout, an error in plain words with Retry.
(3) One loading system for the whole app instead of each screen's own guess.
(4) Empty is its own state with a guided next step (for this card: "No app uses this account yet · Add its app", into AJ-E2327's Add); an error says what failed and offers Retry (D-127).
(5) None.
(6) A bar that never resolves reads as a hang or a broken app.
**Requirements**:
1. Root cause the stuck card first (file:line), then one shared loading primitive (extending `use-resource`) with explicit states: loading, ready, empty, error and stale-while-revalidate. Every skeleton in the app renders only in loading, never because a list is empty or a value is null.
2. A deadline per resource: "Still loading…" after about 3 s; after the resource's own timeout (about 10–15 s) an error row in plain words with Retry, the cause logged. Nothing stays a skeleton forever.
3. An empty state everywhere there is a skeleton; the Account card's is "No app uses this account yet · Add its app", opening AJ-E2327's Add.
4. A lint or test: no component renders a skeleton outside the primitive's loading state; every skeleton site has an empty-state and an error-state test; a Playwright sweep opens every screen and filter and fails if any skeleton is still visible after 15 s.
5. The backend: every query a screen waits on returns within its budget (a typed empty or error included); no IPC hangs; a call over its budget is logged.
6. The same card's cramped wrapping (the "…has 100% left." line squeezed between the Out chip and the button) is fixed in AJ-E1550's pass.
**Quality gates**: `just keep-going verify-static` with the new lint; the skeleton sweep; the per-site empty and error tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1552/manifest.md`.

### AJ-E1553 A sign-in finishes itself: AI Juggler sees it succeed, says so, and clears its entry; one that ends early says so and keeps its last lines | Status: DONE (0.5.1) | Priority P1 | Effort M | Depends: AJ-E1545
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23), after finishing a tool's sign-in in the import wizard: the tool's sign-in file was written and no process of it ran (checked by name, date and size only), yet the rail still showed a "Sign-ins · Sign in · <tool>" entry; opening it showed an empty terminal (the process had ended and its scrollback was gone); and the footer asked for a manual re-check ("…then check the app again from Accounts — it flips to ready").
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone signing in to a tool from the wizard or the rail; they expect AI Juggler to notice when they're done.
(2) "✓ <tool> is signed in" the moment it is; an early end says "Sign-in ended before it finished" with the last lines and Try again.
(3) The same detection as AJ-E1545's sign-in card ("Waiting for you to approve…" → ✓ Signed in), in one place for the wizard and the rail.
(4) A finished or stale sign-in entry leaves the rail by itself, also after a restart.
(5) None new (the browser was opened through AJ-E1547's gate).
(6) An empty terminal and a "check again from Accounts" instruction read as broken.
**Requirements**:
1. While a sign-in runs, AI Juggler watches the adapter's "signed in?" signal (file events plus the adapter's own check; metadata or status only, never a credential's contents). On success it ends the sign-in session, shows "✓ <tool> is signed in", flips the Account and App to ready by themselves, removes the rail's entry and ticks the wizard's row.
2. A process that ends without success leaves an outcome card ("Sign-in ended before it finished") with its last lines kept (scrollback retained) and Try again; never an empty terminal.
3. On restart, sign-in entries whose Account is already signed in are cleared.
4. The "check again from Accounts" words go.
5. Through the adapters' sign-in hooks only (no per-tool code outside them).
**Quality gates**: `just keep-going verify-static`; per-adapter tests for success detection, an early exit and stale-entry cleanup; the persona walk signs in through the wizard and sees the rail's entry leave.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1553/manifest.md`.

### AJ-E1554 The rail's rows hold still on hover, and every group says quietly what more there is, one click from it | Status: DONE (0.5.1) | Priority P1 | Effort M | Depends: AJ-E1550, AJ-E2327
**Found** by the maintainer on the 0.5.0 Nightly (2026-11-23): on hover a session row's actions (grip, pin, open) cover its title and a handle mark slides under the status dot, which looks broken; and the rail, calm with only open sessions, gave no sign of what else exists (a main session with 6 subagents running and 220 in all showed nothing of them).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Everyone, all day: the rail is the map of what is open; they expect rows to stay put under the pointer and the map to hint at what lies beyond it.
(2) Counts are small, muted and summarised, non-zero only, each with a tooltip saying what it counts ("3 sessions need you"); a "+12 more · 3 idle today" footer says what opening it shows.
(3) The rail stays calm (open sessions only) while every group says what else exists; each count is a door to exactly those items, so nothing needs a second list.
(4) Zero values are hidden; a group with nothing more has no footer; inactive Apps collapse to one summary line.
(5) None.
(6) Actions covering the title; a dot that moves; a bare "2" with no meaning; 220 subagents invisible.
**Requirements**:
1. Every rail row type (session, App header, sign-in, not-added) has fixed slots: a leading gutter (the status dot, or the grip on hover in the same slot), the title (truncating), and a trailing slot (the state label, which hover and keyboard focus swap for the action icons). Actions never overlap the title or move the dot. A visual test per row type for rest, hover and focus.
2. An App header shows a compact status cluster on its right, non-zero values only, tabular numbers and tooltips: working (n), needs you (n, amber), done to review (n), subagents running (branch icon + n), replacing the bare count.
3. A session row shows a small branch icon + n when it has running subagents, opening its Subagents tab.
4. An App group with more sessions than shown has a quiet footer row ("+12 more · 3 idle today") opening that App's page filtered.
5. The collapsed inactive Apps header carries a summary ("10 apps · 48 sessions"); expanded, the list as today. A collapsed Needs you keeps its count chip.
6. Every count opens exactly those items, by id; every count comes from the cached relation counts (instant, no new scan). AJ-E1550's tokens throughout (calm, no bright borders).
**Quality gates**: `just keep-going verify-static`; tests for counts against fixtures, zero values hidden, a click target per count, the layout unchanged under hover; the persona walk asks "can a newcomer tell there is more, and get to it in one click?".
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1554/manifest.md`.
