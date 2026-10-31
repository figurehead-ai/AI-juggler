<!-- captures: af775c77d7d6 -->
# 56-history: History of ended and archived sessions, its search, a read-only replay, delete, and the retention dry run

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

History reads cleanly: the subtitle is bright, the Yesterday card stands off the canvas, and in search a strong violet focus ring marks the field while the matched sentence sits in a tinted strip under the row. In dark that ring, the delete dialog's focused field and the bar's New session now use the lighter violet, which stands out more on the near-black. The rail's ai-juggler row now reads "Stopped with an error" whole instead of ending in a cut "· 7…". Delete and the dry run are careful dialogs. Delete needs "delete" typed before "Delete permanently" wakes. The dry run lists what goes, what is kept and why, and gives "Remove 2 records" a red outline. The replay dialog is unchanged and still hard-wraps the transcript mid-word ("the firs / t delivery") at 1024 wide. Its Processes times also wrap to a lone "PM". The retention captures still open scrolled, with the "Default for new accounts · Notify only" row cut under the top of the settings column, and the storage & retention section, the point of the capture, sits at the bottom.

## Weak

- replay-1024x680-dark/light: the transcript breaks "first" across two lines ("firs" / "t delivery"); the process times wrap to a lone "PM".
- retention-1280x800-dark/light and retention-1024x680: the page opens scrolled mid-card, with a row cut under the top edge, and the retention section is pushed to the bottom.
- dry-run (all four): the sessions show "last active 6/5/2024" next to ids dated 2026 (sess-2026-06-02-a1), which looks inconsistent.
- archived (all four): the row shows both "Ended" and an "Archived" tag while the Archived tab is already selected; the tag repeats the tab.
