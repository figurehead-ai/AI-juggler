# Evidence manifest — AJ-E2402 Sightings of other apps' sessions' processes (D-41a)

Task: `docs/tasks/E24-processes.md` AJ-E2402 (no amendments). Re-audited
against the code and tests on `release/0.4.1`, 2026-11-07, replacing the
2026-10-25/26 manifest from branch `e24-processes`. The store is
`crates/juggler-core/src/storage/sightings.rs` (schema v35, module-owned
`MIGRATION`); the in-memory side is
`crates/juggler-app/src/features/processes/sightings.rs` (`Memory::note`,
`cmd_hash`, `salt`), driven from `processes/mod.rs` `tick`. Both shipped in
Release 0.2.0 (f6fb096cb).

Cargo tests below are cited as written and **run in CI**; they were not run
for this audit (shared Mac). No vitest file is cited for this task (it has no
UI of its own; its rows are AJ-E2404's), so no vitest command was run for it.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a sighting holds pid, start time, boot, program name, session and tool, the app, first/last seen, and a keyed hash of its command line (SHA-256 over the install's own random key); never the environment or the line | `cargo test -p juggler-core --test sightings_storage a_sighting_never_holds_the_environment_or_the_command_line` (columns == `SIGHTING_COLUMNS`; no column contains env/cmdline/args/argv/cwd/command); `cargo test -p juggler-app --lib features::processes::sightings::tests::the_hash_is_keyed_and_never_the_line` (two keys → two hashes; 32 hex; no word of the line); code: `sightings::cmd_hash` = SHA-256(key ‖ 0 ‖ line), first 16 bytes as hex; `sightings::salt` makes 32 random bytes (`getrandom`) once, kept in app setting `process_hash_salt` | every field; nothing else; keyed | fields and hash as asserted. **Missing:** no test of `sightings::salt` (`crates/juggler-app/src/features/processes/sightings.rs`): that a fresh store gets ≥ 16 random bytes, the same key on the second call, and a different key in another fresh store **Closed 2026-11-07:** `--lib features::processes::sightings::tests::the_key_is_random_per_install_and_kept` (a fresh store gets ≥ 16 random bytes, the same key on the second call, another store another key) | PASS | 139571792 | 2026-11-07 | Claude (self) |
| R2 kept 7 days after last seen; written once when first seen, then at most once a minute; a process alive < 10 s never sighted | `cargo test -p juggler-core --test sightings_storage a_sighting_is_forgotten_seven_days_after_it_was_last_seen` (`SIGHTING_KEEP_MS == 7 days`; the 8-day one pruned, the 6-day one kept), `seeing_a_process_again_moves_only_its_last_seen`; `cargo test -p juggler-app --lib features::processes::sightings::tests::a_sighting_waits_for_its_own_command_line_and_is_written_once_a_minute` (written at 200 000, not at 229 000, again at 261 000, first seen kept); pruned hourly in `mod.rs` `tick` (`PRUNE_EVERY_MS`) | as asserted | 7 days and once-a-minute asserted. **Missing:** the 10 s rule (`sightings.rs` `Memory::note`, `MIN_AGE_MS` check on a non-root process) has no test; a unit test should note a helper whose `start_time` is < 10 s before `now_ms` and assert no write, then note it again past 10 s and assert one write, and assert the session's root is sighted regardless **Closed 2026-11-07:** `--lib features::processes::sightings::tests::a_helper_younger_than_ten_seconds_is_never_sighted` (the session's own process sighted at once; a 9 s old helper not; the same helper past 10 s, once) | PASS | 139571792 | 2026-11-07 | Claude (self) |
| R3 a sighted helper alive after every sighting of its session's own process ended (and the live view no longer lists the session) is Left behind | `cargo test -p juggler-app --lib features::processes::sightings::tests::a_helper_alive_after_its_session_ended_is_left_behind` (root gone and session not listed → one leftover, `session_id "conv-1"`; a reused pid with another start time → none); `cargo test -p juggler-app --test processes_graph every_state_is_in_the_one_vocabulary` (leftover 601 → `ProcState::LeftBehind`, `StopPath::Observed`); live lab 2026-10-26 step 7 (below) | both conditions required | the positive case passes. **Missing:** the "and the live view no longer lists the session" leg (`sightings.rs` `Memory::note`: `root_alive \|\| live_sessions.contains(&key)` → running) has no negative test; a unit test should end the root but keep `(tool, session)` in `live_sessions` and assert no leftover, then drop it and assert one. Also untested: a zombie root counts as ended (`!live.zombie`) **Closed 2026-11-07:** `--lib features::processes::sightings::tests::a_leftover_waits_until_the_live_view_no_longer_lists_the_session` (own process gone but the session still listed → no leftover; no longer listed → one; a zombie own process counts as ended) | PASS | 139571792 | 2026-11-07 | Claude (self) |
| R4 guardrail tests: columns exactly the reviewed list, none could hold an environment or a line; the 7-day expiry; the hash keyed and never the line | `cargo test -p juggler-core --test sightings_storage a_sighting_never_holds_the_environment_or_the_command_line`, `a_sighting_is_forgotten_seven_days_after_it_was_last_seen`; `cargo test -p juggler-app --lib features::processes::sightings::tests::the_hash_is_keyed_and_never_the_line` | the three guardrails exist and assert | all three exist as named and assert what the requirement says (runs in CI) | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R5 schema v35, module-owned migration (E18 holds v34) | `crates/juggler-core/src/storage/mod.rs`: `const V35: &str = sightings::MIGRATION;` in the 35th slot of the migration list (after `V34`); `cargo test -p juggler-core --test sightings_storage` (a fresh store migrated through every version has `process_sightings` with exactly its columns); `cargo test -p juggler-core --test storage_roundtrip v4_database_upgrades_in_place_to_v5_with_identical_values` (an old database migrates once to `Storage::schema_version()`) | v35, owned by the module | as built; the schema has since moved on (v48), v35 unchanged | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: `cargo test -p juggler-core --test sightings_storage`; `cargo test -p juggler-app --lib features::processes::sightings` | as the task's quality gates | pass | not run here (shared Mac): runs in CI | PENDING (CI) | f6fb096cb | 2026-11-07 | Claude (self) |

