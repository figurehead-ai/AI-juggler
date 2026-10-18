# AJ-E307 WIP plan (started 2026-10-17, branch stage3/e3-e307-import-provenance)

Recon findings (verified this branch):
- `Account` (model.rs:184) has NO provenance fields — add
  `import_source_path: Option<String>` (canonical), `import_origin:
  Option<String>` ("cc-switch"|"t3"|"alias"|"manual"|"home-pattern"),
  `imported_at: Option<i64>`; serde defaults keep old rows loading.
- Migrations: forward-only `MIGRATIONS: &[&str]` V1..V20 in
  storage/mod.rs:370 — add V21 (ALTER TABLE accounts ADD the three
  columns; NULL = provenance-unknown, never guessed — Req.1's
  backfill IS the migration, no guessing pass).
- `import_account` adapter default = Unsupported (adapter.rs:133);
  the real impl lives in the claude adapter — extend for: bounded
  per-folder size walk (≤2s/50k files, cached from scan), the
  settings-only degraded mode (skip projects/transcripts), copy
  progress events (≥4/s, cancel ≤500ms), and import-session-scoped
  cleanup (Req.6 trap: marker-first writes look managed — cleanup
  keys off the session id, not the marker).
- The 2 GiB ceiling does NOT exist yet — new constant + the degraded
  offer ([Import settings only — skip history] / [Add fresh]); the
  raw "2 GiB" string NEVER reaches the UI ("history is large").
- Duplicate protection (Req.2): account_import command checks the
  new provenance field against the source path → coded error
  `already_imported` carrying the existing label; the wizard's
  Detect renders "Already imported as «Work»" + [Open it] (E306's
  Req.5 note) — replaces/augments the E303 managed-marker card.
- Vanished source (Req.5): NotFound from import → "That setup moved
  — [Rescan]" back to Detect; config-evidence failure → plain reason
  + [Pick a different folder] (manual-folder entry point on Detect:
  [Import from a specific folder…] via native picker — metadata-only
  evidence check before accepting).
- Keychain note (Req.8) already exists on ImportResult.login_note —
  surface it at Confirm-source time (the wizard identity step shows
  it pre-copy when the candidate is keychain_bound).
- Confirm-source step (Req.3): the wizard identity step gains the
  source panel: path, origin badge, total size, per-folder
  breakdown (real list markup), keychain note; oversize → the
  degraded pair; the copy button then runs the (possibly degraded)
  import with progress + per-folder breakdown.
- "Just track it" (Req.7): NOT offered — no adopt-in-place anywhere.
- Tests: onboarding.rs (source-immutability tree-hash snapshot
  before/after; duplicate-import blocked; oversize fixture →
  degraded import has settings, ZERO transcript files; injected copy
  error → cleanup + [Remove]/[Retry]); import_t3_profiles.rs gains
  provenance + t3 origin cases; fixtures corpus under
  crates/juggler-app/tests/fixtures/.
- Visual: gallery 39-import — Confirm-source (normal/oversize/
  non-config/vanished), degraded confirm, duplicate-blocked —
  1280×800 + 390×844 × light+dark; a11y: breakdown real list,
  role="alert" on errors, axe 0.
- Dogfood: import the maintainer's real cc-switch set → size/
  provenance table (basenames only) in docs/qa/e3/AJ-E307/.

Order: (1) V21 migration + Account fields + storage round-trip;
(2) import_account extensions (walk/ceiling/degraded/session
cleanup) + ImportOutcome fields; (3) account_import command
(duplicate check + coded errors + provenance write); (4) wizard
Confirm-source UI + failure states + manual-folder entry; (5) tests;
(6) gallery + a11y; (7) gates → evidence → flip → PR.
