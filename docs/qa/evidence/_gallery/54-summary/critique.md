<!-- captures: 5128448977f0 -->
# 54-summary: Home's "Since you were away" summary of what crashed, needs you, ran, finished and ended, and a session opened from it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The "Since you were away" card stands off the darker dark canvas and the grey light canvas, and its groups (✕ Crashed, ! Needs you, ▶ Still running, ✓ Finished, ■ Ended) are bold and easy to scan. Mark all seen is a small neutral button, and New session in the bar is the only strong action; in dark it now uses the lighter violet, which reads better against the near-black bar. The rail status line now shows its age whole or not at all, so the ai-juggler row reads "Stopped with an error" where the first pass had "Stopped with an error · 7…". The rows are still noisy. "tokens: Unavailable · cost: Unavailable" repeats on almost every row, and in Needs you it drops to a second line on the first two rows but sits right-aligned on the third. The Crashed row's "· 2 leftover processes" still starts a new line with a stray leading dot. At 1440 wide the card fills only about half the main column, so a wide empty band sits to its right. The opened captures show the session page with Where it runs, Runway and Continue on another account cards, all clearly separated; there too only the lighter dark accent and the whole rail status line changed, and after the second pass the terminal's black reaches the card's edge in both themes, with the white (light) or lighter (dark) frame gone.

## Weak

- home-1440x900-dark/light: inside the Needs you group, "tokens: Unavailable · cost: Unavailable" moves between a second line and the right edge from row to row, so the rows don't line up.
- home (all four): the Crashed row wraps "· 2 leftover processes" onto its own line, starting with a dot.
- home-1440x900-dark/light: the summary card spans only about half the main column, leaving a wide empty area to its right.
- home (all four): the same "Unavailable" token and cost text on nearly every row is noise; it could say it once.

## 0.4.1 baseline pass (2026-11-10)

home-1440x900-dark rendered in the pinned container from HEAD and looked at by eye. The fixture's rows now carry every field the backend sends. `at_ms` is when a row entered its group; before, it was missing, and the rows read "waiting NaN d" and "ended Invalid Date". The product now leaves the time out of a row that has none. What changed, on purpose:
- **Real times**: Needs you reads "waiting 2 h", "waiting 1 h" and "waiting 25 min", the longest first as the backend sorts it. The ended rows say when they ended ("ended Sun 02:33 AM", "ended Sun 12:58 AM", "ended Sat 11:23 PM"), and Still running says "last change Sat 10:13 PM".
- The heading reads "Since you last looked, Sat 09:13 PM" (D-97 5). Unknown tokens are silent: only Still running shows "19,134 tokens". The Crashed row's "2 leftover processes" sits on its line.

Weak: the summary card still spans only about half of the 1440 column. The times mix a weekday clock ("Sun 02:33 AM") with durations ("waiting 2 h"), which is correct but takes a second look.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: home-1440x900 dark and opened-1024x680 light, beside the 0.4.1 baseline pass. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **home:** the old "Since you were away" card is now the feed's own divider and digest (the mock's): "WHILE YOU WERE AWAY · SINCE YESTERDAY 5:13 PM", "1 stopped with an error, 2 finished." and three lines with their times, then Mark all seen; the stream goes on under YESTERDAY. The 0.4.1 weak points are gone: the summary no longer spans half a column (it's the feed's width) and the mixed clock and duration words read as the mock's.
- **opened:** a session opened from it: "Stopped with an error yesterday", Relaunch · Resume (filled) · Run in AI Juggler with "Fresh conversation" and "Same conversation" under the first two; its conversation from the transcript, "not saved".
- **Weak:** "Launched with opus - now running sonnet-5" shows the models' ids in lower case (the transcript's own words).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What happened while away, in one place, each line opening its session. |
| 2 | Prominence | 4 | The digest sits where the stream meets the last look, as the mock. |
| 3 | Default | 4 | Mark all seen the one quiet action. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches the home-feed mock's "While you were away". |
| 6 | Areas | 4 | One digest card in the feed column. |
| 7 | Colour | 4 | Status words in the stream are quiet; amber and red only in Needs you. |
| 8 | Type | 4 | Digest headline 13 px, lines 12 px with right-aligned times. |
| 9 | Actions | 5 | Resume filled on the session; Mark all seen quiet on Home. |
| 10 | Serious | 4 | Relaunch and Resume each say what they keep (fresh or same conversation). |
| 11 | Doors | 4 | Each digest line opens its session. |
| 12 | Words | 4 | The model ids are the transcript's words (weak). |
| 13 | States | 4 | Said once: a digest line never doubles as a stream card (R5). |
| 14 | Consistency | 4 | The feed's own patterns. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | Catching up is one read. |

Lowest: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80); the lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-26, AJ-E3713, Opus, both themes)

Re-captured on the host; every changed capture opened (contact sheets, four to a sheet) beside its last capture and the approved story-feed mock (`AIJuggler-design-refs/story-feed/`). Captures: home at 1024 and 1440, opened at 1440 (dark, light).

**What changed (AJ-E3713).** Home's feed below the pinned needs is the story feed now: the tabs All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac (the chosen one the accent's soft fill), the state chips Running / Finished / Failed in their soft tints with today's counts, Showing: Every app at that row's end, and posts in place of log rows: a tone card each with the App's own icon (or AI Juggler's mark), the calm line "**Claude Code CLI** · Work Max · 10 min", the story with its thing as an accent-soft chip, a quiet gesture bar (↑ ↓ + … Mark reviewed · ↗ Open). Nothing else on these screens moved. The away digest keeps its divider and lines; the stream under it is posts. opened-1440 is the session it opens (only the rail's own row state differs from its last capture).

Scores unchanged but for: line 5 (Mock-up) 4: the feed follows the approved story mock (68-home-feed's 0.5.2 section lists the deviations); line 19 (Rich-mode colour) **4**: the feed's filters are choices, the chosen tab a soft accent fill, the state chips their own hues with a ✓ when on, the Showing picker's options with their icons; nothing chosen by a ring.
