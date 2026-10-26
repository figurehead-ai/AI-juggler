# Evidence manifest — AJ-E2402

Task: `docs/tasks/E24-processes.md` AJ-E2402, Sightings of other apps' sessions' processes (D-41a). Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

`crates/juggler-core/src/storage/sightings.rs` (schema v35, module-owned) and `crates/juggler-app/src/features/processes/sightings.rs` (in memory, written once and at most once a minute, keyed hash).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1/4 no environment or command line; the columns are the reviewed list | `cargo test -p juggler-core --test sightings_storage a_sighting_never_holds_the_environment_or_the_command_line` | exact columns; no banned word | pass | PASS | — | 2026-10-25 | agent |
| Req.2/4 7-day expiry | `cargo test -p juggler-core --test sightings_storage a_sighting_is_forgotten_seven_days_after_it_was_last_seen` | the 8-day one gone | pass | PASS | — | 2026-10-25 | agent |
| Req.1 the hash is keyed and never the line | `cargo test -p juggler-app --lib processes::sightings::tests::the_hash_is_keyed_and_never_the_line` | two keys → two hashes; no word of the line | pass | PASS | — | 2026-10-25 | agent |
| Req.2 written once, then once a minute; waits for its own command line | `cargo test -p juggler-app --lib processes::sightings::tests::a_sighting_waits_for_its_own_command_line_and_is_written_once_a_minute` | as asserted | pass | PASS | — | 2026-10-25 | agent |
| Req.3 a helper alive after its session ended is left behind; a reused pid never | `cargo test -p juggler-app --lib processes::sightings::tests::a_helper_alive_after_its_session_ended_is_left_behind` | as asserted | pass | PASS | — | 2026-10-25 | agent |
| Req.5 v35 migrates once, after E18's v34 | `cargo test -p juggler-core --test busy_timeout --test storage_roundtrip` | pass | pass | PASS | — | 2026-10-25 | agent |
| Live: another app's session's leftover named by its session | `docs/qa/processes-lab.md` step 7 | as written there | not run yet | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
