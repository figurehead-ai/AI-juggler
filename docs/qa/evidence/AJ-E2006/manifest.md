# Evidence manifest — AJ-E2006

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2006, hostile
files: restore and import. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Restore staging (`crates/juggler-app/src/backup.rs`): the manifest read at most 8 MiB (`manifest_of`); its claims capped at 200 000 files, 64 GiB a file, 256 GiB in all (`limits`); every entry must be listed in the manifest and is read at most one byte past its declared size ("larger than its manifest says"); staging files are `create_new` ("in it twice"). The staged database is checked before use (`juggler_core::storage::foreign::vet`): opened read-only in defensive mode with `trusted_schema` off, `integrity_check`, the manifest's schema version, then a copy migrated and compared with a fresh database (tables and columns, indexes, triggers and views word for word). Every AI Juggler connection is now opened hardened the same way (`storage::harden`). Cloud restore (`vault.rs::open_backup`) streams entries and caps the unpacked size and count with the same limits. Tests: `tests/backup_restore.rs::hostile_archives_are_refused_before_anything_in_them_runs`, `storage::foreign::tests` (including `a_backup_from_every_older_schema_passes`, every schema version 1…current). |
| R2 | PASS | Import (`crates/juggler-adapters/src/claude_environment.rs`, Claude and Codex share it): a link resolving outside the folder (file or folder) is never followed, copied or made in the copy; it is listed with where it pointed (`ImportOutcome::left_outside`). Copied files other than the sign-in files go to the redaction scan (`commands.rs::secrets_in`, the handover redaction engine, files up to 4 MiB). Both are told once as an "Imports" notice (`notifications/sources.rs::import_notice`). Tests: `tests/claude_environment.rs::links_inside_stay_links_and_links_outside_are_left_out_and_listed`, `tests/codex_environment.rs::links_follow_the_shared_rule_and_the_sign_in_stays_back`. |
| R3 | PASS | Fixtures: hostile archives (larger than declared, unlisted, twice, oversized manifest, a planted trigger with a re-signed manifest, `..` escape) in `tests/backup_restore.rs`; symlink escapes (absolute, relative GNU-Stow style, `..`, a file, a dangling link) in both environment tests. |

**Changed behaviour, said in the release notes:** before 0.4.1 a linked
folder pointing outside the setup (e.g. `skills/` → dotfiles) was kept as
a link and a linked file was copied by content (0.0.7 dogfood). Under
AJ-E2006 both are left out and listed in the import's notice, so the user
can link them again: a link out of the folder could hand an agent any
file on the Mac.
