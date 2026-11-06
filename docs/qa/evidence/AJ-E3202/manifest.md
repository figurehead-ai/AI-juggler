# AJ-E3202 — Accounts and Apps stored

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | v40/v41 tables (`subscriptions`, `apps`, `instances.app_id`): `crates/juggler-core/tests/entities_storage.rs`, `terms_storage.rs`; walk A9 opened a 0.3 data folder (v39) as v48 with every row kept (`docs/qa/0.4-walks/A9.md`) |
| R2 | PASS | One Account per sign-in: walk A2 (two folders and a desktop profile on one Account), A9 (Work A and B on one); unreadable: walk A10, "Account unknown — Claude Code's state file couldn't be read" in Accounts, Home and Usage, left out of Personal's count (`A10.md`, captures `walks-0.4/A10/`) |
| R3 | PASS | Default name: masked email (A2, A10), plan once known ("Claude Max 20x", A9); renamed Work → Acme, shown in the picker, ⌘K, notices and Runway (walk A8) |
| R4 | PASS | Walk A9: three questions (plan, renewal, fee), one choice each, the Account then holds them (`A9.md`) |
| R5 | PASS | `crates/juggler-core/tests/pre_migration_copy.rs` (a `.before-v<N>` copy before each move); A9: each folder kept its own plan, renewal and fee after the choice (`walks-0.4/A9/walk.json`) |
| L2 | GAP | The L2 harness on the replica (several Apps per Account) isn't run yet; the lab walks are L4 |
