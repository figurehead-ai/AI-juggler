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

### AJ-E3701 Home in two columns: the layout, Your apps and Runway by account (Home's calm pass) | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3602
**Goal**: Home becomes two calm columns that scroll on their own, with your apps and runway always one glance away.
**User story**: As someone with three Apps on two Accounts, I open Home and see what needs me and what happened on the left, and my apps and their runway on the right, without either pushing the other off screen.
**Requirements**:
1. Home for a user with at least one active Account SHALL be two columns under a fixed head (title and counts): the feed column `minmax(480px, 1fr)` with a 720 px reading cap, the right column `clamp(320px, 30%, 400px)`, a 24 px gap; each column its own scroll container (`overflow-y: auto`, `overscroll-behavior: contain`) with the existing scroll-edge hairline once scrolled.
2. The right column SHALL hold Your apps (compact round-2 rows: App icon in the App tone, name, profile dot and name, "n open", the needs mark, a ghost Open or Start; running first by recent use, then a muted "Not running" group; past 8, "Show all") and Runway by account (round-2 runway rows: neutral meter, amber at ≤ 10 % with a live session, red when out with live work, "Not measured yet · Measure its limits ›").
3. Both right-column sections and the feed's pinned zone SHALL be collapsible, the state remembered per viewer (localStorage in a try/catch), chevrons as D-117 point 3.
4. Below about 1,000 px of main area the columns SHALL stack, feed first: one page (the pinned zone and about 12 items) then "Show more", no automatic paging; the head gains an "Apps & runway ↓" link; under 640 px Your apps becomes a strip of icons.
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

### AJ-E3702 The feed: pinned needs, the stream, rollups, the away digest, the new-updates pill, filters, paging and the voice (F1) | Status: TODO | Priority P0 | Effort XL | Depends: AJ-E3701
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
**Handover note**: summaries written by a model (b) are not in 0.5: they need a terms reading per provider and their own ruling (D-118 point 4); don't build a hook for them.

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
