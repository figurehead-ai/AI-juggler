# Evidence manifest: AJ-E3210 (pre-push walk, 2026-11-20)

Each App on its own Account: the live aij-test walk.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| steps 1-8 of docs/research/split-sign-in-folder.md | not run | | Needs the Dev build on the real data folder (V55 runs once), a terminal run in the aij-test folder and the Claude app's Code tab: not safe while the maintainer uses the Mac, and the lab K2 is not the real data folder. Credential steps (a sign-in) are the maintainer's | NOT RUN | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| steps 1-8 of split-sign-in-folder.md | not run | | Two blockers: (1) it needs Dev on the real Dev data folder (V55 runs once), and AI Juggler Dev there is running the 0.4.6 soak (pid 98313 from ~/Library/Caches/aij-soak-046, running about 2 h): a second Dev on it would migrate the soak's database under it; (2) steps 3-5 start the aij-test folder's CLI and the Claude app's Code tab, a sign-in-bound run that is the maintainer's. Nothing was started | NOT RUN (blocked) | |
