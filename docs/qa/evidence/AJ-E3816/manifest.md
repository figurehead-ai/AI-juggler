# Evidence manifest: AJ-E3816 (pre-push walk, 2026-11-20)

Watches page.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| create, edit, pause, resume, remove + undo, quiet hours, App chip, Go to | not walked | | Mac in use | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| Create from a session | session page › Watch… › Watch | form with a live "Looks like this" preview | New watch form (Watch what, For what, Until, Quiet hours) with a notification preview; Watch → the Watches page, "1 of 50", toast "Watching." within 6 s of the press (driver steps included). The form offers "Finishes" for a session that has already ended (finding N-09) | WORKS with finding | captures/w2-1-new-watch-form.jpg, captures/w2-2-watch-created.jpg |
| Pause, Resume | Pause: …, Resume : … | state shown | "Paused since 11:48 AM" then "Waiting". The two buttons' names differ in form ("Pause: …" and "Resume : …", a space before the colon: finding N-10) | WORKS | captures/w2-3-paused.jpg |
| Edit | Edit › Always › Save changes | changes at once | "Until you remove it" saved; the page header says "Made 11:47 AM · never told you" | WORKS | captures/w2-4-edit.jpg |
| Go to; the session's chip | More › Go to “api · started 10:56 AM” | opens the session; chip "Watched · 1" | opened; "Watched · 1" beside Ask Bot about this | WORKS | captures/w2-5-go-to-watched-chip.jpg |
| An App watch and its chip | App page › Watch… › Asks you › Watch | chip on the App page | App watch "finishes, or asks you. Until midnight today." made; the App page shows "Watched · 1", which opens Watches filtered to that App | WORKS | captures/w2-6-app-page-watched-chip.jpg |
| Remove and Undo | More › Remove; pressed Undo | undo for a few seconds | toast "Watch removed: …" with "Undo (4s)"; the first removal's Undo was missed (the toast lasts 5 s; each driver step takes 1 to 3 s), the second Undo restored the watch | WORKS | captures/w2-7-removed-undo-toast.jpg |
| Quiet hours | Quiet hours link on Watches | the setting | opens Settings › Notifications with "Watches you set" on by default and "Manage watches · 1"; the quiet-hours fold stays closed ("More: quiet hours 22:00-07:00") | WORKS | captures/w2-8-quiet-hours-notifications.jpg |
