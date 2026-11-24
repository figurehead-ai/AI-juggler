<!-- captures: e07c8743fe8b -->
# 57-backup: Settings › Advanced backup and restore: writing a backup, the finished backup, the restore check, and a refused damaged file

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Data: backup & restore card stands off both canvases, and its sentence text reads clearly above the "Back up now…" and "Restore from a backup…" buttons. The progress capture's violet bar at 42 % is easy to see, and in dark it, the focus ring on Back up now… and the Local cache toggle now use the lighter violet. In done, the "✓ Backed up — 18 MB" summary, with the path in monospace and the withheld key named, is complete and calm. The rail's ai-juggler row now reads "Stopped with an error" whole instead of ending in a cut "· 7…". The plan dialog is good: a checked summary line, an amber warning box for the other Mac's identity, and a short list of consequences. But Restore is still only a violet-outlined button, barely stronger than Cancel; the solid violet primaries of the switch dialogs did not reach it. In refused, the error line still breaks a word ("does not m / atch its checksum"), starts with a raw "invalid restore:" prefix, and has no colour beyond a ⚠ on "Nothing was restored", so a damaged backup looks like a routine note. Every page capture still opens scrolled, with the retention card's first line sliced at the top.

## Weak

- refused-1280x800-dark/light: the error breaks "match" across two lines and has no red or amber treatment; "invalid restore:" is a raw internal prefix.
- plan-1280x800-dark/light: Restore, which replaces this Mac's data, is an outline (violet-edged) hardly stronger than Cancel, not the solid primary the switch dialogs now use.
- done, progress and refused (dark/light): the page opens scrolled, with the retention card's first sentence sliced under the top edge.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Backup and restore content is unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
