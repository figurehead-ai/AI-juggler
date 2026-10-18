# Evidence manifest — AJ-E1205

Task: `docs/tasks/E12-optional-account.md` AJ-E1205 (Cloud backup upload,
schedule, restore). Built against the Figurehead API contract's backup
routes. 2026-10-18, reviewer claude-opus.

## How it works

Settings › Account & Cloud shows a **Cloud backup** panel once Cloud
backup is on and the recovery code is saved (AJ-E1204).

- **The panel** shows when the last backup ran (or why it failed, adding
  that nothing on this Mac changed), the schedule (every week by
  default, every day, or only when asked), **Back up now** and **Restore
  from Cloud…**.
- **An upload** builds an AJ-E1009 backup, seals it (ADR-0017) and sends
  `POST /api/v1/backups` as `application/octet-stream`, labelled
  `AI Juggler · <Mac name>`. It never sends more than the plan's
  `maxBytes` (25 MB when the plan names none) and runs with the long
  transfer timeout. The stored checksum is checked against what was
  sent. Only one upload runs at a time, never during a switch, and never
  before the recovery code is saved. Scratch files are removed; local
  data is never deleted.
- **A restore** lists `GET /api/v1/backups`, downloads
  `GET /api/v1/backups/{id}`, checks `X-Backup-Sha256`, and opens the
  backup with this Mac's key or the recovery code typed on a new Mac
  (which then keeps the key). The rebuilt `.ajbackup` goes through
  AJ-E1009's verification, and the plan opens in the same restore dialog
  as a local restore, so the user confirms it there.

**Fixed along the way:** a restore no longer carries the source Mac's
Figurehead link, cloud choices or other per-Mac settings onto the Mac
being restored (ADR-0016 amended). A new Mac restoring another Mac's
backup stays linked as itself.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Upload `POST /api/v1/backups` (octet-stream, optional `X-Backup-Label` ≤ 120 chars, ≤ 25 MB or `maxBytes`), on demand and on a schedule (daily/weekly, default weekly); one at a time; never during a switch | Req.1 | `cloud_backup :: round_trip…`: the server sees octet-stream, a long transfer, and an `FHB1` body with no plaintext. `the_schedule_runs_only_when_due`: daily is due, then not due, then off. The default is weekly. A second upload while one runs is refused; a switch in flight blocks (shared with AJ-E1010) | PASS | 2026-10-18 | claude-opus |
| Restore lists, downloads, verifies `X-Backup-Sha256`, decrypts (key or recovery code), verifies the manifest, and hands off to AJ-E1009's preview and confirm | Req.2 | Same test: on Mac B, without a key → "enter your recovery code"; with the code → plan (other Mac) → AJ-E1009 restore with matching counts; the key is kept. A download with one changed byte → "changed on the way". vitest: the plan goes to the shared restore dialog | PASS | 2026-10-18 | claude-opus |
| With Cloud backup off, no `/backups` call (guardrail) | Req.3 | `off_or_unsaved_code_means_no_backups_request_at_all`: upload, list, restore and tick with the toggle off → **0** `/backups` requests. With the toggle on but the code unsaved → still 0 | PASS | 2026-10-18 | claude-opus |
| `413` and `402` stop uploads with a named reason; local data never deleted | Req.4 | `too_large_and_plan_refusals…`: 413 → "larger than your plan allows"; 402 → "plan doesn't include"; the failure is shown in the status. A 64-byte plan limit → refused before sending (no request). The database is not shrunk | PASS | 2026-10-18 | claude-opus |
| Upload off the UI thread; restore verification < 30 s for 25 MB | perf | Commands run on blocking workers. Restore preparation in the round trip is well inside 30 s (asserted). AJ-E1009 verify is ≈ 8 s/GB, and AJ-E1204 opening ≈ 560 MB/s | PASS | 2026-10-18 | claude-opus |
| Mock-server tests for upload/list/download/sha mismatch; toggle guardrail | quality gate | `cloud_backup` 4/4 against a stateful fake of the contract (it stores blobs and answers with their checksums). Log: `fake-server.log` | PASS | 2026-10-18 | claude-opus |
| Visual: backup list, upload in progress, restore preview at 1280×800 both themes | visual / a11y | Gallery `60-cloud-backup` (panel, uploading, list with the recovery-code field, restore preview). Playwright `cloud backup panel and list — axe 0` (both themes). vitest `cloud-backup.test.tsx` 4/4 | PASS | 2026-10-18 | claude-opus |
| Two-Mac round trip with the recovery code against the website's local dev server | evidence | The fake server follows the published contract. A run against the real service's local dev server is for the maintainer (`docs/qa/dogfood-backlog.md`) | DEFERRED (maintainer) | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test cloud_backup` 4/4; `--test backup_restore` 5/5 (with the per-Mac settings fix).
- vitest `cloud-backup.test.tsx` 4/4. Playwright `60 cloud backup` 4, `cloud backup panel and list — axe 0` 2.
