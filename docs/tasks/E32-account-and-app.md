# E32 — Account and App: one word per thing, stored, each with its page

> Source: the maintainer, 2026-10-30, after clicking an App in the rail's App grouping and landing on an Account page
> (verbatim): "Account - Subscription, App - Physical desktop or CLI app
> or Profile, Projects - Git, code project, Session - Chat session
> belonging to App/Profile which also is connect to account. Multiple
> apps/profiles can have same account … fully research this in depth from
> multiple angles … find all the gaps or misalignments. Think about it
> from user perspective". The audit is
> [`docs/research/entity-model-audit.md`](../research/entity-model-audit.md);
> the maintainer approved its decisions on 2026-10-30 (D-62, decisions
> 1–7), and the New session redesign with them (decision 24).
>
> **Order.** First in 0.4's pools-and-links work: E31's pages and E30's
> App, Account and Tool members are built on these tables and words.
> AJ-E3201 and AJ-E3202 first; the pages after.
>
> **Validated by use (D-62).** Each task names its scenarios in
> [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md) (A1–A10,
> N1–N5) and isn't done until they pass by hand, scored, with nothing
> under 4.
>
> **Law for every task here:** E29's law (design quality, plain words,
> progressive disclosure, instant and paged, honest numbers, nothing acts
> by itself, cross-platform seams, every change evidenced and walked by
> hand), the 14 principles in `docs/design/PRINCIPLES.md`, and the
> colour hierarchy from 0.3.2.

## Step 1 — the words and the tables

