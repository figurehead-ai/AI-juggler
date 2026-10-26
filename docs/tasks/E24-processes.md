# E24 — The Processes view: every app, session and process, one model

> Source: the maintainer, 2026-10-25 (D-41, verbatim): "Yes to both: (a) a
> local 7-day names-only record of processes in sessions other apps run,
> and (b) user-chosen, verified stop of their leftovers after the session
> ended, never automatic, never the app or the session's own process.
> Build the Processes view in 0.2." And, of the view: "our tool is
> supposed to have a really advanced way of seeing which Apps are
> running, which sessions and what processes and subprocesses they have
> started, still running, or maybe ran at one point and now are zombie …
> an advanced way to view, manage and clean up all of that manually and
> automatically."
> Design: ADR-0010 (ownership, verify-or-refuse, tiers), ADR-0019 (a
> feature module), ADR-0022 (driven sessions are ledger children).
> Research: the processes gap analysis (2026-10-25, its captures in the
> coordinator's scratchpad), `docs/review/pass2/05-process-trees-zombies-and-cleanup.md`
> (the zombie taxonomy and the list model), wireframes 08, 21, 22, 24, 26
> and 27 (`docs/wireframes/index.html`).
>
> The E5 safety core is built: the ledger, PGID, verify-or-refuse, the
> reconciler, the port map, the kill sheet's ladder, the tiers, pause,
> crash suspension, the audit, re-adoption and machine ids. What was
> missing: a cross-app tree; one source of truth (Cleanup, a session's
> Processes tab and This Mac disagreed, and the E410 tracker repeated the
> E5 logic on its own clocks); the zombie and didn't-stop states; what
> ran (the ledger kept ended rows 7 days, never shown); live updates
> (`HelperOrphaned` was dropped); the wireframes' actions; role words;
> paging; and observation beyond the Claude tools.
>
> **Law for every task here:**
> - **One model, on the existing tick.** The graph is built from what the
>   2 s tick already holds — the ledger, the collector's table, trees and
>   rings, the live view's sessions, the sightings. No scan loop of its
>   own; every surface reads it, so they agree.
> - **AI Juggler owns what it starts (ADR-0010).** Only the ledger's
>   processes are AI Juggler's; their stop is the verified kill sheet,
>   always. Every automatic rule stays per account, off by default, and
>   only for verified dev servers and watchers (D-08).
> - **Other apps' processes are read-only, except by the user's own
>   click (D-41).** Never written to the ledger; never touched by a rule
>   at any tier; never the app itself, a process inside an app's bundle,
>   or a session's own process. A leftover of an ended session may be
>   stopped by the user through the identity check against its sighting
>   (pid, start time, program, boot), the same sheet and the same audit
>   (`actor = user:observed`).
> - **Never the environment or a credential (D-37).** Sightings keep a
>   program's name and a keyed hash of its command line, never the line;
>   7 days; nothing is sent anywhere.
> - **AI Juggler's own are never cleanup candidates.** Its hooks and
>   tools (`juggler-agent`) show as "AI Juggler hook" and "AI Juggler's
>   tools", hidden unless asked for; its socket and files are never
>   offered.
> - **Plain words, one vocabulary.** The states are words with icons, the
>   same everywhere; "zombie" is explained where it shows; no T1–T5 codes
>   on screen.

## Step 1 — the model

### AJ-E2401 The process graph: one model of apps, sessions and processes | Status: IN-PROGRESS (agent, 2026-10-25) | Priority P0 | Effort L | Depends: —
**Goal**: One read model of every app → session → process → subprocess on this Mac that AI Juggler started or sees, in one vocabulary of states, live and paged.
**User story**: As someone with many sessions open in several apps, I see which app runs which session, what each started, what is still running or left behind, and what is waiting on its parent, without opening Activity Monitor.
**Requirements**:
1. The graph SHALL be built on the existing 2 s tick from the ledger's live rows, the collector's table, trees, rings and listeners, and the live view's sessions; it SHALL add no scan loop (the other apps' trees are measured from the collector's own listing).
2. Nodes: an app (from `host_of`, AI Juggler for its own), a session (ours: a ledger root, PTY or driven; theirs: a live session's verified pid), its processes in tree order.
3. States: Working, Idle, Left behind, Left behind while AI Juggler was closed, Picked up after restart, Finished and waiting for its parent (zombie, naming the parent, no stop), Didn't stop (a failed stop, or a session marked ended whose own process runs), Won't be touched, Ended (7 days).
4. Facts per process: its role in words (the cleanup's vocabulary, "probably"), CPU now with a sparkline, memory now and peak, ports, age, CPU time, whose ("Started by AI Juggler" / "In a session <app> runs"), and what started it from the tool's own events when known (a Claude hook's command, a Codex command item: "Started by Bash: npm run dev"), held in memory only.
5. Totals per session and app; counts (to clean up, left behind, zombies, didn't stop, won't touch) that the top bar and This Mac show.
6. Paged by cursor (≤ 200 rows a page); a change is published as `processes://changed` with the version and counts; `HelperOrphaned` is no longer dropped.
7. AI Juggler's own pid never shows; its hooks and tools are hidden unless asked for and never counted.
**UI/UX reasoning**: Nielsen #1 visibility of system status (live, what runs where); #2 match the real world (role words, not command lines).
**Performance budget**: at 50 sessions and 2,000 processes: a build ≤ 5 ms and a page ≤ 50 ms (release); the tick's extra CPU ≤ 1 % of a core.
**Quality gates**: `cargo test -p juggler-app --test processes_graph --test processes_live`, `--lib features::processes`; clippy `-D warnings`; the structure, polling and tool-literal audits.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2401/manifest.md`.
**Visual testing**: the Processes page (AJ-E2404).
**Files to touch**: `crates/juggler-app/src/features/processes/` (model, observe, query, roles, mod), `crates/juggler-supervisor/src/live_stats.rs` (a zombie flag, CPU time, the observed trees' rows, the listeners, extra observed roots), `crates/juggler-core/src/storage/process_ledger.rs`, `crates/juggler-app/src/lib.rs`.
**Research basis**: the gap analysis; pass2/05 §2 (taxonomy) and §9.1 (list model); SPEC §3.2, §5.2, §9.2.
**Handover note**: The session's own process of another app is often inside its app's observed tree (the Claude app): its subtree is cut from that tree, stopping at another session's root. Short-lived processes (hooks, one command) are never sighted. Retiring the E410 tracker waits for 0.1.5's stop fix (AJ-E2407).

### AJ-E2402 Sightings of other apps' sessions' processes (D-41a) | Status: IN-PROGRESS (agent, 2026-10-25) | Priority P0 | Effort M | Depends: AJ-E2401
**Goal**: A helper that outlives a session another app runs shows as "left behind by <session>".
**User story**: As someone who works in the Claude app, I see the dev server its ended session left running, named by that session.
**Requirements**:
1. A sighting SHALL hold a process's pid, start time, boot, program name, its session and tool, the app that ran it, first and last seen, and a keyed hash of its command line (SHA-256 over the install's own random key); never the environment, never the command line.
2. Kept 7 days after last seen; written once when first seen, then at most once a minute; a process alive less than 10 s is never sighted.
3. A sighted helper alive after every sighting of its session's own process ended (and the live view no longer lists the session) SHALL be Left behind.
4. Guardrail tests: the table's columns are exactly the reviewed list and none could hold an environment or a line; the 7-day expiry; the hash is keyed and never the line.
5. Schema v34 (module-owned migration).
**UI/UX reasoning**: Nielsen #6 recognition rather than recall: the session's name, not a pid.
**Performance budget**: writes batched in one transaction per tick; memory bounded to the live sessions and their leftovers.
**Quality gates**: `cargo test -p juggler-core --test sightings_storage`; `cargo test -p juggler-app --lib features::processes::sightings`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2402/manifest.md`.
**Visual testing**: the Left behind rows of another app's session.
**Files to touch**: `crates/juggler-core/src/storage/sightings.rs`, `crates/juggler-core/src/storage/mod.rs`, `crates/juggler-app/src/features/processes/sightings.rs`.
**Research basis**: D-41a; D-37.
**Handover note**: E18 also claims schema v34 on its branch: whichever merges second renumbers its module-owned migration to v35 (one line in `storage/mod.rs`, two test constants).

### AJ-E2403 The user's stop of another app's leftover (D-41b) | Status: IN-PROGRESS (agent, 2026-10-25) | Priority P0 | Effort M | Depends: AJ-E2402
**Goal**: The user can clean up what another app's ended session left, safely and on record.
**User story**: As a Claude app user, I stop the vite server my closed session left on :5173 from AI Juggler, after it checks it is that process.
**Requirements**:
1. Only by the user's call; no automatic caller at any tier.
2. Only after the session's own process ended; never the session's own process, the app, a process inside an app's bundle, AI Juggler or its hooks, or a ledger process.
3. The identity check against the sighting at click time and again before the first signal: pid, start time, program, boot; a mismatch sends zero signals with the reason.
4. The same verify sheet and TERM → grace → KILL ladder (this process only), the audit intent before any signal with `actor = user:observed` and the program's name in place of a command line.
5. The collateral suite extended: with every rule on and the sweep run past every grace, another app's leftover lives; every refusal sends zero signals.
**UI/UX reasoning**: Nielsen #5 error prevention (verify, name the consequence); WCAG 3.3.4 (a reversible-or-confirmed destructive action).
**Performance budget**: the check ≤ 50 ms (targeted reads only).
**Quality gates**: `cargo test -p juggler-app --test observed_stop`, `--lib features::processes::stop`; the guardrail and collateral suites unchanged.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2403/manifest.md`.
**Visual testing**: the verify sheet for another app's leftover, and its refusal.
**Files to touch**: `crates/juggler-app/src/features/processes/stop.rs`, `commands.rs`.
**Research basis**: D-41b; ADR-0010 verify-or-refuse; pass2/05 §9.2.
**Handover note**: The sheet is the E507 kill sheet with an observed bridge (verify, stop); its scope is this process only.

## Step 2 — the page

### AJ-E2404 The Processes page | Status: TODO | Priority P0 | Effort XL | Depends: AJ-E2401, AJ-E2403
**Goal**: One page to see, manage and clean up every process, replacing Cleanup, and the same view in each session's Processes tab.
**User story**: As someone juggling many sessions, I open Processes, see what needs me first, unfold a session to its subprocesses, and stop or keep what it left.
**Requirements**:
1. Views: Needs attention (default), Everything running, Ended (7 days), Cleanup history.
2. A tree table: App → Session → process → subprocess; a healthy session folds to one line ("web-app · 6 processes · 1.2 GB · 4 %"). Columns with headings: Process (indented, role word), State (icon + word), CPU with a sparkline, Memory, Ports, Age, Owner. A sticky header; virtualised (≤ 60 rows in the DOM); keyboard tree navigation (↑ ↓ ← → Home End, Enter).
3. Filters, all off by default (state, app, profile, project, has a port, heavy, include other apps), and search (`port:5173`).
4. Row actions: Stop… (always the verify sheet), Keep, Remind me in 1 h, Go to session, Reveal folder, Copy PID; for a session: "Clean up what this session left (N)…" with a verify line and outcome per process; other apps' processes: the D-41b stop, Go to its app, Show in Activity Monitor.
5. Cleanup becomes this page's Left behind view; the nav and the top bar's pill lead here; the session Processes tab is the same component filtered to that session (any app's); This Mac links into it and its Left behind agrees with the top bar.
6. "Zombie" explained in plain words where it shows.
**UI/UX reasoning**: Apple HIG tables and outline views; WCAG 2.2 AA (treegrid keyboard, 4.5:1, focus visible); the readability scale (body 14 px, nothing under 12 px, section titles 15 px); D-25 progressive (folded until needed).
**Performance budget**: first paint ≤ 300 ms from the last snapshot; ≤ 60 DOM rows; a page ≤ 50 ms.
**Quality gates**: vitest; typecheck; the a11y gate at 0 serious; size budget; the plain-words and terminology lints.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2404/manifest.md`.
**Visual testing**: 1280, 1680 and 2000 px, dark and light, with the preview's made-up Mac (several apps, dev servers, watchers, MCP servers, leftovers, a zombie, a failed stop, ports, other apps' sessions); every capture opened.
**Files to touch**: `ui/src/screens/processes/`, `ui/src/lib/ipc/processes.ts`, `ui/src/preview/processes*.ts`, `ui/src/App.tsx`, `ui/src/lib/nav.ts`, `ui/src/components/MainNav.tsx`, `ui/src/components/RightInspector.tsx`, `ui/src/screens/ThisMacScreen.tsx`, `ui/tests/visual/processes-gallery.spec.ts`.
**Research basis**: wireframes 08, 21, 22, 27; pass2/05 §9; the redesign canvas.
**Handover note**: Built on the 0.1.5 components (readability scale, AppIcon with the apps' own icons, the page frame) once 0.1.5 is merged into release/0.2; `ProcessRow`, `CleanupCentre` and `KillSheet` are 0.1.5's.

### AJ-E2405 Automatic rules shown, previewed and told | Status: IN-PROGRESS (agent, 2026-10-25) | Priority P1 | Effort M | Depends: AJ-E2401
**Goal**: The automatic cleanup is visible and predictable before it is turned on, and the user hears when a session leaves something behind.
**User story**: As a user about to turn on "Automatically tidy left-behind dev servers" for my work account, I see that it would tidy vite :5173 and esbuild now, and nothing else.
**Requirements**:
1. The per-account tiers stay as they are (ADR-0010); the page shows each account's rules, pause and the crash hold.
2. A dry run before enabling SHALL use the sweep's own rule (`evaluate_tier`) and exclusions (heavy, Keep): now, later (its grace), never (why).
3. Keep and Remind me later are stored marks; the sweep SHALL never tidy a kept process.
4. A macOS notification when a session leaves processes running, once per session, through Settings › Notifications (a new kind, off by default) and its quiet hours.
5. Cleanup history filterable by session and project, paged.
6. Export a process snapshot: names, roles, states, numbers and keyed hashes of command lines only.
**UI/UX reasoning**: Nielsen #1 and #3 (user control: preview before enabling, Keep); D-08.
**Performance budget**: a dry run ≤ 50 ms.
**Quality gates**: `cargo test -p juggler-app --lib features::processes::rules`, `--test system_notifications`, `--test processes_live`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2405/manifest.md`.
**Visual testing**: the rules panel with a dry run; the history filtered to a session.
**Files to touch**: `crates/juggler-app/src/features/processes/rules.rs`, `ended.rs`, `commands.rs`, `crates/juggler-app/src/features/notifications/`, `crates/juggler-core/src/storage/cleanup_audit.rs`, `crates/juggler-app/src/commands.rs` (Keep in the sweep).
**Research basis**: ADR-0010 tiers; pass2/05 §10; wireframes 21 and 26.
**Handover note**: The notification counts what the graph shows the session left, after the reconciler's confirmations settle (10 s).

## Step 3 — one source, documents

### AJ-E2407 One source: the session tab, Cleanup and This Mac on the graph | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2401, AJ-E2404
**Goal**: The E410 tracker's second model of orphans is retired; every surface reads the graph.
**User story**: As a user, the session's Processes tab, Processes and This Mac all say the same thing about what is left behind.
**Requirements**:
1. The session Processes tab SHALL read the graph (its session filter), including other apps' sessions.
2. The E410 `OrphanTracker`'s detection (its own 10 s clock and ppid snapshot) SHALL be retired; leftovers are the ledger's (the reconciler's) and the sightings'; every stop goes through the verified path.
3. This Mac's "Left behind" and the top bar's count come from the graph (done for This Mac's count).
**UI/UX reasoning**: Nielsen #4 consistency.
**Performance budget**: no new work on the tick.
**Quality gates**: the processes, cleanup, guardrails and collateral suites.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2407/manifest.md`.
**Visual testing**: a session's Processes tab, ours and another app's.
**Files to touch**: `crates/juggler-app/src/commands.rs` (`instance_processes`, `processes_tick`, `record_exit_snapshot`), `crates/juggler-supervisor/src/orphans.rs`, `ui/src/components/RightInspector.tsx`, `crates/juggler-app/src/machine_health.rs`.
**Research basis**: the gap analysis (one source of truth).
**Handover note**: Waits for 0.1.5's G-1 stop fix to land in release/0.2 (it changes the same stop path).

### AJ-E2406 Privacy lines, guide, and the processes lab | Status: TODO | Priority P0 | Effort S | Depends: AJ-E2402, AJ-E2403, AJ-E2404
**Goal**: What AI Juggler keeps of other apps' processes, and when it may stop one, is written down and tested; the live check is a checklist.
**User story**: As a careful user, I read on the Privacy page that AI Juggler keeps only names and a hash of other apps' processes for 7 days, and stops one only when I ask.
**Requirements**:
1. Privacy lines (sightings; the user's stop of other apps' leftovers), each tied to tests that exist.
2. Guide sections: the Processes page, its states, Keep and reminders, the rules' dry run, other apps' leftovers.
3. `docs/qa/processes-lab.md`: a live-lab checklist using only lab-started processes; never the maintainer's real ones.
4. Evidence manifests for AJ-E2401–E2405 and E2407.
**UI/UX reasoning**: Plain words.
**Performance budget**: n/a.
**Quality gates**: the trust-claims test; the plain-words and terminology lints; the docs link check.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2406/manifest.md`.
**Visual testing**: the Privacy page lines.
**Files to touch**: `ui/src/lib/trust-claims.json`, `docs/guide/trust.md`, `docs/guide/processes.md`, `docs/guide/this-mac.md`, `docs/qa/processes-lab.md`.
**Research basis**: D-41; D-37.
**Handover note**: —
