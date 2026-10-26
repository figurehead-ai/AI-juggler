# Evidence manifest — AJ-E2405

Task: `docs/tasks/E24-processes.md` AJ-E2405, Automatic rules shown, previewed and told. Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

`crates/juggler-app/src/features/processes/rules.rs` (dry run, policy), `ended.rs` (snapshot), the audit's page query, the Keep check in the sweep, and the `left_behind` notification kind.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.2 the dry run uses the sweep's own rule and exclusions | `cargo test -p juggler-app --lib processes::rules` | now/later/never as asserted; a crash needs tier 2 | pass | PASS | — | 2026-10-25 | agent |
| Req.3 the sweep never tidies a kept process | `cargo test -p juggler-app --test processes_live` (Keep step) | kept counted out; is_kept | pass | PASS | — | 2026-10-25 | agent |
| Req.4 told once, off by default, the count in words | `cargo test -p juggler-app --test system_notifications a_session_that_left_processes_running_is_told_once` | "Left 3 processes running · Personal" | pass | PASS | — | 2026-10-25 | agent |
| Req.5 history by session, paged | `cargo test -p juggler-app --test processes_live` (history step) | the stop listed for the session | pass | PASS | — | 2026-10-25 | agent |
| Req.6 the snapshot holds names and hashes only | review of `ended.rs::snapshot_json` (no title, folder or line) | no line | by reading | UNPROVEN | — | 2026-10-25 | agent |
| Visual: the rules panel with a dry run; history filtered | AJ-E2404 gallery | looked at | not built yet | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
