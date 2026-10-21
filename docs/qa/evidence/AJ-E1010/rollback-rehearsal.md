# Rollback rehearsal — AI Juggler Dev, 2026-10-20

AJ-E1010 Req.2: "A rollback path to the previous version SHALL be
documented and rehearsed once per release channel; user data migrations
SHALL be forward-compatible or ship with a downgrade note."

**Result: a downgrade from 0.0.8 to 0.0.7 needs the downgrade note.**
0.0.8 adds database schema 28 (the in-place accounts column). 0.0.7
reads up to schema 27, refuses newer data rather than risk it, and before
this change it only said so on stderr: the app **quit at launch with no
window and no message**. The way back below works, and is now in the
0.0.8 release notes (Known issues) and in `docs/RELEASING.md` § Rollback.

From this change on, an older build that meets newer data says so in a
window and offers the backup the update made (`newer_data.rs`), so later
downgrades no longer depend on reading the notes.

## Setup

- Builds: AI Juggler Dev from tag `v0.0.7`'s source (schema 27) and from
  this branch (schema 28). Debug builds are always the Dev edition (D-23):
  their own data folder, Keychain items and port.
- `HOME` was a throwaway folder for every run: no real data folder,
  Keychain item or `~/.claude` was read or written. Each app was started
  from the shell and stopped after a few seconds (a window opened each
  time).
- 0.0.7's data and its pre-update backup were written by **0.0.7's own
  code** (`AppState::init` + `backup_export_impl`, the code Settings and
  the updater call) through a scratch test in the 0.0.7 source, named as
  the updater names it (`before-update-0.0.7-to-0.0.8-<ms>.ajbackup`). The
  same scratch test applied the restore with `backup_verify_impl` +
  `backup_restore_impl`, the code behind Settings › Data: backup &
  retention › **Restore from a backup…**.

Log: [`rollback-rehearsal.txt`](rollback-rehearsal.txt).

## Steps and what happened

| # | Step | Observed |
|---|---|---|
| 1 | 0.0.7: two accounts, then the pre-update backup | backup schema 27; 0.0.7 starts |
| 2 | Update: this branch opens the same data | starts; the data is migrated to schema 28 |
| 3 | Go back: 0.0.7 on that data | **exits at once, code 1**; only stderr says "database schema version 28 is newer than supported version 27" |
| 4 | The documented way back: rename `juggler.sqlite` aside (`juggler.sqlite.from-0.0.8`), restore the pre-update backup | restore verified every file; 2 accounts back, schema 27, the replaced database kept beside it (`juggler.sqlite.before-restore-<ms>`); 0.0.7 starts |
| 5 | The new launch path: this branch on data from a newer build (schema set to 29) | **stays up with the alert** instead of quitting; stderr names both schemas; the 0.0.7 backup (schema 27) is the one it can offer |

## What is not covered by hand

- The alert's buttons. No desktop automation is available to this
  session, so "Use the backup from …" and "Quit" were not clicked. The
  go-back itself (move the newer database aside, restore, start on the
  restored data; put everything back if any step fails) is the tested
  code path: `cargo test -p juggler-app --test newer_data` (3 tests).
  **For the maintainer:** on a Dev build, run step 5 above and click
  "Use the backup from …" once; the app should open on the restored data
  with `juggler.sqlite.from-newer-<ms>` beside it.
- The rehearsal on a signed beta build (install 0.0.8 over 0.0.7 from the
  real feed, then reinstall 0.0.7 from its `.dmg`). Signing lives in CI;
  the steps are in `docs/RELEASING.md` § Rollback → Rehearse.

## The scratch helper (0.0.7 source, not committed)

```rust
// crates/juggler-app/tests/rehearsal.rs in a copy of v0.0.7:
// AIJ_REHEARSAL=seed|restore AIJ_DATA=<data dir> [AIJ_BACKUP=<file>]
let state = Arc::new(AppState::init_with_registry(dir.clone(), AdapterRegistry::new())?);
// seed: insert "Work Max" and "Personal", then
backup_export_impl(&state, &dir.join("backups/before-update-0.0.7-to-0.0.8-<ms>.ajbackup"), …);
// restore:
let plan = backup_verify_impl(&state, &backup)?;
backup_restore_impl(&state, &backup, &plan.token)?;
```
