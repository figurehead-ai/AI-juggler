<!-- captures: 3db13dd9225b -->
# 87-burn-down: Usage, an account's week and 5-hour charts, its tokens line and When you work, in comfortable and compact density

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The chart cards and the When you work heatmap stand off the canvas in both themes, the heatmap's violet ramp reads from pale to deep, and the empty cells are a visible grey grid rather than holes; in dark the ramp, the 5-hour projection's solid line and the This week line now use the lighter accent, so the deep cells are a brighter violet. The times changed with the re-render: the peak bars sit on different days and the lines now read "runs out about 5:36 AM, before the reset at 6:06 AM" and "Resets Tue 2:06 AM". All four captures still open scrolled, though: the account card's title and the top of the week chart are above the edge, so it isn't said which account this is, and the second card to its right is the same height but completely empty in view, which reads as a broken blank panel rather than a neighbour whose content is above. The "Each 5-hour window, at its peak" bars are a murky olive on the dark card and a pale khaki in light, the weakest marks on the page next to the week line and the violet projections; the run-out projection is a dashed amber (dark) or brown (light) line with its reset marker, clear in both. The compact captures differ only in the rail, where single-line rows cut the session names to "feat/read-m…" and "Refa…", which no longer names anything.

## Weak

- usage-1280x800-dark-comfortable.png (all four): the right-hand card shows as an empty panel the full height of the chart card; with the scroll position the reader sees no title on either card.
- usage-1280x800-dark-comfortable.png, usage-1280x800-dark-compact.png: the olive peak bars have low contrast on the dark card.
- usage-1280x800-dark-compact.png, usage-1280x800-light-compact.png: compact rail rows cut names to "Refa…" and "feat/read-m…".
