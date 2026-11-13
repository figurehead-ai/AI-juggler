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
| 7 | Sign-off | The maintainer's Stable approval, recorded with every scope task's evidence link (D-109) | AJ-E807 |
| 8 | Gate closure | No TODO gate, no deselected check, no re-threshold without a recorded quality decision | AJ-E806/E807 |

## Gate 1 for Claude Code and Codex: live versus fixtures (2026-11-07, AJ-E1305)

The fidelity register (`crates/juggler-handover/data/fidelity-register.json`,
rendered in `docs/qa/evidence/AJ-E808/register.md`) promotes a cell to PASS
only from a live run on the real binaries in a throwaway lab folder, with
the versions recorded; a tool update returns the cell to UNVALIDATED.

| Pair | Verified live | On fixtures only, or not yet |
|---|---|---|
| Claude Code → Claude Code | transcript, rules, task list, subagents, MCP references, folder and worktree, switch-back (2.1.302, labs 5–11) | file history (the adapter's test) |
| Claude Code → Codex | transcript, rules, task list, MCP references, folder and worktree, switch-back (2.1.310 / 0.163.0, lab 14; labs 19–20 again) | subagents and file history: FAIL with a drop list (left out and named in the handover) |
| Codex → Claude Code | transcript, rules, task list, MCP references, worktree, switch-back (0.163.0 / 2.1.310, lab 14) | subagents, file history: UNVALIDATED |
| Codex → Codex | transcript and plan (0.163.0, lab 11, a second Codex home signed in for the lab) | the other six areas, and two Codex accounts side by side: deferred (D-91), they need a second Codex account |
| Codex app-server | limits read before a turn, a real approval and a question (runtime lab 2) | the rest of the turn lifecycle (interrupt, steer, compact): the fake Codex, its surface pinned from 0.163.0 |

The same split is in SPEC §4.1 and §4.2 and in the release notes.

Supersedes: none (first issue). Sources: decision-log D-04/D-11,
docs/tasks/E8, IMPLEMENTATION-AGENT-PROMPT §8.
