# Evidence manifest — AJ-E203

Task: `docs/tasks/E2-switching-engine.md` AJ-E203.
Branch: `stage2/e2-foundation-graph-pack-fidelity`. 2026-10-16,
reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Artifact inventory + dispositions Copy=1.0/Translate=0.8(0.6 key-dropped)/Regenerate=0.5/Drop=0; weights chat .35 rules .15 git .15 todos .10 MCP .10 skills .05 env .05 subagents .05 | Req.1 | WEIGHTS constants (sum-to-one test); Disposition::score() exact; all eight artifacts scored in every cell | PASS | 2026-10-16 | zcode-primary |
| Per ORDERED pair; no direction-blind aggregation | Req.2 | score_pair takes from/to; pair strings + labels differ by direction (test: claude→zcode F0-target vs zcode→claude F1); full_matrix has 12 ordered cells incl. both directions | PASS | 2026-10-16 | zcode-primary |
| Caps: chat/rules/git Drop → F1; compact_boundary → F2; unvalidated pair → F1 | Req.3 | caps_lower_independently: each cap asserted alone lowers the label; caps recorded in order (AppliedCap feeds state.json caps_applied) | PASS | 2026-10-16 | zcode-primary |
| Label mapping ≥.95/.75-.94/.45-.74/<.45 + F4 only for same-account native resume; badge ships with carry-list | Req.4 | band mapping in score_pair; f4_requires_same_account_native_resume test (label + exact "Continues exactly" copy); PairFidelity always carries carry_list (one row per artifact) | PASS | 2026-10-16 | zcode-primary |
| Translate without written mapping = Drop, unmapped keys named | Req.5 | untranslated:true scores 0.0, carry word "stays (no written mapping)", reason lists keys by name (test: desktop-commander, playwright) | PASS | 2026-10-16 | zcode-primary |
| Todos default Drop; zcode targets cap F1-source/F0-target | Req.6 | todos-default test over the whole matrix; zcode-target cap test; base table zcode→zcode chat = Drop (SQLite clone unproven, SPEC:282) | PASS | 2026-10-16 | zcode-primary |
| Worked examples: proper Claude A→B ≈.9→F3; v0 single-file capped F1 | evidence proof | both tests: .90 → F3 validated / F1 unvalidated; siblings-Drop .85 same caps | PASS | 2026-10-16 | zcode-primary |
| Golden snapshot of the full matrix committed; every cell honest | quality gate | tests/goldens/fidelity-matrix.json (12 ordered pairs, byte-stable, UPDATE_GOLDENS regenerates); snapshot test also asserts every cell ≤ F1 today (D-04: nothing advertises above its validated ceiling) | PASS | 2026-10-16 | zcode-primary |
| Matrix compute ≤20ms, pure, no I/O | performance budget | score_pair is pure arithmetic over 8 weighted entries + ≤5 cap checks; full_matrix (12 cells) runs in <1ms in-suite | PASS | 2026-10-16 | zcode-primary |
| Visual: badge+carry-list F0–F4 renders | task visual spec | DEFERRED to AJ-E209 (the badge UI surface — its spec covers the F0–F4 badge set renders); the label + carry-list data contract is proven here | DEFERRED | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-handover/src/fidelity.rs` (new) — ArtifactKind,
  Disposition, WEIGHTS, FidelityLabel (F0–F4 + verbatim user
  labels), AppliedCap/CarryRow, FidelityInput + PairFlags,
  score_pair (mean → band → monotone caps), base_dispositions
  (§3.2–§3.3 tables, validated=false everywhere), full_matrix.
- `crates/juggler-adapters/src/claude_transcript.rs` —
  ParsedTranscript::has_compact_boundary() (the F2 cap's source
  signal).
- `crates/juggler-handover/tests/fidelity_matrix.rs` (new, 12) +
  `tests/goldens/fidelity-matrix.json`.
