# Evidence manifest — AJ-E2026

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2026, durable commits on macOS. Status: **DONE (0.5.2)**.

| Claim / Req ID | Proof | Verdict | Date |
|---|---|---|---|
| R1 checkpoint_fullfsync on every connection | `storage::harden` sets it, and every connection is opened through `open_hardened`/`harden`; test `durable_storage::every_connection_checkpoints_with_full_fsync` (fails without the change: SQLite's default is 0) | PASS | 2026-11-26 |
| R2 critical writes durable, cost measured | `Storage::make_durable` (a PASSIVE checkpoint, fully synced); the memory store's forget (`memory/secure.rs`) and the backup export (`backup.rs`) already checkpoint right after committing, now with F_FULLFSYNC; moves keep their record in files (R3); cost 8.6 ms (`durable_storage::make_durable_checkpoints_and_its_cost_is_small`) | PASS | 2026-11-26 |
| R3 the move engine's files synced | the journal `sync_all` after each line (`juggler-handover/src/moving/mod.rs` `log`); the manifest by `transfer::write_atomic` (`create_new_private` syncs, rename, `sync_dir`); clone backups `sync_dir` after each change; Rust's `File::sync_all` is `F_FULLFSYNC` on Apple | PASS | 2026-11-26 |
| R4 test and docs | `crates/juggler-core/tests/durable_storage.rs`; ADR-0030's amendment | PASS | 2026-11-26 |