## Performance budget (not a numbered requirement)

Writes are batched: `processes/mod.rs` `tick` hands one tick's `noted.writes`
to `SightingsRepo::record`, which writes them in one transaction
(`unchecked_transaction`); memory holds a session's root only while it
explains a leftover (`Memory::note`'s final `retain`). Not measured.

## Live evidence (2026-10-26, `docs/qa/processes-lab.md`, [lab-1-2026-10-26.md](../AJ-E2406/lab-1-2026-10-26.md))

- Step 7, another app's session's leftover named by its session: Claude Code
  under the Claude app → its session → `claude` → `zsh`, `Python` :8766, "In a
  session Claude runs", no Stop while it ran; after its `claude` ended
  (SIGKILL, the crash route) the server was Left behind under the ended session
  in 4 s. The session's title was empty. Backend PASS; the window was BLOCKED
  (no desktop control). Still consistent with today's code; not re-run.
- Step 11, the table holds names and hashes only: `pid, start_time, boot_id,
  exe_name, cmd_hash, tool, session_id, app, session_root, parent_pid,
  first_seen, last_seen`; `length(cmd_hash)` 32. Matches `SIGHTING_COLUMNS` today.

## Visual

The Left behind rows of another app's session:
`_gallery/90-processes/attention-1680x1000-light.png` (opened: "ChatGPT ›
Landing page · Ended", `node (astro)` Left behind :4321, "In a session
ChatGPT runs", Stop…), last re-shot 6a06e7483 (2026-10-31), from Playwright
`ui/tests/visual/processes-gallery.spec.ts` (runs in CI).

## Binary artifacts

None added; cited: `_gallery/90-processes/attention-1680x1000-light.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
