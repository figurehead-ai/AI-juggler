<!-- captures: d44d9fd0eb37 -->
# 68-home-feed: Home's two columns and its activity feed (D-118): pinned needs, the narrated stream with rollups and the away digest, Your apps and Runway by account, at 1024, 1280, 1440 and stacked, each section folded, empty and flooded

Design critique (D-57, AJ-E2814), against the approved home-feed design (2026-11-14) and its mock (`AIJuggler-design-refs/calm-ui/home-feed/`).

## 0.5.2 AJ-E3831 (2026-11-27, D-154, Opus, both themes): the feed's AI summary, an opt-in Beta

`ai-summary-1280-{dark,light}.png` and `ai-summary-words-1280-{dark,light}.png` (`home-feed-gallery.spec.ts` "the AI summary of a turn (Beta)", axe clean), opened by eye.
- **What changed:** with "AI summaries in the feed" on, a finished turn's post carries one line under its sentence: the AI tag, then what the turn came to against what was asked ("Partly done: … · open: calendar sync · 6 tests pass"). A quiet foot follows: "Summary by Apple’s built-in AI", the Beta pill, "Show its words" and "This is wrong". The words open in place as a quote under "The agent's own words". A post whose every draft was refused shows "Its last reply" quoted instead, with no AI tag.
- **Judged:** the line sits at body size in ink, so it reads as the post's second sentence, not a new card. The foot is at label size in faint, the same as the turn review's AI foot (one pattern for AI lines). The accent appears only in the AI tag and the Beta pill (PAT-COLOR); there are no new borders or shadows (PAT-SURF). The quote rule is the turn review's.
- **Watch:**
  - a long line wraps to two lines at 1280 (as here), still under the post's width;
  - the foot's right-hand links wrap under the model name at narrow widths;
  - with the Beta off, nothing renders, not even an empty space (tested).

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

First critique of this family. Captured fresh (`pnpm visual:capture` of `home-feed-gallery.spec.ts`, local, after F53 and this batch's fixes; the earlier untracked shoot predated F53). Opened at full size: mock-1280 dark, typical-1280 light, typical-1440 dark, busy-1280 light, busy-stacked-1120-rail-open dark, flood-1024 light, away-1280 dark, empty-1280 light and folded-home-needs-now-1280 light (the earlier shoot's, same layout), side by side with `calm-ui/home-feed/mock-home-feed-dark.png` and the approved design (2026-11-14).

