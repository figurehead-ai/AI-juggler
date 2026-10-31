<!-- captures: 4f77325e87a2 -->
# 57-backup: Settings › Advanced backup and restore: writing a backup, the finished backup, the restore check, and a refused damaged file

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Data: backup & restore card now stands off both canvases, and its sentence text reads clearly above the "Back up now…" and "Restore from a backup…" buttons. The progress capture's violet bar at 42 % is easy to see. In done, the "✓ Backed up — 18 MB" summary, with the path in monospace and the withheld key named, is complete and calm. The plan dialog is good: a checked summary line, an amber warning box for the other Mac's identity, and a short list of consequences. But Restore is a plain outlined button, the same weight as Cancel. In refused, the error line breaks a word ("does not m / atch its checksum"), starts with a raw "invalid restore:" prefix, and has no colour, so a damaged backup looks like a routine note. Every capture opens scrolled, with the retention card's first line sliced at the top. All eight captures were re-rendered.

## Weak

- refused-1280x800-dark/light: the error breaks "match" across two lines and has no red or amber treatment; "invalid restore:" is a raw internal prefix.
- plan-1280x800-dark/light: Restore, which replaces this Mac's data, is a neutral outline no stronger than Cancel.
- done, progress and refused (dark/light): the page opens scrolled, with the retention card's first sentence sliced under the top edge.
