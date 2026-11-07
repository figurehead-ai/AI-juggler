# AJ-E3209 — Restart a running app or profile, after a confirm

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS (Accounts) | Restart… in the Accounts screen's Apps row menu (`docs/qa/evidence/AJ-E3208/restart-confirm-*.png`); the rail's App menu and the App page: `apps-section.test.tsx`, by hand in the live sitting |
| R2 | PASS | It asks first and names what works there: "2 sessions are working in it. They stop now; their conversations are kept and can be resumed." Cancel focused, Escape closes, nothing happens without Restart |
| R3 | OPEN | Quit, wait, start on the same profile, never a second copy: `places::tests::a_confirmed_restart_quits_and_starts_one_that_has_its_window`; by hand with the sanctioned test profile in the live sitting (the lab bridge doesn't serve it, and a desktop app runs outside a lab) |
| R4 | OPEN | Keep alive takes it as intended: in the same sitting |
| Visual | PASS | The confirm in both themes (above) |
