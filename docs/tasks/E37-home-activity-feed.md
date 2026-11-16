# E37 — Home as an activity feed: what happened, what needs you, your apps and runway (0.5, D-118)

> Source: the maintainer, 2026-11-14: "it should go into 0.5"; "like a
> twitter feed… imagine we have an agent tweeting about what is going on
> with our system". Approving the proposal and its mock: "Love it lets do
> it". Ruled as D-118, on D-117's calm attention model.
>
> The research (activity feeds, rollups, seen and read, the feed's voice,
> the three ways to write updates), the catalogue of every feed item with
> its source, level and rollup, the proposal and the mock are in
> [`docs/research/home-feed/`](../research/home-feed/). The mock's
> captures: `docs/qa/evidence/AJ-E3701/mock-home-feed-{dark,light}.png`.
>
> **The rule this epic builds.** Home, for someone with active Accounts, is
> two columns that scroll on their own: on the left, the open needs pinned
> on top (Home's one loud mark for each), then what happened, newest first,
> never ranked, rolled up so it never spams; on the right, Your apps and
> Runway by account. Every sentence is written from stored facts (a), with
> the agent's own last reply quoted where the user allows it (a+). Nothing
> is written by a model in 0.5. New users keep first run; the rail's
> Needs you is unchanged.
>
> **Retired into the feed:** the overnight summary card (AJ-E1004 R2)
> becomes the away digest and keeps its last-seen key
> (`summary_seen_at_ms`); "Pick up where you left off" folds into the
> Finished filter.
>
> **Architecture.** One read model (`features/feed/`) unions the existing
> journals with keyset pages; rollups are applied there; every sentence
> comes from one `feed-words.ts`. New facts (F2, F3) are recorded by the
> modules that own them, through the adapters or the existing git reading,
> never a tool check outside the adapters.
>
> **Order.** AJ-E3701 after AJ-E3601 and AJ-E3602; AJ-E3702 next; then
> AJ-E3703…AJ-E3706 beside the E36 screen passes; AJ-E3707 with AJ-E3702
> and closed last.

### AJ-E3701 Home in two columns: the layout, Your apps and Runway by account (Home's calm pass) | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3602
**Goal**: Home becomes two calm columns that scroll on their own, with your apps and runway always one glance away.
**User story**: As someone with three Apps on two Accounts, I open Home and see what needs me and what happened on the left, and my apps and their runway on the right, without either pushing the other off screen.
**Requirements**:
1. Home for a user with at least one active Account SHALL be two columns under a fixed head (title and counts): the feed column `minmax(480px, 1fr)` with a 720 px reading cap, the right column `clamp(320px, 30%, 400px)`, a 24 px gap; each column its own scroll container (`overflow-y: auto`, `overscroll-behavior: contain`) with the existing scroll-edge hairline once scrolled.
2. The right column SHALL hold Your apps (compact round-2 rows: App icon in the App tone, name, profile dot and name, "n open", the needs mark, a ghost Open or Start; running first by recent use, then a muted "Not running" group; past 8, "Show all") and Runway by account (round-2 runway rows: neutral meter, amber at ≤ 10 % with a live session, red when out with live work, "Not measured yet · Measure its limits ›").
3. Both right-column sections and the feed's pinned zone SHALL be collapsible, the state remembered per viewer (localStorage in a try/catch), chevrons as D-117 point 3.
4. Below 860 px of main area (just above the columns' own 824 px minimum, so a 1024 px window and a 1200 px one with the rail open keep two columns; the 0.5 session's design call, 2026-11-16) the columns SHALL stack, feed first: one page (the pinned zone and about 12 items) then "Show more", no automatic paging; the head gains an "Apps & runway ↓" link; under 640 px Your apps becomes a strip of icons.
5. Users with no Accounts SHALL keep today's Welcome and first run unchanged; the rail's Needs you is unchanged.
6. Home's calm pass (the E36 screen pass, done here once): an audit of Home's loud elements against the budget; the calm look applied (the head's title in ink, the pinned zone's count carrying the colour); the attention-budget test on Home's fixtures; contrast and axe, both themes; a Sonnet persona glance and walk by hand, verified by Opus; the design-quality critique; before and after captures on the review page, each with what changed.
**UI/UX reasoning**: desktop-first navigation (what is open, where, one click); NN/g on line length (720 px cap); progressive disclosure (collapsible sections); the right column as periphery (calm technology).
**Performance budget**: the layout paints from cached data in ≤ 100 ms after Home opens; each column scrolls at 60 fps; collapsing a section ≤ 16 ms.
**Quality gates**: verify-static; vitest for the layout states; the attention-budget test; the Home gallery; `idle.spec.ts` green (Home draws only on a change, 0.4.6's AJ-E813).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3701/manifest.md`, R1–R6; the mock's captures already there are the design reference, not evidence.
**Visual testing**: Home at 1024, 1280 and 1440, stacked at 900, both themes; all-clear, typical, busy and flood fixtures; each section collapsed and open.
**Files to touch**: `ui/src/screens/Home.tsx`, `ui/src/screens/home-cards.tsx`, `ui/src/components/HomeApps.tsx`, `HomeRunway.tsx`, `HomeNeeds.tsx`, `HomeExtras.tsx`, `ui/tests/visual/home-gallery.spec.ts`.
**Research basis**: `docs/research/home-feed/proposal.md` §1, §8, §9; `docs/research/calm-attention/proposal.md` §(c2); D-118 point 1.
**Handover note**: AJ-E2105's Home (needs and apps) and 0.4.6's Home work (AJ-E813: Home draws only on a change) are the base; merge the 0.4 line in first. The feed column can ship its pinned zone over today's needs before AJ-E3702 lands, so Home is never half-built in a commit.

### AJ-E3702 The feed: pinned needs, the stream, rollups, the away digest, the new-updates pill, filters, paging and the voice (F1) | Status: IN-PROGRESS | Priority P0 | Effort XL | Depends: AJ-E3701
**Goal**: The left column tells the story of what happened, in one calm stream, from the data AI Juggler already keeps.
**User story**: As someone back after a night away, I see what needs me first, then what finished, switched or broke while I was gone, each line one click from its thing.
**Requirements**:
1. A read model (`crates/juggler-app/src/features/feed/`) SHALL serve `feed_page(before: (at_ms, id), filters, limit: 30)` as one keyset query over the existing journals: `notices`, `instance_lifecycle` (`ended:`), `switch_edges`, `task_events` (done, review), `agent_messages` (Handoff, ReviewRequest, Question, Status) and the conversation index, newest first; never a load-all.
2. The pinned zone SHALL hold every open L2 and L3 need ("Needs you now", the same count as the rail, AJ-E3602), loud there once, with the feed's one filled primary on the first; more than 3 collapse into "+n more need you"; filters never hide it; resolved needs leave it and a quiet history line stays in the stream ("You answered…").
3. The stream SHALL be chronological with no ranking, items without borders, day separators as section labels, the item anatomy of the proposal's §2 (unread dot, author in the App tone, sentence, facts line, ghost actions on hover, focus or selection).
4. Rollups SHALL follow the proposal's §3 and `events.md`: one session one card updated in place; the same verb, App and project within 2 h; pool threads per 30 min (Note chatter only counted); tasks per pool per hour. A rollup key never crosses a level boundary: nothing broken is ever folded into a quiet rollup.
5. The away digest SHALL replace the morning summary card: when the last seen time (`summary_seen_at_ms`) is 2 h or more ago, a "While you were away · since …" divider (the feed's only rule line) and a digest card with counts by kind and the 3 most important items, "Show all" and "Mark all seen"; "Pick up where you left off" folds into the Finished filter, with "Mark all reviewed" in its ⋯.
6. New items while the reader is scrolled down SHALL wait behind an "N new updates" pill; at the top a new item fades in once over 600 ms (off under Reduce Motion); the reader's place never moves.
7. Filters SHALL be type chips (All, Needs you, Finished, Switches & limits, Pools & tasks, This Mac) as a filled segmented control, plus one Project, App, Pool or Account picker (suggested, then All, search past about 8), remembered per viewer; "Include my actions" off by default in ⋯; a muted "Filtered: … ×" line when filtered.
8. Paging SHALL load the next page as the end comes into view for 3 pages, then a Load more button; the cached first page (local cache ruling) paints at once, or 6 skeleton cards without a cache; the live page replaces it without moving the scroll position; an error keeps the cards with "Showing what was loaded at … · Retry".
9. Seen and read SHALL be tracked apart: Home in front marks a page's items seen; opening or acting on an item marks it read and clears its unread dot; the away boundary moves on "Mark all seen" or after Home has been in front 10 s with the digest on screen.
10. Every sentence SHALL come from one `ui/src/lib/feed-words.ts` (as `notice-words.ts`): subject first, what changed, the result, a link to the thing; no emoji, no exclamation marks, no first person; the author is whoever acted (the App, the agent, the Account, or AI Juggler only for its own actions); numbers measured or said to be estimates; times relative under 1 h, then the clock, "Yesterday …", then the date, the exact time in the tooltip and accessible name, re-rendered once a minute.
11. Every item SHALL open its thing (session, task, thread, Account, switch, This Mac) and ↗ goes to the App where there is one; no dead end.
12. An empty feed SHALL say "Quiet so far. When your apps start, finish or ask you something, it shows here." with New session and Open an app.
**UI/UX reasoning**: chronological beats ranked in work tools (GitHub's 2023 feed backlash, research §1–2); rollups by key (Stream, Notion, Datadog); X's new-posts pill instead of moving the reader; NN/g on infinite scroll (Load more after a few pages); "Every object has a door".
**Performance budget**: `feed_page` ≤ 50 ms for 30 items on the reference database and on a 5,000-event fixture; the cached first page painted ≤ 100 ms after Home opens; scrolling 90 items at 60 fps; no polling faster than the existing event push.
**Quality gates**: verify-static; Rust tests for the keyset query and every rollup key (an L3 item never rolled into L0); vitest for `feed-words.ts` (one fixture per item type in `events.md`) and the pill, filters and seen/read; the attention-budget test with the pinned zone (one loud mark per need on Home); a paging test on a 5,000-event fixture proving three automatic pages, then Load more, and that no request asks for more than one page.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3702/manifest.md`, R1–R12; a Sonnet persona walk on a busy-day fixture ("What happened while I was away?", "What needs me?") for the vibe-coder after a night away, a newcomer with 1 App and a heavy day (14 open, 7 needs, a crash), with the 5 s and 10 s glance, verified by Opus; the design-quality critique; before and after captures on the review page.
**Visual testing**: the feed with every item type, rollups folded and open, the away digest, the pill, each filter, the empty, loading and error states; both themes; 1024, 1280 and stacked.
**Files to touch**: `crates/juggler-app/src/features/feed/` (new), `crates/juggler-app/src/overnight.rs`, `crates/juggler-app/src/features/mod.rs`, `ui/src/lib/feed-words.ts` (new), `ui/src/components/HomeFeed*.tsx` (new), `ui/src/components/MorningSummary.tsx` (retired), `ui/src/components/HomeRecent.tsx` (retired), `ui/src/components/PagedList.tsx`, `ui/tests/visual/fixtures/`.
**Research basis**: `docs/research/home-feed/proposal.md` §§2–8, §12; `events.md` §§1–6; `research.md` §§1–4; D-118 points 2–3.
**Handover note**: rollups change in place, so keep the group key, count, sample names and newest time per card (Stream's model); changing a key later means a backfill. The feed's copy of an open need is a quiet history line, never a second loud mark. Work out the item's level with `attentionLevel()`; the feed adds no level of its own.

### AJ-E3703 The agent's own last reply, quoted behind "show last messages" (a+) | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3702
**Goal**: A finished session's card says what it did in the agent's own words, at no model cost.
**User story**: As someone reading the feed, I see "Fixed the race in the login test… all 212 tests pass" under the finished session, quoted from the agent.
**Requirements**:
1. A finished turn's or session's card SHALL quote the first sentence or two of the transcript's last final-answer block (`TextBlock.final_answer`), read with the existing bounded, redacted `last_messages` read, clamped to 2 lines with "Show more" in place.
2. The quote SHALL show only while the existing "show last messages" setting (`show_last_messages`) is on; off, the card shows template facts only, and nothing is read.
3. The excerpt SHALL go into the registered, clearable local cache (D-58), keyed by the conversation and its last turn.
4. A tool that marks no final answer SHALL show no quote (never a guessed excerpt), and the persona walks record how often the quote is missing or unhelpful, the evidence for any later (b).
**UI/UX reasoning**: quoting beats paraphrasing ("Accuracy means what it claims"); privacy by the user's existing choice.
**Performance budget**: the excerpt read ≤ 20 ms per card from cache, ≤ 100 ms cold, off the main thread, only for cards in view.
**Quality gates**: verify-static; tests for the setting's gate (no read when off), redaction, clamping and the cache entry.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3703/manifest.md`, R1–R4; the missing-or-unhelpful rate from the AJ-E3702 walks.
**Visual testing**: a card with a short, a long and no quote; the setting on and off; both themes.
**Files to touch**: `crates/juggler-app/src/features/conversations/last_messages.rs`, `crates/juggler-adapters/src/conversation_feed.rs`, `crates/juggler-app/src/features/feed/`, `ui/src/lib/last-messages.ts`.
**Research basis**: `docs/research/home-feed/research.md` §5 (a+); D-118 point 4.
**Handover note**: on-device summary lines (b) are in 0.5 under D-120 (AJ-E3708 to AJ-E3711); this task's quote becomes the first source of AJ-E3708's excerpts. Summaries through the user's own provider accounts (C) stay out until providers give permission.

### AJ-E3704 The journal gaps the feed needs: a switch's completion time, runway crossings, a daily usage digest, and the files a session left (F2: G2, G3, G6, G7) | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3702
**Goal**: The feed can say when a switch finished, when an Account crossed a threshold, what yesterday used, and what a session left uncommitted.
**User story**: As someone reading the feed, "moved at 9:58 AM", "Work Max dropped to 12 % at 2:10 PM" and "3 files not committed" are facts, not guesses.
**Requirements**:
1. G2: `switch_edges` SHALL record the time its state last changed (`state_changed_at`, set when it is confirmed or fails), additive migration, its rows still append-only in kind; the feed's switch items use it.
2. G3: the quota alert's phase changes (`TrayAmber`, `Actionable`, `Exhausted`, `ResetReached`) SHALL be stored as Earlier notices of kind `runway`, one per Account per window per phase, kept as notices are (30 days).
3. G6: a daily usage digest SHALL be computed lazily at read time from `usage_events` and `quota_samples` (one row per day: tokens, sessions, Accounts, the busiest project), never by a background job.
4. G7: when a session ends, its `git_dirty_count` and the count of files it edited SHALL be recorded on its `ended:` lifecycle event.
5. Each SHALL appear in the feed through `feed-words.ts` with its item type from `events.md` (W1, W4, W10, S12).
**UI/UX reasoning**: "Accuracy means what it claims": a time or count in the feed is measured.
**Performance budget**: no added work on the hot path: G2 and G7 are one column write at an existing event; G3 one insert per phase change; G6 ≤ 30 ms per day computed, then cached.
**Quality gates**: verify-static; migration tests (up from the 0.4 schema, no data lost); unit tests per gap.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3704/manifest.md`, R1–R5.
**Visual testing**: the four item types in the feed, both themes.
**Files to touch**: `crates/juggler-core/src/storage/mod.rs`, `crates/juggler-core/src/storage/switch_graph.rs`, `crates/juggler-app/src/features/notifications/sources.rs`, `crates/juggler-app/src/features/usage_history/`, `crates/juggler-app/src/features/sessions/`, `crates/juggler-app/src/features/feed/`.
**Research basis**: `docs/research/home-feed/events.md` §7 (G2, G3, G6, G7); D-118 point 5.
**Handover note**: the switch graph is shared with E2 and AJ-E213; check no other session is changing `storage/switch_graph.rs` before the migration, and keep the edge's own history append-only.

### AJ-E3705 Other apps' conversation state changes and a turn's end, persisted (F2: G4) | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3702
**Goal**: The feed can say "finished", "asked you" and "ended a turn" for sessions other apps run, after the fact, not only while AI Juggler watched live.
**User story**: As someone who closed AI Juggler overnight, the feed still shows which of the Claude app's sessions finished while I was away.
**Requirements**:
1. Conversation state changes the conversation index sees (`asks_you`, `working`, `idle`, `done_review`) SHALL be stored as `conversation_events(conv_id, from, to, at_ms)`, kept 30 days like notices, written only on a change.
2. A driven session's turn end (`RuntimeEvent::TurnEnded`, with its outcome) and a desktop app's `TurnOutcome` SHALL be stored the same way, so a finished card can say "in 6 turns".
3. Every adapter SHALL feed this through the existing conversation index, with no tool check outside the adapters.
4. On start, a state reached while AI Juggler was closed SHALL be recorded once with the time the tool's own files give (its last activity), marked as seen on start, never as a live time.
**UI/UX reasoning**: the away digest has to cover the apps the user actually used, not only AI Juggler's own sessions.
**Performance budget**: one insert per real change; no new file reads beyond the index's existing ones; retention pruning in the existing cleanup pass.
**Quality gates**: verify-static; tests per adapter fixture (Claude Code, Codex, the Claude app); a test that a steady state writes nothing.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3705/manifest.md`, R1–R4.
**Visual testing**: the feed after a fixture night with three apps' sessions finishing, both themes.
**Files to touch**: `crates/juggler-app/src/features/conversations/`, `crates/juggler-adapter/src/runtime/events.rs`, `crates/juggler-core/src/storage/`, `crates/juggler-app/src/features/feed/`.
**Research basis**: `docs/research/home-feed/events.md` §1 (S5, S9) and §7 (G4); D-118 point 5.
**Handover note**: the index is in memory today; persist only transitions, and prune by the same retention as notices. The undocumented-route standing order (D-106) applies to any app whose state isn't in its files yet.

### AJ-E3706 Git commits, pull requests and the task-close note in the feed, through the adapters (F3: G1, G5) | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3702
**Goal**: The feed can say what an agent shipped: commits, pull requests and a task's closing note.
**User story**: As someone reading the feed, I see "opened PR #142 'Add the export button' in web-app" and "finished the task, note: CSV and JSON both covered".
**Requirements**:
1. G1 commits: when a session ends, a read-only `git log --since=<its start>` in its folder SHALL be recorded as an Earlier notice of kind `commits` (count, branch, project), through the existing worktree reading, for any tool and folder.
2. G1 pull requests: when the user's own `gh` is installed and signed in and the existing Git setting allows it, a read-only `gh pr list --author @me --json …` for the session's repository SHALL record opened and merged pull requests as notices of kind `pr`, a later merge updating the same item; without `gh`, nothing is read and nothing is said.
3. G5 the task-close note: the pool agents' task tool SHALL accept a closing summary (`close(summary)`), stored in `task_events.detail`, shown on the feed's task item.
4. Each SHALL go through the adapters or the existing git reading: no per-tool branch outside the adapters, no write to the repository, no network call other than the user's own `gh`.
5. Before calling any of these a gap for a tool, the undocumented-route research pass (D-106) SHALL be recorded (for example the Claude app's "Auto-fix on pull request" automation).
**UI/UX reasoning**: deploy-list voice (Vercel, Netlify: status, branch, message, author, time); "Enhance, never replace" (the user's own `gh`).
**Performance budget**: the reads run once per session end, off the main thread, each ≤ 2 s with a hard timeout; nothing on a timer.
**Quality gates**: verify-static; tests on a fixture repository; a test that `gh` absent or signed out reads nothing; the bridge's task tool tests for `close(summary)`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3706/manifest.md`, R1–R5; a lab walk on a throwaway repository (never a real one) with a commit and a pull request on the lab account.
**Visual testing**: the commits, PR and task-note items, both themes.
**Files to touch**: `crates/juggler-app/src/worktree.rs`, `crates/juggler-app/src/features/feed/`, `crates/juggler-app/src/features/tasks/`, `crates/juggler-agentbridge/`, `crates/juggler-app/src/features/notifications/sources.rs`.
**Research basis**: `docs/research/home-feed/events.md` §3 (P1, G5), §4 (G-a, G-b) and §7 (G1, G5); D-118 point 5.
**Handover note**: D-118 lists the task-close note under both F2 and F3; it is built here, once, with the git items. The live check uses the lab account in a separate profile (D-107), read-only on real repositories.

### AJ-E3707 The feed's accessibility: the feed pattern, keys and one batched announcement | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3702
**Goal**: The feed works as well with a keyboard and a screen reader as with a mouse.
**User story**: As someone using VoiceOver or only the keyboard, I move item by item, hear what's new once, and never lose my place.
**Requirements**:
1. The stream SHALL follow the WAI-ARIA feed pattern: `role="feed"`, `aria-busy` while a page loads, `aria-setsize="-1"`; each item an `article` with `aria-posinset`, `aria-labelledby` (author and sentence) and `aria-describedby` (quote and facts); a rollup an `article` with a disclosure button (`aria-expanded`).
2. Landmarks SHALL name the columns: the feed a `section` labelled "Activity", the right column an `aside` labelled "Apps and runway", the pinned zone a `region` labelled "Needs you now".
3. Keys: Page Down and Page Up, J and K move between items; Enter opens; A runs the item's action; R, H and S as Home's existing keys; Ctrl+End and Ctrl+Home leave the feed; Tab walks the focused item's controls; focus never moves when items arrive.
4. New items SHALL be announced once, politely, batched to at most one announcement per 30 s ("3 new updates"), with no per-item announcements; the existing `AttentionAnnouncer` keeps covering needs.
5. Contrast SHALL meet round 2 in both themes (quotes in ink-2 on `--color-fill` included), and the fade and tone transitions stop under Reduce Motion.
**UI/UX reasoning**: WAI-ARIA APG feed pattern; WCAG 2.2 (2.1.1 keyboard, 2.4.3 focus order, 4.1.3 status messages).
**Performance budget**: a key move ≤ 16 ms; the announcement batch adds no render.
**Quality gates**: an a11y feed-pattern Playwright test (roles, set sizes, keys, focus kept on arrival, one announcement per batch); axe on every feed state; the keyboard flows test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3707/manifest.md`, R1–R5; a keyboard-only Sonnet walk of "what happened while I was away?" and "what needs me?", verified by Opus.
**Visual testing**: focus rings on items, rollups and the pill; both themes; Increase Contrast on.
**Files to touch**: `ui/src/components/HomeFeed*.tsx`, `ui/src/App.tsx` (the announcer), `ui/tests/visual/a11y.spec.ts`, `ui/tests/visual/keyboard-flows.spec.ts`.
**Research basis**: `docs/research/home-feed/proposal.md` §10; `research.md` §1.6; D-118 point 6.
**Handover note**: build these with AJ-E3702's components, not after them; this task closes when its test and walk pass on the finished feed.

> **Feed summaries (D-120, 2026-11-16).** A (the agents' own excerpts) and B
> (on-device AI lines) are in 0.5: AJ-E3708 to AJ-E3711. Research:
> [`docs/research/feed-summaries-and-orchestrator-options.md`](../research/feed-summaries-and-orchestrator-options.md).
> **Not scheduled:** C (the user's own Claude, Codex or Gemini summarising)
> goes last, only once providers give permission; D (a cloud service) and E
> (the orchestrator bot) are a later release, which the maintainer is choosing.
> Don't build hooks for C, D or E.

### AJ-E3708 Per-agent excerpts from what agents already write, through the adapters (A) | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3702
**Goal**: Each agent's feed item carries a short excerpt of what it is doing or just did, taken from what it already writes, at no model cost and fully local.
**User story**: As someone scrolling Home, I see per agent "Fixed the login race; 212 tests pass; 2 files changed" or "Waiting: which database should I use?", so I know how each is going without opening it.
**Requirements**:
1. Each adapter SHALL declare which excerpt sources it offers (a capability, never a tool check outside the adapters): Claude Code's turn-end `last_assistant_message` from its `Stop` and `SubagentStop` hooks (hooks, not the transcript file, which Anthropic calls internal); Codex's exec and rollout events (agent message, plan update, file changes, commands); task lists where the tool writes them ("3 of 7 tasks done"); and git facts (files changed, tests run, commits) through the existing git reading.
2. The excerpt SHALL be the agent's own text, trimmed to a clean opening (sentence boundary, 2 lines, "Show more" in place), redacted and bounded like the existing `last_messages` read, and shown only while "show last messages" is on; off, only facts (counts, state) show and no text is read.
3. A tool or turn with no text, or no task list (newer Claude models don't write one by default), SHALL show the facts it has and nothing guessed; the item never shows an empty excerpt.
4. Excerpts SHALL be stored in the registered, clearable local cache (D-58), keyed by conversation and turn, and come through `feed-words.ts` like every other sentence.
5. The excerpt SHALL pass through the shared input cleaner of ADR-0033 v3 (join hard-wrapped lines, strip list markers and conventional-commit prefixes, leave clean text unchanged), the same one B and Chat use.
6. The excerpt's caveat line: if the source says "not run", "not tested", "not done", "partial" or "N of M", the shown excerpt SHALL include that sentence.
7. This SHALL be the excerpt layer B reads from and the layer B falls back to; it supersedes AJ-E3703's single quote (AJ-E3703 becomes the first source here, not a separate path).
**UI/UX reasoning**: quoting beats paraphrasing ("Accuracy means what it claims"); Claude Code's own agent view does the same without a model at most every 15 s; privacy by the user's existing choice.
**Performance budget**: an excerpt ≤ 20 ms from cache, ≤ 100 ms cold, off the main thread, only for items in view; hooks add no measurable cost to the agent's turn; idle stays under 2% CPU.
**Quality gates**: verify-static; adapter conformance tests per source (hook payload, rollout events, task list, git facts) with fixtures; tests for the setting's gate, redaction, clamping, the cache entry and the no-text cases; a check that nothing outside `crates/juggler-adapters` names a tool.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3708/manifest.md`, R1–R6; a Sonnet lab walk on a throwaway repository with a Claude Code and a Codex session (a finished turn, a question, a failure), verified by Opus; the missing-or-unhelpful rate recorded for B.
**Visual testing**: items with a short, a long, a question and no excerpt; the setting on and off; both themes.
**Files to touch**: `crates/juggler-adapters/` (each adapter's excerpt capability), `crates/juggler-app/src/features/feed/`, `crates/juggler-app/src/features/conversations/last_messages.rs`, `ui/src/lib/feed-words.ts`, `ui/src/components/HomeFeed*.tsx`.
**Research basis**: `docs/research/feed-summaries-and-orchestrator-options.md` (Option A); `docs/research/home-feed/research.md` §5; D-118 points 4 and 5; D-120 point 1.
**Handover note**: a hook route for Claude Code may need a hook the user has installed; use what the existing hooks reading already offers first, and research undocumented routes (files, logs, local servers) before calling a source a gap.

### AJ-E3709 On-device summary lines: the multi-angle research pass (B research) | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3702
**Goal**: The facts the design needs, checked from many angles, so the ADR rests on evidence rather than on the first research doc.
**User story**: As the maintainer, I want to know what the Mac and a small model can really do for one-line summaries before we design around them.
**Requirements**:
1. The pass SHALL cover, with sources and an UNVERIFIED mark on anything unconfirmed: Apple's Foundation Models (availability, context size by macOS version, background and non-App-Store use, language and region limits, the Swift bridge and the maturity of the Rust crates); small open models (licences, sizes, quality on agent transcripts, Apple-silicon speed and energy per summary); runtimes (llama.cpp, mistral.rs, MLX) and how each ships and signs inside the app; the user's own Ollama or LM Studio (detection, the local ports, what a user may have loaded); and Windows and Linux equivalents (D-52).
2. It SHALL measure, on the real Mac and in a throwaway lab: summary quality and faithfulness (does it add facts?) on a fixture of real-shaped agent messages, latency, memory, CPU and battery per summary against the idle under 2% budget.
3. It SHALL research the automatic-fetch rules: suitable-hardware tests (chip, RAM, free disk), Wi-Fi or unmetered and Low Data Mode detection, on-power checks, checksummed and resumable downloads, a disk cap, hosting and its cost, and the privacy and consent wording.
4. It SHALL research the user-facing side: how comparable apps tell the user once and offer the switch, and how the notice and the Settings control read without internal words.
5. Any model or runtime download for measuring SHALL wait for the maintainer's explicit yes in chat; until then the pass reads sources and uses what is already on the Mac.
**UI/UX reasoning**: "most important is ease of use": the user is never asked to download a model; automation is the default, with a way out.
**Performance budget**: the pass sets the budget the build must keep (per summary latency, RAM, energy).
**Quality gates**: every claim cited or marked UNVERIFIED; figures reproducible from a script in the evidence folder.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3709/manifest.md`, R1–R5; the research file `docs/research/feed-summaries-on-device.md`, checked by Opus before AJ-E3710 starts.
**Visual testing**: none (research); the notice and Settings wording drafts as text.
**Files to touch**: `docs/research/feed-summaries-on-device.md`, `docs/qa/evidence/AJ-E3709/`.
**Research basis**: `docs/research/feed-summaries-and-orchestrator-options.md` (Option B, open questions); D-120 point 2.
**Handover note**: a Sonnet research agent reads the web; Opus verifies the numbers that drive the design. Never send the user's real transcripts to any outside service; measure on fixtures.

### AJ-E3710 On-device summary lines: the design, an ADR for a pluggable SummaryEngine (B design) | Status: IN-PROGRESS (design awaiting the coordinating session's review: [ADR-0033](../adr/0033-feed-summary-engines.md)) | Priority P0 | Effort M | Depends: AJ-E3709
**Goal**: One decided, extensible design for summary lines that the coordinating session has seen before any code is written.
**User story**: As the maintainer, I want one seam that Apple's model, a small local model and plain excerpts all plug into, so a new engine never touches the feed.
**Requirements**:
1. An Opus ADR SHALL define a `SummaryEngine` seam: capability detection, a run call that takes an agent's own text and returns one line or "none", a faithfulness contract (never adds facts), a cost and health report, and implementations for Apple's Foundation Models, a local model and the extractive fallback (A's excerpts); a user's own Ollama or LM Studio is an optional implementation, never required.
2. It SHALL set the order of degrade (Apple, then a background-fetched small model on suitable hardware, then A's excerpts), the hardware and network rules for the fetch (Apple silicon, enough RAM and disk, unmetered, on power, Low Data Mode respected, checksummed, resumable, a disk cap) and where the model lives in the registered, clearable cache (D-58).
3. It SHALL specify the notice (told once) and the Settings switch, the faithfulness check, the failure behaviour (a slow, wrong or missing engine falls back silently to A), and the OS seams for Windows and Linux (D-52).
4. It SHALL keep every tool check inside adapters and every sentence in `feed-words.ts`, and no summary text leaves the Mac.
5. The design SHALL go to the coordinating session for the maintainer's eyes before AJ-E3711 starts.
**UI/UX reasoning**: graceful degradation with automation as the default; the Settings switch and the one-time notice are the maintainer's explicit asks.
**Performance budget**: the ADR states the build's budget: per-summary latency, RAM, energy, and idle under 2% CPU.
**Quality gates**: the ADR passes the architecture review (no per-tool branches outside adapters, no duplication); a trait-level sketch compiles in a spike branch that is not merged.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3710/manifest.md`, R1–R4; the ADR in `docs/adr/`, checked by Opus.
**Visual testing**: none; wireframes of the notice and the Settings control, plain words only.
**Files to touch**: `docs/adr/` (a new ADR), `docs/qa/evidence/AJ-E3710/`.
**Research basis**: AJ-E3709's research; `docs/research/feed-summaries-and-orchestrator-options.md`; D-120 point 4; the architecture standard.
**Handover note**: the seam must also serve a later cloud or orchestrator engine (D and E, not scheduled) without change, but build no hook for them now. The design is [ADR-0033](../adr/0033-feed-summary-engines.md) (Proposed, 2026-11-16), written to D-120 as amended: Apple's model automatic, the local model offered on one click with its model left as a slot for the selection study. Open, both the maintainer's: Q1 (the measurement download) and Q5 (the battery measurement).

### AJ-E3711 On-device summary lines: detection, the three tiers, the tier 2 offer card and one-click download, the notice and the Settings switch (B build) | Status: TODO | Priority P0 | Effort XL | Depends: AJ-E3710
**Goal**: The feed shows one clean line per agent when the Mac can write it comfortably (no step from the user for Apple's own model, one click for the optional download), and falls back to excerpts when it can't.
**User story**: As a vibe-coder with several agents, I see "Fixing the login test; two failures left" instead of a long message, I was told once that my Mac does this itself, and I can turn it off.
**Requirements**:
1. Detection SHALL decide per Mac, with no prompt: tier 1, Apple's Foundation Models (macOS 26+, Apple Intelligence on, supported language and region, no download); tier 2, a small open model that is only OFFERED (an offer card, then one click) and never downloaded automatically, only on suitable hardware (Apple silicon, enough RAM and disk, never an 8 GB Mac), on Wi-Fi or another unmetered link, on power, respecting Low Data Mode; tier 3, A's excerpts only. A running Ollama or LM Studio MAY be used if present, never required.
2. The download SHALL start only on the user's click on the offer card. It SHALL check a SHA-256 pinned in the app against a revision-pinned source URL, be resumable, capped on disk, paused on battery, a metered link or low disk, kept in the registered, clearable cache (D-58), and removed when the user removes it in Settings, switches the feature off or clears the cache. No model bytes SHALL be fetched before that click.
2a. The offer card SHALL say what the user gets, the size and that it stays on the Mac, in plain words (for example: "Get better one-line summaries. Download a small model, about 2 GB. It stays on this Mac."), with one Download button and a way to decline that is remembered. It is shown mainly on Macs without Apple Intelligence, and as an optional upgrade on Macs that have it. It SHALL NOT be shown on 8 GB Macs. Settings SHALL show the model's state (offered, downloading, installed) with a Remove button.
3. For tier 1 the user SHALL be told once, in plain words, when the feature turns on and which tier runs ("Your Mac now writes one-line summaries on this device. Nothing leaves it."), and Settings SHALL have one switch to turn it off (and on), showing the current tier and the space used.
4. The engine SHALL only shorten the agent's own text, never add facts: a faithfulness check (every number, file name and verdict in the line appears in the source) rejects a line that fails and shows A's excerpt instead; a missing, slow or crashed engine falls back silently.
5. Summaries SHALL run only when a turn ends or an item enters the view, never in a loop, within the budget AJ-E3710 sets and the idle under 2% CPU; the engine stops under thermal pressure and on battery saver.
6. All of it SHALL sit behind the `SummaryEngine` seam; no tool check outside adapters, every sentence from `feed-words.ts`, no summary text sent off the Mac. Windows and Linux compile with the OS seams stubbed (D-52).
**UI/UX reasoning**: "most important is ease of use… never ask user to download models… as much automation as possible"; the one-time notice and the switch keep it honest.
**Performance budget**: from AJ-E3710; idle stays under 2% CPU with the feature on; a line appears within the ADR's latency; the fetch never competes with an agent's work.
**Quality gates**: verify-static; unit tests for detection on a hardware and network matrix (each tier, each reason to fall back), the download rules (checksum, resume, cap, metered, battery), a test that no model bytes are fetched without the user's click (network stubbed to fail on any request before it), the offer card's wording and the 8 GB rule, removal from Settings, the faithfulness check with invented-fact fixtures and the silent fallback; the idle CPU gate; the Settings and notice tests; the a11y gate for the notice, the offer card and the switch.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3711/manifest.md`, R1–R8; a Sonnet lab walk on a real Apple-silicon Mac (each tier reachable, the notice shown once, the switch off and on, a metered link and battery refused) verified by Opus; before and after captures of the feed; CPU and battery measured over an hour.
**Visual testing**: items with a summary line, with the excerpt fallback and with neither; the notice; the Settings control in each tier and off; both themes.
**Files to touch**: `crates/juggler-app/src/features/feed/summary/` (the seam and engines), `crates/juggler-adapters/` (excerpt input from AJ-E3708), a small Swift bridge for Apple's framework, `crates/juggler-app/src/features/settings/`, `ui/src/lib/feed-words.ts`, `ui/src/components/HomeFeed*.tsx`, Settings UI.
**Research basis**: AJ-E3709's research file; AJ-E3710's ADR; `docs/research/feed-summaries-and-orchestrator-options.md`; D-120 point 2.
**Handover note**: D-120 as amended 2026-11-16: tier 2 is an offer and a one-click download, never automatic ("we can't download 2GB models without user approval"). Downloading a model during development still needs the maintainer's explicit yes in chat, given only after the model-selection study names the model and its pinned source. Build order: A, tier 1 with the extractive check, then tier 2. Build against AJ-E3708's excerpts, which are also the fallback. The design is [ADR-0033](../adr/0033-feed-summary-engines.md); build to it once the coordinating session has reviewed it. D-121 (2026-11-16): this engine now runs through the AI-jobs router of [AJ-E3801](E38-chat.md) and AJ-E3802, and at most one model is ever downloaded (AJ-E3803). B's calm-line rewriting is enabled for calm items only if blind reviewers prefer it over A's excerpt on the realistic corpus with zero FAILs; otherwise A's excerpt stays.
