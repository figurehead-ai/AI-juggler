# Evidence manifest — AJ-E1303

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1303 (watch Codex sessions:
rollout parser, linking, identity). Status: **DONE-EVIDENCED** on
fixtures. 2026-10-20, reviewer claude-opus.

## Basis

The rollout schema of codex-cli 0.163.0 (source tag `rust-v0.163.0`:
`protocol/src/protocol.rs`, `rollout/src/policy.rs`,
`rollout/src/rollout_file_name.rs`; summary in
[research/codex.md §10](../../../research/codex.md)). The fixtures
`crates/juggler-adapters/tests/fixtures/codex/rollout-paginated.jsonl` and
`rollout-legacy.jsonl` are synthetic, written to that schema — no real
session was read.

## Requirements

| Requirement | Proof | Result |
|---|---|---|
| Req.1 rollout → `ParsedTranscript`: user/assistant from `response_item`, injected developer/environment turns skipped, model from `turn_context`, tokens from `token_usage_record` only, `compacted` caps fidelity, malformed lines counted, never a panic | `codex_transcript::paginated_rollout_reads_the_conversation_without_injected_context`, `legacy_rollout_parses_and_counts_damage`, `garbage_never_panics` | PASS |
| Req.2 every parse site dispatches: switch preflight, switch-back delta (base ids for both formats), transcript side (redaction preview, packs, back facts), replay-cost floor, cross-machine bundle turns, quality signals (`token_usage_record` turns, `token_count` context, error events), running model (`turn_context`) | `parse_any` by content (`any_transcript_parses_by_its_own_format`: Claude output identical to before); app sites in `commands.rs`, `crossmachine.rs`, `crossmachine/switch_back.rs`; full `juggler-app` suite unchanged | PASS |
| Req.3 watcher over `<home>/sessions`, native id = thread UUID | `the_watcher_reports_thread_ids`; `thread_id_from_name` incl. reverted (`_rev`) and `.jsonl.zst` names | PASS |
| Req.4 a launched session is linked to its rollout | `codex_provider::a_launched_codex_session_is_linked_to_its_rollout_on_first_use` (real `launch_profile`, then the preflight links and persists it); locator ordering `a_launched_codex_session_finds_its_rollout` (older and other-folder files ignored, the first after the launch wins); the same fix for Claude: `a_launched_claude_session_finds_its_transcript_too` | PASS |
| Req.5 identity: `tokens.account_id` subject, masked label, no token decoded; API key / no id → Unverifiable; no file → signed out | `identity_is_the_account_id_masked_never_a_token` | PASS |
| Req.6 tokens counted once | the usage pipeline's `rollout_events` reads `token_usage_record` only (unchanged); the parser and the quality scan read the same rows and never `token_count` totals | PASS |

Commands: `cargo test -p juggler-adapters` (all suites; `codex_transcript`
9), `cargo test -p juggler-app --test codex_provider` (5); `just verify`
on the PR.

## Not claimed

- `HarnessAdapter::watcher()` still has no production caller in the app
  (true for Claude too); linking happens on first use instead. Wiring
  live watching into the app is a follow-up, not part of this task.
- Rollouts from a live Codex session were not parsed — AJ-E1305.
