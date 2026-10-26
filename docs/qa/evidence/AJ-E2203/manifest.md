# Evidence manifest — AJ-E2203

The usage history's summaries: what they sum and their time budget on a
synthetic store of 989,707 messages over a year on seven profiles (no
real data). Debug builds, SQLite at -O3 as the workspace builds it; the
best of three runs (the Mac is shared).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 sums | `cargo test -p juggler-core --test usage_history` | hourly, per-session, per-model sums and readings exact | 2 passed | PASS | f6b87d8c | 2026-10-25 | agent |
| Req.1 folding | `cargo test -p juggler-app --lib usage_history` | series, shares, top sessions, buckets, Other fold, checks | 23 passed | PASS | f6b87d8c | 2026-10-25 | agent |
| Budget, per read | `cargo test -p juggler-core --test usage_history -- --include-ignored --nocapture` | each SQL read under 50 ms up to 90 days | 24 h 0.2–1.5 ms, 7 days 1.2–1.7 ms, 30 days 4–6 ms, 90 days 20–28 ms a profile; a session's turns 20.6 ms; by model 20.0 ms; a window's share 0.7 ms | PASS | f6b87d8c | 2026-10-25 | agent |
| Budget, a page | `cargo test -p juggler-app --lib settled -- --ignored --nocapture` | a page's 14 reads under 50 ms | first read of a year, 7 profiles: 586 ms once; then 24 h 0.5 ms, 7 days 0.5 ms, 30 days 1.1 ms, 90 days 1.6 ms, all 2.9 ms | PASS | f6b87d8c | 2026-10-25 | agent |
| Req.2 no migration | `git diff a8119e03 -- crates/juggler-core/src/storage/mod.rs` | no new numbered migration | derived table and triggers made at open, like `usage_spans` | PASS | f6b87d8c | 2026-10-25 | agent |

## Binary artifacts

None: the budget runs print their timings.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
