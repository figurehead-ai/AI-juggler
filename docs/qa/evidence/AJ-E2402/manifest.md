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
| Live: another app's session's leftover named by its session | `docs/qa/processes-lab.md` step 7, 2026-10-26 ([lab-1-2026-10-26.md](docs/qa/evidence/AJ-E2406/lab-1-2026-10-26.md)) | as written there | Claude Code outside AI Juggler (under the Claude app): Claude → its session → `claude` → `zsh`, `Python` :8766, "In a session Claude runs", no Stop while it ran. After its `claude` ended (SIGKILL, the crash route; `/exit` stops Claude's own background tasks), the server was Left behind under the ended session in 4 s. The session's title was empty | PASS (backend; the window: BLOCKED) | — | 2026-10-26 | agent |
| Live: the sightings hold no command line, folder or environment | the same lab, step 11 (`pragma table_info(process_sightings)`) | names and 32-character hashes only | `pid, start_time, boot_id, exe_name, cmd_hash, tool, session_id, app, session_root, parent_pid, first_seen, last_seen`; `length(cmd_hash)` 32 | PASS | — | 2026-10-26 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
