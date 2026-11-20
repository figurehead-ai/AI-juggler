# Evidence manifest: AJ-E3817 (pre-push walk, 2026-11-20)

Watch alerts.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| grouped alert, quiet hours, "· your watch" | not walked | | needs live sessions finishing; Mac in use | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| Grouped alert, quiet hours hold, "· your watch" | needs live sessions finishing | | BLOCKED: every stand-in App of T16 (the only lab with a start-able tool) "needs its sign-in" (10 need you), and a sign-in is the maintainer's step; K2 holds only ended sessions. Nothing fired | NOT WALKED (credential-gated) | |
| "Watches you set" setting | read Settings › Notifications | on by default, quiet hours explained | on, with the sentence "Quiet hours hold it, unless that watch is set to tell you even then"; "A session finishes" and "The digest" are off by default | WORKS | captures/w2-8-quiet-hours-notifications.jpg (in AJ-E3816) |
