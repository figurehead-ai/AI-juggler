# Evidence manifest — AJ-E1406

Task: `docs/tasks/E14-machine-health.md` AJ-E1406 (footprints per launch
setting, learned from history). Status: **DONE-EVIDENCED**.

## What exists now

- Storage **v29** `footprints` (after main's v28, accounts `in_place`) (`crates/juggler-core/src/storage/footprints.rs`).
  There is one row per session, keyed `recipe:<launch setting>` or
  `observed:desktop` / `observed:terminal`. A row holds the peak tree
  memory, the p95 tree CPU, the peak helper count, the duration and the
  Provider/Account (Account only for sessions AI Juggler started). It is
  updated in place, and peaks never go down. The newest 50 per key are kept.
  Medians come from the real rows.
- `machine_health::track_footprints` folds every live tree into its running
  peaks on the 2 s tick: the CPU samples of the last hour for the p95, and
  the max memory and helpers. It writes a footprint when the tree ends and
  every 10 min (300 ticks) while it runs, so a crash still leaves one.
  Observed trees are keyed by place only, never a command line or folder.
  Their id carries when they were first seen, so a reused pid never merges
  two trees.
- Local only: footprints are not in usage stats or in any shared part of a
  backup.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 written at end and while running | `cargo test -p juggler-app --test machine_fixtures` (three real memory-hog sessions each leave one footprint with peak > 220 MB, helpers ≥ 1, Account set; a running hog has one within 300 ticks) | pass | 2 passed | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Req.2 table, medians, retention | `cargo test -p juggler-core --lib footprints` (one row per session, peaks never drop; medians per key incl. even counts; newest 50 kept, other keys untouched); migration tests bumped to v29 | pass | 3 passed; core suite green | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Req.3 observed by place only | code review of `track_footprints` (key `observed:<place>`, no Account, no command line) | — | as stated | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| p95 rule | `cargo test -p juggler-app --lib -- machine_health` (`p95_is_the_nearest_rank`) | pass | pass | PASS | 49722aad+ | 2026-10-20 | claude-opus |
| Retention (review) | `cargo test -p juggler-core --test storage_roundtrip footprints` (`footprints_skip_short_and_failed_sessions_and_go_with_their_setting`): a failed and a 20 s session don't count toward the median; a deleted launch setting's footprints are forgotten. `cargo test -p juggler-app --lib -- machine_health` (`recommendations_come_from_enough_real_sessions`: a deleted setting doesn't count) | pass | pass | PASS | 4a03078c | 2026-10-20 | claude-opus |
| Downgrade | An older build refuses a v29 database with `SchemaTooNew`; the CHANGELOG carries the downgrade note | as described | as described | PASS | 4a03078c | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
