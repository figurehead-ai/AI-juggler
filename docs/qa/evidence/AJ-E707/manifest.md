# Evidence manifest — AJ-E707

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E707 (remote process
ownership and cleanup). 2026-10-18, reviewer claude-opus. Contract:
ADR-0013; local safety model AJ-E501..E512.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Remote spawns are recorded in the owning node's ledger at spawn time | Req.1 | A console's launch request runs the node's own `launch_profile`, so the node's spawn ledger (AJ-E501) records it exactly like a local launch — nothing is reconstructed from `ps` | PASS | 2026-10-18 | claude-opus |
| Identity verified before any kill (pid + start time + exe + owning Instance); mismatch aborts | Req.2 | The node runs `kill_execute_as` — the same click-time verification as a local Clean (with the exec-settle rule). `fleet_link :: a_console_cleans_only_what_the_node_verifies_and_hears_each_outcome`: a real process recorded with the wrong start time → `refused`, **zero signals, still alive** | PASS | 2026-10-18 | claude-opus |
| D-08 tiers on the owning machine; every kill audited on node and console | Req.3 | Automatic tiers are unchanged and stay per-Account on the owning machine (notify-only default); a remote Clean is a manual, user-initiated action. The node's cleanup audit records it as `fleet:<console label>` (asserted `fleet:Laptop`); the console logs `command_sent` + outcome in its hash-chained link log (asserted) | PASS | 2026-10-18 | claude-opus |
| Remote Clean needs View + control and an explicit click; the node re-verifies at execution | Req.4 | Clean is a danger button only on verified rows, disabled without control ("cleaning needs View + control on <machine>") or a live connection; it opens an alertdialog (Cancel focused) before anything is sent; the node verifies again at execution regardless | PASS | 2026-10-18 | claude-opus |
| Orphans surface to consoles within 10 s with age, CPU/RAM, ports and owner | Req.5 | Nodes publish their managed-process inventory every ~6 s (plus on subscribe) as an additive `CleanupDelta` — executable **name only, never a command line** (asserted); rows carry state, age, CPU, RSS, ports, owning Instance + Account | PASS | 2026-10-18 | claude-opus |
| Unowned / unverifiable processes listed "Not owned — never touched", no Clean | Req.6 | vitest `never offers Clean on a row its machine didn't verify` (zero Clean buttons on unverified rows); the verdict is the owning machine's own check | PASS | 2026-10-18 | claude-opus |
| The node's TERM → grace → KILL ladder; per-process outcome on the console | Req.7 | Outcomes in the console's words: `Exited`, `Needed SIGKILL`, `Already gone`, `Refused — identity changed` (+ pid); the test asserts the owned process is gone and the outcome names it | PASS | 2026-10-18 | claude-opus |
| Grouped Machine → Instance, verification before Clean, focus returns to the group header; axe | UI / a11y | Cleanup → Machine: All machines adds a section per paired machine; focus returns to that machine's heading after an action; axe 0 serious/critical on the section and the confirmation, both themes | PASS | 2026-10-18 | claude-opus |
| Visual: grouped view (verified / not owned), confirmation, identity-changed — 1440×900 + 1024×768, light/dark | visual | Gallery `48-remote-cleanup` | PASS | 2026-10-18 | claude-opus |
| Fixture harness child + grandchild + port listener cleaned remotely with zero collateral | quality gate | The remote path is proven with real processes (owned vs impostor); the full child/grandchild/port fixture tree is proven locally by `guardrails` and runs through the same `kill_execute_as` on the node. A remote run of the full tree is left to AJ-E710 | PARTIAL | 2026-10-18 | claude-opus |
| Remote Clean < 2 s for ≤ 50 processes | perf budget | Loopback round trip is the node's ladder time (grace) plus milliseconds; measured on gate hardware at AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Two-machine dogfood: a real `vite` orphan cleaned remotely, before/after `lsof` + audit excerpt | evidence | AJ-E710 gate item | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test fleet_link`: 9/9.
- vitest `remote-cleanup.test.tsx` 3/3.
- Playwright `48 remote cleanup` 4, `fleet — remote cleanup — axe 0` 2.
