# Evidence manifest: AJ-E3805 (pre-push walk, 2026-11-20)

Bot's look-ups, sources, "I don't know".

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| "I don't know" | asked "Why is the sky blue?" | says so, offers a way on | "I don't know: I couldn't find anything about that in AI Juggler." plus a "What finished today" suggestion card and "Looked at sessions, tasks, pools and usage" | WORKS (clear, calm) | captures/bot-4-dont-know.jpg |
| Written answer on Apple's model (Beta on) | asked three questions | an answer written by the model, passing the checks | Not confirmed: the answers read as code-built look-ups ("Here's what I found: 8 sessions finished"); no "written by" label seen. The question that should have triggered it did not get through (Mac in use) | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| F-04 "What changed today?" | asked it on K2 (19 ended today, more than 8 matches) | lead = rows shown + "and n more" | "Here's what I found: 19 sessions finished." with 8 rows and "and 11 more" (8 + 11 = 19); Home's rail says "19 ended today"; answer in about 3 s from pressing Ask (read in 1 s steps) | FIXED, confirmed | captures/w2-1-changed-today-f04.jpg |
| Rows' "Watch…" buttons | read the answer's controls | each says which session | four buttons are all named "Watch… api" / "Watch… web" with no more: the accessibility names don't tell the rows apart (finding N-06) | FINDING | |
