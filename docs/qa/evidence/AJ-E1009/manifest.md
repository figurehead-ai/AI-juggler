# Evidence manifest — AJ-E1009

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1009 (backup,
restore and machine migration of AI Juggler's own data). The format is
ADR-0016. 2026-10-18, reviewer claude-opus.

## How it works

Settings › **Backup & restore**.

**Back up now…** writes one `.ajbackup` file (gzip tar) to a place the
user picks. It reports progress at least every 1 %, and ends by saying
what it wrote and what it withheld. Nothing is uploaded.

What goes in is an allowlist; nothing else in the data folder is ever read:
- `manifest.json`, first;
- a scrubbed database snapshot;
- `heuristics.json`;
- the parked packs.

The snapshot is scrubbed before anything is hashed:
- Launch settings keep their environment variable names, but every value
  is blanked and listed as withheld.
- The process ledger, Fleet sync state, confirmation tokens and the
  remembered link port are removed. They only mean something on the Mac
  that wrote them, and another Mac's pids could mislead cleanup.
- The search index is dropped and rebuilt after a restore.
- A final VACUUM leaves no deleted row in a free page.

The redaction engine (D-14) then scans every entry. If it finds anything,
the export stops, names where it was found with the value masked, and
writes nothing.

**Restore from a backup…** first reads the whole archive into a staging
folder:
- Every checksum must match, nothing may be missing or extra, and every
  entry path must be plain and allowlisted (`..` and absolute paths are
  refused).
- Format 1 and a schema no newer than this build's are required.
- Each refusal names the file and the reason.

The plan dialog then lists the counts and where and when the backup was
made. When the backup comes from another Mac, it says first that it
carries that Mac's data, never its identity.

**Restore** needs the plan's one-time token, and no Instance may be live.
- The current database, `heuristics.json` and parked packs are kept
  beside the restored ones (`*.before-restore-<time>`).
- The staged database is migrated forward and copied in with SQLite's
  backup API.
- This Mac's own process ledger, Fleet sync rows and machine-only
  settings are put back.
- Counts are checked against the manifest, and Instances that were live
  on the writing Mac are recorded as stopped.
- The `fleet/` folder is never touched.

The result says to restart, to sign in to each Account again, and to
re-enter the withheld values.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| One archive (DB snapshot, packs, settings, lineage) with a manifest and checksums; credentials, Provider login material and `fleet/` excluded by construction, verified by the redaction check and a test | Req.1 | `backup_restore :: rehearsal…`: the archive holds exactly the manifest, the DB, `heuristics.json` and `parked/…`, with no `fleet/` or `accounts/` entry. A seeded API key (in a Launch setting's env and in a process command line), seeded sign-in material and the Fleet key bytes appear in no entry; the redaction engine flags nothing in any entry; the manifest lists the withheld env name. `something_that_looks_like_a_secret_stops_the_export`: a key in an unexpected place (a project name) stops the export, masked, with no file written | PASS | 2026-10-18 | claude-opus |
| Restore verifies checksums and schema, refuses incompatible versions with a named reason, and restores into the managed directory without touching vendor installs | Req.2 | `damaged_newer_and_unsafe…`: a flipped byte → "…heuristics.json does not match its checksum", naming the archive; schema 9999 → "made by a newer AI Juggler"; a `..` entry → refused, nothing written outside staging; a live Instance → "stop every running Instance first"; never overwrites an existing file. The restore only writes the database, `heuristics.json` and `parked/` in AI Juggler's own data folder; Account folders and `fleet/` are not touched | PASS | 2026-10-18 | claude-opus |
| A restore rehearsal runs in CI on a fixture archive: restore, launch the read model, assert counts and lineage match the manifest | Req.3 | `the_committed_v1_fixture_archive_restores` restores the committed `tests/fixtures/backup-v1.ajbackup` (origin "Fixture Mac"; scanned for host names, user paths and keys: none) and checks counts equal the manifest and the lineage edge is intact. The rehearsal test does the same Mac A → Mac B. Both run in `just verify`. Log: `rehearsal.log`; fixture manifest: `fixture-manifest.json` | PASS | 2026-10-18 | claude-opus |
| A backup restored on another Mac warns it carries that Mac's data, never its identity; the restored Mac keeps its own key and pairs again | Req.4 | Rehearsal: `other_machine = true`; the first note says "never its identity"; Mac B's Fleet fingerprint is the same before and after the restore; A's process ids do not land on B (ledger 0). vitest and gallery: the warning is shown above the notes in the plan dialog | PASS | 2026-10-18 | claude-opus |
| Export < 60 s for a 1 GB store (streaming, progress shown); verify < 30 s; no upload anywhere | perf | 64 MB of pack text: export **1.5–1.7 s (≈24–26 s per GB)**, verify **0.5 s (≈8 s per GB)**. These are debug-build figures, so the release build is faster. Progress only moves forward and ends at 100 %; the UI is updated at least every 1 %. The only output is the local file | PASS | 2026-10-18 | claude-opus |
| Fixture restore test green in `just verify`; credential-pattern scan of a real export structure passes | quality gate | As above (5/5 in `backup_restore`) | PASS | 2026-10-18 | claude-opus |
| Visual: export progress, restore verify result and the machine-identity warning at 1280×800, both themes; outcome text explicit; failures name the file and reason | visual / a11y | Gallery `57-backup` (progress, done, plan, refused × 2 themes). Playwright `backup and restore plan — axe 0`: the plan opens with Restore focused and Escape closes it without restoring. vitest `backup-restore.test.tsx` 4/4 | PASS | 2026-10-18 | claude-opus |
| Dogfood migration of one real (consented, redacted) store | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test backup_restore`: 5/5.
- vitest `backup-restore.test.tsx` 4/4. Playwright `57 backup` 8,
  `backup and restore plan — axe 0` 2.
