<!-- captures: 53ae2a0eede9 -->
# 60-cloud-backup: Cloud backup in Settings: the panel, uploading, the list of backups to restore, and the restore check

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Figurehead & Cloud card and its inner Cloud backup box stand off both canvases, and the hierarchy inside is sound: status line ("✓ Last backup 2 h ago · 418 KB"), schedule, then Back up now and Restore from Cloud…. In list, the recovery-code field has a strong violet focus ring, lighter in dark now, and three dated backups each have a Restore… button. Uploading only changes the status to "Backing up… (encrypting on this computer, then uploading)". It shows no progress bar, and Back up now still looks enabled. The restore-preview dialog has the same strong amber identity warning as local restore, but Restore is still a violet-edged outline next to Cancel, not the solid primary the switch dialogs now have. The design fixes show here only as the lighter dark accent (focus ring, toggles, Restore's edge, the bar's New session) and the rail's ai-juggler row, which now reads "Stopped with an error" whole. Every page capture still opens scrolled, with the Diagnostics paragraph's first line sliced through at the top edge.

## Weak

- list, panel, uploading (dark/light): the first line of the Diagnostics text above is sliced through at the top edge.
- uploading-1280x800-dark/light: no progress indicator while backing up, and Back up now still looks clickable.
- restore-preview-1280x800-dark/light: Restore, which replaces this Mac's data, is a violet-edged outline hardly stronger than Cancel.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Cloud backup content is unchanged; counts read apps instead of accounts.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
