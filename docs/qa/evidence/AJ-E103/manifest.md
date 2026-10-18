# Evidence manifest — AJ-E103 (Phase B1 domain aliases)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| ProviderId alias additive | cargo test --workspace | all green, wire unchanged | 82 tests green; no serialization change (pure type alias) | PASS | 2026-10-13 | zcode-primary |
| ProviderAdapter alias | cargo check -p juggler-adapter | compiles | green | PASS | 2026-10-13 | zcode-primary |
