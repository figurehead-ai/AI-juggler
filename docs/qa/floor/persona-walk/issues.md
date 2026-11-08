# The Floor walk's issues (AJ-E3514)

## The setup: a window manager on the walk Mac (2026-11-08)

The walk Mac runs BetterStage (a third-party window manager that stages an app's windows one at a time and tiles them). With it, opening the Floor puts it in the main window's frame and stashes the main window off screen; moving the Floor away swaps them back. Nothing in AI Juggler places the Floor (it asks for 1278 × 798 and no position). Reproduced on a lab with the same build (78bf9c22a + floor-harness, bundled):

- A door works: Enter on a selected area brought the main window forward at the App's page.
- `floor_drive open` on a Floor that's already open but stashed didn't bring it back while another app was in front (an app in the background may not bring itself forward; the Floor button, clicked in the app, is the real path).

So the plan's check "the main window stays on screen beside the Floor" can't hold on this Mac. The check is now: **a door brings the main window forward at that page**, and testers are told that one window shows at a time (`main …` verbs need the main window shown: a door, or the main window's own controls, bring it back). A walk on a Mac without a window manager keeps the original check.

## F1 (Maya), run 1 — the 0.4.1 session's run, 2026-11-08

Failed, mostly on the setup above. Product findings, triaged:

| # | Finding | Severity | Lands in |
|---|---|---|---|
| 1 | The waiting desk isn't marked apart from CRASHED and the sleeper's Zzz | 2 | 0.4.x: check its `!` balloon and spotlight against the crash and limit marks at the opening view (captures) |
| 2 | The title's "1 session waiting here" can't be clicked to fly to the desk | 1 | Not a bug: a native title can't be a button; N jumps to the next one waiting (SPEC §9.7). 0.4.x: say N in the hint line |
| 3 | View › Labels › "Attention only" seemed to change nothing; the menu draws past the window's edge | 2 | 0.4.x: check by hand (may be a missed click); the menu's placement is the system's |
| 4 | The red memory-pressure banner competes with the goal | 1 | Lab artefact (the walk Mac's own memory pressure); not a Floor issue |
