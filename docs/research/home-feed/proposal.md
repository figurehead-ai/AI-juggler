# Proposal: Home as a two-column living feed

> **Ruled: D-118 (2026-11-14); tasks E37.** Copied from the design
> session's working folder; its text is kept as written. Where it and
> D-118 differ, D-118 wins:
> - Placement: all of F1, F2 and F3 are in 0.5 (§12's first decision).
> - Folding the morning summary and "Pick up where you left off" into the
>   feed is decided (§12's third decision).
> - The task-close note (G5) is built once, in F3 (AJ-E3706), with the
>   git items; F2 is G2, G3, G4, G6 and G7 (AJ-E3704, AJ-E3705).
> - (b), summaries written by a model, is not in 0.5.

**Inputs:**
- `research.md`: feeds, rollups, voice, and the generation options.
- `events.md`: every feed item type, its source, level and rollup.
- `../calm-attention/proposal.md` and its §c2: the calm attention model and the round-2 look.
- The mock: `mock-home-feed.html`, rendered as `docs/qa/evidence/AJ-E3701/mock-home-feed-dark.png` and `…-light.png`.

**Who this layout is for:** users with active Accounts and Apps. New users keep today's Welcome and first run (`screens/FirstRun*`, `Welcome.tsx`) unchanged.

**Rulings this honours:**
- the calm budget (each need loud once per area);
- Desktop-first (what's open, where, one click);
- Instant and paged;
- Lazy-load;
- Progressive disclosure;
- UI consistency (the row, mark and section-label patterns of round 2);
- Pickers: suggested, then all;
- Every object has a door;
- Local cache;
- Enhance, never replace.

## 1. Layout

```
┌ rail ┐┌──────────────────────── Home ────────────────────────────────────┐
│      ││ 2 sessions need you            8 open · 6 finished today         │  ← fixed head
│      │├──────────── ACTIVITY (feed) ─────────┬── right column ───────────┤
│      ││ NEEDS YOU NOW  ⚠ 2              ▾    │ YOUR APPS            ▾    │
│      ││  [pinned L2/L3 cards]                │  app rows (icon toned)    │
│      ││ All · Needs you · Finished · …  ▾▾   │ RUNWAY BY ACCOUNT    ▾    │
│      ││ TODAY                                │  account rows + meters    │
│      ││  feed items, newest first            │                           │
│      ││ ── While you were away · since 9:12 ─│                           │
│      ││  digest card, then older items       │                           │
│      ││  …pages (auto ×3, then Load more)    │  (scrolls on its own)     │
└──────┘└──────────────────────────────────────┴───────────────────────────┘
```

- **Column widths:**
  - The feed is `minmax(480px, 1fr)` with a **max reading width of 720 px**. Extra width becomes margin, not longer lines.
  - The right column is `clamp(320px, 30%, 400px)`, with a 24 px gap.
  - The head (title + counts) is fixed above both columns. Each column is its own scroll container (`overflow-y: auto`, `overscroll-behavior: contain`) with a `.scroll-edge` hairline that shows only once scrolled, a pattern the app already has.
- **What each column holds:**
  - **Left:** the pinned "Needs you now" zone, then the filters, then the stream.
  - **Right:** Your apps, then Runway by account. Both are collapsible, and the collapse state is remembered per viewer (localStorage, wrapped in try/catch).
- **Narrow widths** (main area under about 1,000 px, e.g. a 1024 px window with the rail open):
  - **Stack, with the feed first.** A feed that keeps loading would bury Apps and Runway, so when stacked:
    1. the feed shows **one page** (the pinned zone plus about 12 items) and then a **Show more** button, with no automatic paging;
    2. the head gains an "Apps & runway ↓" jump link;
    3. Apps and Runway follow below the feed.
  - Under 640 px (rare on a Mac), Your apps becomes a horizontal strip of icons.

## 2. Feed card anatomy (calm, round-2 look)

```
 ●  [icon]  Claude Code · Work Max                         12 min
            finished  Fix the flaky login test  in 42 min
            ┌ "Fixed the race in the login test by awaiting the session
            └  cookie; all 212 tests pass."                   Show more
            web-app · feat/login · 3 files changed
            Open · Mark reviewed · ↗                    (on hover / focus)
```

| Part | Treatment |
|---|---|
| Unread dot | A 6 px `ink-2` dot in the left gutter: neutral, never accent. It clears when the item is read (opened or acted on). |
| Avatar slot (20 px) | The App icon in the **app tone**: muted at rest, full colour at L2/L3, selected, hovered or focused (§c2). A pool agent gets a lettered disc, the narrator (AI Juggler) its own mark (muted), and an Account its round avatar (D-69). |
| Author line | Author in `ink-2` medium, then place in muted, then the time right-aligned in muted (absolute time in the tooltip and for screen readers). |
| Sentence | `ink` at 13.5 px. The object's name, which is the value, is semibold and is the link. One or two sentences. |
| Quote (a+) | The agent's own final answer, clamped to 2 lines, on a `--color-fill` block with no border, in `ink-2`. "Show more" expands it in place. Shown only when Show last messages is on. |
| Facts line | muted: project · branch · files changed · tokens (when measured). |
| Actions | Ghost buttons, shown on hover, focus or the selected item. Pinned items show them always. One filled accent button sits in the whole feed: the first pinned item's primary action. |
| L2 / L3 | Only in the pinned zone. The amber or red word, glyph and dot, and the L3 tint, exactly as in round 1. In the stream, an item about a need is a quiet history line ("You answered…", "It finished after the restart"). |
| Separation | 14 px spacing and no borders. Day separators are section labels ("TODAY"). The **only rule line** in the feed is the "While you were away" divider, so it reads as a boundary. |

**Voice:**
- Subject first, then what changed, then the result, with a link to the thing.
- No emoji, no exclamation marks, no first person.
- The author is whoever acted (`research.md` §4).
- All sentences come from one `feed-words.ts`, the same pattern as `notice-words.ts`, so wording is tested and consistent.

## 3. Rollups and digests

1. **One session, one card.** Started, working and finished coalesce into the latest state, which updates in place ("finished in 42 min, 6 turns, 3 helpers"). Working and idle are never items of their own.
2. **Same verb, same App and project, within 2 h, roll up.** "Codex · Personal finished 4 sessions in trip-notes" shows 3 names and a "Show 4" collapsible. The window restarts if a different kind of event for that App interrupts it, so a crash is never hidden inside a rollup.
3. **Pool threads coalesce per 30 min.** Only Handoff, ReviewRequest, Question and Status messages are shown, and `Note` chatter only counts ("+3 more messages in *Export flow*").
4. **Tasks roll up per pool per hour.** "The Release pool finished 3 tasks", expandable.
5. **Rollup keys never cross a level boundary.** An L3 item is never folded into an L0 rollup.
6. **The away digest (D1).** If the last seen time is 2 h or more ago, a "While you were away · since 9:12 PM" divider and digest card sit at that boundary.
   - It shows counts by kind and the 3 most important items.
   - "Show all" expands it in place, and "Mark all seen" sets `summary_seen_at_ms`.
   - It **replaces MorningSummary's card** with the same data and the same key, moved into the feed.
7. **A daily usage digest (W10)** sits at each day boundary.

## 4. "Since you were away", seen and read

- **Seen:** Home visible means that page's items are *seen*. The away boundary moves on "Mark all seen", or after Home has been in front for 10 s with the digest on screen.
- **Read:** opening or acting on an item marks it *read* (the unread dot clears). Needs clear when they resolve, as today.
- **No surprise moves.** New items while you're scrolled down appear behind a **"3 new updates"** pill at the top of the feed. Clicking it scrolls up. At the top, a new item fades in once over 600 ms. Nothing loops, and reduced motion turns the fade off.

## 5. Filters

- **Type chips:** All · Needs you · Finished · Switches & limits · Pools & tasks · This Mac. They are a segmented control with a filled background and no outline (round 2).
- **One facet picker:** Project, App, Pool or Account, using the existing `Picker`: suggested (most used, related) first, then All, with search past about 8 entries.
- **Optional toggles in ⋯:** "Include my actions" (off by default) and "Show quotes" (follows the Show last messages setting).
- **Memory:** filters are remembered per viewer. When filtered, the pinned zone still shows every open need, because urgent things are never filtered away, and a muted "Filtered: web-app ×" line says so.

## 6. Paging and loading

- **Pages:** keyset `feed_page(before: (at_ms,id), filters, limit: 30)`.
  - Pages load automatically as the end comes into view, for 3 pages (90 items), then a **Load more** button appears (NN/g).
  - The query unions the existing journals, newest first, with the rollup keys applied server-side (`events.md` §7).
- **First paint:** the cached last first page (local cache ruling) renders at once. Without a cache, 6 skeleton cards take the shape of a card. The live page replaces the cache without moving the scroll position.
- **Timestamps:** relative under 1 h ("12 min"), then the clock time today, "Yesterday 4:10 PM", then the date. The exact time is in the tooltip and accessible name. A timer re-renders relative times once a minute, never per second.

## 7. Empty, loading and first-run states

- **No Accounts:** today's Welcome and first run (unchanged).
- **Accounts but nothing has happened yet:** one empty state in the feed:

  > "Quiet so far. When your apps start, finish or ask you something, it shows here."

  It offers [New session] and [Open an app]. The right column shows the Apps and Runway as usual.
- **All clear (no needs):** the pinned zone collapses to one muted line, "Nothing needs you", and the stream follows.
- **Error:** a Banner says "Showing what was loaded at 14:32 · Retry". The cards stay on screen.

## 8. Relation to the rail's Needs you, so nothing is loud twice per area

| Fact | Rail (area 1) | Home pinned zone (area 2) | Home stream |
|---|---|---|---|
| An open L2/L3 need | **Loud** (Needs you row) | **Loud** (pinned card with the primary action) | Not shown while open |
| The same need, resolved | gone | gone | A quiet history line ("You answered…") |
| The count | Needs you **n** | NEEDS YOU NOW **n** (same number) | – |

- **One queue:** the count stays the same in the rail, the bell, the Home head and the pinned zone, and counts L2+L3 only (`../calm-attention/proposal.md`).
- **The Home head title** ("2 sessions need you") stays ink, not amber, because the pinned zone's count already carries the colour.
- **"Pick up where you left off" folds into the feed.** Finished, unreviewed sessions are S5 cards with Mark reviewed, the Finished chip lists them, and "Mark all reviewed" moves to that filter's ⋯.

## 9. Right column: apps, then runway

- **YOUR APPS:** compact rows in the round-2 row style, not tiles.
  - Each row shows: App icon (toned), App name, profile dot and name, "n open", the needs mark when it has needs, and a ghost Open or Start.
  - Order: running first, then by recent use ("Open now means running"). Closed Apps follow under a muted "Not running" label. A long list collapses after 8, with "Show all".
  - The App is the subject here, so the icon is full size (20 px), muted unless the App has a need, is selected, or is hovered.
- **RUNWAY BY ACCOUNT:** the round-2 runway rows (neutral meter; amber at 10% or less with a live session; red when out with live work). A muted "Not measured yet · Measure its limits ›" line stays.
- Both sections are collapsible, and both scroll with the right column.

## 10. Accessibility

- **Landmarks:** the feed column is `<section aria-labelledby="activity-h">` and the right column is `<aside aria-label="Apps and runway">`. The pinned zone is a `region` labelled "Needs you now".
- **Feed pattern** (WAI-ARIA APG):
  - The stream is `role="feed"`, `aria-busy="true"` while a page loads, and `aria-setsize="-1"`.
  - Each card is `article` with `aria-posinset`, `aria-labelledby` (author + sentence) and `aria-describedby` (quote + facts).
  - Rollups are `article`s with a disclosure button (`aria-expanded`).
- **Keyboard:**
  - Page Down/Page Up and J/K move between articles (J/K already move through Home's rows).
  - Enter opens the item and A runs its action. R opens it at the question, H continues it in another app, and S snoozes. These are the existing Home keys.
  - Ctrl+End and Ctrl+Home leave the feed. Tab moves through the focused card's controls.
  - Focus never moves when new items arrive.
- **Live regions:**
  - The existing polite `AttentionAnnouncer` covers needs.
  - The feed adds **one** polite announcement for the new-items pill, batched to at most 1 per 30 s ("3 new updates").
  - No per-item announcements.
- **Contrast:** every text token is from round 2 (all at 4.5:1 or above, `../calm-attention/proposal.md` §c2). Quotes in `ink-2` on `--color-fill` measure 11.8:1 (Lc 81) in dark and 12.7:1 (Lc 99) in light.
- **Motion:** one fade, reduced motion respected. App tone changes are 150 ms transitions, also cut under reduced motion.

## 11. Generating the sentences

- **Base: (a) deterministic templates** over the facts in `events.md`. They are free, local and testable.
- **Plus (a+): the agent's own final answer**, quoted, behind the existing Show last messages setting. There is no model call.
- **Later, if the walks show (a+) often missing:** (b) opt-in summaries written by the user's own installed CLI on the Account they choose.
  - Off by default, with its own Privacy line.
  - A daily cap and a visible count in Usage.
  - It needs a **terms reading per provider** and a **D-number** before it ships (`research.md` §5).
  - It never extracts or reuses tokens.

## 12. Phased plan

These phases fit "One release, not many". Placement is the maintainer's call through the release scope gate.

| Phase | Contents | Size |
|---|---|---|
| **F1: the feed over existing journals** | `features/feed/` read model (keyset union of `notices`, `instance_lifecycle ended:`, `switch_edges`, `task_events` done/review, `agent_messages` highlight kinds); rollups; `feed-words.ts`; (a+) quotes; the two-column Home, pinned zone, filters, paging, away digest replacing MorningSummary, Pick up folded in; a11y; cache-first paint | L: about 4–5 days of Opus code + Sonnet walks |
| **F2: small persistence gaps** | G2 switch `confirmed_at`, G3 runway phase rows, G4 conversation state events (30-day retention), G6 daily digest, G7 dirty and files at session end | M: about 2 days |
| **F3: Git and pools** | G1 commits (`git log`, read-only) and PRs (`gh`, if present and opted in); G5 task close note via the pool MCP tool | M: about 2 days |
| **F4 (only if justified): (b) summaries** | After the terms reading and a D-number | M |

**Validation:**
- Persona walks on a busy-day fixture: the multi-subscription vibe-coder after a night away; a newcomer with 1 App; and a heavy day with 14 open sessions, 7 needs and a crash.
- A 5-second and a 10-second glance: "What needs you? What finished while you were away?"
- The attention-budget Playwright test extended to the pinned zone (one loud mark per need on Home).
- axe and contrast in both themes.
- Feed keyboard tests: Page Down, J/K, and no focus jump on arrival.
- Performance: a page under 50 ms for 30 items on the reference DB, and cache-first paint.
- Opus verifies the Sonnet verdicts.

**Decisions for the maintainer:**
1. The release that carries F1 (0.5 or 0.6).
2. The (b) terms reading and its D-number, when or if we get there.
3. Folding MorningSummary and "Pick up where you left off" into the feed. I recommend this; the data and keys stay.
