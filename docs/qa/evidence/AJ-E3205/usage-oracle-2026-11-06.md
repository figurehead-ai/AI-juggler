# The usage oracle on the replica (AJ-E3205)

2026-11-06, release/0.4.1 at 8a8e19d75. A fresh replica (`scripts/replica.py build`, seed 0: 7 Claude Code folders, 2 Codex homes, 16 projects, 340 conversations), the backend harness `crates/juggler-app/tests/qa_real_readonly.rs` run against it (`env -i HOME=<replica> AIJ_NO_KEYCHAIN=1`, a fresh data folder), then `scripts/qa/real_truth.py --compare` with every replica folder named (`--claude-dir` for the six beside `~/.claude`, `--codex-dir` for the second Codex home). The oracle reads the transcripts itself, deduplicated by message id, independent of the app's readers. The harness output stays out of the repository; the compare's lines are pseudonymised.

| Check | Result |
|---|---|
| Setups found and brought in where they are | 9 of 9 |
| Projects, and each project's conversations and accounts | 16 of 16 rows equal |
| Conversations per account | 9 of 9 equal |
| Tokens per account, last 5 hours and last 7 days | 18 of 18 equal (×1.00), e.g. 1,372,661 = 1,372,661 |
| Cost rows per session (the Usage tab's rows) | 340 of 340 within 5%; min, median and max ×1.00 |
| Codex all time | 11,375,412 tokens in 42 sessions, both (×1.00) |
| Cost rows priced | 355 of 397: the 42 others are `gpt-5-codex`, which the bundled price table doesn't list (it has `gpt-5` and `gpt-5.3-codex`). They show "—", the honest gap, with their tokens right. Finding for the usage area: add a sourced price for `gpt-5-codex` |
| This Mac's Claude app windows | Not a replica check: the harness reads the whole Mac's process table, so the host's own Claude app is counted (as in the A6 note) |

The first compare named `~/.claude` and `~/.codex` again beside the defaults, so the oracle counted them twice (Codex ×0.65, some sessions ×0.50). Named once, everything above matched. The Account page sums its folders' numbers (walk A2), so equal per folder means equal per Account.
