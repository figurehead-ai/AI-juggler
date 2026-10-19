# Evidence manifest — AJ-E1015

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1015 (the screen
says "session", D-25). Status: **DONE-EVIDENCED**. 2026-10-19, reviewer
claude-opus.

## What exists now

- About 80 interface strings and visible attributes, and 33 backend
  messages that reach the screen (cleanup states, update and restore
  refusals, Fleet replies, parking, history, worktrees, recovery), say
  "session" where they said "Instance". The inspector's kind label reads
  "Session" for an Instance and "Conversation" for a Session record.
- Code, SQL, metric names, the schema and design docs keep their names.
- `scripts/terminology_lint.sh` refuses "Instance" in JSX text and in
  `aria-label`, `title`, `placeholder`, `label` and `alt`.
- README's tagline and Inspector line, and the guide ("When a Provider is
  down"), follow. The 0.0.4 release notes are history and stay as written.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 strings and messages | vitest 461/461 and the Rust workspace tests (`--no-fail-fast`) pass with the expectations updated to the new words; the full Playwright suite 907/907 | PASS |
| Req.2 names kept | the sweep touched only string literals and JSX text (reviewed line by line); `cargo clippy -D warnings` and `tsc` clean | PASS |
| Req.3 the lint | passes on the tree; a seeded file with `<p>One Instance</p>` and `placeholder="Go to instance"` fails with both lines named | PASS |
| Req.4 docs | README and `docs/guide/` diffs | PASS |
