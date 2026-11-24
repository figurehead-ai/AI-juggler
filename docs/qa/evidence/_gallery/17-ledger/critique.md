<!-- captures: 7ee2447ef8f9 -->
# 17-ledger: Home with the notice that process tracking is degraded and cleanup is read-only

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The notice "Process tracking degraded — cleanup is read-only until restart." sits as a full-width strip under the page heading with a small warning mark, above the Needs you and Runway cards. In dark it has an amber-brown edge on the dark canvas and reads clearly; in light it is a pale beige fill with a tan edge on the grey canvas, only a little different from the canvas, so it is quieter than the white cards around it. It names the remedy (a restart) but gives no button to restart or to learn more. The rest of Home shows the 0.3.2 hierarchy: raised cards and "Open its page" as the one solid violet button in the Needs you card, in dark now the lighter accent like the logo mark and the Work Max runway bar. The top bar stays one row, and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- degraded-banner-1440x900-light: the beige notice is close to the grey canvas and recedes.
- degraded-banner-1440x900-*: says "until restart" with no restart action.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: degraded-banner-1440x900 light (before the fix) and dark (the gate's render after it), beside the 0.3.2 one. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **Before the fix:** a hand-made beige box with a tan border, "⚠ Process tracking degraded — cleanup is read-only until restart.": "degraded" and "read-only" were our words, "until restart" didn't say whose, and amber was a fill behind words (PAT-COLOR-2) on a screen that has a shared Banner.
- **Fixed** (e2544e734, 647f2a96c): the shared `Banner` at its warning level: an amber bar and bell beside "AI Juggler can't record what it starts right now" over "So Clean up can't stop anything until AI Juggler restarts." (the Processes banner's words). Re-rendered (gate degraded-banner-1440x900-dark; local, both themes) and checked by eye.
- **Weak:** it names the restart but offers none; a Reopen AI Juggler action would need the reopen check's live-session question (OsAccessRows has it), left for when the banner is next touched. The rail's desktop row shows F55's cut word.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Says why Clean up can't act and what brings it back. |
| 2 | Prominence | 4 | Above the columns, as every window-level notice. |
| 3 | Default | n/a | Nothing to choose. |
| 4 | Mode | 4 | Calm; amber only as the banner's bar and mark. |
| 5 | Mock-up | 4 | The shared Banner (§2, AJ-E2504): every notice above a screen has this shape. |
| 6 | Areas | 4 | Title and detail on their own lines; the columns start under it. |
| 7 | Colour | 4 | After the fix: amber as a bar and a mark, no fill behind words (PAT-COLOR-2). |
| 8 | Type | 4 | Title 13 px semibold, detail 12 px ink-2. |
| 9 | Actions | 5 | Open its page in Needs you is the one filled button; the banner has none. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | No restart door yet (weak, above). |
| 12 | Words | 4 | After the fix: plain words, whose restart said (was 3). |
| 13 | States | 4 | Shown only while the ledger is degraded (role alert). |
| 14 | Consistency | 4 | Now the kit's Banner, as 46-fleet-board's and 53-parked's. |
| 15 | Accessibility | 4 | role=alert, read once. Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | A newcomer learns why Clean up is greyed and what fixes it. |

Lowest: 4 (lines 7 and 12 were 3 before the fix).
