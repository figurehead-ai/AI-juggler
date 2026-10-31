<!-- captures: 3b8748db5e31 -->
# 26-quota: a session's Usage tab: token counts, the account's 5-hour and weekly windows, and the live How it's going cards, in blocked, measured, no-data and stale states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The colour steps read well: in dark the No token counts banner, Model card, account-window card and the How it's going cards all lift off the canvas, and in light the white cards sit cleanly on the grey page. The account window is the strongest element: a heading with the account name, each window's percentage in bold on the right, a thin bar, and a coloured reset line (red "Used up until 5:13 AM" in usage-blocked, amber "resets at 5:13 AM" in usage-measured), and usage-no-data swaps the bars for dashed lines with "Not measured yet" and "Measuring — shows after a little use". In dark the weekly bar and the Tokens / min sparkline now carry the lighter violet accent and read more clearly against the card. usage-stale now differs from usage-measured: both reset lines end "· as of 22 min ago", though that is the only stale cue, in the same small text and colour as the reset time. The rail status line now reads "Stopped with an error" whole instead of a cut age. Still off: the Model card is short beside the tall window card, leaving a large empty block under it; at 1280x800 the How it's going row is cut by the window edge; and "1 errors / 12 turns" is still ungrammatical.

## Weak

- usage-stale-1280x800-dark.png, usage-stale-1440x900-light.png (all four): the only stale cue is "· as of 22 min ago" appended to the reset lines in the same small text; no icon or colour marks the reading as old.
- usage-blocked-1440x900-dark.png (all captures): the Model card is much shorter than the account-window card beside it, leaving an empty block.
- usage-measured-1280x800-light.png (all captures): "1 errors / 12 turns" in the Errors card.
- usage-*-1280x800-*.png: the How it's going cards are cut by the window bottom, the Tokens / min sparkline sliced mid-bar.
