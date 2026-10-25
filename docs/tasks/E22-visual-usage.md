# E22 — Usage and projects you can read at a glance

> Source: the maintainer, on 0.1.4 at a 2000 px window (2026-10-25):
> "Our usages should have a lot more info, graphs, piecharts, additional
> historical data … Not just one simple message, this is very poor
> experience. Also project same thing. Everything needs to be a lot more
> visual, easy to read." Usage was one narrow table of hundreds of rows
> that stayed blank for seconds; the project page was three 11 px lines in
> cards stretched across the window. The mockup (a private artifact,
> linked from the ADR-0020 work's report) set the look on the approved
> desktop-first canvas. It ships in 0.1.5.
>
> **Law for every task here:**
> - **The readability scale.** 22 px page titles, 15 px section titles in
>   ink, 14 px body, 13 px secondary, nothing under 12 px; muted text 7:1.
> - **Summaries, not rows.** Every chart reads a few rows the backend
>   summed; a long list is the top ten and See all, a page at a time.
> - **Instant.** The frame draws on the first render; the last summary
>   shows from memory while the fresh one is read.
> - **Honest.** Not measured says so and still shows what was used;
>   colour is never the only key.

### AJ-E2201 The readability scale as tokens | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: —
**Goal**: One type, spacing and contrast scale every new screen uses.
**User story**: As the maintainer on a wide window, I read every label without leaning in.
**Requirements**:
1. Tokens `text-title`, `text-section`, `text-body`, `text-meta`, `text-label`, `text-stat`, `max-w-content`, `p-card`, `gap-grid` in `ui/src/styles.css`.
2. Muted text ≥ 7:1 on every surface in both themes.
**UI/UX reasoning**: Linear, Vercel and Raycast set 13–15 px as the reading floor of a dense tool.
**Performance budget**: n/a.
**Quality gates**: the contrast spec; the a11y gate.
**Evidence/validation proof**: `docs/qa/evidence/_gallery/94-usage/`.
**Visual testing**: every gallery state (the muted colour changes every baseline).
**Files to touch**: `ui/src/styles.css`.
**Research basis**: WCAG 1.4.6 (7:1).
**Handover note**: Other screens adopt the tokens as they're rebuilt.

### AJ-E2202 A hand-built chart kit | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2201
**Goal**: Stat tiles, time charts, donuts, heatmaps, share bars, gauges, the limit history and a work timeline, themed and accessible.
**User story**: As a user, I see usage as pictures I can hover, read by keyboard or as a table.
**Requirements**:
1. No new dependency (ADR-0020); lazy with the screens that use it.
2. Colour by entity; a legend for two or more series; tooltips; one keyboard stop per chart that reads its marks; Show as table; empty states that keep the frame; reduced motion.
**UI/UX reasoning**: ADR-0020.
**Performance budget**: a chart renders in one frame for 400 buckets.
**Quality gates**: `ui/src/components/charts/charts.test.tsx`.
**Evidence/validation proof**: the gallery above.
**Visual testing**: in every usage state.
**Files to touch**: `ui/src/components/charts/`.
**Research basis**: ADR-0020.
**Handover note**: —

### AJ-E2203 The usage history's summaries | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: —
**Goal**: Bounded, fast summaries of usage for any range, scope and split.
**User story**: As a user with a year of heavy use, every usage page opens at once.
**Requirements**:
1. `usage_history_summary`, `usage_history_sessions` (keyset pages), `usage_history_first`, `usage_history_limits`, `usage_history_session`; async, read-only, bounded.
2. No numbered migration; settled hours kept in memory, invalidated by a derived counter of changes to older messages.
3. Sessions other apps run count where their transcripts carry tokens; the rest are counted as not measured.
**UI/UX reasoning**: summaries, not rows.
**Performance budget**: each SQL read under 50 ms for up to 90 days on a million messages; a page's reads under 50 ms once a year is kept.
**Quality gates**: `crates/juggler-core/tests/usage_history.rs`, `features::usage_history` tests, the ignored million-message budget runs.
**Evidence/validation proof**: the budget runs' output in the 0.1.5 notes.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-core/src/storage/usage_history.rs`, `crates/juggler-app/src/features/usage_history/`.
**Research basis**: —
**Handover note**: The first read of a long history is about 0.6 s, once per launch.

### AJ-E2204 Usage, a dashboard | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E2202, AJ-E2203
**Goal**: A range, the figures, usage over time, who and what used it, limits, when, and the top projects and sessions.
**User story**: As the maintainer, I see in five seconds what I used, on which account, and how much room is left.
**Requirements**:
1. Range 24 h, 7, 30, 90 days, all; four figures with their change; usage over time by account, profile, tool, model or project; donuts by account and model; a runway card per account; an hour heatmap; sessions per day; the top ten projects and sessions with See all.
2. Names, never ids; compact numbers, exact on hover; each row opens its page.
**UI/UX reasoning**: the mockup; the approved canvas.
**Performance budget**: the frame in the first render; content from memory on a second open.
**Quality gates**: `ui/src/usage-null-payload.test.tsx`, `ui/tests/visual/usage-gallery.spec.ts`.
**Evidence/validation proof**: `docs/qa/evidence/_gallery/94-usage/usage-*`.
**Visual testing**: 1280, 1680, 2000 px, both themes, 200 % zoom.
**Files to touch**: `ui/src/screens/Usage.tsx`, `ui/src/components/usage/`.
**Research basis**: —
**Handover note**: The per-model price table returns as a paged list once `cost_rows_page` is merged.

### AJ-E2205 A project's page, visual | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2202, AJ-E2203
**Goal**: The Overview leads with what's open and the work's shape; Usage is the dashboard for the project.
**User story**: As the maintainer, a project's page tells me what's running, who did the work, and what to commit.
**Requirements**:
1. Figures, activity, who did the work, open sessions, the checkout as a callout with an action, who starts work here, how the work travelled.
2. History-wide numbers from the summaries, never from the app's working set.
**UI/UX reasoning**: the mockup.
**Performance budget**: as AJ-E2204.
**Quality gates**: `ui/src/components/inspector-*.test.tsx`; the gallery.
**Evidence/validation proof**: `docs/qa/evidence/_gallery/94-usage/project-*`.
**Visual testing**: as AJ-E2204.
**Files to touch**: `ui/src/pages/ProjectPage.tsx`, `ui/src/pages/project/`, `ui/src/pages/ProjectUsage.tsx`.
**Research basis**: —
**Handover note**: Its Sessions tab moves to the shared paged list at merge.

### AJ-E2206 A profile's page and the account cards | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2202, AJ-E2203
**Goal**: Limits and usage first; settings in their own section; readable cards.
**User story**: As the maintainer, I see each account's room and its week at a glance, measured or not.
**Requirements**:
1. Gauges for both windows with resets and the reading's age; the week's limit history; usage by project; the settings grouped below; the limits capture a quiet last row.
2. Account cards: gauges, a 7-day usage line, profile rows at the readable scale with short paths.
3. States: fresh, reset since the last reading, limit hit, measuring, never measured (with its usage).
**UI/UX reasoning**: the canvas's Accounts board.
**Performance budget**: as AJ-E2204.
**Quality gates**: `ui/src/screens/accounts/accounts-grouped.test.tsx`, `ui/src/quota-unmeasured-surfaces.test.tsx`.
**Evidence/validation proof**: `docs/qa/evidence/_gallery/94-usage/profile-*`, `accounts-*`.
**Visual testing**: as AJ-E2204.
**Files to touch**: `ui/src/pages/AccountPage.tsx`, `ui/src/pages/account/`, `ui/src/screens/accounts/`.
**Research basis**: D-35.
**Handover note**: —

### AJ-E2207 A session's usage | Status: IN-PROGRESS (built; waits for the session page's Usage tab to mount it) | Priority P1 | Effort S | Depends: AJ-E2202, AJ-E2203
**Goal**: A session's tokens turn by turn, by kind and model, and its share of its windows.
**User story**: As a user, I see what one session cost and how much of my limits it took.
**Requirements**:
1. `ui/src/pages/session-usage/SessionUsage.tsx` with `profile` and `conversation`.
**UI/UX reasoning**: as AJ-E2204.
**Performance budget**: one read of at most 2,000 turns.
**Quality gates**: typecheck; a gallery state once mounted.
**Evidence/validation proof**: once mounted.
**Visual testing**: once mounted.
**Files to touch**: `ui/src/pages/session-usage/SessionUsage.tsx`.
**Research basis**: —
**Handover note**: Mount it in the session page's Usage tab after both branches merge.
