# Evidence manifest — AJ-E814

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E814,
idle CPU with the window shown but fully covered. Record:
[`docs/qa/evidence/AJ-E805/idle-0.4.6.md`](../AJ-E805/idle-0.4.6.md)
("AJ-E814: covered"). Built on release/0.4.6, 2026-11-16; verified by
Opus (claude-opus), every reading checked against its CSV and the app's
own log.

Quality gate: the covered median (0.98 %) under the shown median (1.58 %); the gap to hidden explained.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The covered state is a lab-owned window: `scripts/qa/window_cover.swift cover` opens an opaque, non-activating panel (an accessory app, `canBecomeKey` false) ordered just above the lab build's window by its window number, its exact size and place, labelled "AI Juggler test: measuring, closes by itself in a minute"; no app is activated and no key is sent. It takes clicks itself since launch 6: before that a click on it reached the lab window under it and made the lab app front (after-5, dropped and recorded). Checked by the window server's list (the panel directly above the window, the window 0.0 % uncovered in every kept covered reading) and by WebKit's own log for the page (`view visibility state changed 1 -> 0` at the cover, hidden for the whole minute). |
| R2 | PASS | Five kept 60 s runs, ten working, the after build (pairs 1, 3, 4, 6, 7): covered 0.98 0.74 0.93 1.01 0.98, median **0.98 %**; shown in the same launches 1.45 1.71 1.64 1.58 1.31, median **1.58 %**; hidden 0.75 0.72 0.70 0.99 0.91, median **0.75 %**. The covered median is under the shown median: covered, the web content falls to 0.05–0.16 % and the GPU process to 0.00–0.05 %, as hidden. The gap to hidden (0.23) is explained in the record: the app process and the web content's last 0.05–0.10 % while the window stays on screen and in the window list, and the covered reading's earlier place in each launch; within the app's own spread when hidden (0.62–0.91 %). The before build covered at low load: median 0.95 (four kept). |
