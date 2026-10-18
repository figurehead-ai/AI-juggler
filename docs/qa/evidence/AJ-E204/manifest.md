# Evidence manifest — AJ-E204

Task: `docs/tasks/E2-switching-engine.md` AJ-E204.
Branch: `stage2/e2-e204-transplant`. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| `handover.rs` in juggler-adapter with SessionBundle/BundleManifest/optional sub-trait HandoverSupport{bundle_session, install_bundle, prime_prompt}; HarnessAdapter unchanged; fake opts in incrementally | Req.1 shape | trait is a supertrait-scoped sub-trait (`HandoverSupport: HarnessAdapter`); HarnessAdapter untouched (semver); registry gained an OPTIONAL `register_handover`/`handover` slot — adapters without it are replay/pack only; InstallOutcome extends the sketch with the H-3-mandated fallback flag + fork notice (documented) | PASS | 2026-10-16 | zcode-primary |
| Source resolved via transcript_ref, never newest-mtime; target dir discovered by scanning target projects/* matching embedded cwd; fallback flagged | Req.2 | session_handover_impl refuses a missing ref with the S-F1 copy; `discover_project_dir` scans + matches embedded cwd (H-F6 test: dots/dashes/spaces/>200-char cwd round-trips via discovery; empty target ⇒ mangle fallback FLAGGED `target_dir_fallback`) | PASS | 2026-10-16 | zcode-primary |
| Full `<session>*` sibling set copied, manifest row per file; single-file copy unreachable | Req.3 | bundle_session builds from `sibling_paths` — no one-file API exists; H-F1 test: main + 2 subagents + spill = 4 files bundled; TransplantOutcome carries per-file hashes | PASS | 2026-10-16 | zcode-primary |
| ID collision forks: new native id + rewritten sessionId chain, parentUuid tree preserved, dedupe first-wins by uuid; source hash-identical | Req.4 | H-F3 test: fork id contains `fork-`, sessionId rewritten on every line, `parentUuid` tree intact, uuid_set preserved (the dedupe key); source-dir hashes asserted equal before/after in H-F1/F3/H-F6 + golden_path | PASS | 2026-10-16 | zcode-primary |
| TRANSPLANT capability off until E210; interim UI label "F1 Transcript-only (experimental)" | Req.5 | bit removed from claude_code.rs + fake.rs advertised sets (grep clean — only the bitflags definition remains); test `transplant_capability_is_off_until_e210` pins it; handover section renders the interim label + honest button title | PASS | 2026-10-16 | zcode-primary |
| session_handover_impl shrinks to parse-args → engine → emit; tier logic in juggler-handover | Req.6 | commands.rs: resolve refs/accounts → `juggler_handover::transplant::hand_over` → launch/spawn/records; newest-mtime + `replace('/','-')` paths DELETED (unreachable, not avoided); transplant.rs owns bundle/install orchestration + source-hash invariants | PASS | 2026-10-16 | zcode-primary |
| H-F1/H-F2/H-F3/H-F6 quality gates; source-hash invariant in every test | quality gate | 6/6 claude_handover.rs (sibling set, torn-tail refusal, collision fork, pathological cwd + fallback flag, capability grep, compact-boundary signal incl. the real `system/compact_boundary` subtype shape from session-malformed.jsonl:6 — parser now captures subtypes) | PASS | 2026-10-16 | zcode-primary |
| Live proof (two real Claude accounts, subagent-heavy repo, pre-switch-only question) + transcript sha256 before/after | live data | PENDING the authenticated session (gate-E5 blocker, same ledger as E6 dogfoods); the fixture suites + golden_path cover the mechanics meanwhile; TRANSPLANT stays off regardless until E210 | PENDING | 2026-10-16 | zcode-primary |
| Visual: wizard carry-list, collision-fork notice, experimental badge | task visual spec | DEFERRED to AJ-E206 (the wizard owns the carry-list surface; its spec includes these shots). The experimental label shipped NOW on the existing handover section (capture re-run green) | DEFERRED | 2026-10-16 | zcode-primary |
| Performance: 50 MB bundle+install ≤3s, streamed copy | performance budget | bundle reads the sibling set into memory per file (bounded by real session sets; no whole-set Vec concatenation); install stages per-file + atomic renames; parse-verify tail is one JSON parse. The 50 MB budget is exercised at E205's pipeline fault-injection (which owns timing) | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-adapter/src/handover.rs` (new) — shapes, InstallOutcome,
  HandoverSupport sub-trait, pure helpers (sibling_paths, tail_parses,
  embedded_cwd, fork_session_id, uuid_set, mangled_dir, sha256_hex) +
  unit tests.
- `crates/juggler-adapter/src/registry.rs` — optional handover slot
  (SharedHandover).
- `crates/juggler-adapters/src/claude_handover.rs` (new) — the Claude
  impl: full-sibling bundle with torn-tail refusal + F1-unvalidated
  manifest, discover-don't-compute install with flagged fallback,
  collision fork with staged atomic writes, deterministic T3 prime;
  bundle_is_compacted.
- `crates/juggler-adapters/src/claude_transcript.rs` — subtype capture
  (the real compact_boundary shape) + honest has_compact_boundary.
- `crates/juggler-handover/src/transplant.rs` (new) — hand_over
  orchestration, TransplantOutcome (source+installed hashes),
  TransplantError with failure-taxonomy ids.
- `crates/juggler-app/src/commands.rs` — session_handover_impl shrunk
  (engine call); `crates/juggler-app/src/state.rs` — default_registry
  registers the handover side.
- `ui/src/components/RightInspector.tsx` — interim experimental label.
- Tests: claude_handover.rs (6), golden_path updated (transcript_ref
  seeding + source-hash assertion); visual 280/280 re-captured.
