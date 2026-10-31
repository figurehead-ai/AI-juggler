<!-- captures: 4432d268d61a -->
# 57-backup: Settings › Advanced backup and restore: writing a backup, the finished backup, the restore check, and a refused damaged file

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Data: backup & restore card stands off both canvases, and its sentence text reads clearly above the "Back up now…" and "Restore from a backup…" buttons. The progress capture's violet bar at 42 % is easy to see, and in dark it, the focus ring on Back up now… and the Local cache toggle now use the lighter violet. In done, the "✓ Backed up — 18 MB" summary, with the path in monospace and the withheld key named, is complete and calm. The rail's ai-juggler row now reads "Stopped with an error" whole instead of ending in a cut "· 7…". The plan dialog is good: a checked summary line, an amber warning box for the other Mac's identity, and a short list of consequences. But Restore is still only a violet-outlined button, barely stronger than Cancel; the solid violet primaries of the switch dialogs did not reach it. In refused, the error line still breaks a word ("does not m / atch its checksum"), starts with a raw "invalid restore:" prefix, and has no colour beyond a ⚠ on "Nothing was restored", so a damaged backup looks like a routine note. Every page capture still opens scrolled, with the retention card's first line sliced at the top.

## Weak

- refused-1280x800-dark/light: the error breaks "match" across two lines and has no red or amber treatment; "invalid restore:" is a raw internal prefix.
- plan-1280x800-dark/light: Restore, which replaces this Mac's data, is an outline (violet-edged) hardly stronger than Cancel, not the solid primary the switch dialogs now use.
- done, progress and refused (dark/light): the page opens scrolled, with the retention card's first sentence sliced under the top edge.
