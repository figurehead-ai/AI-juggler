# E21 — Desktop first: what's open, where, and one click to it

> Source: the maintainer, using 0.1.1 on the real setup (2026-10-24): "the
> whole point of our app is to show projects, sessions and actual Native
> Apps themselves and get to them … users are not going to spend hours in
> our app, they just want to get quick assessment where is what." The
> sessions shown must match what is really open in each app, profiles are
> not accounts, menus need icons and an order, and pages need sections.
> Closed, inactive and other sessions stay reachable, but are not shown by
> default. The approved design is the "AI Juggler redesign: desktop first"
> canvas; D-35 to D-37 record the rulings. Three reviews (feature coverage,
> planned features, journeys and edge cases) were folded in before any
> code. It ships as 0.1.x in four steps (AJ-E2101–E2103, E2104–E2107,
> E2108–E2109, E2110).
>
> **Law for every task here:**
> - **The five-second test.** Every screen answers what needs you, what is
>   open, where (app, profile, window), and gives one click to get there.
> - **Only what is really open, honestly.** "Open now" means a live
>   process; anything else is folded or behind View. Never claim "in
>   front" or "waiting for you" from a guess: say "viewed 2 min ago" or
>   "probably".
> - **Nothing hidden is silent.** Every view that is off by default leaves
>   a count where its rows would be, and the count turns it on.
> - **Never read a process's environment, a token or a key file.** The
>   desktop app's sessions carry a sign-in token in their environment.
> - **No prompt from a click.** Permissions are offered in Settings, never
>   raised by Go to it.
> - **Tool and app specifics live in adapters and data (ADR-0019).** A host
>   app is a registry entry, not a branch.

## Step 1 — what is open, and getting there (0.1.3)

### AJ-E2101 Live sessions from the apps' own files | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: —
**Goal**: AI Juggler knows which sessions are really open, in which app and profile, without reading any process's environment.
**User story**: As the maintainer with six Claude app profiles, the Codex app and a terminal open, I see exactly the sessions that are running, under the app and profile each runs in.
**Requirements**:
1. Each adapter SHALL report its live sessions per profile (`live_sessions(profile)`), from documented or observed per-session files:
   - Claude Code: `<profile>/sessions/<pid>.json` (pid, sessionId, cwd, status, `procStart`, `hostSessionId`);
   - the Claude app: its per-session state under the app's data folder (`lastFocusedAt`, archived, `postTurnSummary.status_category`), read field by field;
   - Codex: the app's loaded threads (thread-writer locks and the rollouts the app-server holds), and `codex resume <id>` processes.
2. The host app SHALL come from the process's parent chain (the first `.app`) and the Claude app instance from its `--user-data-dir` argument, never from environment variables.
3. Every environment read SHALL be removed (`running_setups`, `config_dirs_of`, `live::running_sessions`), with a guardrail test that the process scan never asks for environments.
4. A pid SHALL match its recorded start time before it counts as the same process.
5. Changes SHALL be watched (file events on the session folders), with liveness checks of known pids every 5 s and a full scan at most every 60 s.
6. An unknown file format SHALL fall back to what the documented files say, never to stale values.
**UI/UX reasoning**: Invisible; it feeds every list.
**Performance budget**: a refresh under 20 ms; no main-thread work.
**Quality gates**: fixtures for each file shape; the no-environment guardrail; PID reuse test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2101/manifest.md`, with a read-only count on the maintainer's Mac (files, live processes, matches).
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-adapter/` (trait), `crates/juggler-adapters/src/{claude_code,codex}*.rs`, `crates/juggler-app/src/running_setups.rs`, `features/conversations/live.rs`.
**Research basis**: the desktop-first technical research (2026-10-24).
**Handover note**: The Claude app's files are undocumented; parse tolerantly.

### AJ-E2102 Go to it | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E2101
**Goal**: One click or Enter brings the session's app, profile and, where possible, its window or tab to the front, and says what it did.
**User story**: As a user, I press Enter on a session and land in the right Claude app profile, or I'm told to pick it in that app's sidebar.
**Requirements**:
1. The app instance SHALL be activated by its process (`NSRunningApplication`), never by bundle id, so the right profile comes forward.
2. The command SHALL run off the main thread with a 1.5 s limit and show progress at once.
3. Exact windows (Accessibility) and terminal tabs (Automation) SHALL be used only when the user turned them on in Settings; a click never raises a prompt.
4. The outcome SHALL name the step reached: "Brought Claude · Work forward; pick the session in its sidebar", "(3 windows)", "minimised — click it in the Dock".
5. Codex threads SHALL open with `codex://threads/<id>`; editors with `cursor://file/<folder>` or `code <folder>`. A CLI session in an editor's terminal never uses the editor extension's session link.
6. A session on another Mac reads "Bring forward on <Mac>" and runs there (AJ-E711).
**UI/UX reasoning**: The primary action everywhere, one word and ↵.
**Performance budget**: feedback < 100 ms; done < 1.5 s.
**Quality gates**: tests for each outcome; no prompt without the setting.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2102/manifest.md`.
**Visual testing**: the outcome toast.
**Files to touch**: `crates/juggler-app/src/focus.rs`, `commands.rs` (`desktop_focus`), a host-app registry.
**Research basis**: as above.
**Handover note**: macOS lets an app pass activation on only while active, which holds after a click in AI Juggler.

### AJ-E2103 Open first: the rail's order | Status: IN-PROGRESS | Priority P0 | Effort S | Depends: AJ-E2101
**Goal**: The rail lists what needs you, then what is open, with ended sessions folded.
**User story**: As a user, the top of the rail matches what's on my desktop.
**Requirements**:
1. Sections SHALL be Needs you, Open now, then one line "Also: n ended today · n inactive · n background · History ›", each part turning its view on.
2. Ended sessions from other apps SHALL no longer be added under every project.
3. A session open in two places SHALL be one row that says so.
**UI/UX reasoning**: The smallest change that fixes the order, before the new rail.
**Performance budget**: no extra reads.
**Quality gates**: vitest for the order and folds.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2103/manifest.md`.
**Visual testing**: the rail in both themes.
**Files to touch**: `ui/src/components/OutsideConversations.tsx`, `WorkflowRail.tsx`.
**Research basis**: the journeys review.
**Handover note**: —