- **mock-1280:** the mock's own data. "2 need you · 7 open · 11 finished today"; Migrate the billing webhooks (Claude app · Work Max) with Open its page ↵ (filled), Switch account… and ↗; "builder-1 Release pool · Export flow — Asks you: “Use the v2 export API, or keep v1 for this release?”" with Reply · Open the thread; TODAY: finished Fix the flaky login test with the agent's quoted words, Work Max at 12 % of its 5-hour window, AI Juggler's switch "copied Answer the release question from Claude Code CLI · Work Max to Codex CLI · Personal — carried: full · by copy". Your apps 6 (ChatGPT, Codex CLI, two not running with Start), Runway with Work Max low (amber bar, "Low · 5-hour window resets at 3:13 PM"), Personal 64 % (green), Team Plus "Not measured yet · How it's measured ›".
- **typical, busy, flood:** three needs pinned and "+2 more need you" (flood: "+14 more"); rollups ("finished 3 sessions" with each one under it); a crash, a stop with its exit code ("stopped with an error (exit code 1) after 4 min"), "crashed (signal 9)".
- **away:** the "WHILE YOU WERE AWAY · SINCE YESTERDAY 5:13 PM" divider and digest with Mark all seen, after the needs (the left column scrolled to it).
- **stacked (1120, rail open):** one column, the feed first, "Apps & runway ↓" in the head, as the mock's narrow frame.
- **empty:** "All clear — nothing needs you", the feed's empty state with New session filled (nothing pinned) and Open an app, the permission card.
- **folded:** Needs you now folded to its head and count; the feed moves up.
- **Deviations from the mock, reasoned:** "Showing: Everything ▾" for its Project ▾ App ▾ and a "Needs you" chip (E37 R7, accepted in 67-home); at 1024 and 1280 Showing: drops under the chips (the column is narrower than the mock's); an item's Open · Mark reviewed · ↗ and its facts line show on hover, focus or selection (R3), so a still capture shows the quiet line; App names are "Claude Code CLI"/"Codex CLI" (D-100); the top bar's room words fold to its dot at 1280 ("↺ 1" when an account runs low).
- **Weak:** Your apps puts "✕ 2 need you" on its own middle line when the App's name and the note don't both fit the 316 px column, so its rows are three lines and the notes sit at two heights (the mock keeps them on the name line). The empty feed shows its filters (**F56**).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | The approved design end to end: pinned needs, a narrated stream with rollups and the away digest, apps and runway. |
| 2 | Prominence | 4 | Needs first, then newest first; apps and runway beside, or a link away when stacked. |
| 3 | Default | 4 | All by default; the first need's primary is Enter. |
| 4 | Mode | 4 | Calm dashboard (D-117): borderless items, tone on hover, one accent. |
| 5 | Mock-up | 4 | Matches the mock; deviations listed above. |
| 6 | Areas | 4 | Two columns at 1024–1440, stacked under 860 px of main area; Your apps' wrapped notes (weak). |
| 7 | Colour | 4 | Amber only on needs and a low runway; red for broken and out; the stream's words quiet. |
| 8 | Type | 4 | Author 13 px ink-2, sentence 13 px with the subject bold, times 12 px muted and right-aligned. |
| 9 | Actions | 5 | One filled button in each state (Open its page, or New session when nothing waits). |
| 10 | Serious | n/a | Nothing destructive; Mark all has Undo. |
| 11 | Doors | 4 | Every card opens its thing; ↗ to its App; rollups open in place. |
| 12 | Words | 4 | F56 fixed: the empty state has no filters to filter nothing; elsewhere the sentences are templates over stored data, the agent's own words quoted. |
| 13 | States | 4 | Paged (three pages, then Load more), the away digest once, empty said once; flood real-sized (40 sessions). |
| 14 | Consistency | 4 | The feed's cards and chips; Home's other cards. |
| 15 | Accessibility | 4 | role=feed of articles; axe clean on every capture (the spec). Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 4 | The permission card primes Accessibility and Automation; the feed causes no prompt. |
| 18 | Usability | 4 | "Like an agent tweeting about our system" (the maintainer): what happened across apps, work, projects and pools, chronological and never ranked. |

Lowest as first scored: 3 (line 12, F56 logged).

**Re-captured after F56 (2026-11-24, Opus):** the family re-rendered; opened empty-1280 (both themes) and a populated one. The empty feed is its card alone: "Quiet so far…", New session and Open an app, then a muted line "Your own actions aren't shown. Include my actions" (the switch the hidden ⋯ held, the one thing that can change an empty feed). The chips, Showing: and ⋯ are gone above it; with anything in the feed, or a filter on, they are as before. The chosen segment is F53's solid fill.

Lowest: 4 (line 12 was 3: F56 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-26, AJ-E3713, Opus side-by-side, both themes)

Re-captured on the host (`home-feed-gallery.spec.ts`, every state), plus a new state, **story-1280** (dark, light): the approved story-feed mock's own data at the mock's 1280×1560, Less like this pressed on the switch post, the pool post followed, the first post's More like this hovered. Opened at full size beside `AIJuggler-design-refs/story-feed/mock-story-feed-{dark,light}.png`: story-1280 both themes; every other changed capture opened in contact sheets (four to a sheet), busy-1280 and flood-1280 dark also full size.

**What changed.** The feed's log rows are posts now: a surface card each (tone on the canvas, no border, no shadow), the author's picture at 40 px (the App's own icon read at run time, or AI Juggler's mark on its tile), a calm line "**Claude app** · work-max · 9 min", the story in 14 px with the things it names as accent-soft chips ("You answered [Login refactor] — it's moving again."), an attachment where the facts carry one (a switch's before → after with each App's icon, the agent's reply quoted with "Codex CLI's last reply", an Account's room as a meter "61% used · 5-hour window · resets 3:00 PM"), follow-ups threaded under the post ("↳ 2 min later · [Login refactor] finished in 18 min."), and the gesture bar (↑ ↓ + 💬 … ↗ Open; the tip "More like this" under the icon). The filters are the mock's: the tabs All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac (the chosen one the accent's soft fill), then the state chips Running / Finished / Failed in their soft tints with today's counts and Showing: Every app at the row's end; a ⋯ keeps Include my actions and Mark all read. Days read Today, This morning, Yesterday. Under the last post: the j k / l / m keys. Rollups say who did it ("Claude Code CLI finished 3 sessions") and list their things in place.

**Side by side with the mock.** Matches: the post anatomy, the calm line, chips, the switch's before → after, the quote block, the thread's rule and "↳ 2 min later", the gesture bar's icons and right-aligned Open, the why panel ("You'll see fewer switch posts. You chose Less like this for switch posts in web-app. Undo · Manage in What Bot remembers"), "+ Following" in the accent, the tabs and state chips, This morning. **Deviations, reasoned:** (1) Remind me is not drawn: the watches offer no reminder condition yet (D-127 hides it; the manifest's R3). (2) The first post's diff stat and three-session summary are left out: the feed keeps no git line yet (AJ-E3708). (3) No card shadow (PAT-SURF-2; tone separates). (4) The switch says "carries on in … AI Juggler copied it over" (F9: a switch copies, never "moved"), and App names are "Codex CLI", "Claude Code CLI" (D-100). (5) The usage attachment is one measured reading as a meter, not the mock's six bars (only one reading is kept), and no run-out estimate. (6) The pinned need and Home's head are Home's existing ones (AJ-E3701), not the mock's simplified card. (7) "Manage in What Bot remembers" names where memories really are (the mock's "Settings › AI › Memory" doesn't exist). (8) "Mark reviewed" stays on a finished session's post (the A key's action).

**Weak.** "Finished 0" beside posts from yesterday reads odd at a glance (the counts are today's; the tooltip says so). The story repeats the App's word after the calm line ("Claude Code CLI · Work Max / Claude Code CLI finished …"), as the mock does.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | What happened, told as stories, at a glance; what needs you stays pinned above. |
| 2 | Prominence | 4 | Needs first; then posts newest first, a "more" lifted within its day, a quiet "less" folded to one line. |
| 3 | Default | 4 | All, every state, every app. |
| 4 | Mode | 4 | Calm: tone cards, faint gestures, one accent for chips and the chosen tab. |
| 5 | Mock-up | 4 | The approved story mock, deviations above. |
| 6 | Areas | 4 | Two columns 1024–1440, stacked under 860 px; posts fill the column. |
| 7 | Colour | 4 | Accent for chips, the chosen tab and a lit gesture; state chips their soft tints; amber only on a meter under half and the needs. |
| 8 | Type | 4 | Author 13 px semibold ink, its line muted; story 14 px; thread and attachments 13 px. |
| 9 | Actions | 4 | One filled button (the pinned need's); every post Open, its gestures quiet. |
| 10 | Serious | n/a | Nothing destructive; a lean has Undo. |
| 11 | Doors | 5 | Every chip opens its thing (the pool chip its pool); Open; Following opens its watches. |
| 12 | Words | 4 | Stories from facts only, never "Resolved: …"; weak: "Finished 0" beside yesterday's posts. |
| 13 | States | 4 | Paged as before; empty; a filter with nothing says so with Show all; the away digest. |
| 14 | Consistency | 4 | The rail's icon path for authors; the menus' and tips' look. |
| 15 | Accessibility | 4 | role=feed of articles; gestures named (their tips presentational); tabs and states are pressed buttons; axe clean on every capture. |
| 16 | Hand walk | 3 | Not walked in Dev this round (the captures and the vitest flows only). |
| 17 | System prompts | 4 | Follow primes Notifications first (PAT-OSP); nothing else asks. |
| 18 | Usability | 4 | The six answers match (manifest), but for Remind me. |
| 19 | Rich-mode colour | 4 | The filters are choices: the chosen tab a soft accent fill, the state chips their hues with a ✓ when on, the Showing picker's options lead with their icons; no ring for chosen. |

Lowest: 3 (line 16: no hand walk in Dev yet).

## 0.5.2 G8 (2026-11-26, AJ-E3713 R6a, Opus against the approved story mock)

Re-captured on the host from a clean export of HEAD (`home-feed-gallery.spec.ts`, every state); the ten that changed opened at full size: story-1280 (dark, light) beside `AIJuggler-design-refs/story-feed/mock-story-feed-light.png`, and busy at 1024, 1280, 1440 and stacked 1120, both themes.

**What changed.** A finished turn's post carries the mock's diff stat: the story mock's first post reads "14 files +412 −96" with the bar 81 % green and 19 % red, in the attachment's tone box between the story and its thread; busy's first post "3 files +58 −12". The stat is the turn's own edit and write calls from the session's transcript (Claude Code and Codex, never git); a post whose calls don't count lines shows "2 files" and their names, faint (the gate's Pool lens, Floor menu and rail-splitter baselines show it; busy's files-only post and the rollup's sum are below the fold here, covered by vitest).

**Against the mock.** Matches: the order (files, +added, −removed, then the bar filling the rest), tabular numbers, weights (the counts semibold), the tone box (`surface-2`, no border), the bar's 6 px height on `surface-3`. Colours follow PAT-COLOR: the counts in `success` and `danger` words, the bar in their strong steps; in dark the green and red stay readable and calm against the card. Deviation: the mock's three-session summary sentence isn't drawn (no such fact; the story is the template's "You answered Login refactor — it's moving again"), so the stat sits on the thread's post and counts its follow-up's turn.

**Weak.** In the files-only form the names can run long; they end in "…" and the full list is in the tooltip only.

| # | Line | Score | Note |
|---|---|---|---|
| 5 | Mock-up | 4 | The mock's diff stat, drawn; the summary sentence still not (no fact). |
| 7 | Colour | 4 | Green added, red removed, words AA, bars the strong steps; nothing else coloured. |
| 12 | Words | 4 | "14 files", "1 file"; screen readers hear "412 lines added, 96 lines removed". |
| 13 | States | 4 | Counted, files only, none (a tool whose transcripts aren't read, or no file changed). |
| 18 | Usability | 4 | Answer 2 now matches in full (the manifest). |

The other lines are as this release's AJ-E3713 section scored them (lowest 3: line 16, no hand walk in Dev).

## 0.5.2 AJ-E2027 (2026-11-26, Opus): Remind me shown

Re-captured on the host from a clean export of HEAD (`home-feed-gallery.spec.ts`), every state, both themes; the 28 captures that changed diffed against the last ones (scripts/visual_sweep_diff.mjs: 84–1,911 pixels each, all in one 44 px column of the gesture bars) and opened, story-1280 cropped at full size.

**What changed.** A post about something (its session, pool, project, Account or App) now has Remind me in its gesture bar, an alarm-clock glyph between Less like this and Follow, because the one scheduler answers (D-127: it was built and hidden until reminders were possible). Its menu is unchanged (In 30 minutes, In 1 hour, In 3 hours, Tomorrow at 9 AM), and picking one primes Notifications first (PAT-OSP). Nothing else moved: the bar keeps its spacing and the glyph is the same faint tone as its neighbours until hovered.

**Judged.** Dark and light: the glyph reads at the same weight as ↑ ↓ + and the speech bubble, with no new colour (a gesture bar is calm; the accent comes only on hover and in the menu). Its tooltip and accessible name say "Remind me".

| # | Line | Score | Note |
|---|---|---|---|
| 12 | Words | 4 | "Remind me"; the toast says "Reminder set for 4:00 PM. About api." with Reminders as its way there. |
| 17 | System prompts | 4 | Remind me and Follow both prime Notifications at the click; nothing asks when a reminder comes due. |
| 18 | Usability | 4 | Answer 2 now matches for Remind me too (AJ-E2027's six answers). |
| 19 | Rich-mode colour | n/a (calm) | A monitoring feed; its menu is the shared RowMenu. |

The other lines are as the G8 section scored them (lowest 3: line 16, no hand walk in Dev).

## 0.5.2 G21, G22, G24 (2026-11-26, Opus): the RW1 walk's feed findings

Re-captured on the host (`home-feed-gallery.spec.ts`), every state, both themes, two workers: the 48 captures from before are byte for byte the same (the diff stat's box moved its tab stop and ring onto the attachment's own box, same classes, same pixels; the folded line now sits in an article of its own, same look). Six new captures from one test on the `releaseWalkFeed` fixture, each opened at full size: `g21-digest-beside-posts-1280-*`, `g22-diff-stat-names-*`, `g24-less-folded-1280-*`.

**What changed.**
- G21: the digest item sits above the finished post it tells, and the post stays a post with its diff stat and its gesture bar (before, the digest took it in, its stat and review unreachable). The digest's lines name each session as the rail names it and say its state now: "Add retries to the sync job · ended" (the fixture's session has no ending how, so the rail's own word "Ended"), "Fix the flaky login test · working"; a session the rail no longer lists keeps the digest's own words ("docs · finished"). Before: "api (1) · working" for a finished session.
- G22: focused by keyboard, the diff stat's box takes the focus ring and the app's tip opens under it with the files one a line in mono ("src/sync/retry.ts", "src/sync/client.ts", "tests/sync/retry.test.ts"); the pointer opens the same tip. It was a native `title` the web view shows only after a still hover and never on focus.
- G24: the delete thread leaned less, its why closed: it folds into "1 post shown less · Show" at the section's end; the patch thread of the same App in another project is untouched (before, the lean took both: Codex recorded `/private/tmp/…` folders the projects kept as `/tmp/…`, so neither post had a project).

**Judged.** Dark and light. The digest's names are now the bold, longest words in its lines; three lines still read at a glance, and the state word stays muted after the dot. The tip is the app's one tip (portalled, wrapped, inside the window), clear of the bar below it. The focus ring on the stat's box is the accent ring every focused box has; on light it reads strong next to the calm box, as focus should (PAT-SURF allows it for focus).

**Weak.** The fold line "1 post shown less · Show" is faint and small at the section's end; it is the existing line, unchanged. In g21 the digest says "ended" where the post says "finished": the rail's word for a session whose ending isn't known; a real session that finished on its own says "Finished" in both.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | The digest a summary line, the posts the detail; nothing hidden by the summary. |
| 5 | Mock-up | 4 | The story mock's attachment and gestures; its tip as the mock's hover. |
| 7 | Colour | 4 | Nothing new coloured; the focus ring the accent's. |
| 12 | Words | 4 | Sessions by their names, states in the rail's words; "1 post shown less · Show". |
| 13 | States | 4 | Digest with live and unlisted sessions; tip by pointer and keyboard; folded after a lean. |
| 15 | Accessibility | 4 | The stat a tab stop described by its tip while shown; the fold line in an article of the feed (axe clean); axe clean on both captures with axe. |
| 16 | Hand walk | 3 | The re-walk on the fixed build follows (RW1). |
| 18 | Usability | 4 | Answer 2 now holds: a finished turn's stat and review stay reachable after a digest. |
| 19 | Rich-mode colour | 4 | A calm monitoring feed, scored as asked: the App tiles and the stat's green and red carry the colour, the digest's lines stay ink and muted, nothing monochrome or loud. |

The other lines are as the AJ-E2027 section scored them (lowest 3: line 16).

## 0.5.2 G63 (2026-11-27, Opus, both themes)

Captured from a clean checkout of 7f39c9b81 (`home-feed-gallery.spec.ts`, the g63 case only; the other captures not re-taken: nothing else in the feed moved); both frames opened.

**What changed.** `g63-new-pill-scrolled-1280-*` (new): the feed scrolled so a Codex post's header sits at the column's top, then one new item told (the stand-in's change, as the backend tells one). The "↑ 1 new update" pill now sits on a band of the page's own colour across the column's top, fading out below it; the post slides under the band, so its header line is covered rather than running out on both sides of the pill (RW1's S-R26 capture: "Change the date format in the API responses" struck through around the pill). Its first visible line ("Codex CLI worked on Add a retry helper to the API client in web.") starts clear below the pill.

**Judged.** Light and dark: the band is the canvas colour, so it reads as the page, not a new surface; the pill keeps its own raised chip and shadow. Clicking still scrolls to the top and shows the new post (vitest unchanged).

**Weak.** The band hides about 30 px of whatever card is under it while the pill shows; that is the price of never overlapping text, and it goes with the pill.

| # | Line | Score | Note |
|---|---|---|---|
| 6 | Areas and spacing | 4 | Nothing overlaps; the band takes no room of its own in the column's flow. |
| 7 | Colour | 4 | The band is the canvas, no new hue; the pill unchanged. |
| 14 | Consistency | 4 | The same fade Bot's conversation head uses over its turns. |
| 15 | Accessibility | 4 | axe clean on both; the band takes no pointer, the pill keeps its focus ring. |
| 16 | Hand walk | 3 | Preview only; the re-walk on the fixed build follows. |

The other lines are as the AJ-E2027 section scored them (lowest 3: line 16).

## 0.5.2 G70 (2026-11-27, Opus, both themes)

`g63-new-pill-scrolled-1280-*` re-taken from a clean checkout of c55c152da: dark byte-equal, light changed (it scrolled to another post: the feed's order at the scroll moment), both opened. The band's canvas colour now reaches 12 px above its top edge (clipped by the column), for RW1 S15's sliver of the covered post above the edge on the app. The sliver didn't show in Chromium or WebKit at 1280 in the preview, so these frames look as before: the pill on the band, the post under it, nothing beside the pill.

| # | Line | Score | Note |
|---|---|---|---|
| 6 | Areas and spacing | 4 | No text above or beside the band. |
| 16 | Hand walk | 3 | The app's re-walk confirms the sliver gone. |
