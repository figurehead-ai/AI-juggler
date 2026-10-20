# Evidence manifest — AJ-E1304

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1304 (switch with Codex,
beta: transplant and cross-Provider replay). Status: **DONE-EVIDENCED**
on fixtures and stub binaries; every Codex pair stays Beta and every
AJ-E808 register cell stays UNVALIDATED until the live runs of AJ-E1305.
2026-10-20, reviewer claude-opus.

## Basis

Codex 0.163.0 looks a thread up by scanning `sessions/` for its id and
re-registers it in its own database (source tag `rust-v0.163.0`:
`thread-store/src/local/thread_rollout_resolver.rs`, `rollout/src/list.rs`;
[research/codex.md §10](../../../research/codex.md)) — so installing is
placing the rollout under the target's dated folder. That a copied
paginated rollout then resumes is **not** proven here (a live cell).

## Requirements

| Requirement | Proof | Result |
|---|---|---|
| Req.1 `HandoverSupport` for Codex, registered: bundle (torn tail refused, cwd/version/caps), install under `sessions/YYYY/MM/DD/` with its own name, staged then renamed, fork a new UUID on collision (name and every line), source byte-identical | `juggler-adapters` `codex_handover` (6): `bundle_carries_the_rollout_with_honest_caps`, `a_torn_tail_is_refused`, `install_places_it_where_codex_looks_and_forks_on_a_collision`; `default_registry()` registers the handover | PASS |
| Req.1 end to end: Codex→Codex switch installs the rollout in the target home and resumes `codex resume <thread> … --no-daemon`; source `HandedOver`, edge Transplant, source hash unchanged | `juggler-app` `codex_switch::codex_to_codex_transplants_the_rollout_and_resumes_it` | PASS |
| Req.2 a rollout built on another (`history_base` / `forked_from_id`) carries `base-rollout-not-carried` | `a_rollout_built_on_another_says_so` | PASS |
| Req.3 cross-Provider switches replay (never transplant): Claude→Codex and Codex→Claude prime a new session with the conversation, redacted under the policy; edge Replay; nothing installed into the other Provider's home; source hash unchanged | `codex_switch::claude_to_codex_replays_the_redacted_conversation` (a seeded `sk-` key never reaches the target's argv; `•••` does), `codex_to_claude_replays_too` | PASS |
| Req.3 an unset policy, or "Ask each time" with no answer, blocks the replay before anything launches | `codex_switch::a_replay_waits_for_a_secret_policy` | PASS |
| Req.4 switch-back forks and cross-machine bundles use Codex's own id and cwd: `fork_original` rewrites the thread id in the rollout's name and lines; `embedded_cwd` / `remap_cwd` read and rewrite `payload.cwd` (Claude's shape unchanged) | `cwd_helpers_read_and_remap_codex_payloads`; `fork_rollout` covered by the collision test; `juggler-app` crossmachine suites unchanged | PASS |
| Req.5 Beta everywhere: Codex pairs `ci_green` (real-adapter fixtures) but never validated without a live PASS and register PASS; register cells untouched; `docs/qa/gate-E2-validation.md` regenerated (Codex rows PENDING) | `juggler-handover` `validation_matrix`, unit test `codex` row stays "F1 · Beta"; `crates/juggler-handover/data/fidelity-register.json` unchanged | PASS |

## Review (2026-10-20)

| Finding | Fix and proof | Result |
|---|---|---|
| HIGH: a paired peer's bundle could name a path (`../../.zshrc-x`, an absolute path) or a non-UUID id | `install_bundle` accepts only a UUID thread id and a one-component `rollout-…-<id>[_rev].jsonl`, refused before any write — `a_bundle_from_a_peer_can_never_name_a_path` (7 malicious name/id pairs, nothing written) | PASS |
| MEDIUM: a replay could launch with no conversation (an oversized newest turn dropped everything; an unreadable transcript became zero turns) | the newest turn always travels, cut on a character boundary (the limit counts characters); an unreadable transcript or zero turns refuses before anything launches — `an_oversized_newest_turn_still_travels_cut_to_fit` (multi-byte text), `a_session_with_no_conversation_is_not_replayed` | PASS |
| MEDIUM: a fork could overwrite (a name without its id made the "new" name the original's; `rename` replaced an existing rollout; `.jsonl.zst` threads were not seen) | switch-back refuses a name without its id and writes with `create_new`; installs link into place with `hard_link` (never replaces); collision checks count `.jsonl.zst` — `fork_tests::*` (2), `a_compressed_rollout_of_the_thread_counts_as_present` | PASS |
| LOW: a turn containing "[user]: …" could forge a turn in the prime | `TurnText::quoted()` (from #71) quotes every turn line (`> …`) in the Codex, Claude and plain primes — `a_turn_cannot_forge_another_turn` | PASS |

Commands: `cargo test -p juggler-adapters --test codex_handover` (6),
`cargo test -p juggler-app --test codex_switch` (4),
`cargo test -p juggler-handover`; `just e2-gate`; `just verify` on the PR.

## Not claimed

- No switch ran on real Codex binaries or accounts — AJ-E1305 (the
  maintainer's live cells fill the AJ-E808 register).
- A compressed (`.jsonl.zst`) rollout is refused with a plain reason
  ("resume it once on its own account first"), not decompressed.
