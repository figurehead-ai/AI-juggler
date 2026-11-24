# Evidence manifest — AJ-E2022

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2022 — remote
configuration, steps 2–3: the compat section, the app's wiring and
the Publish config workflow (P0, ADR-0035, D-128 pt 3, D-129).
Audited 2026-11-23 by zcode-primary against HEAD.

| Claim | Requirement | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A `compat` section carries per-tool allowed ranges, shape-check versions and kill switches; only switches compiled-in features and narrows/widens within compiled hard limits | R1 | `juggler-adapter/src/compat.rs`: layer-0 compiled declaration per adapter (`compat_features`), `<key>_versions` ranges clamped to compiled hard limits, kill switches only OFF for compiled features; the section is in the exported schema (`config/remote/schema.json` name "compat") | PASS | 2026-11-23 | zcode-primary |
| The switch and move paths read the section through the registry, falling back to the compiled list on no file | R2 | `service.rs` registers `compat::section(&adapters.compat_features())` and installs `Gate::from_effective(&snapshot.effective)` per pass; `handover/moving.rs` and `service_accounts` consult the Gate; on no file the service runs compiled defaults (the fault test class) | PASS | 2026-11-23 | zcode-primary |
| A publish workflow validates in CI, signs with the environment key (never the repo), publishes to the fixed URL per channel, through release-grade review | R3 | `.github/workflows/publish-config.yml`: `publish` (manual, `config-publish` environment = required reviewer, main only) validates the source against the exported schema, canonicalises, serial = live+1 (signature checked), signs with `REMOTE_CONFIG_SIGNING_KEY` (environment secret), verifies as the app does, uploads to R2, smoke-checks the URL, re-verifies; weekly `resign` + daily `expiry` guards; CodeQL fixes landed (c2e337f16: no key ever printed) | PASS | 2026-11-23 | zcode-primary |
| An option hidden by the file records which tool and version, in plain words in the option's place (D-127) | R4 | `moving.rs`'s option rows carry `words` (plain words, no path) for why an option is unavailable; the Gate's hidden-option path feeds it. PARTIAL: the end-to-end visual (a switch menu route hidden by a live file) belongs to the task's visual spec and awaits a published live file to screenshot against | PARTIAL | 2026-11-23 | zcode-primary |

## Test counts at audit

`cargo test -p juggler-remote-config`: 101 passed (sections/lists
suites cover the compat section's tolerance, targeting and
publisher strictness).

## Note

The task's visual item ("the switch menu with a route hidden by the
file") and the three-version dry-run matrix both await the same
thing: the channel's first real published history. Everything
machine-checkable without a live file is green above.
