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
