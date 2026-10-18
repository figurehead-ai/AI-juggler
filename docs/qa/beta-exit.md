# Beta-exit criteria (v0.1-beta → wider beta/stable)

Beta may be claimed only when **every** item below is green on measured
evidence (stored per AJ-E801 under `docs/qa/evidence/`); anything less
is "in beta development" and the UI/README/release notes must say
exactly that.

| # | Gate | Threshold / evidence | Owning tasks |
|---|---|---|---|
| 1 | Fidelity register | Same-Provider pairs PASS; cross-Provider pairs PASS or FAIL-with-drop-list, from the fixture matrix + live-binary dogfood per pair | AJ-E210, AJ-E808 |
| 2 | Dogfood programme | ≥80% unaided task completion; ≥80% A→B→A continuity (resumed conversation carries the thread); median switch <2 min | AJ-E809 |
| 3 | Cleanup safety | Zero false cleanup kills across the dogfood programme (any false kill blocks exit) | AJ-E503/E507/E512, AJ-E809 |
| 4 | Accessibility | a11y gate green on measured evidence (keyboard-complete core loop, contrast, no colour-only signals) | AJ-E804 |
| 5 | Performance | Perf gate green on measured evidence (rail <300 ms @50 rows; inspector <200 ms; 60 fps with 20 streaming terminals) | AJ-E805 |
| 6 | Fleet security review | STRIDE review passed (pairing, transport, replay, identity theft, malicious peer) | AJ-E710 |
| 7 | Gate sign-off | Maintainer-signed gate report; every gate claim links its evidence path | AJ-E807 |
| 8 | Unsigned-gate closure | No TODO gate, no deselected check, no re-threshold without a recorded quality decision | AJ-E806/E807 |

Supersedes: none (first issue). Sources: decision-log D-04/D-11,
docs/tasks/E8, IMPLEMENTATION-AGENT-PROMPT §8.
