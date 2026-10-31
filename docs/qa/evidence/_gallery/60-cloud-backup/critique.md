<!-- captures: 6ad27ebd205e -->
# 60-cloud-backup: Cloud backup in Settings: the panel, uploading, the list of backups to restore, and the restore check

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Figurehead & Cloud card and its inner Cloud backup box stand off both canvases, and the hierarchy inside is sound: status line ("✓ Last backup 2 h ago · 418 KB"), schedule, then Back up now and Restore from Cloud…. In list, the recovery-code field has a strong violet focus ring, lighter in dark now, and three dated backups each have a Restore… button. Uploading only changes the status to "Backing up… (encrypting on this computer, then uploading)". It shows no progress bar, and Back up now still looks enabled. The restore-preview dialog has the same strong amber identity warning as local restore, but Restore is still a violet-edged outline next to Cancel, not the solid primary the switch dialogs now have. The design fixes show here only as the lighter dark accent (focus ring, toggles, Restore's edge, the bar's New session) and the rail's ai-juggler row, which now reads "Stopped with an error" whole. Every page capture still opens scrolled, with the Diagnostics paragraph's first line sliced through at the top edge.

## Weak

- list, panel, uploading (dark/light): the first line of the Diagnostics text above is sliced through at the top edge.
- uploading-1280x800-dark/light: no progress indicator while backing up, and Back up now still looks clickable.
- restore-preview-1280x800-dark/light: Restore, which replaces this Mac's data, is a violet-edged outline hardly stronger than Cancel.
