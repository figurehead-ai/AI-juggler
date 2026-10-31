<!-- captures: 73b4967cb7da -->
# 26-quota: a session's Usage tab: token counts, the account's 5-hour and weekly windows, and the live How it's going cards, in blocked, measured, no-data and stale states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The colour steps read well: in dark the No token counts banner, Model card, account-window card and the How it's going cards all lift off the canvas, and in light the white cards sit cleanly on the grey page. The account window is the strongest element: a heading with the account name, each window's percentage in bold on the right, a thin bar, and a coloured reset line (red "Used up until 5:13 AM" in usage-blocked, amber "resets at 5:13 AM" in usage-measured), and usage-no-data swaps the bars for dashed lines with "Not measured yet" and "Measuring — shows after a little use". The Model card is short beside the tall window card, which leaves a large empty block under it. usage-stale is byte-for-byte the same image as usage-measured in every size and theme, so the stale state is not shown at all (the previous render carried a "last measured 22m ago" marker). At 1280x800 the How it's going row is cut by the window edge, "1 errors / 12 turns" is still ungrammatical, and the rail shows the known cut age on "Stopped with an error · 7…".

## Weak

- usage-stale-1280x800-dark.png, usage-stale-1440x900-light.png (all four): identical to usage-measured; nothing marks the reading as stale.
- usage-blocked-1440x900-dark.png (all captures): the Model card is much shorter than the account-window card beside it, leaving an empty block.
- usage-measured-1280x800-light.png (all captures): "1 errors / 12 turns".
- All captures: rail row "Stopped with an error · 7…" loses its age.
