# Evidence manifest: AJ-E2323

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2323: the switch's verification matrix: every ownership shape as a fixture in CI, and the real cases walked in lab PL1.

**Branch:** `release/0.5.1`. R1 and R5 are fixtures only (`procs::fixture` tables over the move lab's temp folders; no process read, quit or started). R2–R4 and R6 are open: they wait for the move chain (AJ-E2321, AJ-E2322) and the PL1 lab walk.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 D1's 13 ownership shapes as `procs::fixture` tables, each asserting what blocks, what is reported and the refusal class | PASS | `handover::moving::tests::shapes::d1_01` … `d1_13` (13 passed). Each runs the real decisions: the profiles' trees by `ownership` (`places::tree::of`), the writers by Claude's hook, the self rule, the offer and `stop_places`. Blocks / reported / class: 1 main + the resuming session / 9 helpers and node / Transient if the session doesn't end; 2 nothing of the other profile's `--add-dir` wrapper / nothing / none; 3 main + session / node / none; 4 main / crashpad when the main process started it, nothing when launchd's at the look / none; 5, 6 the self rule before any quit / — / NeedsYou; 7 slash, link and case spellings are the profile's (reported), the two-word form, a second flag and one after `--` tell nothing / — / none; 8 the holder only, never its running sibling (source by its records: `picking::two_profiles_on_one_account…`); 9 the picked target only (`picking::the_target_is_the_app_picked_by_its_id…`); 10 a reused session pid neither waited nor reported, a reused main never signalled / — / Transient; 11 a stale pid file no writer (reported), a current one waited / — / Transient; 12 a headless `claude -p --resume` from a terminal, named before any quit / — / NeedsYou; 13 see the note | 3f7bb2a30 | 2026-11-23 |
| R2 D2's engine fault injection and D3's app-level integration | OPEN | waits for the move chain | — | — |
| R3 the PL1 lab walk | OPEN | waits for the lab walk | — | — |
| R4 the walk from an empty Dev-identity data folder, every add path, sessions shown (AJ-E2327) | OPEN | waits for the lab walk | — | — |
| R5 a timing test that the offer's checks stay within 500 ms on a large fixture; AJ-E2318's evidence manifest | PASS | `shapes::the_offers_checks_stay_within_500_ms_on_a_large_table`: 3,114 processes, 6 running profiles (helpers, 3 Code tabs each), 400 pid files in both accounts' folders, one process table per offer; best of 3: 107 ms (debug build). `docs/qa/evidence/AJ-E2318/manifest.md` (R1–R4 PASS) | 3f7bb2a30 | 2026-11-23 |
| R6 AJ-E2328's caveats closed on the real Claude app in two lab profiles | OPEN | waits for the lab walk | — | — |

**Note on shape 13:** D1 asked for class Bug. AJ-E2315's rule makes contradictory evidence nobody's: each profile sees the other's evidence, so neither tree has the process and it is never waited on (asserted). If it writes the family, the move refuses before anything quits, naming it, as NeedsYou (the person can end it); writing nothing, it is ignored. No path gives it Bug. That is a decision for the coordinator: keep NeedsYou, or add a Bug class for a writer both profiles' evidence contradicts.

## Checks

- `cargo test -p juggler-app --lib -- handover::moving`: 90 passed. `cargo clippy -p juggler-app --all-targets -- -D warnings`: clean. `rustfmt` on the new file.
