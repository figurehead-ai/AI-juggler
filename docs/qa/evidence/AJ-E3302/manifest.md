# AJ-E3302 — Account and App validated

2026-11-07, release/0.4.1. The four layers of the validation plan (§3, §5) for Accounts and Apps.

| Req | Result | Evidence |
|---|---|---|
| R1 (L1) | PASS | Grouping, exhaustive: `crates/juggler-app/tests/accounts_l1.rs` runs every case of three folders × tool (Claude Code, Codex) × sign-in (as A, A in a team, B, unreadable, not signed in) × plan (none, Pro, Max), 27 000, through `subscriptions::group` and checks each against the reference model `scripts/qa/account_oracle.py`, written from D-62 and D-35 (its own 6 hand-checked cases run in `just qa-kit`). 27 000 of 27 000 agree; a rule broken in the model on purpose (a team seat merged into the person's own Account) is caught. Migration: `crates/juggler-core/tests/entities_storage.rs`, `terms_storage.rs`, `pre_migration_copy.rs`; walk A9 opened a 0.3 data folder (v39) with every row kept |
| R1 (L2) | PASS | On a replica of 9 setups: 7 Claude folders give 6 Accounts, the two signed in as one person one Account with two Apps (`docs/qa/evidence/AJ-E3202/l2-replica-2026-11-06.md`); usage equal to the transcripts' own, 18 of 18 windows and 340 of 340 sessions (`docs/qa/evidence/AJ-E3205/usage-oracle-2026-11-06.md`) |
| R1 (L3) | PASS | New session: 18 of 18 pairwise rows (`docs/qa/evidence/l3/new-session.json`); the App and Account pages: 16 of 16 rows of the pages table (`docs/qa/evidence/l3/pages.json`) |
| R1 (L4) | PASS | Walks A1–A10 and N1–N5 in the Dev edition through the lab write mode, both themes at 1024 and 1280, captures opened by eye: 23 steps, none under 4 on works, speed, clarity or layout (`docs/qa/0.4-walks/A*.md`, `N*.md`) |
| R2 | PASS | Every finding on those sheets is fixed in a named commit and its scenario walked again from the start, the same day (each sheet's "Findings" section) |

Not here: the persona walks (AJ-E3307) and the release candidate walked whole (AJ-E3306), which walk these scenarios again on the release build.
