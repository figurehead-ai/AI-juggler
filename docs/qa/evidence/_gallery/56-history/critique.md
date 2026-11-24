<!-- captures: 14c1f5e6f72a -->
# 56-history: History of ended and archived sessions, its search, a read-only replay, delete, and the retention dry run

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

History reads cleanly: the subtitle is bright, the Yesterday card stands off the canvas, and in search a strong violet focus ring marks the field while the matched sentence sits in a tinted strip under the row. In dark that ring, the delete dialog's focused field and the bar's New session now use the lighter violet, which stands out more on the near-black. The rail's ai-juggler row now reads "Stopped with an error" whole instead of ending in a cut "· 7…". Delete and the dry run are careful dialogs. Delete needs "delete" typed before "Delete permanently" wakes. The dry run lists what goes, what is kept and why, and gives "Remove 2 records" a red outline. The replay dialog is unchanged and still hard-wraps the transcript mid-word ("the firs / t delivery") at 1024 wide. Its Processes times also wrap to a lone "PM". The retention captures still open scrolled, with the "Default for new accounts · Notify only" row cut under the top of the settings column, and the storage & retention section, the point of the capture, sits at the bottom.

## Weak

- replay-1024x680-dark/light: the transcript breaks "first" across two lines ("firs" / "t delivery"); the process times wrap to a lone "PM".
- retention-1280x800-dark/light and retention-1024x680: the page opens scrolled mid-card, with a row cut under the top edge, and the retention section is pushed to the bottom.
- dry-run (all four): the sessions show "last active 6/5/2024" next to ids dated 2026 (sess-2026-06-02-a1), which looks inconsistent.
- archived (all four): the row shows both "Ended" and an "Archived" tag while the Archived tab is already selected; the tag repeats the tab.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (search, archived, replay, delete, dry run, retention at 1280 and 1024, both themes); opened at full size: search-1280x800-light, retention-1280x800-dark and -light, replay-1024x680-dark.

- **Fixed** (51ea3254f): Settings' retention rows were outlined boxes inside the card with outlined "Review cleanup…" buttons; they are a step of tone with the kit's secondary (disabled under Keep forever). Gate retention and dry-run re-rendered.
- **Closed since 0.3.2:** the replay no longer breaks words mid-line at 1024.
- **Weak, logged F79:** the replay predates the conversation feed: "user" / "assistant" over monospace, a sha256 in the title, "root 48211 · exited · clean", "helper … reaped", "· reviewed", "confirmed · F1", an outlined Close.
- **Weak, kept:** retention opens scrolled to the page's end (a cut row at the top; the section's title is in view).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Find an ended session by its words, read it, archive or delete it. |
| 2 | Prominence | 4 | Search first, then the days. |
| 3 | Default | 4 | Ended tab; retention 90 days or Keep forever per App. |
| 4 | Mode | 4 | Calm list; dialogs on the one frame. |
| 5 | Mock-up | 4 | AJ-E2601's History. |
| 6 | Areas | 4 | After the fix no outlined rows in the retention card. |
| 7 | Colour | 4 | Violet for the search focus; red outline on Remove only. |
| 8 | Type | 4 | Titles ink, meta muted. |
| 9 | Actions | 4 | Delete needs "delete" typed; Remove 2 records last. |
| 10 | Serious | 4 | Dry run lists what goes, what's kept and why. |
| 11 | Doors | 4 | Every row opens its session. |
| 12 | Words | 4 | F79 (8212c0185): the replay says its facts in words ("Marked reviewed", "Came from Claude Code CLI · Personal, with the full story, fresh start", "A helper process was cleaned up by AI Juggler", times as "5 h ago"). |
| 13 | States | 4 | "1 session" for a search; empty days not drawn. |
| 14 | Consistency | 4 | F79: the replay draws the conversation with the session page's own turns (ADR-0024's `Turn`). |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: retention removes only AI Juggler's records, said in the card; a replay reads like the session it was. Match. |

Lowest: 3, each line under 4 logged (F79); the rest 4 or 5. After F79: lowest 4.

**Fixed after the critique (F79), re-captured (every state at 1280 and 1024, both themes; opened replay-1280x800-dark and replay-1024x680-light):** the replay is "Replay · Read-only: the conversation as it was stored · 48 KB" with Close (the kit's secondary), the conversation on a tone panel as the session page draws it ("The start of the conversation", your words in bubbles on the right, Claude's under its mark and name), and beside it What happened (Working, Ended, Marked reviewed, each "… ago"), Switches and Processes in words (it was "user" / "assistant" over monospace on black, a sha256, "root 48211 · exited · clean", "· reviewed", "confirmed · F1", an outlined Close). Lines 12 and 14 3 → 4; lowest now 4.
