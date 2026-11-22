# Evidence manifest — AJ-E1615

Task: `docs/tasks/E16-more-tools.md` AJ-E1615, the sign-in surface (P1). Found by the AJ-E1612 sitting-2 dry runs ([dry-run.md](../AJ-E1612/dry-run.md), [dry-run-2.md](../AJ-E1612/dry-run-2.md)).
Status: **IN-PROGRESS.** R1 to R3 pass, and R4's role-query tests pass; the a11y gate has not been run on the Sign in terminal page (see R4). 2026-11-18, claude-sonnet; dry run 2 (949d8e231) saw the fixes work for four tools.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1: each sign-in row is one group or list item named with its App's tool; its Sign in button sits inside it with a name unique per App | `cd ui && ./node_modules/.bin/vitest run src/components/home-needs-names.test.tsx` | distinct names, each button inside its own row | 2 files, 5 tests pass (with R2's file); the test gives two and three same-labelled Apps distinct buttons, each within its own row; dry run 2 pressed by row name for OpenCode, Kimi Code and Grok Build | PASS | 11f3df7b3 | 2026-11-18 | claude-sonnet |
| R2: the Sign in terminal's footer names the App's own provider | `vitest run src/screens/sign-in-footer.test.tsx` | a Kimi Code sign-in never names Anthropic or Claude | passes; the footer reads the tool name from the tools catalog | PASS | 67266d82f | 2026-11-18 | claude-sonnet |
| R3: a Sign in terminal's exit re-reads that App's sign-in at once; Home and the rail update | `crates/juggler-app/tests/sign_in_ended.rs` (a Kimi Code need stays on a plain tick, goes on `sign_in_ended`); `rail-needs-follow.test.tsx` and `centre/tests.rs` for the notices' version kept below 2^53 | need gone at once, rail and bell follow | the Rust test and the UI test are in the commits; dry run 2 showed the sign-in re-read after the terminal ended. The Rust test was not re-run on this tree for this manifest | PASS (commit and dry run 2; not re-run today) | b1f51a136, 045c97acb | 2026-11-18 | claude-sonnet |
| R4: role-query tests for R1 to R3 pass | the vitest run above (5 tests) | pass | pass | PASS | 67266d82f, 11f3df7b3 | 2026-11-18 | claude-sonnet |
| R4: the a11y gate on the touched screens | 11f3df7b3's message: axe on Home and the rail, 16 passed. The Sign in terminal page (WorkPanels.tsx's footer) has no axe run of its own | axe clean on every touched screen | Home and the rail: clean; the terminal page: not run | UNPROVEN | 11f3df7b3 | 2026-11-18 | claude-sonnet |

## Redaction checklist

- [x] No credentials, tokens or keys
- [x] No transcripts or real names (the lab's "lab A" Apps only)
