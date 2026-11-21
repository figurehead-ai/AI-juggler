# Evidence manifest: AJ-E3804 (pre-push walk, 2026-11-20)

Bot: icon, panel, page, sheet, scope, memory, history.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| R1 icon with tooltip, shortcut, focus ring | pressed Bot in the header (by its accessibility name) | opens the panel | Icon found as `Bot`; panel docked right in one step (not timed; feels instant) | WORKS (shortcut Cmd+J, focus ring not walked: no global key allowed) | captures/bot-1-panel-empty.jpg |
| R2 panel beside content, stays while navigating | Home with the panel open | content not covered | Home reflowed to the left of the 340 px panel; nothing covered | WORKS | captures/bot-1-panel-empty.jpg |
| R3 empty state, suggested questions | read the empty panel | one suggestion per state from the words file | Three suggestions; the third is German ("Was hängt gerade fest?") in an English app: finding F-03 | FINDING | captures/bot-1-panel-empty.jpg |
| R4 answers link their sources | asked "What changed today?" | records as links | Answer lists session rows (App, Account, project, time) with links; count disagrees with Home (finding F-04) | WORKS with finding | (text read; capture bot-3 not kept) |
| Look-up "What's blocked?" | typed and Return | honest empty answer | "I found nothing that matches right now." + "Looked at every session, task and pool" | WORKS | captures/bot-2-blocked.jpg |
| Full page, 1024 sheet, scope chip and x, Remember/Forget, What Bot remembers, Watch this, New topic, search, clear | not walked | | The Mac was in use; the driver stopped each step | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| R1/R3 empty panel, no model | opened Bot with Apple's model not offered (app run from /tmp) | look-up panel | "Look things up", four go-straight cards, "Watch a session…"; no German | WORKS | captures/w2-1-panel-no-model.jpg |
| F-03 three English examples | Models › Use it (Apple, Beta), opened Bot | "Try one": What needs me? / What finished today? / What's blocked? | exactly those three, all English | FIXED, confirmed | captures/w2-2-panel-empty-model-f03.jpg |
| Full page, Expand and Back | pressed Expand to the Bot page, then Back to the panel | page with the conversation and "What Bot remembers" | Page shows the whole conversation, search box, "What Bot remembers 0" tab and a side card; Back returns to the docked panel. Under 1 s each (not timed to ms). Layout: the side card sits top right; the notes tab leaves the left half empty beside "Your notes" | WORKS (layout: see W-03) | captures/w2-3-full-page.jpg |
| Sheet at 1024 px wide | tried to set the window to 1024x800 through System Events | sheet under 1100 px | Dev's window does not resize through accessibility (size stays 1701x1348) | NOT WALKED (walk kit) | |
| "Ask Bot about this" from an App page, chip, x | App page of Claude Code CLI · Personal · personal, Ask Bot about this, asked "What's running?", pressed the chip's x | chip "About: …"; question scoped; x returns to everything | chip "About: Claude Code CLI · Personal · personal" above the field and the field's hint; the answer's "Looked at every session in Claude Code CLI · Personal · personal"; x gives "Ask about your sessions, tasks, pools or usage…" | WORKS | captures/w2-4-ask-bot-app-scope-chip.jpg |
| "Ask Bot about this" from a session, chip, x | session page "api · started 10:56 AM", Ask Bot about this, then x | the same | chip "About: api · started 10:56 AM", x removed it | WORKS | captures/w2-5-ask-bot-session-scope-chip.jpg |
| Menu "Remember this…" | More ways to ask › Remember this… | the field gets "Remember:" and the cursor | the field is filled with "Remember:" but the cursor is not in it: the first typed words went nowhere; a click or a `focus` was needed (finding N-04) | WORKS with finding | |
| Watch a session… from Bot's menu | More ways to ask › Watch a session… | a way to pick a session | it asks "What's running?" and answers "I found nothing that matches right now." on a lab with nothing running: no way on to an ended session (finding N-05). Watches were made from the session and App pages instead (AJ-E3816) | WORKS with finding | |