## Step 2 — the new rail, Home and menus (0.1.4)

### AJ-E2104 Places, rows and the View | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2103
**Goal**: The rail groups open sessions by place (app + profile), each row says what, where and its state, and every other view is one click away.
**User story**: As the maintainer, I read the rail in five seconds even with 40 sessions.
**Requirements**:
1. A place header SHALL show the app icon, the profile, a count and Open (brings a running app forward only).
2. A row SHALL show the app icon with the profile badge (monogram, 4.5:1), the title, a place line (project · app · profile · window or tab), and a state dot with its word. Runway shows on a row only when low.
3. States SHALL be one vocabulary everywhere: Asks you, Working, Idle, Done (review it), and the folded Ended. Every tool and app status maps onto them.
4. Quiet places SHALL fold ("Work · 4 idle"); a place shows working rows and the last 3, then "+n quiet".
5. View (⇧V, and a button with a dot when not default) SHALL offer Group by (Place, Project, Pool, Account), Show (inactive, quiet, ended, background runs, sub-agents, other Macs, archived) and Only (app, profile, account, project, pool, Mac), plus Back to the defaults. Search and ⌘K take the same words (`is:ended`, `app:` …).
6. AI Juggler's own sessions SHALL be a place ("AI Juggler"); other Macs fold after this Mac.
**UI/UX reasoning**: The approved canvas.
**Performance budget**: a 40-session rail renders in < 16 ms; virtualised.
**Quality gates**: vitest per rule; a11y (labels state first; keyboard).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2104/manifest.md`.
**Visual testing**: the rail at 1, 8 and 40 sessions, both themes.
**Files to touch**: `ui/src/components/WorkflowRail.tsx`, `ui/src/lib/railview.ts`, new place components.
**Research basis**: AltTab, Raycast, Vibe Kanban, Linear display options.
**Handover note**: ⌘1–9 go to places (stable), never to rows that re-sort.

### AJ-E2105 Home: what needs you and your apps | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2104
**Goal**: Home says what needs you, shows each running app and profile, and the runway per account.
**User story**: As a user, I open AI Juggler and see "2 things need you", the apps I have open, and how much room each account has.
**Requirements**:
1. The heading SHALL be one sentence about what needs you, with the other counts as links.
2. Needs you SHALL hold every kind of item, each with its own primary: a session asking (Go to it), a limit reached (Continue elsewhere or Wait), a sign-in, a heavy Mac, left-behind processes, and on 0.2 agent messages (Deliver), agent requests (Allow once), prepared launches (Start) and paused chains (Continue).
3. "Your apps" SHALL show one tile per running app and profile (count, needs-you badge, the account's runway, Open), at most 6 then "+n".
4. Runway by account SHALL list subscriptions with their profiles, and say "Not measured yet" with the way to measure it.
**UI/UX reasoning**: The approved canvas.
**Performance budget**: Home interactive < 300 ms.
**Quality gates**: vitest; the load gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2105/manifest.md`.
**Visual testing**: Home empty, typical, busy; both themes.
**Files to touch**: `ui/src/screens/Home.tsx`, `ui/src/components/`.
**Research basis**: as above.
**Handover note**: —

