# Evidence manifest — AJ-E104 (Phase B2 Instance/Session split)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Rename + alias | cargo test --workspace | green | InstanceRecord + SessionRecord alias; 83 tests green | PASS | 2026-10-13 | zcode-primary |
| ConversationId typing | unit + roundtrip suites | native_id typed | conversation: Option<ConversationId>; storage rows unchanged | PASS | 2026-10-13 | zcode-primary |
| Req.4 v4 fixture roundtrip | `cargo test -p juggler-core --test storage_roundtrip v4_fixture` | values equal, user_version 4 unchanged | PASS (raw-SQL v4 row → InstanceRecord → update → re-read equal) | PASS | 2026-10-13 | zcode-primary |
| Req.5 view-model exposes conversation | ts-rs export; UI mocks | conversation_id in SessionInfo | generated + mocks + gallery green | PASS | 2026-10-13 | zcode-primary |
| Visual (rail + panel) | pnpm visual:capture | 41 cells, both themes | re-captured green post-change | PASS | 2026-10-13 | zcode-primary |
| Live dogfood note | deferred to next app run | — | dogfood of the split rides the next E-milestone dogfood (AJ-E809 programme); UI renders conversation in view-model now | DEFERRED → AJ-E809 | 2026-10-13 | zcode-primary |
