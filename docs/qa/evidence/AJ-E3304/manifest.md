# Evidence manifest: AJ-E3304

**Task:** `docs/tasks/E33-validated-by-use.md`, AJ-E3304: pools validated by the exhaustive tables, the pairs, and P1–P11 by hand.

**Branch:** `release/0.4.1`.

**Status:** IN-PROGRESS. L1 and L2 run in CI. The P sheets are walked, except the live parts listed as PENDING below.

| Claim | Command / sheet | Threshold | Observed | Verdict | Commit | Date |
|---|---|---|---|---|---|---|
| L1: the resolver, talk and seal against the reference model | `cargo test -p juggler-core --test pools_l1 --test pools_resolver --test pools_storage --test pools_v45` | every row of the tables | green | PASS | (CI) | 2026-11-07 |
| L2: the app's pool commands, the shared limit, broad Leads (D-92) | `cargo test -p juggler-app --test pools --test pools_l2 --test pools_shared_limit`; `a_broad_member_is_never_lead`; `v49_moves_a_broad_lead_to_member_and_keeps_a_sessions` | green | green | PASS | 8e2fc687e | 2026-11-07 |
| L2 on the replica | [l2-replica](l2-replica-2026-11-07.md) | as recorded | as recorded | PASS | — | 2026-11-07 |
| P1 Writer and reviewer | [P1](../../0.4-walks/P1.md) | each step's expect | steps 1–5 live (7b). D-92 found at step 7 (a broad Lead) and fixed. Step 6 walked on a lab with a seeded limit: the notice, the offer and the reviewer pass. The switch check's pool line is re-walked on rc2 | PASS (step 6's switch check on rc2) | 8e2fc687e, ea0a143bc | 2026-11-07 |
| P2 Feature team | [P2](../../0.4-walks/P2.md) | each step's expect | all six walked live. Steps 2 and 3 failed and were fixed: the sender told (2674e6919), the line's own rule on the label (2674e6919, 0c213f042). Step 2 re-walked on rc1: PASS. Step 3 re-walked on rc2 | PASS (step 3 on rc2) | b69cac079 | 2026-11-07 |
| P3–P6, P9, P10 | their sheets | each step's expect | walked on labs 2026-11-05, findings fixed and re-walked there | PASS | (sheets) | 2026-11-05 |
| P7 Sealed client project | [P7](../../0.4-walks/P7.md) | step 3: Claude's own message refused | failed live: Claude Code's own SendMessage reached a sealed session. Fixed: the seal is its own launch step, with the agent tools off too (57f89e1bb, a3903836e); [the pools lab](../../pools-lab.md). The rc1 re-walk joined the session by hand after its start, so the seal applied only from its next start. Re-walked on rc2 with the App as a member | PASS (rc2, live) | a3903836e | 2026-11-07 |
| P8 Scouts | [P8](../../0.4-walks/P8.md) | steps 1–2 live | set-up walked; steps 1–2 need scouts that run (signed-in agents). Step 3's OpenCode half waits on the parked OpenCode adapter (E1602) | PENDING (live) | — | 2026-11-05 |
| P11 Overlap and deletion | [P11](../../0.4-walks/P11.md) | step 1 live | L1 holds one limit per session against the reference model; walked live in P2 | PASS (L1 + P2) | — | 2026-11-07 |
| E3002: which rules can be enforced | [pools-lab.md](../../pools-lab.md) | yes or no per rule, with evidence | 1 yes (live); 2 yes from the launch (live on rc2); 3 not run (OpenCode parked, no Grok) | PARTIAL | d45c7b3a5 | 2026-11-07 |
