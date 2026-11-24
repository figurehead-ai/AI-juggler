<!-- captures: cb6b6fce58ee -->
# 68-home-feed: Home's two columns and its activity feed (D-118): pinned needs, the narrated stream with rollups and the away digest, Your apps and Runway by account, at 1024, 1280, 1440 and stacked, each section folded, empty and flooded

Design critique (D-57, AJ-E2814), against the approved home-feed design (2026-11-14) and its mock (`AIJuggler-design-refs/calm-ui/home-feed/`).

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
