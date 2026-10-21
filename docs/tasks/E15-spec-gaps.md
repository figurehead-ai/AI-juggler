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

### AJ-E1501 Manage accounts: rename, remove, set default | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1024
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

### AJ-E1502 Manage sessions and projects | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1014
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

### AJ-E1504 Keys: a cheat sheet, inspector collapse, go-to keys | Status: TODO | Priority P1 | Effort S | Depends: —
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

### AJ-E1505 Settings the SPEC promised: stuck thresholds, terminal font and scrollback | Status: TODO | Priority P1 | Effort S | Depends: AJ-E405
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

### AJ-E1506 Terminal tools | Status: TODO | Priority P0 | Effort M | Depends: —
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

### AJ-E1507 Rules and MCP sync across tools, with a diff preview | Status: TODO | Priority P0 | Effort L | Depends: AJ-E202
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
**Research basis**: SPEC §7.5; EPICS M4-6; expected-worfklow ("rules and guidelines need to work").
**Handover note**: The MCP formats: Claude `.mcp.json` / `settings.json`, Codex `config.toml` `[mcp_servers]`. E16 adds more.

### AJ-E1508 Copy shared settings into a new account | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1507, AJ-E1022
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

### AJ-E1509 Auto-restart a crashed session (opt-in) | Status: TODO | Priority P1 | Effort S | Depends: AJ-E505
**Goal**: A session that crashes can come back by itself, when the user asked for that.
**User story**: As a user running an overnight job, I turn on auto-restart for that launch setting, and a crash resumes it instead of ending the night.
**Requirements**:
1. An opt-in per launch setting: a session that exits with an error SHALL be started again with the same settings and resumed. This never applies to a quota stop, a stop the user made, or a sign-in failure.
2. At most 3 tries in 10 minutes; then it stays ended and says why. Each try is recorded in the session's history.
**UI/UX reasoning**: Off by default; ADR-0003 allows a behaviour the user turned on.
**Performance budget**: n/a.
**Quality gates**: tests for the limit and for each never-restart case.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1509/manifest.md`.
**Visual testing**: the setting, and a session's history with a restart.
**Files to touch**: `crates/juggler-supervisor/src/lifecycle.rs`, launch settings.
**Research basis**: SPEC §1 and §5.4; EPICS M2-7.
**Handover note**: —

### AJ-E1510 System notifications | Status: TODO | Priority P1 | Effort M | Depends: AJ-E611, AJ-E1408
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

### AJ-E1511 Open a context pack in any tool | Status: TODO | Priority P1 | Effort M | Depends: AJ-E202, AJ-E208
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

## Wave 2 — usage you can act on, Fleet and support

### AJ-E1512 Sessions left and a burn-down chart | Status: TODO | Priority P1 | Effort M | Depends: AJ-E608
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

### AJ-E1513 Prepare the pack before quota runs out | Status: TODO | Priority P2 | Effort S | Depends: AJ-E611, AJ-E205
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

### AJ-E1514 Budget caps | Status: TODO | Priority P2 | Effort M | Depends: AJ-E609
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

### AJ-E1515 Fleet: start a session on another Mac, and usage per machine | Status: TODO | Priority P1 | Effort M | Depends: AJ-E705
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

### AJ-E1517 Export diagnostics | Status: TODO | Priority P1 | Effort S | Depends: —
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

### AJ-E1518 A trust page and vendor terms on Add account | Status: TODO | Priority P1 | Effort S | Depends: —
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

### AJ-E1519 Migration guides and importers | Status: TODO | Priority P2 | Effort M | Depends: AJ-E307
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

### AJ-E1521 Robustness: a database busy timeout and an adapter conformance kit | Status: TODO | Priority P1 | Effort S | Depends: —
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
**Handover note**: E16's adapters must pass it.

### AJ-E1522 Quality debt | Status: TODO | Priority P1 | Effort M | Depends: AJ-E803
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
**Handover note**: —

### AJ-E1523 North-star metric: weekly juggling loops completed | Status: TODO | Priority P2 | Effort S | Depends: AJ-E1103
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
