# Evidence manifest — AJ-E710 Fleet beta gate, dogfood, and security evidence pack

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E710. The gate file is `docs/qa/gate-E7.md`. Brought up to date 2026-11-07 on `release/0.4.1`, on the main session's ruling (B): no new security surface before the release. On one Mac the in-process loopback suites (`just gate-e7`: two AI Juggler backends over real loopback connections) are the proof of R1's rows, together with the gallery set and the keyboard-only sweep; the live rows are DEFERRED (D-89) to the maintainer's two-Mac check after the release (`docs/qa/second-mac-check.md`), which covers pairing, the board, A→B→A, a remote clean and Bring forward live.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1a pair < 60 s on two physical Macs | one Mac: `fleet_link :: the_right_code_and_both_confirmations_pair_and_connect_both_macs`; live: second-Mac check, check 2 (timed) | < 60 s, two Macs | loopback pairing green in `just gate-e7` (`automated-2026-11-07.md`) | DEFERRED (D-89) | d64b09478 | 2026-11-07 | Claude (self) |
| R1b one board across machines (or a filed waiver) | one Mac: `fleet_link :: a_console_sees_the_other_macs_instances_live_and_its_commands_go_through_the_node`, `fleet_switch :: new_session_starts_on_the_other_mac_and_follows_it_within_five_seconds`; gallery `46-fleet-board`; live: checks 3 and 4 | one board | green; waiver for a third machine filed in gate-E7.md (the maintainer has two Macs) | DEFERRED (D-89) | d64b09478 | 2026-11-07 | Claude (self) |
| R1c cross-machine A→B→A with the continuity checklist | one Mac: `fleet_switch :: t2_at_the_same_path_switches_to_the_other_mac_with_shared_lineage`, `switching_back_resumes_the_original_with_one_delta_turn_and_never_copies_back`, `a_diverged_original_blocks_switch_back_until_the_user_chooses`; gallery `50-cross-switch`; live: check 6 | A→B→A, continuity | green | DEFERRED (D-89) | d64b09478 | 2026-11-07 | Claude (self) |
| R1d remote orphan cleanup with zero collateral | one Mac: `fleet_link :: a_console_cleans_only_what_the_node_verifies_and_hears_each_outcome` (a real detached process cleaned, an impostor untouched); gallery `48-remote-cleanup`; live: check 7. Bring forward: `fleet_link :: a_remote_focus_runs_on_the_owning_mac_and_reports_its_honest_outcome`; live: check 5 | zero collateral | green | DEFERRED (D-89) | d64b09478 | 2026-11-07 | Claude (self) |
| R2 the security checklist, one negative per STRIDE row | gate-E7.md "Security negatives"; `just gate-e7` | every row green | green (2026-11-07 run) | PASS | afcbefe3d | 2026-11-07 | Claude (self) |
| R3 every unvalidated pair or path listed by name; Unvalidated at the point of action | gate-E7.md "Unvalidated combinations"; gallery `50-cross-switch` (badges "Not proven yet") | complete list | listed; the wizard's badges say it | PASS | 07314c0a1 | 2026-11-07 | Claude (self) |
| R4 measured performance vs budgets on the gate hardware | `just gate-e7` (heartbeat RTT p95 0.69 ms, 5.5 MB handover 15 ms, outbox p95 0.06 ms, catch-up, target list) on one Mac | measured, not estimated | loopback measured on this Mac; the two-Mac numbers come from the second-Mac check | DEFERRED (D-89) | afcbefe3d | 2026-11-07 | Claude (self) |
| R5 mixed versions: current ↔ previous minor | `transport :: the_mixed_version_matrix` | telemetry works, commands degrade with the banner, nothing crashes | green; protocol v1.0 has no previous minor to run physically yet | PASS | afcbefe3d | 2026-11-07 | Claude (self) |
| R6 Link off returns the app to fully local behaviour, tested | `fleet_switch :: link_off_returns_both_macs_to_fully_local_behaviour` | tested | green | PASS | afcbefe3d | 2026-11-07 | Claude (self) |
| R7 no pass with a P0 security negative red | `scripts/gate_e7.sh` exits non-zero on any red suite | by construction | holds | PASS | afcbefe3d | 2026-11-07 | Claude (self) |
| Visual: the screenshot set on the gate build; keyboard-only pair, switch and clean | the 12 Fleet gallery families retaken (07314c0a1), each critique noted; `keyboard-sweep-2026-11-07.md` | no state reachable only by mouse; no colour-only meaning | three focus faults found and fixed (2d6df1e07); a second filled button on Home fixed (7a2b75aea); no colour-only meaning | PASS | 07314c0a1 | 2026-11-07 | Claude (self) |
| Two tester sign-offs | gate-E7.md "Sign-off" | signed | the maintainer's, with the second-Mac check | DEFERRED (D-89) | — | 2026-11-07 | Claude (self) |

Lows noted in the critiques, not fixed here (cosmetic, for the 0.4.x round): 24-fleet and 48-remote-cleanup crop the node row at the table's foot; faint disabled buttons (24-fleet, 42-fleet, 45-remote, 61-account-macs); 45-remote's stale and offline overviews also say "measured 1 s ago"; 61-account-macs breaks fingerprints inside a group; 88-fleet-launch says "~$" and "≈ $" for one meaning.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Captures from the preview's generic fixtures
- [x] No conversation content
