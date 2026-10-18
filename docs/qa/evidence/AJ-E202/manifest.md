# Evidence manifest — AJ-E202

Task: `docs/tasks/E2-switching-engine.md` AJ-E202.
Branch: `stage2/e2-foundation-graph-pack-fidelity`. 2026-10-16,
reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Pack = exactly PACK.md, conversation.md, state.json, manifest.json, diffs/, rules/; readers ignore unknown fields | Req.1 | write_pack lays out exactly this tree (+ diffs/EMPTY marker); unknown-field tolerance test injects `future_field` and parses cleanly | PASS | 2026-10-16 | zcode-primary |
| state.json requires the full Req.2 field set; merges list two parents | Req.2 | PackState carries format_version/pack_id/created_at/work_id/edge_id/opt + source{6}/target_hint/project/git{5}/conversation{6}/artifacts{rules,mcp,skills,todos,drops}/fidelity{level,score,caps}/quota_at_creation/lineage{parent_pack_ids,parent_edge_ids}/redactions (reserved []); lineage is a Vec — two parents representable | PASS | 2026-10-16 | zcode-primary |
| Every Drop appears in artifacts.drops AND manifest rows; credentials never written | Req.3 | manifest test: drops/session-todos row with disposition=drop; credential guard (private-key/api-key/bearer patterns) runs BEFORE any write — poisoned-input test: write refused, prior pack byte-intact | PASS | 2026-10-16 | zcode-primary |
| Deterministic from IR: same input → byte-identical state/manifest | Req.4 | determinism test (double generation byte-equal) + committed goldens (state.json, manifest.json, PACK.md, conversation.md) — golden drift fails CI | PASS | 2026-10-16 | zcode-primary |
| Native-resume/transplant edges generate manifest-only packs | Req.5 | generate_pack is pure over input — an empty-turns input produces the manifest-only pack (uuid-less test covers the empty-conversation shape); E205 wires the always-generate rule into the pipeline | PASS | 2026-10-16 | zcode-primary |
| Atomic writes: temp dir → sha256 verify → rename; crash leaves old-or-none | Req.6 | write_pack: temp sibling, per-row sha256 verify BEFORE rename, verify failure removes temp (tested via round-trip + no-leftover-temp assertion); no in-place mutation path exists | PASS | 2026-10-16 | zcode-primary |
| PACK.md is human-readable What travels / What stays; no tier codes | UI/UX | golden PACK.md renders the carry-list in words; test greps: no T1/T2/T4 codes; "session todos" Drop appears under What stays | PASS | 2026-10-16 | zcode-primary |
| Visual: PACK.md rendered light+dark 1280×800 | task visual spec | DEFERRED to AJ-E206 — the wizard's "What's carried" step (its own visual spec includes the pack preview shot) hosts this exact rendering; content conformance proven here by goldens + tests, not silently omitted | DEFERRED | 2026-10-16 | zcode-primary |
| Live export dogfood (real Claude session, PACK.md readable unaided) | live data | PENDING the authenticated session (gate-E5 blocker, same as E6 dogfoods); fixture goldens cover the format meanwhile | PENDING | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-handover/src/manifest.rs` (new) — sha256_hex,
  ManifestRow, the always-on credential guard (AJ-E208 extends with
  the full matrix + user policy).
- `crates/juggler-handover/src/pack.rs` (new) — PackInput IR + all
  state.json blocks, generate_pack (pure), PACK.md /
  conversation.md renderers (Full + Distilled), write_pack (atomic +
  guard), PackWriteError.
- `crates/juggler-handover/tests/pack_v1.rs` (new, 9) +
  `tests/goldens/` (4 committed golden files).
- Cargo: handover gains serde/serde_json/sha2/thiserror (+tempfile
  dev).
