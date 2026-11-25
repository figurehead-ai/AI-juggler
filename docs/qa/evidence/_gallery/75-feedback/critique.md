<!-- captures: 4cd85aca4382 -->
# 75-feedback: loading states, the quit check, and undo toasts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

loading-accounts and loading-home now read as loading in both themes. In light, after the design fixes, the skeleton bars use a darker grey: the page-title bar on the grey canvas, the four rail skeleton rows and the bars and button blocks inside the white cards all show plainly, where the first pass looked nearly blank; the second pass made them one step darker again, so the page-title bar, the rail rows and the button blocks now read as solid shapes at a glance in both loading-accounts and loading-home. In dark the skeleton cards stand out from the canvas as before. quit-sheet is clear: one short sentence and three buttons, with Quit anyway the solid violet primary, its focus ring now in the lighter dark accent. reviewed and stop-held/stop-undone put the toast bottom right with its Undo (5s) countdown, and the session page behind them has cards that lift off the canvas; the toast still covers the lower part of the right column ("Continue on another account" in stop-held). In stop-held and stop-undone the terminal's black now runs to its card's edges, with black padding before the text. In the rail of reviewed, stop-held and stop-undone, the feat/read-model row now reads "Stopped with an error" whole, without the cut-off age.

## Weak

- stop-held-1280x800: the toast sits over the "Continue on another account" card.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: stop-held-1280x800 light, quit-sheet-1280x800 dark and loading-home-1280x800 dark, beside the 0.3.2 ones.

- **stop-held:** the toast "Stopping Fix the flaky login test. ai-juggler · Claude Code CLI · Work Max. Undo keeps it running." with Undo (5s) and its draining line, Stop greyed as "Stopping…" in the head; it still sits over the Apps that can take it over card at 1280 (the 0.3.2 weak point; toasts live there).
- **quit-sheet:** "Quit AI Juggler? 3 sessions are running. 2 end when AI Juggler quits; 1 keeps running." Cancel focused, Stop them first, Quit anyway filled.
- **loading-home:** the rail's and Home's placeholders. **Weak:** Home's skeleton is one column of three outlined bars (RowPlaceholders' home shape), where Home lands in two columns of borderless cards: the first frame doesn't hold Home's shape (AJ-E1552, D-103.6) and the outlines go against PAT-SURF-1. Logged **F64**; its fix re-renders the regression's loading-home-* baselines.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Every action answers at once: a toast with Undo, a placeholder while loading, a check before quitting work. |
| 2 | Prominence | 4 | The toast at the corner; the sheet centred. |
| 3 | Default | 4 | Cancel focused on the quit sheet (the safe answer). |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | No mock; the toast and sheet kit. |
| 6 | Areas | 4 | F64 fixed: Home's skeleton is Home's two columns, by tone. |
| 7 | Colour | 4 | Accent only on Undo's line and the sheet's primary. |
| 8 | Type | 4 | Toast title 13 px semibold, detail 12 px. |
| 9 | Actions | 4 | Quit anyway filled and last; Cancel the focused default. |
| 10 | Serious | 4 | The sheet says what ends and what keeps running; Stop has Undo, not a confirm. |
| 11 | Doors | 4 | The toast names the session; the sheet's Stop them first does it. |
| 12 | Words | 4 | Plain and counted. |
| 13 | States | 4 | F64 fixed: the heading, Needs you now's card, the feed, Your apps and Runway stand where they land. |
| 14 | Consistency | 4 | Toasts and sheets from the kit. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | Nothing happens unseen or without a way back. |

Lowest as first scored: 3 (lines 6 and 13, F64 logged).

**Re-captured after F64 (2026-11-24, Opus):** the family re-rendered; opened loading-home-1280x800 in both themes beside 67-home's needs-you-1280x800. The skeleton is Home's shape: a heading bar where "5 need you" lands, then the two columns at Home's widths (the left column's edge at 912, the right from 940 to 1256): a section head over a need card (mark, two lines, two buttons), the feed's chip track and four rows; on the right Your apps' head over its card of three rows and Runway's over its two meters. Cards by tone, no outlines (it was one column of three outlined bars).

Lowest: 4 (lines 6 and 13 were 3: F64 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
