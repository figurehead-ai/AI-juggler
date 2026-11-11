# AJ-E3207 — New session: the usual choice made and said

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Walk N1: "Start in web on Claude Code · Work · work — the last you used here", open 19/49 ms (`docs/qa/0.4-walks/N1.md`) |
| R2 | PASS | Walk N2: Change folder and Change app one step each, the sentence follows |
| R3 | PASS | Walk N3: one fold, its label its values ("the defaults" when none, fixed), nothing remembered |
| R4 | PASS | Walk N4: "a first guess: no app has run a session yet" (fixed: it hadn't said why); walk N5: out of room said before you start, another Account's App offered (fixed: also when its room is unmeasured), never switched by itself (`docs/qa/0.4-walks/N4.md`, `N5.md`) |
| L3 | PASS | `ui/tests/visual/l3-new-session.spec.ts`, one journey per row of `docs/qa/pairwise/new_session.csv` (every pair of history, room, folder, tool, rare option, start key, theme and width): 18 of 18 rows pass on 2026-11-06 (release/0.4.1), 163 clicks, feedback p95 5 ms, max 6 ms against 100 ms (`docs/qa/evidence/l3/new-session.json`). The live start in the lab is done too (N1, A7: the process runs on the App's own folder) |
