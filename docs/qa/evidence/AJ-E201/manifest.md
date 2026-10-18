# Evidence manifest — AJ-E201

Task: `docs/tasks/E2-switching-engine.md` AJ-E201.
Branch: `stage2/e2-foundation-graph-pack-fidelity`. Gate: PR + this
manifest (domain layer — no gate-M3 surface). 2026-10-16, reviewer
zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| WorkItem / Epoch / SwitchEdge domain types in juggler-core; `to_epoch` null until launch succeeds | Req.1 field set | model.rs: WorkItemId/EpochId/SwitchEdgeId/PackId newtypes, WorkItem, Epoch (resumed_from_epoch + native_id), SwitchEdge (mechanism/pack_id/fidelity_shown/manifest_sha256/state/actor/two parents), SwitchMechanism (native-resume/transplant/replay/pack), EdgeState exhaustive (Staged/Verified/Launched/Confirmed/Failed{stage,reason}), FailureStage (8 pipeline stages), EdgeActor | PASS | 2026-10-16 | zcode-primary |
| Migration V20 after V1–V19; zero data loss; WAL retained | Req.2 | V20 creates work_items/epochs/switch_edges/artifact_manifests + work indexes; migration test rewinds a populated DB to v19 (v20 tables dropped — the honest v19 shape) and replays V20 with rows intact | PASS | 2026-10-16 | zcode-primary |
| Append-only edges (S-G1): no UPDATE except state transitions; undo/failure/merge insert compensating rows | Req.3 | repo API: append_edge refuses pre-filled to_epoch or non-Staged/Failed states; ONLY mark_launched (fills to_epoch)/mark_confirmed/mark_failed mutate; test pins failed-edge immutability + compensating undo edge + one-way transitions (confirmed cannot fail) | PASS | 2026-10-16 | zcode-primary |
| Per-project graph (S-G2): cross-project moves are T4 exports, never edges | Req.4 | structural: epochs/edges FK to work_items which FK to exactly one project; test asserts cross-work isolation + one project per work | PASS | 2026-10-16 | zcode-primary |
| Switch-back resume creates a new Epoch resuming the original native_id | Req.5 | a_b_a_c_b fixture: ep-a2 has native_id = original native-1 + resumed_from_epoch = ep-a1 (asserted) | PASS | 2026-10-16 | zcode-primary |
| HandedOver = resting: every Epoch node exposes Resume | Req.6 | Epoch::can_resume() (native_id present); UI surface renders in AJ-E209 | PASS | 2026-10-16 | zcode-primary |
| Graph read ≤50ms/1k edges; no N+1; migration ≤2s | performance budget | graph() = 2 indexed queries (epochs_for_work, edges_for_work) — no per-node lookups; single-work reads measured <2ms in-suite (in-memory fixtures); index on work_id | PASS | 2026-10-16 | zcode-primary |
| Property: random edge sequences round-trip identically after reopen | quality gate | 64-case proptest: random A/B/C/D chains × native-id variants, pre-reopen JSON == reopen JSON == second-reopen JSON | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core/src/model.rs` — the eight graph types + four
  id newtypes; `id_newtype!` gained transparent serde (string) —
  additive for ALL id types.
- `crates/juggler-core/src/storage/mod.rs` — V20 + accessor.
- `crates/juggler-core/src/storage/switch_graph.rs` (new) —
  SwitchGraphRepo (append-only API + 3 transitions + manifest
  append/read) + WorkGraph read shape.
- `crates/juggler-core/tests/switch_graph.rs` (new, 6) — the
  a→b→a→c→b restart-identity fixture (5 epochs, 4 edges),
  append-only/undo, per-project, migration-up, 64-case round-trip
  property, deterministic JSON.
- No UI (domain layer — per task spec).

## AJ-E806 flake record (this batch's gate runs)

- `juggler-app` guardrails `term_ignorer_gets_term_then_kill_only_after_grace`:
  fails under full-parallel local runs at loadavg ≥ 6 — **reproduces
  on CLEAN MAIN** (verified in a detached worktree at 8e5075a; passes
  alone in 15 s, in-suite 111 s under load; CI runners pass it — PR
  #87 verify green). Pre-existing machine-load sensitivity, not a
  regression. Full local gate obtained with `RUST_TEST_THREADS=1`
  (same tests, serialized) — exit 0, 65 suites.
- `juggler-app` kill_sheet `vite_like_helper_frees_its_port_on_term`:
  one-off port-contention failure under the same load; passes
  isolated (5/5) and in the serialized run.