### AJ-E3201 One word per thing: Account, App, Tool, Project, Session, on screen, in the spec and in the guide | Status: IN-PROGRESS (the rail by App, the words on screen, SPEC §3/§9, the guide and the Privacy page's claims; the terminology lint's new words remain) | Priority P0 | Effort M | Depends: —
**Goal**: Every screen, menu, filter and doc uses the maintainer's words, and a lint keeps them.
**User story**: As someone reading the rail, "Claude app · b-work" is an App and "Work (Max)" is an Account, everywhere I see them.
**Requirements**:
1. SPEC §3 and §9.1, the guide and D-35/D-36's text SHALL use Account (subscription), App (an app or its profile; a CLI App is tool plus folder), Tool (the product), Project, Session; launch settings stay launch settings.
2. Every on-screen use (filters, ⌘K, menus, switch sheets, New session's "Which account?", Overview's "Accounts used", the session's Account chip that shows a folder) SHALL be corrected.
3. CLI sessions SHALL read "Claude Code · <folder>" with the terminal as a row detail.
4. The terminology lint SHALL flag the old uses.
**UI/UX reasoning**: one word per thing (D-62 decision 1).
**Performance budget**: n/a.
**Quality gates**: the terminology lint; the gallery's changed captures critiqued.
**Evidence/validation proof**: scenarios A1–A8 read every label.
**Visual testing**: every screen whose words change.
**Files to touch**: `docs/SPEC.md`, `docs/guide/`, `ui/src/**` (copy), `scripts/terminology_lint*`.
**Research basis**: the audit's gaps table.
**Handover note**: —

### AJ-E3202 Accounts and Apps stored: two additive tables, named, with plan, renewal and fee set once | Status: IN-PROGRESS (v40's tables; the places book and Keep alive kept as the desktop Apps' rows; each sign-in stored as its Account; req. 3 done: the Account's own name, team else plan else masked email, renamable, twins told apart by email; req. 4, plan, renewal and fee kept on the Account rather than written to each folder, with a choice where they disagree, remains) | Priority P0 | Effort L | Depends: AJ-E3201
**Goal**: An Account is a thing you can name and set once; an App is remembered when it isn't running.
**User story**: As someone with two Claude Code folders and a Claude app profile on one plan, I see one Account named "Work", renamable, with its renewal set once.
**Requirements**:
1. Tables `subscriptions` (Account) and `apps` SHALL be added, with `instances.app_id`; nothing existing changes meaning.
2. An Account SHALL be keyed by the vendor's sign-in, so one plan never splits; an unreadable sign-in reads "Account unknown" and is left out of counts with a note.
3. An Account's name SHALL default to its team, else its plan, else the masked email, and be renamable.
4. Plan, renewal and fee SHALL move to the Account where the folders agree; where they disagree, the user chooses once.
5. The move SHALL take a backup first and lose nothing.
**UI/UX reasoning**: say a thing once; set it once.
**Performance budget**: grouping from the stored tables ≤ 5 ms; the move < 1 s.
**Quality gates**: L1 grouping and migration tables; L2 on the replica (several Apps per Account).
**Evidence/validation proof**: scenarios A8, A9, A10.
**Visual testing**: the conflict choice, both themes.
**Files to touch**: `crates/juggler-core/src/storage/`, `crates/juggler-core/src/accounts/`, `crates/juggler-app/src/newer_data.rs`.
**Research basis**: the audit's "What the code has today".
**Handover note**: Schema numbers follow 0.3.x and AJ-E3003's.

### AJ-E3203 Claude app profiles found when they aren't running | Status: IN-PROGRESS (built: the adapter says which app keeps its profiles side by side (`HostApp::siblings`, the Claude app) and lists the folder names beside one that ran, names only, never home or outside it, never beside the default instance, at most 64, under 50 ms; asked once on the App page ("Find this app's other profiles", naming the folder), kept, off in Settings › Desktop apps, which forgets what it found; each found profile is a desktop App row that starts from its sibling's bundle with only its folder; a profile is named by its Claude Code folder once its sessions link one, else its own folder; the Privacy page's line with its tests, including that nothing inside a profile folder is opened; gallery 99-app-profiles critiqued. L2 on the replica and A6 walked on the real setup remain) | Priority P1 | Effort M | Depends: AJ-E3202
**Goal**: Every Claude app profile is an App, even when it's closed.
**User story**: As someone with six Claude app profiles, the App list shows all six, including the ones I haven't opened today.
**Requirements**:
1. AI Juggler SHALL list the folder names beside one seen running, never their contents, after asking once on the App page.
2. The Privacy page SHALL say what is read.
3. Each SHALL become an App named "Claude app · <profile>", linked to its Account when its Code sessions show one.
**UI/UX reasoning**: no hidden things the user has.
**Performance budget**: the listing ≤ 50 ms.
**Quality gates**: L2 on the replica; a test that no file inside a profile is opened.
**Evidence/validation proof**: scenario A6.
**Visual testing**: the ask, both themes.
**Files to touch**: `crates/juggler-adapters/src/claude_app/`, `ui/src/screens/settings/Privacy*`.
**Research basis**: the audit, "Each tool".
**Handover note**: —

## Step 2 — the pages

### AJ-E3204 The App page | Status: IN-PROGRESS (built: the page, its tabs with Processes, Go to it / Start / New session, rename, the doors from the rail and Home, gallery 99-app-page critiqued; A1 and A4 hold in tests and captures; the persona walk and A3, which needs AJ-E3206, remain) | Priority P0 | Effort L | Depends: AJ-E3202
**Goal**: An App has its own page: its sessions, projects, usage, settings and processes.
**User story**: Clicking "Claude app · b-work" in the rail opens that App, with only its own sessions.
**Requirements**:
1. Header chips: Tool, Account (with "room shared with n Apps"), Mac; Go to it brings the app forward; a CLI App's main button is New session here.
2. Tabs with counts: Overview, Sessions, Projects, Pools (once E30 lands), Launch settings (CLI), Usage, Processes while it runs.
3. Settings holds launch settings, the window preview, tidy-up and "show in the rail".
**UI/UX reasoning**: the five-second test; the review's App page wireframe.
**Performance budget**: opens ≤ 150 ms from cache.
**Quality gates**: L3 pairs; critique.
**Evidence/validation proof**: scenarios A1, A3, A4.
**Visual testing**: both themes, 1024 and 1280.
**Files to touch**: `ui/src/pages/AppPage*.tsx` (new), `ui/src/pages/registry.ts`.
**Research basis**: the audit's "Proposed App page".
**Handover note**: —

### AJ-E3205 The Account page | Status: IN-PROGRESS (built: the page with limits first, its Apps, sessions, projects, usage across its folders and its Settings; the Accounts cards, Home's runway, the runway cards, "Measure its limits" and limit notices open it; no tool letter on the cards (req. 4); the folded rail's own sessions show their app's icon (req. 5); gallery 99-account-page critiqued. Pools waits for E30; A2 and A5 walked remain) | Priority P0 | Effort M | Depends: AJ-E3202
**Goal**: The subscription has its own page, with its limits first.
**User story**: From a session's Account chip, I see the account's room, its Apps, sessions, projects and pools.
**Requirements**:
1. Limits, runway and the week first; plan, renewal and fee; no email or id, only its name (D-33).
2. Tabs: Overview, Apps, Sessions, Projects, Pools, Usage; quiet Apps last on a tinted surface.
3. Every "account" door today (runway cards, Home's runway, Accounts cards, "Measure its limits", notices for limits and renewal) SHALL open it.
4. The Accounts screen's cards SHALL drop the lone tool letter in the header (the maintainer, 2026-10-31, on the "C" on every card: "what does that represent ??"). The tool shows on each App row ("Claude Code · …", "Claude app · …"). The header names a tool, in words, only when the Accounts span more than one tool. No mark there reads as a button. ("Even Codex account has it": Claude Code and Codex both draw "C", so the letter told them apart for no one. A test pins a Claude and a Codex card being told apart in words.)
5. The folded rail's icons (the rail at its icon column, narrow windows and phone width) SHALL show each session's app icon, never the same tool letter ("C" for Claude Code and Codex alike). Found in 0.3.4's gate review (`folded-390x844`): the App grouping kept the letters. A test pins a Claude Code and a Codex session told apart in the folded rail.
**UI/UX reasoning**: the account is where limits live.
**Performance budget**: opens ≤ 150 ms from cache.
**Quality gates**: L3 pairs; usage numbers equal the vendor's own (oracle).
**Evidence/validation proof**: scenarios A2, A5.
**Visual testing**: both themes, 1024 and 1280.
**Files to touch**: `ui/src/pages/AccountPage*.tsx`, `ui/src/components/RunwayCards.tsx`, `HomeRunway.tsx`.
**Research basis**: the review's Account page wireframe.
**Handover note**: —

### AJ-E3206 Every name opens its own page | Status: IN-PROGRESS (an App's name opens the App: the rail's App headers, inactive apps, Home's tiles, a session's App chip and "Where it runs"; an Account's opens the Account: the Account grouping's headers, the session menu and "Where it runs" (A3), the Accounts cards, runway names and limit notices; a folder is its CLI App now, its page retired; the Accounts tab stays lit on both pages. Req. 2 (one mark and colour per App), ⌘K and pickers (with AJ-E3105), and req. 3 (needs the Tool page, AJ-E3104) remain) | Priority P0 | Effort M | Depends: AJ-E3204, AJ-E3205
**Goal**: An App's name opens the App, an Account's name opens the Account, and ↗ brings the real app forward, on every screen.
**User story**: In the App grouping, a header opens the App; in the Account grouping, the Account.
**Requirements**:
1. The rail's grouping headers, chips, "Where it runs", runway names, notices, ⌘K results and pickers SHALL follow the rule.
2. One mark and one colour per App (today one folder can get two marks and three colours).
3. Settings › Accounts › "Tools on this computer": each tool's row SHALL open that Tool's page (AJ-E3104). Today the row shows the install and its account count, but nothing opens from it (found in the 0.3.3 walk).
**UI/UX reasoning**: one rule everywhere.
**Performance budget**: n/a.
**Quality gates**: a test over every link component's target; L3 pairs.
**Evidence/validation proof**: scenarios A1–A3, K1.
**Visual testing**: the rail in each grouping.
**Files to touch**: `ui/src/components/PlaceBoard.tsx`, `rail-items.ts`, `WhereItRuns.tsx`, `Avatar.tsx`, `ProfileBadge.tsx`, `ui/src/screens/settings/AccountsPane.tsx`.
**Research basis**: the audit's top gaps.
**Handover note**: —

### AJ-E3207 New session: the usual choice made and said, one button, the rest a step away | Status: IN-PROGRESS (built: one sentence with its why from the cached launch settings, runways and Accounts, ⌘↩ and one primary; Change folder and Change app on the shared picker (Recent, Pinned, then All; Apps by room, the ones not signed in last); the rare options behind one fold whose label says their values; no history guesses with why; out of room said with the App that has room offered, never switched (D-31); the choice pure in lib/new-session-choice with unit tests; gallery 99-new-session critiqued. The L3 pairs run, the live start in the lab, and new regression baselines for the sheet (the maintainer's approval) remain) | Priority P0 | Effort M | Depends: AJ-E3202
**Goal**: Starting the usual session is one key, and nothing rare is in the way.
**User story**: New session opens with "Start in web-shop on Claude Code · Work" and ⌘↩ starts it.
**Requirements**:
1. It SHALL open with the usual folder and App chosen and stated in one sentence with a short why.
2. Change folder and Change app SHALL be one step away, using the pickers rule.
3. Rare options SHALL be folded behind one fold whose label shows their current values.
4. With no history it SHALL make its best guess and say why; when the usual App's Account is out of room it SHALL say so and offer the one with room, never switching by itself (D-31).
**UI/UX reasoning**: principles 2–4 (prominence follows frequency, make the common choice and say it, fold the rare).
**Performance budget**: opens ≤ 100 ms with its choice made from cache.
**Quality gates**: L3 pairs over history × room × folder × tool × options × start key; a live start in the lab.
**Evidence/validation proof**: scenarios N1–N5.
**Visual testing**: both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/NewSession.tsx` and its parts.
**Research basis**: `docs/design/PRINCIPLES.md`; the review's New session wireframes.
**Handover note**: —
