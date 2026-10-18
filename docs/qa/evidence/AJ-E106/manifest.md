# Evidence manifest — AJ-E106 (schema v5)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1-2 v5 pure RENAME migration | `cargo test -p juggler-core --test storage_roundtrip` (`v4_database_upgrades_in_place_to_v5…`) | v4→v5 once on open; values identical | PASS: real v4 file DB (old names, real values) auto-upgraded; user_version=5; re-open stable | PASS | 2026-10-13 | zcode-primary |
| Req.3 values byte-identical | same test asserts account/recipe/record fields incl. conversation + kind + usage JSON | equal | PASS | PASS | 2026-10-13 | zcode-primary |
| Req.4 repos use new names only | grep old names in repos | 0 old identifiers | provider_id / instances.conversation_id / launch_recipe_id / launch_recipes only | PASS | 2026-10-13 | zcode-primary |
| Req.5 downgrade rejected | existing `reopen_is_idempotent_and_future_schema_rejected` (still green) | SchemaTooNew | PASS | PASS | 2026-10-13 | zcode-primary |
| Full gates | just verify | green | 84 tests green | PASS | 2026-10-13 | zcode-primary |
