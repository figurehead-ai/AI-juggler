# ADR-0016: Backup archive format (v1)

**Status:** Accepted
**Date:** 2026-10-18
**Deciders:** implementation agent (AJ-E1009); maintainer review on the PR

## Context

AJ-E1009 lets a user carry AI Juggler's own data to a new Mac, or keep a
copy against a corrupt store, as one file they can restore with proof
that it matches. The task's handover note asks for the format to be
decided openly, not invented in passing, because a backup format
outlives the build that wrote it.

Constraints:

- **Never credentials.** No vendor login material, no API keys, no
  Fleet identity keys or peer pins (AJ-E701). This must hold by
  construction, not by hoping a scan catches everything.
- **No identity transfer.** A backup restored on another Mac carries
  that Mac's data, never its identity. The restored Mac keeps its own
  key and pairs again.
- **Verifiable.** A restore must prove every file matches before it
  changes anything, and must refuse a version it cannot read, with the
  reason.
- **Local only.** A file the user holds. Nothing uploads.

## Decision

A backup is one **gzip-compressed tar** file, `*.ajbackup`, laid out as:

```text
manifest.json             always the first entry
db/juggler.sqlite         a consistent snapshot (VACUUM INTO), scrubbed
settings/heuristics.json  when present
parked/…                  staged packs of parked Instances, when present
```

`manifest.json` holds:

| Field | Meaning |
|---|---|
| `format` | always `"ai-juggler-backup"` |
| `format_version` | `1` |
| `schema_version` | the database schema the snapshot is at |
| `app_version` | the app version that wrote it |
| `created_at_ms` | when the backup was made |
| `origin` | the writing Mac's Fleet fingerprint and host name (to recognise, not to impersonate) |
| `counts` | Accounts, Launch settings, projects, Instances, history events, switch edges and epochs |
| `files` | path, size and sha256 of every other entry |
| `withheld` | what was left out on purpose and must be re-supplied |
| `excluded` | what is never in a backup, and why |

**Allowlist, not denylist.** Only the paths above are written. Account
folders (vendor sign-ins), the `fleet/` folder (identity keys and pins),
supervision journals and everything else in the data directory are
never read by the exporter.

**The snapshot is scrubbed** before it is hashed:

- Launch settings keep their environment variable names, but every
  value is blanked and listed under `withheld`, because values are often
  keys.
- Rows that only mean something on the writing Mac are removed: the
  process ledger (its pids belong to that Mac, and restoring them could
  point cleanup at unrelated processes), Fleet sync state, one-time
  confirmation tokens, and the remembered link port.
- The transcript search index is dropped and rebuilt from the
  transcripts on the restored Mac.

The redaction engine (D-14) then scans every entry. A backup in which it
detects anything is not written, and the user is told where it was
found, with the value masked.

**Restore** has two steps:

1. **Verify.** The archive is read in full into a staging folder. Every
   file's sha256 must match the manifest, no file may be missing or
   extra, `format_version` must be 1, and `schema_version` must not be
   newer than this build's. The result lists the counts, what was
   withheld, and whether the backup came from another Mac.
2. **Restore.** This needs the verify step's one-time token, and no
   Instance may be live. The current database and settings are kept
   beside the restored ones (`*.before-restore-<time>`), so a restore
   can be undone by hand. The staged database is opened, migrated
   forward if it is older, and copied into place with SQLite's backup
   API. Instances that were live on the writing Mac are recorded as
   stopped. The Fleet folder is never touched, so this Mac keeps its
   identity.

## Alternatives considered

- **A directory or zip of the raw data folder.** This is a denylist: one
  new file type in the data folder and a secret travels. Rejected.
- **A custom binary container.** It needs its own reader forever and
  cannot be inspected with standard tools. Rejected.
- **A SQL dump instead of a database snapshot.** Restore would be slower
  and less faithful (FTS tables, types), for no gain over a scrubbed
  snapshot. Rejected.
- **Encrypting the archive.** It is a local file the user controls,
  and it holds no credentials by construction. Encryption belongs to the
  optional Cloud backup (D-18), which encrypts on top of this format.
  Deferred to that feature.

## Consequences

- A backup can be opened with `tar -xzf` and read by a person; the
  manifest says exactly what is in it and what is not.
- After restoring on a new Mac the user signs in to each Account again
  and re-enters withheld environment values. The restore result says so.
- A newer build must keep reading `format_version` 1, or refuse it with
  a reason. Any change to the layout or manifest bumps
  `format_version` and amends this ADR.