### AJ-E2106 Menus and ⌘K with icons and an order | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: Menus read in one glance: getting there first, then working on it, then stopping, each with its icon.
**User story**: As a user, right-clicking a session shows Go to it first, and the same action looks the same everywhere.
**Requirements**:
1. Every action in the registry SHALL carry an icon, a group (getting there, working on it, stopping) and a rank.
2. Menus SHALL show at most three groups, most used first, stopping last in red; one level of submenus.
3. Never possible here SHALL be left out; not possible right now SHALL be greyed with the reason.
4. ⌘K SHALL list the selected item's actions with their shortcuts, then sessions, apps and projects.
**UI/UX reasoning**: Apple's menu guidance; the approved canvas.
**Performance budget**: menu opens < 50 ms.
**Quality gates**: a test that every action has an icon and a group; a11y menu roles.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2106/manifest.md`.
**Visual testing**: each menu, both themes.
**Files to touch**: `ui/src/lib/actions.ts`, `ui/src/lib/native-menu.ts`, the menu components.
**Research basis**: Apple HIG Menus and Context menus (2026).
**Handover note**: —

### AJ-E2107 Needs you from real signals | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2101
**Goal**: "Asks you" and "Done, review it" come from what the apps record, not from idle time.
**User story**: As a user, the one session waiting for my answer is on top, and a session idle for 20 hours is not.
**Requirements**:
1. The Claude app's recorded status SHALL map: blocked → Asks you; review ready → Done, review it; completed → Idle. Only the status word is read, never the summary text.
2. Terminal sessions SHALL be read from the transcript's tail: a pending permission or question → Asks you.
3. "Done, review it" SHALL clear once the user has looked (a seen mark kept by AI Juggler).
4. Anything inferred SHALL say "probably".
**UI/UX reasoning**: Honest fields (D-06).
**Performance budget**: tails only for live sessions, ≤ 64 KB each.
**Quality gates**: fixtures per signal.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2107/manifest.md`.
**Visual testing**: each state.
**Files to touch**: the adapters' transcript readers, `features/conversations/`.
**Research basis**: the read-only field count on the maintainer's Mac (2026-10-24).
**Handover note**: —

## Step 3 — the session page and accounts (0.1.5)

### AJ-E2108 The session page: where it runs and its last messages | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2102
**Goal**: A session's page says where it runs, shows its last messages, and offers Go to it and Continue on another account.
**User story**: As a user, I read the question a session is asking before I jump to it.
**Requirements**:
1. Header: app icon and profile badge, title, chips (state, place, profile, account, folder), Go to it.
2. Last messages: the last 2–3 turns from the transcript when the page opens, with secrets redacted, kept nowhere; a Settings switch (on); Claude app chats show their title only. Never in rows, the tray or notifications.
3. Where it runs; Runway; Continue on another account (accounts with their profiles, a one-line setup check each, profiles on the same account marked "no new room").
4. Tabs: Overview, Window preview, Usage, History; sessions AI Juggler starts keep their Terminal tab first.
**UI/UX reasoning**: The approved canvas.
**Performance budget**: page < 150 ms; tail read < 30 ms.
**Quality gates**: redaction tests; the privacy claims.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2108/manifest.md`.
**Visual testing**: the page for each kind of session, both themes.
**Files to touch**: `ui/src/pages/`, `crates/juggler-adapters/src/transcript_tail.rs`.
**Research basis**: as above.
**Handover note**: —

### AJ-E2109 Accounts are subscriptions; profiles sit under them | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: The Accounts page shows each subscription once, with the profiles that use it.
**User story**: As the maintainer with two subscriptions shared across six profiles, I see two accounts, each with its profiles.
**Requirements**:
1. Profiles SHALL be grouped by their signed-in identity (account and organisation), worked out when read; no database change in 0.1.x.
2. An account card SHALL show name, email, plan, and its limits, or "Not measured yet" with the way to measure it.
3. A profile row SHALL show its badge, name, apps using it, folder, how many are open, and its menu (sign in, launch settings, default, remove, measure limits).
4. Two profiles on one account SHALL share one runway, and the same login twice is normal, not a warning.
**UI/UX reasoning**: Chrome and Arc profiles; the approved canvas.
**Performance budget**: page < 150 ms.
**Quality gates**: grouping tests (personal vs team seat on one email stay apart).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2109/manifest.md`.
**Visual testing**: one, two and six profiles; both themes.
**Files to touch**: `ui/src/screens/Profiles.tsx`, `crates/juggler-app/src/identity.rs`.
**Research basis**: D-35.
**Handover note**: The stored entity stays per folder until 0.2.

## Step 4 — Settings, docs and trust (0.1.6)

### AJ-E2110 Settings panes, docs and the Privacy page | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2101..AJ-E2109
**Goal**: Settings is a set of named panes with grouped sections, and the docs and Privacy page say what the new reads are.
**User story**: As a user, I find any setting by its pane, and the Privacy page tells me what AI Juggler reads to know what's open.
**Requirements**:
1. Panes: General (About & updates), Desktop apps, Accounts, Sessions, Switching, Notifications, Privacy, Your Macs (when paired), Advanced (Cleanup, Data, Diagnostics, Figurehead & Cloud); every existing setting keeps a home.
2. Sections have headers and one-line footers saying what a setting does.
3. The Privacy page and its tests SHALL name the new reads: the apps' session files, the parent chain of processes, transcript tails.
4. The guide, README and terminology charter SHALL use profile and account as D-35 defines them.
**UI/UX reasoning**: Apple's System Settings pattern; the approved canvas.
**Performance budget**: n/a.
**Quality gates**: trust tests; terminology and plain-words lints.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2110/manifest.md`.
**Visual testing**: each pane, both themes.
**Files to touch**: `ui/src/screens/Settings.tsx`, `docs/guide/`, `README.md`, `docs/adr/0008-terminology-charter.md`.
**Research basis**: as above.
**Handover note**: —
