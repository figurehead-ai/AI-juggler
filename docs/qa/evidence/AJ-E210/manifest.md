# Evidence manifest — AJ-E210

Task: `docs/tasks/E2-switching-engine.md` AJ-E210.
Branch: `stage2/e2-e210-validation-matrix`. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| CI fixtures H-F1..H-F8; every test asserts source bytes hash-identical afterwards | Req.1 | H-F1/H-F2/H-F3/H-F6 landed with E204 and stay green; E210 adds H-F4 (`hf4_compacted_source_labels_f2_and_bytes_stay_identical` — bundle labels F2 with `compact-boundary` + `unvalidated-pair` caps, wired in `bundle_session`), H-F5 stays in `claude_transcript` (malformed-tail degrade counters, never panic), H-F7 (`hf7_golden_path_is_pty_independent_end_to_end` — piped bundle→install→parse, zero PTY; CI's `verify` job IS the three-OS proof: build-linux + build-windows + build-macos all run `cargo test --workspace`), H-F8 (`hf8_a_to_b_to_a_chain_preserves_tree_with_no_duplicate_uuids` — A→B→B+1 turn→return fork; u1/u2/b1 each EXACTLY once; both originals byte-identical, the return's fork files additive). Every test compares `dir_hashes` before/after (H-F8 compares the original-name subset — the return legitimately ADDS fork files) | PASS | 2026-10-16 | zcode-primary |
| The matrix covers every ordered pair of pass2/02 §10 with per-pair fixture tests, one live dogfood cell slot, and an advertised ceiling | Req.2 | `juggler_handover::validation::validation_matrix()` enumerates all 11 rows (3 same-Provider + 6 cross + 2 pack-only); `validation_matrix` integration test (handover) runs the MECHANISM layer for EVERY pair: a fixture pack built over the pair's E203 disposition table survives an IR value round-trip + deterministic serialization, the distilled render keeps only the tail with earlier turns NAMED, and every Drop is a manifest row with its reason; per-pair live-cell slots exist (PENDING until humans run them); advertised ceiling = researched, gated per Req.4 | PASS | 2026-10-16 | zcode-primary |
| ZCode→ZCode transplant stays blocked/F1 until a SQLite clone passes two schema versions; ZCode target prime stays F0; unproven cells RECORDED as unvalidated, never skipped | Req.3 | The z2z row carries the BLOCKED note verbatim (SQLite clone, two-schema test pending); `claude-code→zcode` and `codex→zcode` research F0 with the prime note; the matrix test asserts every non-CI-green row has non-empty notes — "record, never skip" is enforced by test; nothing validates without BOTH real CI + a live pass | PASS | 2026-10-16 | zcode-primary |
| A pair advertises its E203 label only when CI-green AND a live dogfood cell passed; a failing cell lowers the ceiling one level until fixed; TRANSPLANT follows the same rule; no edge without a user click; UI never says lossless below F4 | Req.4 | `PairCell::validated()` = `ci_green && dogfood.Pass`; `advertised_ceiling()` lowers one level per FAILED cell (`FidelityLabel::lowered`, floor F0); unit tests 7/7 + matrix tests 3/3 pin: pass+real-CI→"F1 · Validated", pass+fake-only→stays Beta, fail→"F0 · Beta", later pass recovers; `advertised_badge`/`advertised_ceiling_number` now drive `switch_targets` badges AND the ranking fidelity slot (one source); the TRANSPLANT capability bit stays OFF (`transplant_capability_is_off_until_e210` — no row has both gates green); guardrail suites untouched-green; copy lint (E209) still bars "lossless" | PASS | 2026-10-16 | zcode-primary |
| Live dogfood per pair on real binaries across project shapes; pass = continuity answerable only from pre-switch context; results per cell | Req.5 | The RECORD path ships: `validation_dogfood_record` (human-run cells: verdict pass/fail, ISO date, project shape, before/after sha256, note) persisted in KV (`dogfood_cells`), restart-pinned (integration reopens the data dir and the fail survives), unknown pairs REFUSED; badge flow proven end-to-end (`switch_target_badges_derive_from_the_recorded_cells`: F1·Beta → record pass → F1·Validated → record fail → F0·Beta). The live RUNS themselves need authenticated vendor binaries + a human (never automated under the security constraints) — every cell PENDING in `docs/qa/gate-E2-validation.md` open-beta-debt list | PASS (mechanism) / PENDING (live cells) | 2026-10-16 | zcode-primary |
| Beta-exit per pair: matrix row green + dogfood green + S-F taxonomy evidence + D-04 debt closed; the gate report lists unvalidated pairs as open debt, never as passed | Req.6 | `docs/qa/gate-E2-validation.md` is GENERATED from the matrix (`just e2-gate` → example bin; EPICS M4-7 links it): 0/11 validated, 11 open debt rows with reasons, S-F1..S-F9 evidence section pointing at the E205-E208 suites; the report cannot drift from code because it IS code | PASS | 2026-10-16 | zcode-primary |
| Settings → Beta shows the matrix users' badges derive from; unvalidated reads "Beta — not yet validated" everywhere; copy rule "available in beta" never "lossless" | UI reasoning | `BetaMatrix` component (Profiles — today's settings home): a REAL table (row `th scope=row`, scoped columns), summary line (N validated · M open debt), ceiling-lowering disclosure, live-cell pass/fail + date, verbatim notes; disclosure-only (no record interaction — cells are maintainer-recorded); vitest 4/4 incl. the honest default and the copy rule; the wizard's in-situ badge captured in gallery 33 | PASS | 2026-10-16 | zcode-primary |
| Full fixture matrix in CI ≤ 15 min; single-pair re-run ≤ 3 min; fixture corpus ≤ 5 MB/pair; canary re-verification monthly without blocking PR CI | performance budget | The matrix + all H-F tests run inside the existing workspace suite (the `verify` CI job runs them on all three OSes within its 13-14 min budget — no separate job); fixtures are generated in tempdirs, the committed corpus is unchanged (0 bytes added); per-pair mechanism fixtures are µs-scale pack generations; nothing blocks PR CI on live cells (D-04: label graduation only) | PASS | 2026-10-16 | zcode-primary |
| `just verify` + guardrail suite + coverage ≥85% (handover/adapters) green | quality gates | `just verify` green (below); guardrails green (untouched suites in the workspace run); coverage measured with cargo-llvm-cov: **juggler-handover libs 92.7%**, **combined handover+adapters libs 85.9%** (≥85 ✓); **juggler-adapters libs 77.9%** — the gap is concentrated in `claude_code.rs` (60.6%: launch/env/PTY paths that need real vendor binaries — E808 live-validation + E707 adapter territory, pre-existing profile, not an E210 regression; E210's own additions: `validation.rs` 97.5%, `claude_handover.rs` 77.8%, H-F tests green). Recorded, not padded — no assertion was loosened to move a number | PARTIAL (adapters libs 77.9% recorded with owner) | 2026-10-16 | zcode-primary |
| H-F7 golden path runs without PTY on all three OSes | quality gate | The golden path is pure file/process plumbing (no PTY anywhere in bundle/install); CI's build-linux + build-windows + build-macos jobs all execute the workspace suite containing `hf7_*` — three-OS green is the PR's five-check record | PASS | 2026-10-16 | zcode-primary |
| Visual: Settings → Beta matrix (mixed validated/unvalidated), one unvalidated badge in situ, gate-report summary; a11y real table, status in text, WCAG AA | visual testing | Gallery `33-beta` 16 captures (matrix-default all-debt, matrix-failed lowered-ceiling disclosure, matrix-validated mixed, badge-in-situ on the wizard; 1280×800 + 800×600 × light+dark); a11y 2 themes: real-table semantics asserted (scope=row/col) + text statuses + scoped axe scans 0 critical/serious after fixing TWO real contrast misses caught by the new scan (column headers + row-header `<th>` inheriting no theme color); full playwright 404/404 (was 386) | PASS | 2026-10-16 | zcode-primary |

## Recorded load flake (AJ-E806 protocol)

- `juggler-supervisor --test reconciler::fifty_instance_soak_and_construction_regression`
  failed ONCE inside the full workspace verify under external load 4.4-5.6
  (p95 tick 101 ms vs the 100 ms wall-clock budget — a 1 ms miss on a
  machine also compiling the workspace). Verified 3/3 standalone
  immediately after, each comfortably inside budget; no E210 code path
  touches the supervisor/reconciler.
- `juggler-app --test guardrails::foreign_machine_row_is_refused…`
  failed ONCE in the serialized re-run ("local rows verify normally" —
  its helper processes were reaped mid-run under load; the log is full
  of `kill: No such process` from sibling tests' helpers). Verified 3/3
  standalone immediately after. Same class as the E201-recorded
  `guardrails::term_ignorer` and E208-recorded `recovery::crash_drill`
  flakes — CI (quiet runners) is the arbiter; if either fails CI it
  gets its own fix/quarantine record rather than a gate skip.

## Test counts

- `cargo test -p juggler-handover`: 75/75 (lib 37 incl. `validation` 7 new;
  integration 38 incl. NEW `validation_matrix` 3/3; pack_v1 10).
- `cargo test -p juggler-adapters`: 57/57 (claude_handover 9 — H-F4/H-F7/H-F8
  new; suites unchanged otherwise).
- `cargo test -p juggler-app` gates: validation_gate 2/2 (NEW) +
  switch_graph_targets 3/3 re-green under the matrix-driven badges.
- vitest: 249/249 (26 files) incl. NEW `beta-matrix.test.tsx` 4/4.
- Playwright: 404/404 (gallery 33-beta 16 new; a11y +2).
- `just verify` green on this machine; `just e2-gate` regenerates the report.
