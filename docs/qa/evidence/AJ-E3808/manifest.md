# Evidence manifest: AJ-E3808 (pre-push walk, 2026-11-20)

Bot history.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| New topic, search, clear | not walked | | Mac in use | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| Remember, then What Bot remembers | "Remember: api is the example project for the walk", opened the page's notes tab | the note is listed, kept on this Mac | "I'll remember: …" with Edit / Undo; the tab lists it under "Your notes 1", "you said, just now"; the tab's count and the side card still read 0 / "Nothing yet" until the tab was opened (finding N-03) | WORKS with finding | captures/w2-1-what-bot-remembers.jpg |
| Forget and its notice | pressed Forget on the note | a confirmation that says what stays | "Forget “…”? … It's deleted from this Mac, and the message where you said it is cleared. You can't undo this." plus the backup notice; Cancel has focus; Forget removed it and the turn reads "Undone: I won't remember that." | WORKS | captures/w2-2-forget-confirm.jpg |
| Search | More › Search the conversation, typed "finished", Done | a match count | "1 of 1": it counts message text only, not the result rows' "Finished" marks; "Undone" gave "No matches" though that notice is in the thread (finding N-07) | WORKS with finding | captures/w2-3-search.jpg |
| New topic | pressed New topic | a new topic starts | the cursor is in the field; nothing in the thread shows a new topic began (finding N-08) | WORKS with finding | |
| Clear | More › Clear the conversation… | says what goes and what stays | "Deletes every question and answer from this Mac: 5 questions since Nov 20. You can't undo this." and "Your 0 notes and 3 watches stay" while the Watches page says 1 (finding N-02); Clear conversation returned the empty panel | WORKS with finding | captures/w2-4-clear-confirm.jpg, captures/w2-5-cleared-panel.jpg |
