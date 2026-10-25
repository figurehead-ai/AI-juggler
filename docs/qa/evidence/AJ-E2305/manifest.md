# Evidence manifest — AJ-E2305

Task: `docs/tasks/E23-session-runtime.md` AJ-E2305, Switching and hand-over across runtimes. Branch `e23-runtime`. Status: **DONE on fakes**: a switch releases a driven session first, and its thread is found and resumed in either runtime.

## What exists now

`features/runtime/mod.rs` (`release_for_switch`), `commands.rs` (freeze source); `crates/juggler-adapters/src/fake_driven.rs`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 released at a turn boundary before a switch | `cargo test -p juggler-app --test runtime` (`switching_away_from_a_driven_session_releases_it_first`) | released, then switched | pass | PASS | e5d29514 | 2026-10-25 | agent |
| Req.2–3 found and resumed, both runtimes | `cargo test -p juggler-adapters --test fake_driven` (`a_driven_conversation_is_found_and_resumed_in_either_runtime`) | one conversation | pass | PASS | e5d29514 | 2026-10-25 | agent |
| Existing switching suites unchanged | `cargo test -p juggler-app --test switch_setup --test fleet_switch` | pass | pass at e5d29514 | PASS | e5d29514 | 2026-10-25 | agent |
| Live A→B→Codex→A | the switching lab (AJ-E1539), after AJ-E2301 | history, tasks, context kept | not run | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
